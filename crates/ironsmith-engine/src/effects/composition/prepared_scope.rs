//! Reinstall a prepared participant's input bindings for each proposal and completion phase.

use crate::effect::EffectOutcome;
use crate::effects::{
    ExecutionContext, ExecutionError, SimultaneousEffectCommit, SimultaneousEffectCompletion,
    SimultaneousEffectProposal,
};
use crate::game_state::{GameState, TargetAssignment};
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;

use super::prepared_iteration::with_object_iteration;

/// Captured inputs reapplied around each phase, including deferred continuations.
pub(crate) trait PreparedBindings: Send + Sized + 'static {
    fn with_bindings<T>(
        &self,
        ctx: &mut ExecutionContext,
        body: impl FnOnce(&mut ExecutionContext) -> Result<T, ExecutionError>,
    ) -> Result<T, ExecutionError>;

    fn bind_completion(
        self,
        inner: Box<dyn SimultaneousEffectCompletion>,
    ) -> Box<dyn SimultaneousEffectCompletion> {
        Box::new(ScopedCompletion {
            bindings: self,
            inner,
        })
    }
}

#[derive(Debug)]
struct ObjectIteration {
    object: ObjectId,
    player: PlayerId,
    tags: Vec<(TagKey, Vec<ObjectSnapshot>)>,
}

impl PreparedBindings for ObjectIteration {
    fn with_bindings<T>(
        &self,
        ctx: &mut ExecutionContext,
        body: impl FnOnce(&mut ExecutionContext) -> Result<T, ExecutionError>,
    ) -> Result<T, ExecutionError> {
        with_object_iteration(ctx, self.object, self.player, self.tags.clone(), body)
    }
}

impl PreparedBindings for Vec<TargetAssignment> {
    fn with_bindings<T>(
        &self,
        ctx: &mut ExecutionContext,
        body: impl FnOnce(&mut ExecutionContext) -> Result<T, ExecutionError>,
    ) -> Result<T, ExecutionError> {
        ctx.with_temp_target_assignments(self.clone(), body)
    }
}

pub(crate) fn scope_prepared_iteration(
    inner: Box<dyn SimultaneousEffectProposal>,
    object: ObjectId,
    player: PlayerId,
    tags: Vec<(TagKey, Vec<ObjectSnapshot>)>,
) -> Box<dyn SimultaneousEffectProposal> {
    Box::new(ScopedProposal {
        bindings: ObjectIteration {
            object,
            player,
            tags,
        },
        inner,
    })
}

pub(crate) fn scope_prepared_target_assignments(
    inner: Box<dyn SimultaneousEffectProposal>,
    assignments: Vec<TargetAssignment>,
) -> Box<dyn SimultaneousEffectProposal> {
    Box::new(ScopedProposal {
        bindings: assignments,
        inner,
    })
}

#[derive(Debug)]
struct ScopedProposal<B> {
    bindings: B,
    inner: Box<dyn SimultaneousEffectProposal>,
}

struct ScopedCompletion<B> {
    bindings: B,
    inner: Box<dyn SimultaneousEffectCompletion>,
}

impl<B: PreparedBindings> crate::effects::composition::CompletionDecorator for ScopedCompletion<B> {
    fn phase_status(&self) -> crate::effects::OriginalPhaseStatus {
        self.inner.original_phase_status()
    }

    fn freeze_inner(&mut self, game: &mut GameState) -> Result<(), ExecutionError> {
        self.inner.freeze(game)
    }

    fn observe_inner(
        &mut self,
        game: &mut GameState,
        ctx: &mut crate::effects::ExecutionContext,
        original: &mut EffectOutcome,
    ) -> Result<(), crate::effects::ExecutionError> {
        self.bindings
            .with_bindings(ctx, |ctx| self.inner.observe_original(game, ctx, original))
    }

    fn advance(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        phase: crate::effects::composition::CompletionPhase,
    ) -> Result<SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>, ExecutionError>
    {
        let Self { bindings, inner } = *self;
        let mut receipt = bindings.with_bindings(ctx, |ctx| phase.dispatch(inner, game, ctx))?;
        receipt.completion = receipt
            .completion
            .map(|inner| bindings.bind_completion(inner));
        Ok(receipt)
    }

    fn finish(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: crate::effects::composition::CompletionInput,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        let Self { bindings, inner } = *self;
        bindings.with_bindings(ctx, |ctx| original.dispatch(inner, game, ctx))
    }
}

impl<B: PreparedBindings + std::fmt::Debug> SimultaneousEffectProposal for ScopedProposal<B> {
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
        bindings.with_bindings(ctx, |ctx| inner.bind_damage_action(game, ctx, owner))
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
            .with_bindings(ctx, |ctx| self.inner.prepare_selection(game, ctx))
    }

    fn prepare_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        self.bindings
            .with_bindings(ctx, |ctx| self.inner.prepare_original(game, ctx))
    }

    fn seal_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        self.bindings
            .with_bindings(ctx, |ctx| self.inner.seal_original(game, ctx))
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
        let mut receipt =
            bindings.with_bindings(ctx, |ctx| inner.commit_original_with_outputs(game, ctx))?;
        receipt.completion = receipt
            .completion
            .map(|inner| bindings.bind_completion(inner));
        Ok(receipt)
    }

    fn commit(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let Self { bindings, inner } = *self;
        bindings.with_bindings(ctx, |ctx| inner.commit(game, ctx))
    }
}
