//! Win the game effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
pub use ironsmith_core::WinTheGameEffect;

/// Effect that causes a player to win the game.
///
/// Checks for effects that prevent winning (e.g., opponent has Platinum Angel).
/// When a player wins, all other players lose.
///
/// # Fields
///
/// * `player` - The player who wins the game
///
/// # Example
///
/// ```ignore
/// // You win the game (alternate win condition)
/// let effect = WinTheGameEffect::you();
///
/// // Target player wins the game
/// let effect = WinTheGameEffect::new(PlayerFilter::Any);
/// ```
impl EffectExecutor for WinTheGameEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;

        // Check if player can win the game (Platinum Angel opponent effect)
        if !game.can_win_game(player_id) {
            return Ok(EffectOutcome::prevented());
        }

        // Player wins - mark all other players as lost
        let losing_players: Vec<_> = game
            .players
            .iter()
            .filter(|other_player| other_player.id != player_id && other_player.is_in_game())
            .map(|other_player| other_player.id)
            .collect();
        for losing_player in losing_players {
            game.mark_player_lost(losing_player);
        }
        Ok(EffectOutcome::resolved())
    }
}
