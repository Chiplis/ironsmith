//! Hold an authored original subtree ahead of its enclosing additions.
use crate::effect::EffectOutcome;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;

/// Compose an already selected authored original with its enclosing additions.
/// The child owns the entire internal subtree, including its internal additions;
/// the enclosing cohort holds the returned additions until all originals finish.
/// This boundary does not make arbitrary combined proposals phase-separable or
/// permit their child instructions to join a wider simultaneous cohort.
pub(crate) fn defer_authored_original_additions_with_outputs(
    mut original: crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
    additions: Box<dyn crate::effects::SimultaneousEffectCompletion>,
) -> crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs> {
    original.completion = Some(match original.completion.take() {
        Some(continuation) => Box::new(HeldAuthoredOriginal {
            original: continuation,
            additions,
        }),
        None => additions,
    });
    original
}

/// The retained authored subtree is one original of its enclosing expansion.
/// Finish that subtree in its existing order before exposing outer additions.
struct HeldAuthoredOriginal {
    original: Box<dyn crate::effects::SimultaneousEffectCompletion>,
    additions: Box<dyn crate::effects::SimultaneousEffectCompletion>,
}

impl HeldAuthoredOriginal {
    fn finish_original(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: crate::effects::composition::CompletionInput,
    ) -> Result<
        (
            crate::effects::CompletedEffectOutputs,
            Box<dyn crate::effects::SimultaneousEffectCompletion>,
        ),
        ExecutionError,
    > {
        let Self {
            original: continuation,
            additions,
        } = *self;
        let outputs = original.dispatch(continuation, game, ctx)?;
        Ok((outputs, additions))
    }

    fn advance_original(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: crate::effects::composition::CompletionInput,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        ExecutionError,
    > {
        let (outputs, additions) = self.finish_original(game, ctx, original)?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::SimultaneousEffectCommit::finished(
                crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            ));
        }
        Ok(crate::effects::SimultaneousEffectCommit {
            outcome: outputs,
            completion: Some(additions),
        })
    }

    fn finish(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: crate::effects::composition::CompletionInput,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        let (outputs, additions) = self.finish_original(game, ctx, original)?;
        additions.complete_from_original_outputs(game, ctx, outputs)
    }
}

impl crate::effects::SimultaneousEffectCompletion for HeldAuthoredOriginal {
    fn original_phase_status(&self) -> crate::effects::OriginalPhaseStatus {
        crate::effects::OriginalPhaseStatus::Retained
    }

    fn complete_original_phase_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        ExecutionError,
    > {
        self.advance_original(
            game,
            ctx,
            crate::effects::composition::CompletionInput::Outcome(original),
        )
    }

    fn complete_original_phase_from_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: crate::effects::CompletedEffectOutputs,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        ExecutionError,
    > {
        self.advance_original(
            game,
            ctx,
            crate::effects::composition::CompletionInput::Outputs(original),
        )
    }

    fn prepare_draw_boundary_with_outputs(
        self: Box<Self>,
        _game: &mut GameState,
        _ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        ExecutionError,
    > {
        Ok(crate::effects::SimultaneousEffectCommit {
            outcome: crate::effects::CompletedEffectOutputs::aggregate_only(original),
            completion: Some(self),
        })
    }

    fn prepare_draw_boundary_from_outputs(
        self: Box<Self>,
        _game: &mut GameState,
        _ctx: &mut ExecutionContext,
        original: crate::effects::CompletedEffectOutputs,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        ExecutionError,
    > {
        Ok(crate::effects::SimultaneousEffectCommit {
            outcome: original,
            completion: Some(self),
        })
    }

    fn observe_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        outcome: &mut EffectOutcome,
    ) -> Result<(), ExecutionError> {
        self.original.observe_original(game, ctx, outcome)
    }

    fn freeze(&mut self, game: &mut GameState) -> Result<(), ExecutionError> {
        self.original.freeze(game)
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
