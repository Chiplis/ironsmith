//! Schedule an extra turn after a player's next turn.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
pub use ironsmith_core::ExtraTurnAfterNextTurnEffect;

/// Effect that schedules a player's extra turn after that player's next turn.
impl EffectExecutor for ExtraTurnAfterNextTurnEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        super::skip_scheduled::execute_schedule_operation(
            game,
            ctx,
            &self.player,
            "extra_turn_after_next_turn",
            super::skip_scheduled::ScheduleOperation::ExtraTurnAfterNextTurn,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::PlayerId;
    use crate::target::PlayerFilter;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    #[test]
    fn extra_turn_after_next_turn_waits_for_target_players_turn_to_end() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ExtraTurnAfterNextTurnEffect::new(PlayerFilter::Specific(bob));
        let result = effect.execute(&mut game, &mut ctx).unwrap();
        assert_eq!(result.status, crate::effect::OutcomeStatus::Succeeded);
        assert!(
            game.turn_store.extra_turns.is_empty(),
            "the extra turn should not be queued immediately"
        );
        // CR 500.7: no triggered ability is involved.
        assert!(game.effect_store.delayed_triggers.is_empty());

        game.next_turn();
        assert_eq!(
            game.turn.active_player, bob,
            "Bob should take their normal turn"
        );
        assert!(
            game.turn_store.extra_turns.is_empty(),
            "the extra turn should still wait until Bob's turn ends"
        );

        game.cleanup_player_control_end_of_turn();
        game.next_turn();
        assert_eq!(
            game.turn.active_player, bob,
            "Bob should take the extra turn immediately after their turn ends"
        );
    }
}
