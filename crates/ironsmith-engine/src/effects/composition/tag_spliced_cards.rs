//! Tag the cards spliced onto a spell (CR 702.47) for later reference.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_objects_from_spec;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
pub use ironsmith_core::TagSplicedCardsEffect;

impl EffectExecutor for TagSplicedCardsEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // A spell that already left the stack has no entry to read; that is
        // an empty capture, not a failure of the following instructions.
        let spells = resolve_objects_from_spec(game, &self.spell, ctx).unwrap_or_default();
        let mut snapshots: Vec<ObjectSnapshot> = Vec::new();
        for spell in spells {
            let Some(entry) = game
                .stack
                .iter()
                .find(|entry| entry.object_id == spell && !entry.is_ability)
            else {
                continue;
            };
            for stable_id in &entry.spliced_cards {
                let Some(card) = game
                    .find_object_by_stable_id(*stable_id)
                    .and_then(|id| game.object(id))
                else {
                    continue;
                };
                if !snapshots
                    .iter()
                    .any(|existing| existing.stable_id == card.stable_id)
                {
                    snapshots.push(ObjectSnapshot::from_object(card, game));
                }
            }
        }
        let object_ids = snapshots
            .iter()
            .map(|snapshot| snapshot.object_id)
            .collect::<Vec<_>>();
        ctx.set_tagged_objects(self.tag.clone(), snapshots);
        Ok(EffectOutcome::with_objects(object_ids))
    }
}
