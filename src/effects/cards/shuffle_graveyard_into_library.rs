//! Shuffle graveyard into library effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::PlayerFilter;
use crate::zone::Zone;

/// Effect that moves all cards from a player's graveyard to their library, then shuffles.
#[derive(Debug, Clone, PartialEq)]
pub struct ShuffleGraveyardIntoLibraryEffect {
    /// Which player's graveyard/library to use.
    pub player: PlayerFilter,
}

impl ShuffleGraveyardIntoLibraryEffect {
    /// Create a new effect for the provided player filter.
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }
}

impl EffectExecutor for ShuffleGraveyardIntoLibraryEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;

        let graveyard_cards = game
            .player(player_id)
            .map(|player| player.graveyard.clone())
            .unwrap_or_default();

        for card_id in graveyard_cards {
            let _ = game.move_object(card_id, Zone::Library);
        }

        if let Some(player) = game.player_mut(player_id) {
            player.shuffle_library();
        }

        Ok(EffectOutcome::resolved())
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}
