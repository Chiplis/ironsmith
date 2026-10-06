//! Monstrosity effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::{CompletedEffectOutputs, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::object::CounterType;
pub use ironsmith_core::MonstrosityEffect;

/// Effect that makes a creature monstrous.
///
/// Monstrosity N is an activated ability that, when resolved:
/// 1. Checks if the creature is already monstrous (if so, does nothing)
/// 2. Puts N +1/+1 counters on the creature
/// 3. Marks the creature as monstrous
///
/// This enables "When this creature becomes monstrous" triggered abilities.
///
/// # Fields
///
/// * `n` - The number of +1/+1 counters to put on the creature
///
/// # Example
///
/// ```ignore
/// // Monstrosity 3
/// let effect = MonstrosityEffect::new(3);
///
/// // Monstrosity X (where X was chosen when activating)
/// let effect = MonstrosityEffect::new(Value::X);
/// ```
impl EffectExecutor for MonstrosityEffect {
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
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                let source = ctx.source;
                if game.object(source).is_none() {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::target_invalid(),
                    ));
                }
                if !game
                    .object(source)
                    .is_some_and(|object| object.zone == crate::zone::Zone::Battlefield)
                    || game.is_phased_out(source)
                    || super::designation::PermanentDesignation::Monstrous.is_present(game, source)
                {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                let amount = crate::effects::helpers::resolve_nonnegative_u32(game, &self.n, ctx)?;
                let event = crate::events::Event::put_counters(
                    source,
                    CounterType::PlusOnePlusOne,
                    amount,
                    ctx.cause.clone(),
                )
                .with_provenance(ctx.provenance);
                let placement = crate::effects::counters::execute_counter_placement_with_outputs(
                    game, ctx, event,
                )?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                let designation = super::designation::apply_designation(
                    game,
                    ctx,
                    source,
                    super::designation::PermanentDesignation::Monstrous,
                    amount,
                )?;
                let outcome = EffectOutcome::aggregate_with_primary_result(
                    designation.summary_projection(),
                    [placement.outcome.clone(), designation.clone()],
                );
                let mut outputs = placement;
                outputs
                    .retain_batch_children([CompletedEffectOutputs::aggregate_only(designation)]);
                Ok(outputs.project_aggregate(outcome))
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
