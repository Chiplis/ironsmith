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
        }
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
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            ReplacedEventMatcher {
                event: self.event.clone(),
            },
            ReplacementAction::Instead(self.replacement_effects.clone()),
        ))
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
        }
    }
}
