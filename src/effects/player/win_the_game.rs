//! Win the game effect implementation.

use crate::effect::{EffectOutcome, EffectResult};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::PlayerFilter;

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
#[derive(Debug, Clone, PartialEq)]
pub struct WinTheGameEffect {
    /// The player who wins the game.
    pub player: PlayerFilter,
}

impl WinTheGameEffect {
    /// Create a new win the game effect.
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }

    /// The controller wins the game.
    pub fn you() -> Self {
        Self::new(PlayerFilter::You)
    }
}

impl EffectExecutor for WinTheGameEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;

        // Check if player can win the game (Platinum Angel opponent effect)
        if !game.can_win_game(player_id) {
            return Ok(EffectOutcome::from_result(EffectResult::Prevented));
        }

        // Player wins - mark all other players as lost
        for other_player in &mut game.players {
            if other_player.id != player_id && other_player.is_in_game() {
                other_player.has_lost = true;
            }
        }
        Ok(EffectOutcome::resolved())
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}
