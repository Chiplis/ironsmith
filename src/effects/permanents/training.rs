//! Training keyword effect implementation.

use crate::effect::{EffectOutcome, EffectResult};
use crate::effects::EffectExecutor;
use crate::events::other::{KeywordActionEvent, KeywordActionKind};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::object::CounterType;
use crate::triggers::TriggerEvent;

/// "Put a +1/+1 counter on this creature." for a resolved training trigger.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TrainingEffect;

impl TrainingEffect {
    pub const fn new() -> Self {
        Self
    }
}

impl EffectExecutor for TrainingEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if game.object(ctx.source).is_none() {
            return Ok(EffectOutcome::count(0));
        }

        let mut outcome = EffectOutcome::new(EffectResult::Count(1), Vec::new());
        if let Some(counter_event) = game.add_counters_with_source(
            ctx.source,
            CounterType::PlusOnePlusOne,
            1,
            Some(ctx.source),
            Some(ctx.controller),
        ) {
            outcome = outcome.with_event(counter_event);
        }
        outcome = outcome.with_event(TriggerEvent::new(KeywordActionEvent::new(
            KeywordActionKind::Train,
            ctx.controller,
            ctx.source,
            1,
        )));
        Ok(outcome)
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effect::EffectResult;
    use crate::executor::ExecutionContext;
    use crate::ids::{CardId, PlayerId};
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
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
    fn training_adds_plus_one_plus_one_counter_to_source() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = create_creature(&mut game, alice, 1);
        let mut ctx = ExecutionContext::new_default(source, alice);

        let outcome = TrainingEffect::new()
            .execute(&mut game, &mut ctx)
            .expect("execute training");

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
}
