//! Reveal top card effect implementation.

use crate::effect::{EffectOutcome, EffectResult};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::filter::PlayerFilter;
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;

/// Effect that reveals the top card of a player's library and tags it.
///
/// This is a composable primitive for effects like Goblin Guide.
#[derive(Debug, Clone, PartialEq)]
pub struct RevealTopEffect {
    pub player: PlayerFilter,
    pub tag: Option<TagKey>,
}

impl RevealTopEffect {
    /// Create a new reveal-top effect, optionally tagging the revealed card.
    pub fn new(player: PlayerFilter, tag: Option<TagKey>) -> Self {
        Self { player, tag }
    }

    /// Create a tagged reveal-top effect.
    pub fn tagged(player: PlayerFilter, tag: impl Into<TagKey>) -> Self {
        Self::new(player, Some(tag.into()))
    }
}

impl EffectExecutor for RevealTopEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;

        let top_card_id = game
            .player(player_id)
            .and_then(|p| p.library.last().copied());

        let Some(card_id) = top_card_id else {
            return Ok(EffectOutcome::count(0));
        };

        if let Some(obj) = game.object(card_id)
            && let Some(tag) = &self.tag
        {
            let snapshot = ObjectSnapshot::from_object(obj, game);
            ctx.set_tagged_objects(tag.clone(), vec![snapshot]);
        }

        Ok(EffectOutcome::from_result(EffectResult::Count(1)))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}
