//! Move the top card of the pile exiled by the resolving source.

use crate::effect::{EffectOutcome, OutcomeObjectMemory};
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
pub use ironsmith_core::MoveTopSourceExiledToZoneEffect;

impl EffectExecutor for MoveTopSourceExiledToZoneEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let Some(object_id) = game.top_exiled_with_source_link(ctx.source) else {
            return Ok(EffectOutcome::impossible());
        };
        let memory = game
            .object(object_id)
            .map(|object| ObjectSnapshot::from_object(object, game))
            .map(|snapshot| OutcomeObjectMemory::from_snapshot(&snapshot))
            .into_iter()
            .collect();
        let Some(new_id) = game.move_object_by_effect(object_id, self.zone) else {
            return Ok(EffectOutcome::impossible());
        };

        Ok(EffectOutcome::with_objects(vec![new_id]).with_affected_object_memory(memory))
    }
}
