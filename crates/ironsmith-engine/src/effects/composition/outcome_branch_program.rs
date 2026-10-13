//! Branch on an exact child result without publishing a global EffectId binding.
use crate::effect::{Effect, EffectOutcome};
use crate::effects::{CompletedEffectOutputs, EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;

#[derive(Debug, Clone, Copy)]
pub(crate) enum OutcomeBranchTest {
    PositiveCount,
    HasObjects,
}

#[derive(Debug, Clone)]
pub(crate) struct OutcomeBranchProgram {
    pub producer: Effect,
    pub test: OutcomeBranchTest,
    pub if_true: Vec<Effect>,
    pub if_false: Vec<Effect>,
}

impl EffectExecutor for OutcomeBranchProgram {
    fn library_movement_policy(
        &self,
    ) -> ironsmith_core::effect_capabilities::LibraryMovementPolicy {
        ironsmith_core::effect_capabilities::LibraryMovementPolicy::Inspect
    }
    fn inspect_library_movement_override(
        &self,
        context: &mut ironsmith_core::library_movement::LibraryMovementContext,
    ) -> Option<bool> {
        use ironsmith_core::library_movement::LibraryMovement;
        Some(
            self.producer.inspect_library_movement(context)
                || context.inspect_branches([self.if_true.as_slice(), self.if_false.as_slice()]),
        )
    }
    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) {
        visitor(&self.producer);
        for child in self.if_true.iter().chain(&self.if_false) {
            visitor(child);
        }
    }
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
        super::execute_transaction(
            game,
            ctx,
            || CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                let producer = self.producer.0.execute_child_with_outputs(game, ctx)?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                let result = producer.outcome.instruction_result();
                let yes = match self.test {
                    OutcomeBranchTest::PositiveCount => result.count_or_zero() > 0,
                    OutcomeBranchTest::HasObjects => {
                        result.objects().is_some_and(|objects| !objects.is_empty())
                    }
                };
                let branch = if yes { &self.if_true } else { &self.if_false };
                let mut outputs = vec![producer];
                for effect in branch {
                    if ctx.resolution_stopped() { break; }
                    outputs.push(effect.0.execute_child_with_outputs(game, ctx)?);
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::count(0),
                        ));
                    }
                    if ctx.resolution_stopped() {
                        break;
                    }
                }
                Ok(CompletedEffectOutputs::from_children(
                    outputs,
                    EffectOutcome::aggregate,
                ))
            },
        )
    }
}
