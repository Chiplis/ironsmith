//! Input bindings for every phase of a prepared object iteration.

use crate::effect::EffectOutcome;
use crate::effects::{
    ExecutionContext, ExecutionError, SimultaneousEffectCommit, SimultaneousEffectCompletion,
    SimultaneousEffectProposal,
};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;

#[derive(Debug, Clone)]
struct IterationBindings {
    object: ObjectId,
    player: PlayerId,
    tags: Vec<(TagKey, Vec<ObjectSnapshot>)>,
}

impl IterationBindings {
    fn run<T>(
        &self,
        ctx: &mut ExecutionContext,
        body: impl FnOnce(&mut ExecutionContext) -> Result<T, ExecutionError>,
    ) -> Result<T, ExecutionError> {
        let scope =
            IterationScope::enter(ctx, Some(self.object), Some(self.player), self.tags.clone());
        let result = body(ctx);
        scope.leave(ctx);
        result
    }
}

/// Own tag installation/removal and restoration for the whole selected
/// iteration or one captured phase. None keeps an absent binding absent;
/// it is distinct from installing an empty set. Reverse restoration also
/// preserves the existing precedence of repeated tag names.
pub(crate) fn with_iteration_tags<T>(
    ctx: &mut ExecutionContext,
    tags: impl IntoIterator<Item = (TagKey, Option<Vec<ObjectSnapshot>>)>,
    body: impl FnOnce(&mut ExecutionContext) -> Result<T, ExecutionError>,
) -> Result<T, ExecutionError> {
    let scope = IterationTagScope::enter(ctx, tags);
    let result = body(ctx);
    scope.leave(ctx);
    result
}

/// A retained tag frame uses the same installation and reverse restoration
/// owner as synchronous iteration helpers. Body writes remain visible until
/// this frame leaves; phase re-entry never re-installs stale input tags.
pub(super) struct IterationTagScope(Vec<(TagKey, Option<Vec<ObjectSnapshot>>)>);
impl IterationTagScope {
    pub(super) fn enter(
        ctx: &mut ExecutionContext,
        tags: impl IntoIterator<Item = (TagKey, Option<Vec<ObjectSnapshot>>)>,
    ) -> Self {
        Self(
            tags.into_iter()
                .map(|(tag, snapshots)| {
                    let previous = ctx.tagged_objects.remove(&tag);
                    if let Some(snapshots) = snapshots {
                        ctx.set_tagged_objects(tag.clone(), snapshots);
                    }
                    (tag, previous)
                })
                .collect(),
        )
    }
    pub(super) fn leave(self, ctx: &mut ExecutionContext) {
        for (tag, previous) in self.0.into_iter().rev() {
            match previous {
                Some(snapshots) => {
                    ctx.tagged_objects.insert(tag, snapshots);
                }
                None => {
                    ctx.tagged_objects.remove(&tag);
                }
            }
        }
    }
}

/// Selected object/player bindings live for the complete iteration body in
/// a participant frame. Only the installed fields are restored on completion.
pub(super) struct IterationScope {
    object: Option<Option<ObjectId>>,
    player: Option<Option<PlayerId>>,
    tags: IterationTagScope,
}
impl IterationScope {
    pub(super) fn enter(
        ctx: &mut ExecutionContext,
        object: Option<ObjectId>,
        player: Option<PlayerId>,
        tags: Vec<(TagKey, Vec<ObjectSnapshot>)>,
    ) -> Self {
        let tags = IterationTagScope::enter(
            ctx,
            tags.into_iter()
                .map(|(tag, snapshots)| (tag, Some(snapshots))),
        );
        let object = object
            .map(|object| std::mem::replace(&mut ctx.iteration.iterated_object, Some(object)));
        let player = player
            .map(|player| std::mem::replace(&mut ctx.iteration.iterated_player, Some(player)));
        Self {
            object,
            player,
            tags,
        }
    }
    pub(super) fn leave(self, ctx: &mut ExecutionContext) {
        if let Some(player) = self.player {
            ctx.iteration.iterated_player = player;
        }
        if let Some(object) = self.object {
            ctx.iteration.iterated_object = object;
        }
        self.tags.leave(ctx);
    }
}

/// Ordinary children and prepared phases share the same iteration-local
/// binding owner. Other successful child context outputs stay visible.
pub(crate) fn with_object_iteration<T>(
    ctx: &mut ExecutionContext,
    object: ObjectId,
    player: PlayerId,
    tags: Vec<(TagKey, Vec<ObjectSnapshot>)>,
    body: impl FnOnce(&mut ExecutionContext) -> Result<T, ExecutionError>,
) -> Result<T, ExecutionError> {
    IterationBindings {
        object,
        player,
        tags,
    }
    .run(ctx, body)
}

#[derive(Debug)]
struct ScopedIterationProposal {
    bindings: IterationBindings,
    inner: Box<dyn SimultaneousEffectProposal>,
}

struct ScopedIterationCompletion {
    bindings: IterationBindings,
    inner: Box<dyn SimultaneousEffectCompletion>,
}

impl SimultaneousEffectCompletion for ScopedIterationCompletion {
    fn original_phase_status(&self) -> crate::effects::OriginalPhaseStatus {
        self.inner.original_phase_status()
    }

    fn complete_original_phase_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut crate::effects::ExecutionContext,
        original: EffectOutcome,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        crate::effects::ExecutionError,
    > {
        let Self { bindings, inner } = *self;
        let mut receipt = bindings.run(ctx, |ctx| {
            inner.complete_original_phase_with_outputs(game, ctx, original)
        })?;
        receipt.completion = receipt.completion.map(|inner| {
            Box::new(Self { bindings, inner })
                as Box<dyn crate::effects::SimultaneousEffectCompletion>
        });
        Ok(receipt)
    }

    fn complete_original_phase_from_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut crate::effects::ExecutionContext,
        original: crate::effects::CompletedEffectOutputs,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        crate::effects::ExecutionError,
    > {
        let Self { bindings, inner } = *self;
        let mut receipt = bindings.run(ctx, |ctx| {
            inner.complete_original_phase_from_outputs(game, ctx, original)
        })?;
        receipt.completion = receipt.completion.map(|inner| {
            Box::new(Self { bindings, inner })
                as Box<dyn crate::effects::SimultaneousEffectCompletion>
        });
        Ok(receipt)
    }

    fn prepare_draw_boundary_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut crate::effects::ExecutionContext,
        original: EffectOutcome,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        crate::effects::ExecutionError,
    > {
        let Self { bindings, inner } = *self;
        let mut receipt = bindings.run(ctx, |ctx| {
            inner.prepare_draw_boundary_with_outputs(game, ctx, original)
        })?;
        receipt.completion = receipt.completion.map(|inner| {
            Box::new(Self { bindings, inner })
                as Box<dyn crate::effects::SimultaneousEffectCompletion>
        });
        Ok(receipt)
    }

    fn prepare_draw_boundary_from_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut crate::effects::ExecutionContext,
        original: crate::effects::CompletedEffectOutputs,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        crate::effects::ExecutionError,
    > {
        let Self { bindings, inner } = *self;
        let mut receipt = bindings.run(ctx, |ctx| {
            inner.prepare_draw_boundary_from_outputs(game, ctx, original)
        })?;
        receipt.completion = receipt.completion.map(|inner| {
            Box::new(Self { bindings, inner })
                as Box<dyn crate::effects::SimultaneousEffectCompletion>
        });
        Ok(receipt)
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
        let Self { bindings, inner } = *self;
        bindings.run(ctx, |ctx| inner.complete_with_outputs(game, ctx, original))
    }

    fn complete_from_original_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: crate::effects::CompletedEffectOutputs,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        let Self { bindings, inner } = *self;
        bindings.run(ctx, |ctx| {
            inner.complete_from_original_outputs(game, ctx, original)
        })
    }
}

impl SimultaneousEffectProposal for ScopedIterationProposal {
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
            Box::new(ScopedIterationCompletion { bindings, inner })
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

/// Restore only iteration-local bindings. Other child context outputs remain
/// visible, matching ordinary iteration and source-scoped execution.
pub(crate) fn scope_prepared_iteration(
    inner: Box<dyn SimultaneousEffectProposal>,
    object: ObjectId,
    player: PlayerId,
    tags: Vec<(TagKey, Vec<ObjectSnapshot>)>,
) -> Box<dyn SimultaneousEffectProposal> {
    Box::new(ScopedIterationProposal {
        bindings: IterationBindings {
            object,
            player,
            tags,
        },
        inner,
    })
}
