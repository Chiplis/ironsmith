//! Evolve keyword effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::EnterBattlefieldEvent;
use crate::events::ZoneChangeEvent;
use crate::events::other::{KeywordActionEvent, KeywordActionKind};
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::object::CounterType;
use crate::triggers::TriggerEvent;
use crate::types::CardType;
use crate::zone::Zone;
pub use ironsmith_core::EvolveEffect;

/// "Put a +1/+1 counter on this creature if a larger creature entered under your control."
fn effective_power(game: &GameState, id: crate::ids::ObjectId) -> Option<i32> {
    game.calculated_power(id)
        .or_else(|| game.object(id).and_then(|obj| obj.power()))
}

fn effective_toughness(game: &GameState, id: crate::ids::ObjectId) -> Option<i32> {
    game.calculated_toughness(id)
        .or_else(|| game.object(id).and_then(|obj| obj.toughness()))
}

fn entered_object_from_trigger_event(event: &TriggerEvent) -> Option<ObjectId> {
    if let Some(etb) = event.downcast::<EnterBattlefieldEvent>() {
        return Some(etb.object);
    }

    let zone_change = event.downcast::<ZoneChangeEvent>()?;
    (zone_change.to == Zone::Battlefield)
        .then(|| zone_change.destination_objects().first().copied())
        .flatten()
}

/// Power and toughness of the creature that entered. If it has since left
/// the battlefield, its last-known information from the zone change that
/// removed it is used (Gatecrash evolve rulings, CR 608.2h).
fn entered_creature_power_toughness(game: &GameState, entered_id: ObjectId) -> Option<(i32, i32)> {
    if let Some(object) = game.object(entered_id)
        && object.zone == Zone::Battlefield
    {
        if !game.object_has_card_type(entered_id, CardType::Creature) {
            return None;
        }
        return Some((
            effective_power(game, entered_id)?,
            effective_toughness(game, entered_id)?,
        ));
    }
    let history = &game.turn_store.turn_history;
    let snapshot = history
        .event_records
        .iter()
        .chain(history.staged_event_records.iter())
        .rev()
        .filter_map(|record| record.event.downcast::<ZoneChangeEvent>())
        .flat_map(|event| event.snapshot.iter().chain(event.snapshots.iter()))
        .find(|snapshot| snapshot.object_id == entered_id && snapshot.zone == Zone::Battlefield)?;
    if !snapshot.card_types.contains(&CardType::Creature) {
        return None;
    }
    Some((snapshot.power?, snapshot.toughness?))
}

/// CR 702.100a evolve comparison, used both as the trigger's intervening-if
/// and again on resolution (CR 603.4).
pub(crate) fn evolve_entering_creature_is_larger(
    game: &GameState,
    source_id: ObjectId,
    event: &TriggerEvent,
) -> bool {
    let Some(entered_id) = entered_object_from_trigger_event(event) else {
        return false;
    };
    let source_id = game
        .object(source_id)
        .map(|object| object.id)
        .unwrap_or(source_id);
    if source_id == entered_id {
        return false;
    }
    let (Some(source_power), Some(source_toughness)) = (
        effective_power(game, source_id),
        effective_toughness(game, source_id),
    ) else {
        return false;
    };
    let Some((entered_power, entered_toughness)) =
        entered_creature_power_toughness(game, entered_id)
    else {
        return false;
    };
    entered_power > source_power || entered_toughness > source_toughness
}

impl EffectExecutor for EvolveEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let Some(triggering_event) = &ctx.triggering_event else {
            return Ok(EffectOutcome::count(0));
        };

        let source_id = ctx.source;
        if !game
            .object(source_id)
            .is_some_and(|object| object.zone == Zone::Battlefield)
        {
            return Ok(EffectOutcome::count(0));
        }
        // CR 702.100a: the comparison is rechecked on resolution; the entered
        // creature's controller no longer matters, and its LKI is used if it
        // left the battlefield.
        if !evolve_entering_creature_is_larger(game, source_id, triggering_event) {
            return Ok(EffectOutcome::count(0));
        }

        let placed = crate::events::processing::process_put_counters_with_event_with_dm(
            game,
            source_id,
            CounterType::PlusOnePlusOne,
            1,
            ctx.cause.clone(),
            &mut *ctx.decision_maker,
        );
        let mut outcome = EffectOutcome::count(1);
        if let Some(stable_id) = game.object(source_id).map(|o| o.stable_id) {
            game.record_ui_effect_event(
                "level_up",
                Some(ctx.controller),
                None,
                vec![stable_id],
                Some(1),
                Some("evolve".to_string()),
            );
        }
        if placed > 0
            && let Some(counter_event) = game.add_counters_with_source(
                source_id,
                CounterType::PlusOnePlusOne,
                placed,
                Some(source_id),
                Some(ctx.controller),
            )
        {
            outcome = outcome.with_event(counter_event);
        }
        outcome = outcome.with_event(TriggerEvent::new_with_provenance(
            KeywordActionEvent::new(KeywordActionKind::Evolve, ctx.controller, source_id, 1),
            ctx.provenance,
        ));
        Ok(outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effects::ExecutionContext;
    use crate::events::cause::EventCause;
    use crate::ids::{CardId, PlayerId};
    use crate::triggers::TriggerEvent;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn create_creature(
        game: &mut GameState,
        owner: PlayerId,
        card_id: u32,
        power: i32,
        toughness: i32,
    ) -> crate::ids::ObjectId {
        let card = CardBuilder::new(CardId::from_raw(card_id), format!("Creature {card_id}"))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(power, toughness))
            .build();
        game.create_object_from_card(&card, owner, Zone::Battlefield)
    }

    #[test]
    fn evolves_when_larger_creature_enters_under_your_control() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = create_creature(&mut game, alice, 1, 2, 2);
        let entered = create_creature(&mut game, alice, 2, 3, 3);

        let event_provenance = game
            .provenance_graph_mut()
            .alloc_root_event(crate::events::EventKind::EnterBattlefield);
        let event = TriggerEvent::new_with_provenance(
            EnterBattlefieldEvent::new(entered, Zone::Hand),
            event_provenance,
        );
        let mut ctx = ExecutionContext::new_default(source, alice).with_triggering_event(event);
        let outcome = EvolveEffect::new()
            .execute(&mut game, &mut ctx)
            .expect("execute evolve");

        assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(1));
        let source_obj = game.object(source).expect("source exists");
        assert_eq!(
            source_obj
                .counters
                .get(&CounterType::PlusOnePlusOne)
                .copied()
                .unwrap_or(0),
            1
        );
    }

    #[test]
    fn does_not_evolve_when_creature_is_not_larger() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = create_creature(&mut game, alice, 1, 3, 3);
        let entered = create_creature(&mut game, alice, 2, 2, 3);

        let event_provenance = game
            .provenance_graph_mut()
            .alloc_root_event(crate::events::EventKind::EnterBattlefield);
        let event = TriggerEvent::new_with_provenance(
            EnterBattlefieldEvent::new(entered, Zone::Hand),
            event_provenance,
        );
        let mut ctx = ExecutionContext::new_default(source, alice).with_triggering_event(event);
        let outcome = EvolveEffect::new()
            .execute(&mut game, &mut ctx)
            .expect("execute evolve");

        assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(0));
        let source_obj = game.object(source).expect("source exists");
        assert_eq!(
            source_obj
                .counters
                .get(&CounterType::PlusOnePlusOne)
                .copied()
                .unwrap_or(0),
            0
        );
    }

    #[test]
    fn evolves_from_zone_change_etb_trigger_event() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = create_creature(&mut game, alice, 1, 0, 1);
        let entered = create_creature(&mut game, alice, 2, 2, 5);

        let event_provenance = game
            .provenance_graph_mut()
            .alloc_root_event(crate::events::EventKind::ZoneChange);
        let event = TriggerEvent::new_with_provenance(
            ZoneChangeEvent::with_cause(
                entered,
                Zone::Stack,
                Zone::Battlefield,
                EventCause::from_game_rule(),
                None,
            ),
            event_provenance,
        );
        let mut ctx = ExecutionContext::new_default(source, alice).with_triggering_event(event);
        let outcome = EvolveEffect::new()
            .execute(&mut game, &mut ctx)
            .expect("execute evolve from zone change");

        assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(1));
        let source_obj = game.object(source).expect("source exists");
        assert_eq!(
            source_obj
                .counters
                .get(&CounterType::PlusOnePlusOne)
                .copied()
                .unwrap_or(0),
            1
        );
    }
}
