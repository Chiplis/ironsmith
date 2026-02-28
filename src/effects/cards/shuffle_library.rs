//! Shuffle library effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::PlayerFilter;

/// Effect that shuffles a player's library.
///
/// # Fields
///
/// * `player` - Which player's library to shuffle
///
/// # Example
///
/// ```ignore
/// // Shuffle your library
/// let effect = ShuffleLibraryEffect::you();
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ShuffleLibraryEffect {
    /// Which player's library to shuffle.
    pub player: PlayerFilter,
}

impl ShuffleLibraryEffect {
    /// Create a new shuffle library effect.
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }

    /// Create an effect to shuffle your library.
    pub fn you() -> Self {
        Self::new(PlayerFilter::You)
    }
}

impl EffectExecutor for ShuffleLibraryEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;

        if let Some(p) = game.player_mut(player_id) {
            p.shuffle_library();
        }

        Ok(EffectOutcome::resolved())
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}
