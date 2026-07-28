//! Shuffle graveyard into library effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::ShuffleLibraryEvent;
use crate::game_state::GameState;
use crate::target::PlayerFilter;
use crate::triggers::TriggerEvent;
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
            let _ = game.move_object_with_commander_options(
                card_id,
                Zone::Library,
                ctx.cause.clone(),
                &mut *ctx.decision_maker,
            );
        }

        game.shuffle_player_library(player_id);

        Ok(
            EffectOutcome::resolved().with_event(TriggerEvent::new_with_provenance(
                ShuffleLibraryEvent::new(player_id, ctx.cause.clone()),
                ctx.provenance,
            )),
        )
    }
}
