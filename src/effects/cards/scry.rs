//! Scry effect implementation.

use crate::decisions::{ScrySpec, make_decision};
use crate::effect::{EffectOutcome, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::events::{KeywordActionEvent, KeywordActionKind};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::target::PlayerFilter;
use crate::triggers::TriggerEvent;

/// Effect that lets a player scry N cards.
///
/// Per Rule 701.18, look at the top N cards, then put any number on the bottom
/// of the library in any order and the rest on top in any order.
///
/// # Fields
///
/// * `count` - Number of cards to scry
/// * `player` - The player who scries
///
/// # Example
///
/// ```ignore
/// // Scry 2
/// let effect = ScryEffect::new(2, PlayerFilter::You);
///
/// // Scry 1
/// let effect = ScryEffect::you(1);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ScryEffect {
    /// Number of cards to scry.
    pub count: Value,
    /// The player who scries.
    pub player: PlayerFilter,
}

impl ScryEffect {
    /// Create a new scry effect.
    pub fn new(count: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            count: count.into(),
            player,
        }
    }

    /// The controller scries N.
    pub fn you(count: impl Into<Value>) -> Self {
        Self::new(count, PlayerFilter::You)
    }
}

impl EffectExecutor for ScryEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let count = resolve_value(game, &self.count, ctx)?.max(0) as usize;

        if count == 0 {
            return Ok(EffectOutcome::count(0));
        }

        // Get the top N cards (they're at the end of the library vec)
        let top_cards: Vec<ObjectId> = game
            .player(player_id)
            .map(|p| {
                let lib_len = p.library.len();
                let scry_count = count.min(lib_len);
                p.library[lib_len.saturating_sub(scry_count)..].to_vec()
            })
            .unwrap_or_default();

        if top_cards.is_empty() {
            return Ok(EffectOutcome::count(0));
        }

        let scry_count = top_cards.len();

        // Ask player which cards to put on bottom using the new spec-based system
        let spec = ScrySpec::new(ctx.source, top_cards.clone());
        let cards_to_bottom: Vec<ObjectId> = make_decision(
            game,
            &mut ctx.decision_maker,
            player_id,
            Some(ctx.source),
            spec,
        )
        .into_iter()
        .filter(|c| top_cards.contains(c))
        .collect();

        // Remove the scried cards from library temporarily
        if let Some(p) = game.player_mut(player_id) {
            let lib_len = p.library.len();
            p.library.truncate(lib_len.saturating_sub(scry_count));
        }

        // Put cards going to bottom first (they go under the remaining library)
        // Then put the rest back on top
        let cards_to_top: Vec<ObjectId> = top_cards
            .iter()
            .filter(|c| !cards_to_bottom.contains(c))
            .copied()
            .collect();

        if let Some(p) = game.player_mut(player_id) {
            // Insert bottom cards at the beginning of library
            for &card_id in cards_to_bottom.iter().rev() {
                p.library.insert(0, card_id);
            }
            // Add top cards back to end (top of library)
            p.library.extend(cards_to_top);
        }

        Ok(
            EffectOutcome::count(scry_count as i32).with_event(TriggerEvent::new(
                KeywordActionEvent::new(
                    KeywordActionKind::Scry,
                    player_id,
                    ctx.source,
                    scry_count as u32,
                ),
            )),
        )
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}
