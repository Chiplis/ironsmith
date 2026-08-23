use crate::effect::EffectOutcome;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;

pub use ironsmith_core::ReverseTurnOrderEffect;

impl EffectExecutor for ReverseTurnOrderEffect {
    fn execute(
        &self,
        game: &mut GameState,
        _ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        game.turn_store.turn_order.reverse();
        Ok(EffectOutcome::resolved())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::PlayerId;

    #[test]
    fn reversing_turn_order_keeps_the_active_player_and_changes_who_goes_next() {
        let mut game = GameState::new(
            vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Cara".to_string(),
                "Dan".to_string(),
            ],
            20,
        );
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let cara = PlayerId::from_index(2);
        let dan = PlayerId::from_index(3);
        game.turn.active_player = bob;

        let mut ctx = ExecutionContext::new_default(crate::ids::ObjectId::from_raw(1), bob);
        ReverseTurnOrderEffect::new()
            .execute(&mut game, &mut ctx)
            .expect("turn order reversal resolves");

        assert_eq!(game.turn.active_player, bob);
        assert_eq!(game.turn_store.turn_order, vec![dan, cara, bob, alice]);
        game.next_turn();
        assert_eq!(game.turn.active_player, alice);
    }
}
