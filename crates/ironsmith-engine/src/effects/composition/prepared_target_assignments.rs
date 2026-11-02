//! Retained scalar target roles for every prepared lifecycle phase.
use crate::effect::EffectOutcome;
use crate::effects::{ExecutionContext,ExecutionError,SimultaneousEffectCommit,SimultaneousEffectCompletion,SimultaneousEffectProposal};
use crate::game_state::{GameState,TargetAssignment};
use crate::ids::PlayerId;
#[derive(Debug,Clone)]
struct TargetAssignmentBindings { assignments:Vec<TargetAssignment> }
impl TargetAssignmentBindings {
    fn run<T>(&self,ctx:&mut ExecutionContext,body:impl FnOnce(&mut ExecutionContext)->Result<T,ExecutionError>)->Result<T,ExecutionError>{
        ctx.with_temp_target_assignments(self.assignments.clone(),body)
    }
}
#[derive(Debug)]
struct ScopedTargetAssignmentsProposal {
    bindings: TargetAssignmentBindings,
    inner: Box<dyn SimultaneousEffectProposal>,
}

struct ScopedTargetAssignmentsCompletion {
    bindings: TargetAssignmentBindings,
    inner: Box<dyn SimultaneousEffectCompletion>,
}

impl ScopedTargetAssignmentsCompletion {
    fn advance(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        phase: crate::effects::composition::CompletionPhase,
    ) -> Result<SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>, ExecutionError>
    {
        let Self { bindings, inner } = *self;
        let mut receipt = bindings.run(ctx, |ctx| phase.dispatch(inner, game, ctx))?;
        receipt.completion = receipt.completion.map(|inner| {
            Box::new(Self { bindings, inner })
                as Box<dyn crate::effects::SimultaneousEffectCompletion>
        });
        Ok(receipt)
    }

    fn finish(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: crate::effects::composition::CompletionInput,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        let Self { bindings, inner } = *self;
        bindings.run(ctx, |ctx| original.dispatch(inner, game, ctx))
    }
}

impl SimultaneousEffectCompletion for ScopedTargetAssignmentsCompletion {
    fn original_phase_status(&self) -> crate::effects::OriginalPhaseStatus {
        self.inner.original_phase_status()
    }

    fn complete_original_phase_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>, ExecutionError>
    {
        self.advance(
            game,
            ctx,
            crate::effects::composition::CompletionPhase::OriginalOutcome(original),
        )
    }

    fn complete_original_phase_from_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: crate::effects::CompletedEffectOutputs,
    ) -> Result<SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>, ExecutionError>
    {
        self.advance(
            game,
            ctx,
            crate::effects::composition::CompletionPhase::OriginalOutputs(original),
        )
    }

    fn prepare_draw_boundary_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>, ExecutionError>
    {
        self.advance(
            game,
            ctx,
            crate::effects::composition::CompletionPhase::DrawOutcome(original),
        )
    }

    fn prepare_draw_boundary_from_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: crate::effects::CompletedEffectOutputs,
    ) -> Result<SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>, ExecutionError>
    {
        self.advance(
            game,
            ctx,
            crate::effects::composition::CompletionPhase::DrawOutputs(original),
        )
    }

    fn observe_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut crate::effects::ExecutionContext,
        original: &mut EffectOutcome,
    ) -> Result<(), crate::effects::ExecutionError> {
        self.bindings
            .run(ctx, |ctx| self.inner.observe_original(game, ctx, original))
    }

    fn freeze(&mut self, game: &mut GameState) -> Result<(), ExecutionError> {
        self.inner.freeze(game)
    }

    fn complete(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.complete_with_outputs(game, ctx, original)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn complete_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        self.finish(
            game,
            ctx,
            crate::effects::composition::CompletionInput::Outcome(original),
        )
    }

    fn complete_from_original_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: crate::effects::CompletedEffectOutputs,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        self.finish(
            game,
            ctx,
            crate::effects::composition::CompletionInput::Outputs(original),
        )
    }
}

impl SimultaneousEffectProposal for ScopedTargetAssignmentsProposal {
    fn has_simultaneous_originals(&self) -> bool {
        self.inner.has_simultaneous_originals()
    }

    fn nominal_payment_quantity(&self) -> Option<u64> {
        self.inner.nominal_payment_quantity()
    }

    fn damage_action_inputs(&self) -> Option<crate::effects::damage::DamageActionInputs> {
        self.inner.damage_action_inputs()
    }

    fn bind_damage_action(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        owner: &crate::effects::CompletedEffectOutputs,
    ) -> Result<crate::effects::DamageActionBinding, ExecutionError> {
        let Self { bindings, inner } = *self;
        bindings.run(ctx, |ctx| inner.bind_damage_action(game, ctx, owner))
    }
    fn declared_life_payment(&self) -> Option<(PlayerId, u32)> {
        self.inner.declared_life_payment()
    }

    fn declared_payment_resources(&self) -> Vec<crate::effects::PaymentResourceClaim> {
        self.inner.declared_payment_resources()
    }

    fn declared_life_payments(&self) -> Vec<(PlayerId, u32)> {
        self.inner.declared_life_payments()
    }

    fn prepare_selection(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        self.bindings
            .run(ctx, |ctx| self.inner.prepare_selection(game, ctx))
    }

    fn prepare_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        self.bindings
            .run(ctx, |ctx| self.inner.prepare_original(game, ctx))
    }

    fn seal_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        self.bindings
            .run(ctx, |ctx| self.inner.seal_original(game, ctx))
    }

    fn commit_original(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<SimultaneousEffectCommit, ExecutionError> {
        self.commit_original_with_outputs(game, ctx)
            .map(crate::effects::SimultaneousEffectCommit::into_aggregate)
    }

    fn commit_original_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>, ExecutionError>
    {
        let Self { bindings, inner } = *self;
        let mut receipt = bindings.run(ctx, |ctx| inner.commit_original_with_outputs(game, ctx))?;
        receipt.completion = receipt.completion.map(|inner| {
            Box::new(ScopedTargetAssignmentsCompletion { bindings, inner })
                as Box<dyn SimultaneousEffectCompletion>
        });
        Ok(receipt)
    }

    fn commit(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let Self { bindings, inner } = *self;
        bindings.run(ctx, |ctx| inner.commit(game, ctx))
    }
}


/// Scope inputs only; unrelated child outputs remain in the enclosing context.
pub(crate) fn scope_prepared_target_assignments(inner:Box<dyn SimultaneousEffectProposal>,assignments:Vec<TargetAssignment>)->Box<dyn SimultaneousEffectProposal>{
    Box::new(ScopedTargetAssignmentsProposal {bindings:TargetAssignmentBindings {assignments},inner})
}
