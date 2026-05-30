//! Rad counters effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::object::CounterType;
pub use ironsmith_core::RadCountersEffect;

impl EffectExecutor for RadCountersEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let count = resolve_value(game, &self.count, ctx)?.max(0) as u32;

        let mut outcome = EffectOutcome::count(count as i32);
        if let Some(event) = game.add_player_counters_with_source(
            player_id,
            CounterType::Rad,
            count,
            Some(ctx.source),
            Some(ctx.controller),
        ) {
            outcome = outcome.with_event(event);
        }

        Ok(outcome)
    }
}
