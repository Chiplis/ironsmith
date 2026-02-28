//! Mill effect implementation.

use crate::effect::{EffectOutcome, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::target::PlayerFilter;
use crate::zone::Zone;

/// Effect that mills cards from a player's library to their graveyard.
///
/// # Fields
///
/// * `count` - How many cards to mill (can be fixed or variable)
/// * `player` - Which player mills
///
/// # Example
///
/// ```ignore
/// // Mill 3 cards
/// let effect = MillEffect::new(3, PlayerFilter::You);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct MillEffect {
    /// How many cards to mill.
    pub count: Value,
    /// Which player mills.
    pub player: PlayerFilter,
}

impl MillEffect {
    /// Create a new mill effect.
    pub fn new(count: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            count: count.into(),
            player,
        }
    }

    /// Create an effect where you mill cards.
    pub fn you(count: impl Into<Value>) -> Self {
        Self::new(count, PlayerFilter::You)
    }
}

impl EffectExecutor for MillEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let count = resolve_value(game, &self.count, ctx)?.max(0) as usize;

        // Get the cards to mill (from top of library)
        let milled: Vec<ObjectId> = game
            .player(player_id)
            .map(|p| {
                let lib_len = p.library.len();
                let mill_count = count.min(lib_len);
                p.library[lib_len.saturating_sub(mill_count)..].to_vec()
            })
            .unwrap_or_default();

        let milled_count = milled.len();

        // Remove from library
        if let Some(p) = game.player_mut(player_id) {
            p.library
                .truncate(p.library.len().saturating_sub(milled_count));
        }

        // Move each card to graveyard
        for card_id in milled {
            if let Some(obj) = game.object_mut(card_id) {
                obj.zone = Zone::Graveyard;
            }
            if let Some(p) = game.player_mut(player_id) {
                p.graveyard.push(card_id);
            }
        }

        Ok(EffectOutcome::count(milled_count as i32))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}
