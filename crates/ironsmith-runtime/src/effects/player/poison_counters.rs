//! Poison counters effect implementation.

use crate::effect::{EffectOutcome, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::object::CounterType;
use crate::target::PlayerFilter;

/// Effect that gives a player poison counters.
///
/// # Fields
///
/// * `count` - How many poison counters to add (can be fixed or variable)
/// * `player` - Which player receives the poison counters
///
/// # Example
///
/// ```ignore
/// // Give yourself 2 poison counters (e.g., from a cost)
/// let effect = PoisonCountersEffect::you(2);
///
/// // Give a specific player 3 poison counters
/// let effect = PoisonCountersEffect::new(3, PlayerFilter::Specific(opponent_id));
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct PoisonCountersEffect {
    /// How many poison counters to add.
    pub count: Value,
    /// Which player receives the counters.
    pub player: PlayerFilter,
}

impl PoisonCountersEffect {
    /// Create a new poison counters effect.
    pub fn new(count: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            count: count.into(),
            player,
        }
    }

    /// Create an effect where you get poison counters.
    pub fn you(count: impl Into<Value>) -> Self {
        Self::new(count, PlayerFilter::You)
    }
}

impl EffectExecutor for PoisonCountersEffect {
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
            CounterType::Poison,
            count,
            Some(ctx.source),
            Some(ctx.controller),
        ) {
            outcome = outcome.with_event(event);
        }

        Ok(outcome)
    }
}
