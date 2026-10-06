use crate::effects::CompletedEffectOutputs;
use crate::effect::EffectOutcome;
use crate::effects::{EffectExecutor, consult_helpers::*};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
pub use ironsmith_core::PutTaggedRemainderOnLibraryBottomEffect;

impl EffectExecutor for PutTaggedRemainderOnLibraryBottomEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        let chooser =
            crate::effects::helpers::resolve_player_filter_as_chooser(game, &self.player, ctx)?;
        move_tagged_remainder_to_library_bottom_with_outputs(
            game,
            ctx,
            &self.tag,
            self.keep_tagged.as_ref(),
            self.order,
            chooser,
        )
    }
}
