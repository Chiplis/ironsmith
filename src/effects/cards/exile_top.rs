//! Exile top cards of library effect implementation.

use crate::effect::{EffectOutcome, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::PlayerFilter;
use crate::zone::Zone;

/// Effect that exiles cards from the top of a player's library.
#[derive(Debug, Clone, PartialEq)]
pub struct ExileTopOfLibraryEffect {
    /// How many cards to exile.
    pub count: Value,
    /// Which player's library to exile from.
    pub player: PlayerFilter,
}

impl ExileTopOfLibraryEffect {
    /// Create a new exile-top effect.
    pub fn new(count: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            count: count.into(),
            player,
        }
    }
}

impl EffectExecutor for ExileTopOfLibraryEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let count = resolve_value(game, &self.count, ctx)?.max(0) as usize;

        let top_cards = game
            .player(player_id)
            .map(|p| {
                let lib_len = p.library.len();
                let exile_count = count.min(lib_len);
                p.library[lib_len.saturating_sub(exile_count)..].to_vec()
            })
            .unwrap_or_default();

        let mut moved = 0i32;
        for card_id in top_cards {
            if game.move_object(card_id, Zone::Exile).is_some() {
                moved += 1;
            }
        }

        Ok(EffectOutcome::count(moved))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}
