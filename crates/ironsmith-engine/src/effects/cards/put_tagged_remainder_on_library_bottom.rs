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
        let chooser =
            crate::effects::helpers::resolve_player_filter_as_chooser(game, &self.player, ctx)?;
        move_tagged_remainder_to_library_bottom(
            game,
            ctx,
            &self.tag,
            self.keep_tagged.as_ref(),
            self.order,
            chooser,
        )
    }
}
