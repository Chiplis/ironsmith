use crate::effect::EffectOutcome;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;

/// Ends the current combat phase using the ordered CR 724.2 procedure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EndCombatPhaseEffect;

impl EndCombatPhaseEffect {
    pub const fn new() -> Self {
        Self
    }
}

impl EffectExecutor for EndCombatPhaseEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        super::end_turn::execute_ending_procedure_with_outputs(
            game,
            ctx,
            super::end_turn::EndingProcedure::CombatPhase,
        )
    }
}
