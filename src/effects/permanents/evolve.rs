//! Evolve keyword effect implementation.

use crate::effect::{EffectOutcome, EffectResult};
use crate::effects::EffectExecutor;
use crate::events::EnterBattlefieldEvent;
use crate::events::other::{KeywordActionEvent, KeywordActionKind};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::object::CounterType;
use crate::triggers::TriggerEvent;
use crate::types::CardType;

/// "Put a +1/+1 counter on this creature if a larger creature entered under your control."
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EvolveEffect;

impl EvolveEffect {
    pub const fn new() -> Self {
        Self
    }

    fn effective_power(game: &GameState, id: crate::ids::ObjectId) -> Option<i32> {
        game.calculated_power(id)
            .or_else(|| game.object(id).and_then(|obj| obj.power()))
    }

    fn effective_toughness(game: &GameState, id: crate::ids::ObjectId) -> Option<i32> {
        game.calculated_toughness(id)
            .or_else(|| game.object(id).and_then(|obj| obj.toughness()))
    }
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
        let Some(etb) = triggering_event.downcast::<EnterBattlefieldEvent>() else {
            return Ok(EffectOutcome::count(0));
        };

        let source_id = ctx.source;
        let entered_id = etb.object;
        if source_id == entered_id {
            return Ok(EffectOutcome::count(0));
        }

        let Some(source_obj) = game.object(source_id) else {
            return Ok(EffectOutcome::count(0));
        };
        let Some(entered_obj) = game.object(entered_id) else {
            return Ok(EffectOutcome::count(0));
        };

        if !entered_obj.has_card_type(CardType::Creature)
            || entered_obj.controller != source_obj.controller
        {
            return Ok(EffectOutcome::count(0));
        }

        let Some(source_power) = Self::effective_power(game, source_id) else {
            return Ok(EffectOutcome::count(0));
        };
        let Some(source_toughness) = Self::effective_toughness(game, source_id) else {
            return Ok(EffectOutcome::count(0));
        };
        let Some(entered_power) = Self::effective_power(game, entered_id) else {
            return Ok(EffectOutcome::count(0));
        };
        let Some(entered_toughness) = Self::effective_toughness(game, entered_id) else {
            return Ok(EffectOutcome::count(0));
        };

        if entered_power <= source_power && entered_toughness <= source_toughness {
            return Ok(EffectOutcome::count(0));
        }

        let mut outcome = EffectOutcome::new(EffectResult::Count(1), Vec::new());
        if let Some(counter_event) = game.add_counters_with_source(
            source_id,
            CounterType::PlusOnePlusOne,
            1,
            Some(source_id),
            Some(ctx.controller),
        ) {
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
    use crate::effect::EffectResult;
    use crate::executor::ExecutionContext;
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
            .provenance_graph
            .alloc_root_event(crate::events::EventKind::EnterBattlefield);
        let event = TriggerEvent::new_with_provenance(
            EnterBattlefieldEvent::new(entered, Zone::Hand),
            event_provenance,
        );
        let mut ctx = ExecutionContext::new_default(source, alice).with_triggering_event(event);
        let outcome = EvolveEffect::new()
            .execute(&mut game, &mut ctx)
            .expect("execute evolve");

        assert_eq!(outcome.result, EffectResult::Count(1));
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
            .provenance_graph
            .alloc_root_event(crate::events::EventKind::EnterBattlefield);
        let event = TriggerEvent::new_with_provenance(
            EnterBattlefieldEvent::new(entered, Zone::Hand),
            event_provenance,
        );
        let mut ctx = ExecutionContext::new_default(source, alice).with_triggering_event(event);
        let outcome = EvolveEffect::new()
            .execute(&mut game, &mut ctx)
            .expect("execute evolve");

        assert_eq!(outcome.result, EffectResult::Count(0));
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
}
