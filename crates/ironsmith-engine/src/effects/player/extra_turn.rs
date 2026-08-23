//! Extra turn effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
pub use ironsmith_core::ExtraTurnEffect;

/// Effect that gives a player an extra turn.
///
/// Adds a turn to the extra turns queue.
///
/// # Fields
///
/// * `player` - The player who gets the extra turn
///
/// # Example
///
/// ```ignore
/// // Take an extra turn after this one
/// let effect = ExtraTurnEffect::you();
///
/// // Target player takes an extra turn
/// let effect = ExtraTurnEffect::new(PlayerFilter::Any);
/// ```
impl EffectExecutor for ExtraTurnEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        if !ctx.claim_shared_team_structure_operation(game, player_id, "extra_turn") {
            return Ok(EffectOutcome::resolved());
        }

        // Add an extra turn for this player
        let turn_player = game.team_turn_representative(player_id);
        game.turn_store.extra_turns.push(turn_player);
        game.record_ui_effect_event("extra_turn", Some(player_id), None, Vec::new(), None, None);

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
    fn test_extra_turn() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        assert!(game.turn_store.extra_turns.is_empty());

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = ExtraTurnEffect::you();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.status, crate::effect::OutcomeStatus::Succeeded);
        assert_eq!(game.turn_store.extra_turns.len(), 1);
        assert_eq!(game.turn_store.extra_turns[0], alice);
    }

    #[test]
    fn test_extra_turn_for_opponent() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = ExtraTurnEffect::new(PlayerFilter::Specific(bob));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.status, crate::effect::OutcomeStatus::Succeeded);
        assert_eq!(game.turn_store.extra_turns.len(), 1);
        assert_eq!(game.turn_store.extra_turns[0], bob);
    }

    #[test]
    fn test_multiple_extra_turns() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = ExtraTurnEffect::you();

        // Take three extra turns
        effect.execute(&mut game, &mut ctx).unwrap();
        effect.execute(&mut game, &mut ctx).unwrap();
        effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(game.turn_store.extra_turns.len(), 3);
    }

    #[test]
    fn most_recent_extra_turn_is_taken_first() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.turn.active_player = alice;
        game.turn_store.extra_turns.push(bob);
        game.turn_store.extra_turns.push(alice);

        game.next_turn();
        assert_eq!(game.turn.active_player, alice);
        game.next_turn();
        assert_eq!(game.turn.active_player, bob);
    }

    #[test]
    fn skip_next_turn_applies_to_an_extra_turn() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.turn.active_player = alice;
        game.turn_store.extra_turns.push(alice);
        game.turn_store.extra_turns.push(alice);
        game.turn_store.skip_next_turn.insert(alice);

        game.next_turn();

        assert_eq!(
            game.turn.active_player, alice,
            "one extra turn is skipped and the next extra turn is still taken"
        );
        assert!(game.turn_store.extra_turns.is_empty());
        assert!(!game.turn_store.skip_next_turn.contains(&alice));
    }

    #[test]
    fn test_extra_turn_clone_box() {
        let effect = ExtraTurnEffect::you();
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("ExtraTurnEffect"));
    }
}
