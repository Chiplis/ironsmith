//! Lose the game effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::PlayerFilter;

/// Effect that causes a player to lose the game.
///
/// Checks for effects that prevent losing (e.g., Platinum Angel).
///
/// # Fields
///
/// * `player` - The player who loses the game
///
/// # Example
///
/// ```ignore
/// // Target player loses the game
/// let effect = LoseTheGameEffect::new(PlayerFilter::Opponent);
///
/// // You lose the game (alternate win condition trigger)
/// let effect = LoseTheGameEffect::you();
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct LoseTheGameEffect {
    /// The player who loses the game.
    pub player: PlayerFilter,
}

impl LoseTheGameEffect {
    /// Create a new lose the game effect.
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }

    /// The controller loses the game.
    pub fn you() -> Self {
        Self::new(PlayerFilter::You)
    }

    /// Target opponent loses the game.
    pub fn opponent() -> Self {
        Self::new(PlayerFilter::Opponent)
    }
}

impl EffectExecutor for LoseTheGameEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;

        // Check if player can lose the game (Platinum Angel effect)
        if !game.can_lose_game(player_id) {
            return Ok(EffectOutcome::prevented());
        }

        if let Some(player) = game.player_mut(player_id) {
            player.has_lost = true;
        }
        Ok(EffectOutcome::resolved())
    }
}
