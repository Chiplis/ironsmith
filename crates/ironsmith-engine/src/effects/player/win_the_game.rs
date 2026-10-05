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

        // CR 104.3h: with a limited range of influence, "wins the game"
        // instead makes each opponent in range lose. Those are ordinary
        // losses, so an opponent who can't lose the game doesn't. Otherwise
        // the game simply ends with this player winning.
        let limited_range = game.limited_range_of_influence().is_some();
        let losing_players = game
            .players
            .iter()
            .filter(|player| {
                player.id != player_id
                    && player.is_in_game()
                    && game.are_opponents(player_id, player.id)
                    && game.player_is_within_range(player_id, player.id)
                    && (!limited_range || game.can_lose_game(player.id))
            })
            .map(|player| player.id)
            .collect::<Vec<_>>();
        game.mark_players_lost_simultaneously(&losing_players)?;
        Ok(EffectOutcome::resolved())
    }
}
