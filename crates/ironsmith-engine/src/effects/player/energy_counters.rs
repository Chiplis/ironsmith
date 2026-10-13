//! Energy counters effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::{CompletedEffectOutputs, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::object::CounterType;
pub use ironsmith_core::EnergyCountersEffect;

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
impl EffectExecutor for EnergyCountersEffect {
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
            CounterType::Energy, self.count.clone(), self.player.clone(),
        ).execute_selected_with_outputs(game, ctx)
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
