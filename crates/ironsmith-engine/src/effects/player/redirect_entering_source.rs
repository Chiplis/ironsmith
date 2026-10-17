//! "put this creature into its owner's graveyard instead of onto the
//! battlefield" inside an as-enters program (Frankenstein's Monster).

use crate::effect::EffectOutcome;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;

pub use ironsmith_core::RedirectEnteringSourceEffect;

impl EffectExecutor for RedirectEnteringSourceEffect {
    fn execute(
        &self,
        _game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // Only an entry program's own entering source can be redirected; the
        // battlefield-entry owner moves it to the destination instead.
        if ctx.replacement.entry_counter_source != Some(ctx.source) {
            return Ok(EffectOutcome::count(0));
        }
        ctx.replacement.entry_redirect = Some(self.destination);
        Ok(EffectOutcome::count(1))
    }
}
