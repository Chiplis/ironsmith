//! Draw-the-game effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
pub use ironsmith_core::DrawTheGameEffect;

impl EffectExecutor for DrawTheGameEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let drawn = game.draw_game_for_controller_and_range(ctx.controller);
        Ok(EffectOutcome::count(drawn.len() as i32))
    }
}
