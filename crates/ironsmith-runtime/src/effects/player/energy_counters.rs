//! Energy counters effect implementation.

use crate::effect::{EffectOutcome, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::object::CounterType;
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

        let mut outcome = EffectOutcome::count(count as i32);
        if let Some(event) = game.add_player_counters_with_source(
            player_id,
            CounterType::Energy,
            count,
            Some(ctx.source),
            Some(ctx.controller),
        ) {
            outcome = outcome.with_event(event);
        }

        Ok(outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::EventKind;
    use crate::ids::PlayerId;

    #[test]
    fn energy_counters_effect_emits_markers_changed_event() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let outcome = EnergyCountersEffect::you(3)
            .execute(&mut game, &mut ctx)
            .expect("energy counters should resolve");

        assert_eq!(game.player(alice).expect("alice exists").energy_counters, 3);
        assert!(
            outcome
                .events
                .iter()
                .any(|event| event.kind() == EventKind::MarkersChanged),
            "adding player energy counters should emit MarkersChangedEvent"
        );
    }
}
