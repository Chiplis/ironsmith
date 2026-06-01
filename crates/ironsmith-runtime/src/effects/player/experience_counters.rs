//! Experience counters effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::object::CounterType;
pub use ironsmith_core::ExperienceCountersEffect;

/// Effect that gives a player experience counters.
///
/// # Fields
///
/// * `count` - How many experience counters to add (can be fixed or variable)
/// * `player` - Which player receives the experience counters
///
/// # Example
///
/// ```ignore
/// // Get 1 experience counter
/// let effect = ExperienceCountersEffect::you(1);
/// ```
impl EffectExecutor for ExperienceCountersEffect {
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
            CounterType::Experience,
            count,
            Some(ctx.source),
            Some(ctx.controller),
        ) {
            outcome = outcome.with_event(event);
        }

        Ok(outcome)
    }
}
