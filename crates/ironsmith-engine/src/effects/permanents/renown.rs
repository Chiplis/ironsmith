//! Renown keyword effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::other::{KeywordActionEvent, KeywordActionKind};
use crate::game_state::GameState;
use crate::object::CounterType;
use crate::triggers::TriggerEvent;
pub use ironsmith_core::RenownEffect;

/// "If this creature isn't renowned, put N +1/+1 counters on it and it becomes renowned."
impl EffectExecutor for RenownEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let checkpoint = game.clone();
        let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        let result = (|| {
            if !game
                .object(ctx.source)
                .is_some_and(|object| object.zone == crate::zone::Zone::Battlefield)
                || game.is_phased_out(ctx.source)
                || game.is_renowned(ctx.source)
            {
                return Ok(EffectOutcome::count(0));
            }

            let event = crate::events::Event::put_counters(
                ctx.source,
                CounterType::PlusOnePlusOne,
                self.amount,
                ctx.cause.clone(),
            )
            .with_provenance(ctx.provenance);
            let placement =
                crate::effects::counters::execute_object_counter_placement(game, ctx, event)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            // A replacement payload can make the original permanent leave.
            // The later instruction cannot designate that departed object.
            if !game
                .object(ctx.source)
                .is_some_and(|object| object.zone == crate::zone::Zone::Battlefield)
                || game.is_phased_out(ctx.source)
            {
                let mut outcome = placement;
                outcome.set_value(crate::effect::OutcomeValue::Count(0));
                return Ok(outcome);
            }
            // Becoming renowned follows counter placement, even when that
            // placement was prevented or replaced (CR 702.112).
            game.set_renowned(ctx.source);
            if let Some(stable_id) = game.object(ctx.source).map(|o| o.stable_id) {
                game.record_ui_effect_event(
                    "level_up",
                    Some(ctx.controller),
                    None,
                    vec![stable_id],
                    Some(i64::from(self.amount)),
                    Some("renown".to_string()),
                );
            }
            let mut outcome = EffectOutcome::aggregate([EffectOutcome::count(1), placement]);
            outcome.set_value(crate::effect::OutcomeValue::Count(1));
            outcome = outcome.with_event(TriggerEvent::new_with_provenance(
                KeywordActionEvent::new(
                    KeywordActionKind::Renown,
                    ctx.controller,
                    ctx.source,
                    self.amount,
                ),
                ctx.provenance,
            ));
            crate::events::other::freeze_completed_lifecycle_events(game, &mut outcome.events)?;
            Ok(outcome)
        })();
        if result.is_err() || ctx.decision_maker.awaiting_choice() {
            *game = checkpoint;
            context_checkpoint.restore(ctx);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
        }
        result
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
