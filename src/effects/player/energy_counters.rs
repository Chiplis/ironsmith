//! Energy counters effect implementation.

use crate::effect::{EffectOutcome, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::PlayerFilter;

/// Effect that gives a player energy counters.
///
/// # Fields
///
/// * `count` - How many energy counters to add (can be fixed or variable)
/// * `player` - Which player receives the energy counters
///
/// # Example
///
/// ```ignore
/// // Get 3 energy
/// let effect = EnergyCountersEffect::you(3);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct EnergyCountersEffect {
    /// How many energy counters to add.
    pub count: Value,
    /// Which player receives the counters.
    pub player: PlayerFilter,
}

impl EnergyCountersEffect {
    /// Create a new energy counters effect.
    pub fn new(count: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            count: count.into(),
            player,
        }
    }

    /// Create an effect where you get energy counters.
    pub fn you(count: impl Into<Value>) -> Self {
        Self::new(count, PlayerFilter::You)
    }
}

impl EffectExecutor for EnergyCountersEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let count = resolve_value(game, &self.count, ctx)?.max(0) as u32;

        if let Some(p) = game.player_mut(player_id) {
            p.energy_counters += count;
        }

        Ok(EffectOutcome::count(count as i32))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}
