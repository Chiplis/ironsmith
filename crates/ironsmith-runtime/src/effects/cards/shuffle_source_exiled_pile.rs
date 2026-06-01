//! Shuffle the face-down pile exiled by the resolving source.

use crate::effect::EffectOutcome;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
pub use ironsmith_core::ShuffleSourceExiledPileEffect;

impl EffectExecutor for ShuffleSourceExiledPileEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        game.shuffle_exiled_with_source_links(ctx.source);
        Ok(EffectOutcome::resolved())
    }
}
