//! Shuffle library effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::{ChooseSpec, PlayerFilter};

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
    /// Target metadata, when this effect targets a player.
    pub target_spec: Option<ChooseSpec>,
}

impl ShuffleLibraryEffect {
    /// Create a new shuffle library effect.
    pub fn new(player: PlayerFilter) -> Self {
        let target_spec = match &player {
            PlayerFilter::Target(inner) => {
                Some(ChooseSpec::target(ChooseSpec::Player((**inner).clone())))
            }
            _ => None,
        };
        Self {
            player,
            target_spec,
        }
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

        game.shuffle_player_library(player_id);

        Ok(EffectOutcome::resolved())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        self.target_spec.as_ref()
    }

    fn target_description(&self) -> &'static str {
        "player to shuffle"
    }
}
