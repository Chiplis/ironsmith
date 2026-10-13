//! Experience counters effect implementation.

use crate::effect::{EffectOutcome, Value};
use crate::effects::{CompletedEffectOutputs, EffectExecutor};
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
        self.execute_with_outputs(game, ctx)
            .map(CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        super::player_counters::PlayerCountersEffect::new(
            CounterType::Experience, self.count.clone(), self.player.clone(),
        ).execute_selected_with_outputs(game, ctx)
    }
}
