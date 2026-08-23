use crate::effect::{EffectOutcome, OutcomeStatus};
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
pub use ironsmith_core::NoteLifeTotalEffect;

impl EffectExecutor for NoteLifeTotalEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let Some(life_total) = game.note_life_total_for_source(ctx.source, ctx.controller) else {
            return Ok(EffectOutcome::from_status(OutcomeStatus::Impossible));
        };
        Ok(EffectOutcome::count(life_total))
    }
}
