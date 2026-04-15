//! Renown keyword effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::events::other::{KeywordActionEvent, KeywordActionKind};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::object::CounterType;
use crate::triggers::TriggerEvent;

/// "If this creature isn't renowned, put N +1/+1 counters on it and it becomes renowned."
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenownEffect {
    pub amount: u32,
}

impl RenownEffect {
    pub const fn new(amount: u32) -> Self {
        Self { amount }
    }
}

impl EffectExecutor for RenownEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if game.object(ctx.source).is_none() || game.is_renowned(ctx.source) {
            return Ok(EffectOutcome::count(0));
        }

        game.set_renowned(ctx.source);

        let mut outcome = EffectOutcome::count(1);
        if self.amount > 0
            && let Some(counter_event) = game.add_counters_with_source(
                ctx.source,
                CounterType::PlusOnePlusOne,
                self.amount,
                Some(ctx.source),
                Some(ctx.controller),
            )
        {
            outcome = outcome.with_event(counter_event);
        }
        outcome = outcome.with_event(TriggerEvent::new_with_provenance(
            KeywordActionEvent::new(
                KeywordActionKind::Renown,
                ctx.controller,
                ctx.source,
                self.amount,
            ),
            ctx.provenance,
        ));
        Ok(outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::executor::ExecutionContext;
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
