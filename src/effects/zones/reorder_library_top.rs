//! Reorder top of library effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::tag::TagKey;
use crate::zone::Zone;

/// Effect that lets a player reorder a tagged set of cards that are currently
/// on top of a library.
///
/// This powers "Put them back in any order" after a `LookAtTopCardsEffect`.
#[derive(Debug, Clone, PartialEq)]
pub struct ReorderLibraryTopEffect {
    pub tag: TagKey,
}

impl ReorderLibraryTopEffect {
    pub fn new(tag: impl Into<TagKey>) -> Self {
        Self { tag: tag.into() }
    }
}

fn normalize_order_response(
    response: Vec<crate::ids::ObjectId>,
    original: &[crate::ids::ObjectId],
) -> Vec<crate::ids::ObjectId> {
    let mut remaining = original.to_vec();
    let mut out = Vec::with_capacity(original.len());
    for id in response {
        if let Some(pos) = remaining.iter().position(|x| *x == id) {
            out.push(id);
            remaining.remove(pos);
        }
    }
    out.extend(remaining);
    out
}

impl EffectExecutor for ReorderLibraryTopEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        use crate::decisions::make_decision;
        use crate::decisions::specs::OrderLibraryTopSpec;

        let Some(snapshots) = ctx.tagged_objects.get(&self.tag) else {
            return Ok(EffectOutcome::resolved());
        };

        let mut cards: Vec<_> = snapshots.iter().map(|s| s.object_id).collect();
        cards.retain(|id| {
            game.object(*id)
                .is_some_and(|obj| obj.zone == Zone::Library)
        });
        if cards.len() <= 1 {
            return Ok(EffectOutcome::resolved());
        }

        // All tagged cards should be in the same library; use their owner.
        let owners: std::collections::HashSet<_> = cards
            .iter()
            .filter_map(|id| game.object(*id).map(|obj| obj.owner))
            .collect();
        if owners.len() != 1 {
            return Ok(EffectOutcome::resolved());
        }
        let library_owner = *owners.iter().next().unwrap();

        let Some(player) = game.player(library_owner) else {
            return Ok(EffectOutcome::resolved());
        };

        // Preserve the current top-to-bottom order as the default.
        let mut current_top_to_bottom = Vec::new();
        for &id in player.library.iter().rev() {
            if cards.contains(&id) {
                current_top_to_bottom.push(id);
            }
        }
        if current_top_to_bottom.len() <= 1 {
            return Ok(EffectOutcome::resolved());
        }

        let spec = OrderLibraryTopSpec::new(ctx.source, current_top_to_bottom.clone());
        let ordered =
            make_decision(game, &mut ctx.decision_maker, ctx.controller, Some(ctx.source), spec);
        let ordered = normalize_order_response(ordered, &current_top_to_bottom);

        if let Some(player) = game.player_mut(library_owner) {
            // Remove the affected cards from the library, preserving other cards.
            player.library.retain(|id| !current_top_to_bottom.contains(id));

            // Decision order is top-to-bottom; internal library is bottom-to-top.
            for id in ordered.iter().rev() {
                player.library.push(*id);
            }
        }

        Ok(EffectOutcome::resolved())
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}
