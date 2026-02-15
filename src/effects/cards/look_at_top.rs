//! Look at top cards effect implementation.

use crate::effect::{EffectOutcome, EffectResult};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::filter::PlayerFilter;
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;

/// Effect that looks at the top N cards of a player's library and tags them.
#[derive(Debug, Clone, PartialEq)]
pub struct LookAtTopCardsEffect {
    pub player: PlayerFilter,
    pub count: usize,
    pub tag: TagKey,
}

impl LookAtTopCardsEffect {
    pub fn new(player: PlayerFilter, count: usize, tag: impl Into<TagKey>) -> Self {
        Self {
            player,
            count,
            tag: tag.into(),
        }
    }
}

impl EffectExecutor for LookAtTopCardsEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let Some(player) = game.player(player_id) else {
            return Ok(EffectOutcome::count(0));
        };
        if self.count == 0 {
            return Ok(EffectOutcome::count(0));
        }

        let top_cards: Vec<_> = player
            .library
            .iter()
            .rev()
            .take(self.count)
            .copied()
            .collect();
        if top_cards.is_empty() {
            return Ok(EffectOutcome::count(0));
        }

        let snapshots: Vec<ObjectSnapshot> = top_cards
            .iter()
            .filter_map(|&id| {
                game.object(id)
                    .map(|obj| ObjectSnapshot::from_object(obj, game))
            })
            .collect();
        if snapshots.is_empty() {
            return Ok(EffectOutcome::count(0));
        }

        ctx.set_tagged_objects(self.tag.clone(), snapshots.clone());
        Ok(EffectOutcome::from_result(EffectResult::Count(
            snapshots.len() as i32,
        )))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}
