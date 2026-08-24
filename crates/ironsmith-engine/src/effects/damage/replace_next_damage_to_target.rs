//! One-shot replacement of the next damage event to a chosen target.

use crate::effect::{Effect, EffectOutcome};
use crate::effects::helpers::{resolve_objects_for_effect, resolve_players_from_spec};
use crate::effects::{EffectExecutionCategory, EffectExecutor, ExecutionContext, ExecutionError};
use crate::events::damage::DamageEvent;
use crate::events::{DamageTarget, EventContext, EventKind, GameEventType, ReplacementMatcher};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::replacement::{ReplacementAction, ReplacementEffect};
use crate::target::ChooseSpec;

#[derive(Debug, Clone)]
enum ExactDamageTarget {
    Object(ObjectId),
    Player(PlayerId),
}

#[derive(Debug, Clone)]
struct DamageToExactTargetMatcher {
    target: ExactDamageTarget,
}

impl ReplacementMatcher for DamageToExactTargetMatcher {
    fn matches_event(&self, event: &dyn GameEventType, _ctx: &EventContext) -> bool {
        if event.event_kind() != EventKind::Damage {
            return false;
        }
        let Some(damage) = crate::events::downcast_event::<DamageEvent>(event) else {
            return false;
        };
        match (&self.target, damage.target) {
            (ExactDamageTarget::Object(expected), DamageTarget::Object(actual)) => {
                *expected == actual
            }
            (ExactDamageTarget::Player(expected), DamageTarget::Player(actual)) => {
                *expected == actual
            }
            _ => false,
        }
    }

    fn display(&self) -> String {
        "The next time damage would be dealt to the chosen target".to_string()
    }
}

/// Registers a one-shot, turn-scoped replacement whose payload is a reusable
/// sequence of effects. Unlike prevention shields, this also matches damage
/// that can't be prevented.
#[derive(Debug, Clone, PartialEq)]
pub struct ReplaceNextDamageToTargetEffect {
    pub target: ChooseSpec,
    pub replacement_effects: Vec<Effect>,
}

impl ReplaceNextDamageToTargetEffect {
    pub fn new(target: ChooseSpec, replacement_effects: Vec<Effect>) -> Self {
        Self {
            target,
            replacement_effects,
        }
    }
}

impl EffectExecutor for ReplaceNextDamageToTargetEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let target = if let Ok(objects) = resolve_objects_for_effect(game, ctx, &self.target)
            && let Some(object) = objects.into_iter().next()
        {
            ExactDamageTarget::Object(object)
        } else if let Ok(players) = resolve_players_from_spec(game, &self.target, ctx)
            && let Some(player) = players.into_iter().next()
        {
            ExactDamageTarget::Player(player)
        } else {
            return Err(ExecutionError::InvalidTarget);
        };
        let replacement = ReplacementEffect::with_matcher(
            ctx.source,
            ctx.controller,
            DamageToExactTargetMatcher { target },
            ReplacementAction::Instead(self.replacement_effects.clone()),
        );
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(replacement);
        Ok(EffectOutcome::resolved())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "damage replacement target"
    }

    fn primary_execution_category(&self) -> EffectExecutionCategory {
        EffectExecutionCategory::ReplacementRegistration
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::SelectFirstDecisionMaker;
    use crate::effects::ResolvedTarget;
    use crate::events::cause::EventCause;
    use crate::events::processing::process_damage_with_event;
    use crate::filter::ObjectFilter;
    use crate::{CardDefinitionBuilder, CardId, CardType, PowerToughness, Zone};

    #[test]
    fn nonmatching_damage_preserves_shield_then_matching_damage_destroys_instead() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let creature = |id, name| {
            CardDefinitionBuilder::new(CardId::from_raw(id), name)
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(4, 4))
                .build()
        };
        let source = game.create_object_from_definition(
            &creature(99_401, "Replacement Source"),
            alice,
            Zone::Battlefield,
        );
        let protected = game.create_object_from_definition(
            &creature(99_402, "Protected Creature"),
            bob,
            Zone::Battlefield,
        );
        let other = game.create_object_from_definition(
            &creature(99_403, "Other Creature"),
            bob,
            Zone::Battlefield,
        );

        let damaged_tag = crate::TagKey::from("replaced_damage_target");
        let replacement_effects = vec![
            Effect::tag_triggering_damage_target(damaged_tag.clone()),
            Effect::new(crate::effects::DestroyEffect::with_spec(
                ChooseSpec::Tagged(damaged_tag),
            )),
        ];
        let effect = ReplaceNextDamageToTargetEffect::new(
            ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature())),
            replacement_effects,
        );
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm)
            .with_targets(vec![ResolvedTarget::Object(protected)]);
        effect
            .execute(&mut game, &mut ctx)
            .expect("register shield");

        let (other_damage, other_replaced) = process_damage_with_event(
            &mut game,
            source,
            DamageTarget::Object(other),
            1,
            false,
            EventCause::from_effect(source, alice),
        );
        assert_eq!(other_damage, 1);
        assert!(!other_replaced);
        assert_eq!(
            game.object(other).map(|object| object.zone),
            Some(Zone::Battlefield)
        );

        let (protected_damage, protected_replaced_or_prevented) = process_damage_with_event(
            &mut game,
            source,
            DamageTarget::Object(protected),
            1,
            false,
            EventCause::from_effect(source, alice),
        );
        assert_eq!(protected_damage, 0);
        assert!(
            protected_replaced_or_prevented,
            "the compatibility result flag records both replacement and prevention"
        );
        assert!(
            game.player(bob)
                .expect("Bob should exist")
                .graveyard
                .iter()
                .any(|&id| {
                    game.object(id)
                        .is_some_and(|object| object.name == "Protected Creature")
                })
        );
    }

    #[test]
    fn matcher_includes_damage_that_cannot_be_prevented() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(99_410);
        let target = ObjectId::from_raw(99_411);
        let matcher = DamageToExactTargetMatcher {
            target: ExactDamageTarget::Object(target),
        };
        let event = DamageEvent::unpreventable_with_cause(
            source,
            DamageTarget::Object(target),
            3,
            false,
            EventCause::effect(),
        );
        let ctx = EventContext::for_replacement_effect(alice, source, &game);
        assert!(matcher.matches_event(&event, &ctx));
    }
}
