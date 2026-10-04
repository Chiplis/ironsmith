//! Experience counters effect implementation.

use crate::effect::{EffectOutcome, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_nonnegative_u32};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::object::CounterType;
use crate::target::PlayerFilter;

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
#[derive(Debug, Clone, PartialEq)]
pub struct ExperienceCountersEffect {
    /// How many experience counters to add.
    pub count: Value,
    /// Which player receives the counters.
    pub player: PlayerFilter,
}

impl ExperienceCountersEffect {
    /// Create a new experience counters effect.
    pub fn new(count: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            count: count.into(),
            player,
        }
    }

    /// Create an effect where you get experience counters.
    pub fn you(count: impl Into<Value>) -> Self {
        Self::new(count, PlayerFilter::You)
    }
}

impl EffectExecutor for ExperienceCountersEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let checkpoint = game.clone();
        let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        let result = (|| {
            let player = resolve_player_filter(game, &self.player, ctx)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            let count = resolve_nonnegative_u32(game, &self.count, ctx)?;
            let event = crate::events::Event::put_player_counters(
                player,
                CounterType::Experience,
                count,
                ctx.cause.clone(),
            )
            .with_provenance(ctx.provenance);
            crate::effects::counters::execute_player_counter_placement(game, ctx, event)
        })();
        if result.is_err() || ctx.decision_maker.awaiting_choice() {
            *game = checkpoint;
            context_checkpoint.restore(ctx);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
        }
        result
    }
}
