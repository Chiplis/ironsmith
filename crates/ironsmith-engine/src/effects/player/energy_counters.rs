//! Energy counters effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::{CompletedEffectOutputs, EffectExecutor};
use crate::effects::helpers::{resolve_player_filter, resolve_nonnegative_u32};
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
        let result = crate::effects::composition::execute_transaction(
            game,
            ctx,
            || CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                let player = resolve_player_filter(game, &self.player, ctx)?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                let count = resolve_nonnegative_u32(game, &self.count, ctx)?;
                let event = crate::events::Event::put_player_counters(
                    player,
                    CounterType::Energy,
                    count,
                    ctx.cause.clone(),
                )
                .with_provenance(ctx.provenance);
                crate::effects::counters::execute_player_counter_placement_with_outputs(
                    game, ctx, event,
                )
            },
        );
        // Preserve this adapter's existing neutral result for a suspended child,
        // including a child that failed after opening its decision. The shared
        // transaction owns rollback; an ordinary failure still propagates.
        if ctx.decision_maker.awaiting_choice() {
            return Ok(CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        result
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
