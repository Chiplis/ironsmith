//! Skip turn effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::PlayerFilter;

/// Effect that causes a player to skip their next turn.
///
/// Marks the player to skip their next turn.
///
/// # Fields
///
/// * `player` - The player who skips their turn
///
/// # Example
///
/// ```ignore
/// // Target player skips their next turn
/// let effect = SkipTurnEffect::new(PlayerFilter::Opponent);
///
/// // Skip your next turn (cost of a powerful effect)
/// let effect = SkipTurnEffect::you();
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct SkipTurnEffect {
    /// The player who skips their turn.
    pub player: PlayerFilter,
}

impl SkipTurnEffect {
    /// Create a new skip turn effect.
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }

    /// The controller skips their next turn.
    pub fn you() -> Self {
        Self::new(PlayerFilter::You)
    }

    /// Target opponent skips their next turn.
    pub fn opponent() -> Self {
        Self::new(PlayerFilter::Opponent)
    }
}

impl EffectExecutor for SkipTurnEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        if !ctx.claim_shared_team_structure_operation(game, player_id, "skip_turn") {
            return Ok(EffectOutcome::resolved());
        }

        // Mark the player to skip their next turn
        let turn_player = game.team_turn_representative(player_id);
        game.turn_store.skip_next_turn.insert(turn_player);

        Ok(EffectOutcome::resolved())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::PlayerId;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    #[test]
    fn test_skip_turn() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        assert!(!game.turn_store.skip_next_turn.contains(&alice));

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = SkipTurnEffect::you();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.status, crate::effect::OutcomeStatus::Succeeded);
        assert!(game.turn_store.skip_next_turn.contains(&alice));
    }

    #[test]
    fn test_skip_turn_opponent() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = SkipTurnEffect::new(PlayerFilter::Specific(bob));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.status, crate::effect::OutcomeStatus::Succeeded);
        assert!(!game.turn_store.skip_next_turn.contains(&alice));
        assert!(game.turn_store.skip_next_turn.contains(&bob));
    }

    #[test]
    fn test_skip_turn_idempotent() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = SkipTurnEffect::you();

        // Skip twice - should still only skip once (it's a set)
        effect.execute(&mut game, &mut ctx).unwrap();
        effect.execute(&mut game, &mut ctx).unwrap();

        assert!(game.turn_store.skip_next_turn.contains(&alice));
        assert_eq!(game.turn_store.skip_next_turn.len(), 1);
    }

    #[test]
    fn test_skip_turn_clone_box() {
        let effect = SkipTurnEffect::you();
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("SkipTurnEffect"));
    }
}
