//! Renown keyword effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::{CompletedEffectOutputs, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
#[cfg(test)]
use crate::events::other::{KeywordActionEvent, KeywordActionKind};
use crate::game_state::GameState;
#[cfg(test)]
use crate::object::CounterType;
pub use ironsmith_core::RenownEffect;

/// "If this creature isn't renowned, put N +1/+1 counters on it and it becomes renowned."
impl EffectExecutor for RenownEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(CompletedEffectOutputs::into_outcome)
    }

    fn canonical_program(&self) -> Option<crate::effect::Effect> {
        Some(crate::effect::Effect::new(
            super::designation::DesignationProgram {
                designation: super::designation::PermanentDesignation::Renowned,
                amount: self.amount.into(),
                missing_source_is_invalid: false,
            },
        ))
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        self.canonical_program()
            .expect("designation program")
            .0
            .execute_child_with_outputs(game, ctx)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effects::ExecutionContext;
    use crate::ids::{CardId, PlayerId};
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn create_creature(
        game: &mut GameState,
        owner: PlayerId,
        card_id: u32,
    ) -> crate::ids::ObjectId {
        let card = CardBuilder::new(CardId::from_raw(card_id), format!("Creature {card_id}"))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        game.create_object_from_card(&card, owner, Zone::Battlefield)
    }

    #[test]
    fn renown_counter_payload_departure_does_not_recreate_old_designation() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = create_creature(&mut game, alice, 1);
        let mut replacement = crate::static_abilities::StaticAbility::double_counters_replacement(
            crate::target::ObjectFilter::creature(),
            Some(CounterType::PlusOnePlusOne),
            "Destroy instead of placing counters".into(),
        )
        .generate_replacement_effect(source, alice)
        .unwrap();
        replacement.replacement =
            crate::replacement::ReplacementAction::Instead(vec![crate::effect::Effect::destroy(
                crate::target::ChooseSpec::Source,
            )]);
        game.effect_store
            .replacement_effects
            .add_resolution_effect(replacement);
        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = RenownEffect::new(2).execute(&mut game, &mut ctx).unwrap();
        assert!(game.object(source).is_none());
        assert!(!game.is_renowned(source));
        assert_eq!(outcome.count_or_zero(), 0);
        assert!(!outcome.events.iter().any(|event| {
            event
                .downcast::<KeywordActionEvent>()
                .is_some_and(|action| action.action == KeywordActionKind::Renown)
        }));
        // The replacement program captures departure triggers at its
        // instruction boundary, before returning to the renown operation.
        let actual_events: Vec<_> = game
            .turn_store
            .turn_history
            .projected_records()
            .map(|record| &record.event)
            .collect();
        assert_eq!(
            actual_events
                .iter()
                .filter_map(|event| event.downcast::<crate::events::ZoneChangeEvent>())
                .filter(|change| change.from == Zone::Battlefield
                    && change.to == Zone::Graveyard
                    && change.objects.contains(&source))
                .count(),
            1
        );
        assert!(!actual_events.iter().any(|event| {
            event
                .downcast::<KeywordActionEvent>()
                .is_some_and(|action| action.action == KeywordActionKind::Renown)
        }));
    }

    #[test]
    fn renown_marks_and_adds_counters_once() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = create_creature(&mut game, alice, 1);
        let mut ctx = ExecutionContext::new_default(source, alice);

        let first = RenownEffect::new(2)
            .execute(&mut game, &mut ctx)
            .expect("execute first renown");
        assert_eq!(first.value, crate::effect::OutcomeValue::Count(1));
        assert!(game.is_renowned(source));
        assert_eq!(
            game.object(source)
                .expect("source exists")
                .counters
                .get(&CounterType::PlusOnePlusOne)
                .copied()
                .unwrap_or(0),
            2
        );

        let second = RenownEffect::new(2)
            .execute(&mut game, &mut ctx)
            .expect("execute second renown");
        assert_eq!(second.value, crate::effect::OutcomeValue::Count(0));
        assert_eq!(
            game.object(source)
                .expect("source exists")
                .counters
                .get(&CounterType::PlusOnePlusOne)
                .copied()
                .unwrap_or(0),
            2
        );
    }
}
