//! Shuffle graveyard into library effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::PlayerFilter;
use crate::zone::Zone;

/// Effect that moves all cards from a player's graveyard to their library, then shuffles.
#[derive(Debug, Clone, PartialEq)]
pub struct ShuffleGraveyardIntoLibraryEffect {
    /// Which player's graveyard/library to use.
    pub player: PlayerFilter,
    /// Preserve the longer authored "all cards from ... graveyard" surface.
    pub explicit_all_cards_from: bool,
}

impl ShuffleGraveyardIntoLibraryEffect {
    /// Create a new effect for the provided player filter.
    pub fn new(player: PlayerFilter) -> Self {
        Self {
            player,
            explicit_all_cards_from: false,
        }
    }

    pub fn with_all_cards_from_surface(player: PlayerFilter) -> Self {
        Self {
            player,
            explicit_all_cards_from: true,
        }
    }
    fn instruction(&self) -> crate::effects::ShuffleObjectsIntoLibraryEffect {
        crate::effects::ShuffleObjectsIntoLibraryEffect::new(
            crate::target::ChooseSpec::All(crate::target::ObjectFilter::default()
                .in_zone(Zone::Graveyard).owned_by(self.player.clone())),
            self.player.clone(),
        )
    }

}

impl EffectExecutor for ShuffleGraveyardIntoLibraryEffect {
    fn supports_simultaneous_player_action(&self) -> bool { true }
    fn prepare_simultaneous_player_action(&self, game: &GameState, ctx: &mut ExecutionContext)
        -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        self.instruction().prepare_simultaneous_player_action(game, ctx)
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // Keep zone replacement outcomes, exact receipts, actual move counts,
        // commander destination choices and rollback in the common owner.
        self.instruction().execute(game, ctx)
    }
}
