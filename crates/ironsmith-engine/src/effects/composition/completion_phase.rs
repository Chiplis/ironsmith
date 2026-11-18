//! Actual original-phase requests forwarded through retained owner decorators.

use crate::effect::EffectOutcome;
use crate::effects::{
    CompletedEffectOutputs, ExecutionContext, ExecutionError, SimultaneousEffectCommit,
    SimultaneousEffectCompletion,
};
use crate::game_state::GameState;

/// Keep scalar compatibility and retained packets distinct across a phase
/// handoff. The request owns its input; forwarding never clones a packet or
/// reconstructs a child program from its aggregate.
pub(crate) enum CompletionPhase {
    OriginalOutcome(EffectOutcome),
    OriginalOutputs(CompletedEffectOutputs),
    DrawOutcome(EffectOutcome),
    DrawOutputs(CompletedEffectOutputs),
}

impl CompletionPhase {
    /// Recording decorators update the actual input before child dispatch.
    /// Retained packets synchronize their observation views at this boundary;
    /// scalar inputs keep the existing scalar callback without a fake packet.
    pub(crate) fn update_original(&mut self, update: impl FnOnce(&mut EffectOutcome)) {
        match self {
            Self::OriginalOutcome(original) | Self::DrawOutcome(original) => update(original),
            Self::OriginalOutputs(original) | Self::DrawOutputs(original) => {
                update(&mut original.outcome);
                original.synchronize_observations();
            }
        }
    }

    /// Consume the actual child owner once in exactly the requested phase.
    /// This dispatch does not establish readiness, grouping, scope or ordering;
    /// those contracts remain with the coordinator and retained decorators.
    pub(crate) fn dispatch(
        self,
        owner: Box<dyn SimultaneousEffectCompletion>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
        match self {
            Self::OriginalOutcome(original) => {
                owner.complete_original_phase_with_outputs(game, ctx, original)
            }
            Self::OriginalOutputs(original) => {
                owner.complete_original_phase_from_outputs(game, ctx, original)
            }
            Self::DrawOutcome(original) => {
                owner.prepare_draw_boundary_with_outputs(game, ctx, original)
            }
            Self::DrawOutputs(original) => {
                owner.prepare_draw_boundary_from_outputs(game, ctx, original)
            }
        }
    }
}

/// The final handoff owns the actual original input. Scalar compatibility calls
/// and retained-packet calls remain distinct; no aggregate is promoted into an
/// invented packet. Scope, recording and acknowledgement stay with decorators.
pub(crate) enum CompletionInput {
    Outcome(EffectOutcome),
    Outputs(CompletedEffectOutputs),
}

impl CompletionInput {
    pub(crate) fn update_original(&mut self, update: impl FnOnce(&mut EffectOutcome)) {
        match self {
            Self::Outcome(original) => update(original),
            Self::Outputs(original) => {
                update(&mut original.outcome);
                original.synchronize_observations();
            }
        }
    }

    /// Consume the actual completion owner in the caller's existing frame.
    /// This does not advance an original phase or establish scheduling readiness.
    pub(crate) fn dispatch(
        self,
        owner: Box<dyn SimultaneousEffectCompletion>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        match self {
            Self::Outcome(original) => owner.complete_with_outputs(game, ctx, original),
            Self::Outputs(original) => owner.complete_from_original_outputs(game, ctx, original),
        }
    }
}

/// Completion owners that decorate each phase of another prepared effect.
/// The adapter preserves which entry point the caller used and transfers the
/// original input by value. Each decorator owns its state and scope policy.
pub(crate) trait CompletionDecorator: Send {
    fn phase_status(&self) -> crate::effects::OriginalPhaseStatus;
    fn freeze_inner(&mut self, game: &mut GameState) -> Result<(), ExecutionError>;
    fn observe_inner(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: &mut EffectOutcome,
    ) -> Result<(), ExecutionError>;
    fn advance(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        phase: CompletionPhase,
    ) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError>;
    fn finish(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: CompletionInput,
    ) -> Result<CompletedEffectOutputs, ExecutionError>;
}

impl<T: CompletionDecorator> SimultaneousEffectCompletion for T {
    fn original_phase_status(&self) -> crate::effects::OriginalPhaseStatus {
        self.phase_status()
    }

    fn freeze(&mut self, game: &mut GameState) -> Result<(), ExecutionError> {
        self.freeze_inner(game)
    }

    fn observe_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: &mut EffectOutcome,
    ) -> Result<(), ExecutionError> {
        self.observe_inner(game, ctx, original)
    }

    fn complete_original_phase_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
        self.advance(game, ctx, CompletionPhase::OriginalOutcome(original))
    }

    fn complete_original_phase_from_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: CompletedEffectOutputs,
    ) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
        self.advance(game, ctx, CompletionPhase::OriginalOutputs(original))
    }

    fn prepare_draw_boundary_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
        self.advance(game, ctx, CompletionPhase::DrawOutcome(original))
    }

    fn prepare_draw_boundary_from_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: CompletedEffectOutputs,
    ) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
        self.advance(game, ctx, CompletionPhase::DrawOutputs(original))
    }

    fn complete_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        self.finish(game, ctx, CompletionInput::Outcome(original))
    }

    fn complete_from_original_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: CompletedEffectOutputs,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        self.finish(game, ctx, CompletionInput::Outputs(original))
    }

    fn complete(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.complete_with_outputs(game, ctx, original)
            .map(CompletedEffectOutputs::into_outcome)
    }
}
