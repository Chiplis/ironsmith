//! "If <event> would happen, <effects> instead." (CR 614.1a)
//!
//! The replacement program runs in place of the replaced event through the
//! shared instead-payload owner (`ReplacementAction::Instead`), which executes
//! it in a child context whose triggering event is the replaced event ("that
//! much" / "that many" read its amount), whose iterated player is the affected
//! player ("that player"), and whose replacement history suppresses this
//! replacement for the events its own program produces (CR 614.5).

use crate::effect::Effect;
use crate::events::DamageTarget;
use crate::events::context::EventContext;
use crate::events::damage::DamageEvent;
use crate::events::life::LifeGainEvent;
use crate::events::life::matchers::WouldLoseLifeMatcher;
use crate::events::permanents::matchers::WouldBeDestroyedMatcher;
use crate::events::zones::matchers::WouldChangeZoneMatcher;
use crate::events::traits::{EventKind, GameEventType, ReplacementMatcher, downcast_event};
use crate::filter::{ObjectFilterExt as _, PlayerFilterExt as _};
use crate::ids::{ObjectId, PlayerId};
use crate::replacement::{ReplacementAction, ReplacementEffect};
use crate::static_abilities::{StaticAbilityId, StaticAbilityKind};
use crate::target::ObjectFilter;
use ironsmith_core::ReplacedEventSpec;

/// A generic "instead" replacement over one watched event.
#[derive(Debug, Clone, PartialEq)]
pub struct EventReplacementWithEffects {
    pub event: ReplacedEventSpec,
    pub replacement_effects: Vec<Effect>,
    pub display: String,
    /// "you may ... instead": the affected player may decline it, and then
    /// the event happens unchanged (CR 616.1).
    pub optional: bool,
}

impl EventReplacementWithEffects {
    pub fn new(
        event: ReplacedEventSpec,
        replacement_effects: Vec<Effect>,
        display: impl Into<String>,
    ) -> Self {
        Self {
            event,
            replacement_effects,
            display: display.into(),
            optional: false,
        }
    }

    pub fn with_optional(mut self, optional: bool) -> Self {
        self.optional = optional;
        self
    }
}

impl StaticAbilityKind for EventReplacementWithEffects {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::EventReplacementWithEffects
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        let action = ReplacementAction::Instead(self.replacement_effects.clone());
        let replacement = match &self.event {
            ReplacedEventSpec::DamageToPlayer { .. }
            | ReplacedEventSpec::DamageToObject { .. }
            | ReplacedEventSpec::LifeGain { .. }
            | ReplacedEventSpec::DrawInstruction { .. }
            | ReplacedEventSpec::Untap { .. } => ReplacementEffect::with_matcher(
                source,
                controller,
                ReplacedEventMatcher {
                    event: self.event.clone(),
                },
                action,
            ),
            ReplacedEventSpec::LifeLoss { player } => ReplacementEffect::with_matcher(
                source,
                controller,
                WouldLoseLifeMatcher::new(player.clone()),
                action,
            ),
            // The destruction owner binds the permanent as "it" and as the
            // program's target.
            ReplacedEventSpec::Destroy { target } => ReplacementEffect::with_matcher(
                source,
                controller,
                WouldBeDestroyedMatcher::new(target.clone()),
                action,
            ),
            // Regeneration is its own destruction replacement (CR 701.19a);
            // its matcher is the regeneration-shield matcher, so "can't be
            // regenerated" suspends it (CR 701.19c).
            ReplacedEventSpec::SourceDestructionRegenerates => ReplacementEffect::with_matcher(
                source,
                controller,
                crate::events::permanents::matchers::RegenerationShieldMatcher::new(source),
                ReplacementAction::Instead(vec![
                    Effect::tap(crate::target::ChooseSpec::SpecificObject(source))
                        .tag(crate::tag::TagKey::from("__it__")),
                    Effect::clear_damage(crate::target::ChooseSpec::SpecificObject(source)),
                    Effect::new(crate::effects::RemoveFromCombatEffect::with_spec(
                        crate::target::ChooseSpec::SpecificObject(source),
                    )),
                ]),
            ),
            // Zone-change owners bind the moving object as "it".
            ReplacedEventSpec::ZoneChange { object, from, to } => ReplacementEffect::with_matcher(
                source,
                controller,
                WouldChangeZoneMatcher::new(object.clone(), *from, *to),
                action,
            ),
        };
        Some(if self.optional {
            replacement.optional()
        } else {
            replacement
        })
    }
}

/// Matches the event a [`ReplacedEventSpec`] describes. Unlike prevention
/// matchers, an "instead" replacement also applies to damage that can't be
/// prevented (CR 615 governs prevention only).
#[derive(Debug, Clone)]
pub struct ReplacedEventMatcher {
    pub event: ReplacedEventSpec,
}

fn damage_source_matches(source: ObjectId, filter: &ObjectFilter, ctx: &EventContext) -> bool {
    if !ctx.game.is_phased_out(source)
        && let Some(object) = ctx.game.object(source)
    {
        return filter.matches(object, &ctx.filter_ctx, ctx.game);
    }
    ctx.event_source_snapshot
        .filter(|snapshot| snapshot.object_id == source)
        .is_some_and(|snapshot| filter.matches_snapshot(snapshot, &ctx.filter_ctx, ctx.game))
}

fn damage_target_object_matches(
    damage: &DamageEvent,
    target: ObjectId,
    filter: &ObjectFilter,
    ctx: &EventContext,
) -> bool {
    ctx.game
        .object(target)
        .is_some_and(|object| filter.matches(object, &ctx.filter_ctx, ctx.game))
        || damage
            .target_snapshot
            .as_ref()
            .filter(|snapshot| snapshot.object_id == target)
            .is_some_and(|snapshot| filter.matches_snapshot(snapshot, &ctx.filter_ctx, ctx.game))
}

impl ReplacementMatcher for ReplacedEventMatcher {
    fn may_match_event_kind(&self, kind: EventKind) -> bool {
        match &self.event {
            ReplacedEventSpec::DamageToPlayer { .. } | ReplacedEventSpec::DamageToObject { .. } => {
                kind == EventKind::Damage
            }
            ReplacedEventSpec::LifeGain { .. } => kind == EventKind::LifeGain,
            ReplacedEventSpec::DrawInstruction { .. } => kind == EventKind::KeywordAction,
            ReplacedEventSpec::Untap { .. } => kind == EventKind::BecomeUntapped,
            // Installed through their dedicated matchers instead.
            ReplacedEventSpec::LifeLoss { .. }
            | ReplacedEventSpec::Destroy { .. }
            | ReplacedEventSpec::ZoneChange { .. }
            | ReplacedEventSpec::SourceDestructionRegenerates => false,
        }
    }

    fn matches_prepared_event(
        &self,
        event: &dyn GameEventType,
        ctx: &crate::events::context::PreparedEventContext,
    ) -> bool {
        match &self.event {
            ReplacedEventSpec::DamageToPlayer {
                player,
                source_filter,
                combat_only,
            } => {
                let Some(damage) = downcast_event::<DamageEvent>(event) else {
                    return false;
                };
                if *combat_only && !damage.is_combat {
                    return false;
                }
                let DamageTarget::Player(target) = damage.target else {
                    return false;
                };
                player.matches_player(target, &ctx.filter_ctx)
                    && source_filter
                        .as_ref()
                        .is_none_or(|filter| damage_source_matches(damage.source, filter, ctx))
            }
            ReplacedEventSpec::DamageToObject {
                target,
                source_filter,
                combat_only,
            } => {
                let Some(damage) = downcast_event::<DamageEvent>(event) else {
                    return false;
                };
                if *combat_only && !damage.is_combat {
                    return false;
                }
                let DamageTarget::Object(object) = damage.target else {
                    return false;
                };
                damage_target_object_matches(damage, object, target, ctx)
                    && source_filter
                        .as_ref()
                        .is_none_or(|filter| damage_source_matches(damage.source, filter, ctx))
            }
            ReplacedEventSpec::LifeGain { player } => {
                let Some(gain) = downcast_event::<LifeGainEvent>(event) else {
                    return false;
                };
                player.matches_player(gain.player, &ctx.filter_ctx)
            }
            ReplacedEventSpec::Untap {
                object,
                during_controllers_untap_step,
            } => {
                let Some(untap) = downcast_event::<crate::events::UntapEvent>(event) else {
                    return false;
                };
                let Some(permanent) = ctx.game.object(untap.permanent) else {
                    return false;
                };
                if *during_controllers_untap_step
                    && !(ctx.game.turn.step == Some(crate::game_state::Step::Untap)
                        && ctx
                            .game
                            .turn_players()
                            .contains(&ctx.game.controller_of(permanent)))
                {
                    return false;
                }
                object.matches(permanent, &ctx.filter_ctx, ctx.game)
            }
            ReplacedEventSpec::DrawInstruction { player, minimum } => {
                let Some(action) =
                    downcast_event::<crate::events::KeywordActionEvent>(event)
                else {
                    return false;
                };
                action.action == crate::events::KeywordActionKind::DrawCards
                    && action.amount >= *minimum
                    && player.matches_player(action.player, &ctx.filter_ctx)
            }
            ReplacedEventSpec::LifeLoss { .. }
            | ReplacedEventSpec::Destroy { .. }
            | ReplacedEventSpec::ZoneChange { .. }
            | ReplacedEventSpec::SourceDestructionRegenerates => false,
        }
    }

    fn display(&self) -> String {
        match &self.event {
            ReplacedEventSpec::DamageToPlayer { .. } => {
                "When damage would be dealt to a matching player".to_string()
            }
            ReplacedEventSpec::DamageToObject { .. } => {
                "When damage would be dealt to a matching permanent".to_string()
            }
            ReplacedEventSpec::LifeGain { .. } => {
                "When a matching player would gain life".to_string()
            }
            ReplacedEventSpec::LifeLoss { .. } => {
                "When a matching player would lose life".to_string()
            }
            ReplacedEventSpec::Destroy { .. } => {
                "When a matching permanent would be destroyed".to_string()
            }
            ReplacedEventSpec::ZoneChange { .. } => {
                "When a matching object would change zones".to_string()
            }
            ReplacedEventSpec::SourceDestructionRegenerates => {
                "When this permanent would be destroyed".to_string()
            }
            ReplacedEventSpec::Untap { .. } => {
                "When a matching permanent would untap".to_string()
            }
            ReplacedEventSpec::DrawInstruction { minimum, .. } => {
                format!("When a matching player would draw {minimum} or more cards")
            }
        }
    }
}
