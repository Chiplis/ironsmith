//! Ticket counters effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::helpers::resolve_value;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
pub use ironsmith_core::TicketCountersEffect;

impl EffectExecutor for TicketCountersEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let count = resolve_value(game, &self.count, ctx)?.max(0);
        Ok(EffectOutcome::count(count))
    }
}
