//! Amount-modifying replacements (CR 614.1a, 616.1).
//!
//! "If an opponent would mill one or more cards, they mill twice that many
//! cards instead." (Bruvac), "If you would scry a number of cards, scry that
//! many cards plus one instead." (Kenessos), "If a source would deal 4 or more
//! damage to a permanent or player, that source deals 3 damage to that
//! permanent or player instead." (Divine Presence). The watched event still
//! happens; only its proposed amount changes, through the shared
//! `ReplacementAction::Modify` owner, so several such replacements compose in
//! the order the affected player chooses (CR 616.1).

use crate::events::context::PreparedEventContext;
use crate::events::damage::DamageEvent;
use crate::events::other::WouldKeywordActionMatcher;
use crate::events::traits::{EventKind, GameEventType, ReplacementMatcher, downcast_event};
use crate::ids::{ObjectId, PlayerId};
use crate::replacement::{EventModification, ReplacementAction, ReplacementEffect};
use crate::static_abilities::{StaticAbility, StaticAbilityId, StaticAbilityKind};
use crate::target::ObjectFilter;
use ironsmith_core::{AmountEventSpec, AmountModifierSpec};

/// One amount-modifying replacement over one watched event kind.
#[derive(Debug, Clone, PartialEq)]
pub struct EventAmountReplacement {
    pub event: AmountEventSpec,
    pub modifier: AmountModifierSpec,
    pub optional: bool,
    pub condition: Option<crate::ConditionExpr>,
    pub display: String,
}

impl EventAmountReplacement {
    pub fn new(
        event: AmountEventSpec,
        modifier: AmountModifierSpec,
        optional: bool,
        display: impl Into<String>,
    ) -> Self {
        Self {
            event,
            modifier,
            optional,
            condition: None,
            display: display.into(),
        }
    }
}

/// The shared event modification for an authored amount change.
pub(crate) fn amount_modification(modifier: AmountModifierSpec) -> EventModification {
    match modifier {
        AmountModifierSpec::Multiply(factor) => EventModification::Multiply(factor),
        AmountModifierSpec::Add(additional) => {
            EventModification::Add(i32::try_from(additional).unwrap_or(i32::MAX))
        }
        AmountModifierSpec::SetTo(amount) => EventModification::SetTo(amount),
        AmountModifierSpec::Half { round_up } => EventModification::Halve { round_up },
    }
}

impl StaticAbilityKind for EventAmountReplacement {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::EventAmountReplacement
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn with_static_condition(&self, condition: crate::ConditionExpr) -> Option<StaticAbility> {
        let mut combined = self.clone();
        combined.condition = Some(match combined.condition.take() {
            Some(existing) => crate::ConditionExpr::And(Box::new(condition), Box::new(existing)),
            None => condition,
        });
        Some(StaticAbility::new(combined))
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        let replacement = ReplacementEffect::with_matcher(
            source,
            controller,
            AmountEventMatcher {
                event: self.event.clone(),
                condition: self.condition.clone(),
            },
            ReplacementAction::Modify(amount_modification(self.modifier)),
        );
        Some(if self.optional {
            replacement.optional()
        } else {
            replacement
        })
    }
}

/// Matches the event an [`AmountEventSpec`] watches. Like the instead
/// replacements and unlike prevention, it also matches damage that can't be
/// prevented: changing an amount is not preventing it (CR 615 governs
/// prevention only).
#[derive(Debug, Clone)]
pub struct AmountEventMatcher {
    pub event: AmountEventSpec,
    pub condition: Option<crate::ConditionExpr>,
}

impl AmountEventMatcher {
    fn condition_holds(&self, ctx: &PreparedEventContext) -> bool {
        let Some(condition) = &self.condition else {
            return true;
        };
        let Some(source) = ctx.source else {
            return false;
        };
        let eval_ctx = crate::condition_eval::ExternalEvaluationContext {
            controller: ctx.controller,
            source,
            defending_player: None,
            attacking_player: None,
            filter_source: Some(source),
            iterated_player: None,
            triggering_event: None,
            trigger_identity: None,
            ability_index: None,
            options: Default::default(),
        };
        crate::condition_eval::evaluate_condition_external(ctx.game, condition, &eval_ctx)
    }
}

impl ReplacementMatcher for AmountEventMatcher {
    fn may_match_event_kind(&self, kind: EventKind) -> bool {
        match &self.event {
            AmountEventSpec::Damage { .. } => kind == EventKind::Damage,
            AmountEventSpec::KeywordAction { .. } => kind == EventKind::KeywordAction,
        }
    }

    fn matches_prepared_event(&self, event: &dyn GameEventType, ctx: &PreparedEventContext) -> bool {
        let matches = match &self.event {
            AmountEventSpec::Damage {
                source_filter,
                player,
                object,
                combat_only,
                minimum,
            } => {
                let Some(damage) = downcast_event::<DamageEvent>(event) else {
                    return false;
                };
                // "4 or more damage" reads the proposed amount as modified so
                // far by earlier replacements (CR 616.1).
                if minimum.is_some_and(|minimum| damage.amount < minimum) {
                    return false;
                }
                super::DamageAmountReplacementMatcher {
                    source_filter: source_filter.clone().unwrap_or_default(),
                    target_player_filter: player.clone(),
                    target_object_filter: object.clone(),
                    condition: None,
                    combat_only: *combat_only,
                    noncombat_only: false,
                    amount_less_than: None,
                    maximum_damage: None,
                }
                .matches_prepared_event(event, ctx)
            }
            AmountEventSpec::KeywordAction { action, performer } => {
                WouldKeywordActionMatcher::new(*action, ObjectFilter::default())
                    .with_performer_filter(Some(performer.clone()))
                    .matches_prepared_event(event, ctx)
            }
        };
        matches && self.condition_holds(ctx)
    }

    fn display(&self) -> String {
        match &self.event {
            AmountEventSpec::Damage { .. } => {
                "If matching damage would be dealt".to_string()
            }
            AmountEventSpec::KeywordAction { action, .. } => {
                format!("If a matching player would {}", action.infinitive())
            }
        }
    }
}

/// Whether any live or ability-generated replacement could watch a keyword
/// action proposal. Instructions that only propose their magnitude for
/// modification (mill) skip the replacement pass when nothing could apply.
pub(crate) fn may_have_keyword_action_replacements(game: &crate::game_state::GameState) -> bool {
    let related = |effect: &ReplacementEffect| {
        effect
            .matcher
            .as_ref()
            .is_none_or(|matcher| matcher.may_match_event_kind(EventKind::KeywordAction))
    };
    game.effect_store
        .replacement_effects
        .effects()
        .iter()
        .any(related)
        || crate::replacement_ability_processor::generate_replacement_effects_from_abilities(game)
            .map_or(true, |effects| effects.iter().any(related))
}
