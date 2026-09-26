//! Schedule an extra turn after a player's next turn.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
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
        let selected_player = resolve_player_filter(game, &self.player, ctx)?;
        if !ctx.claim_shared_team_structure_operation(
            game,
            selected_player,
            "extra_turn_after_next_turn",
        ) {
            return Ok(EffectOutcome::resolved());
        }
        // CR 500.7: the extra turn is added directly after that player's next
        // turn; it isn't a triggered ability, so it can't be countered and
        // doesn't depend on that turn having an end step.
        let player_id = game.team_turn_representative(selected_player);
        game.turn_store
            .extra_turns_after_next_turn
            .push((player_id, game.turn.turn_number));

        Ok(EffectOutcome::resolved())
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
