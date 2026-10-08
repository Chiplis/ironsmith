//! Deferred additions share phase forwarding and draw routing across event families.
use super::ReplacementProgramBindings;
use crate::effect::EffectOutcome;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::processing::ReplacementEventContext;
use crate::game_state::GameState;

type ProgramBindings = Box<
    dyn Fn(&ReplacementEventContext) -> Result<ReplacementProgramBindings, ExecutionError> + Send,
>;

/// Retain the original continuation and append programs only after it finishes.
/// Event-family adapters provide bindings from their immutable captured event.
pub(crate) fn defer_replacement_programs_with_outputs(
    mut original: crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
    programs: Vec<crate::events::processing::PreparedReplacementProgram>,
    bindings: impl Fn(&ReplacementEventContext) -> Result<ReplacementProgramBindings, ExecutionError>
    + Send
    + 'static,
) -> crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs> {
    if !programs.is_empty() || original.completion.is_some() {
        let additions = Box::new(DeferredReplacementPrograms {
            programs,
            bindings: Box::new(bindings),
        });
        original.completion = Some(match original.completion.take() {
            Some(continuation) => Box::new(DeferredReplacementOriginal {
                original: continuation,
                additions,
            }),
            None => additions,
        });
    }
    original
}

/// The retained authored replacement subtree is one original of this expansion.
/// Finish that subtree in its existing order before exposing appended programs.
struct DeferredReplacementOriginal {
    original: Box<dyn crate::effects::SimultaneousEffectCompletion>,
    additions: Box<DeferredReplacementPrograms>,
}

impl DeferredReplacementOriginal {
    fn finish_original_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<
        (
            crate::effects::CompletedEffectOutputs,
            Box<DeferredReplacementPrograms>,
        ),
        ExecutionError,
    > {
        let Self {
            original: continuation,
            additions,
        } = *self;
        let outputs = continuation.complete_with_outputs(game, ctx, original)?;
        Ok((outputs, additions))
    }

    fn finish_original_from_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: crate::effects::CompletedEffectOutputs,
    ) -> Result<
        (
            crate::effects::CompletedEffectOutputs,
            Box<DeferredReplacementPrograms>,
        ),
        ExecutionError,
    > {
        let Self {
            original: continuation,
            additions,
        } = *self;
        let outputs = continuation.complete_from_original_outputs(game, ctx, original)?;
        Ok((outputs, additions))
    }
}

impl crate::effects::SimultaneousEffectCompletion for DeferredReplacementOriginal {
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
        let (outputs, additions) = self.finish_original_with_outputs(game, ctx, original)?;
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

    fn complete_original_phase_from_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: crate::effects::CompletedEffectOutputs,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        ExecutionError,
    > {
        let (outputs, additions) = self.finish_original_from_outputs(game, ctx, original)?;
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
        let (outputs, additions) = self.finish_original_with_outputs(game, ctx, original)?;
        additions.complete_with_original_outputs(game, ctx, outputs)
    }

    fn complete_from_original_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: crate::effects::CompletedEffectOutputs,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        let (outputs, additions) = self.finish_original_from_outputs(game, ctx, original)?;
        additions.complete_with_original_outputs(game, ctx, outputs)
    }
}

struct DeferredReplacementPrograms {
    programs: Vec<crate::events::processing::PreparedReplacementProgram>,
    bindings: ProgramBindings,
}

impl crate::effects::SimultaneousEffectCompletion for DeferredReplacementPrograms {
    fn original_phase_status(&self) -> crate::effects::OriginalPhaseStatus {
        crate::effects::OriginalPhaseStatus::Complete
    }

    fn prepare_draw_boundary_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        ExecutionError,
    > {
        self.prepare_draw_boundary_from_outputs(
            game,
            ctx,
            crate::effects::CompletedEffectOutputs::aggregate_only(original),
        )
    }

    fn prepare_draw_boundary_from_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: crate::effects::CompletedEffectOutputs,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        ExecutionError,
    > {
        let bindings_for_program = self.bindings;
        let programs = self
            .programs
            .into_iter()
            .map(|program| {
                let bindings = bindings_for_program(&program.context)?;
                Ok((program, bindings))
            })
            .collect::<Result<Vec<_>, ExecutionError>>()?;
        crate::effects::replacement::prepare_zone_draw_tail_with_outputs(
            game,
            ctx,
            original,
            programs,
            &[],
        )
    }

    fn freeze(&mut self, _game: &mut GameState) -> Result<(), ExecutionError> {
        Ok(())
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
        self.complete_with_original_outputs(
            game,
            ctx,
            crate::effects::CompletedEffectOutputs::aggregate_only(original),
        )
    }

    fn complete_from_original_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: crate::effects::CompletedEffectOutputs,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        self.complete_with_original_outputs(game, ctx, original)
    }
}

impl DeferredReplacementPrograms {
    /// Append to the actual original packet without projecting away its children.
    fn complete_with_original_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        mut outputs: crate::effects::CompletedEffectOutputs,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        // Added programs have no complete authored mapping in this owner.
        // An empty addition list preserves the original's existing coverage.
        if !self.programs.is_empty() {
            outputs.projections_complete = false;
        }
        let bindings_for_program = self.bindings;
        let programs = self.programs;
        crate::effects::replacement::complete_replacement_programs_with_original_outputs(
            game,
            ctx,
            outputs,
            |game, ctx, original| {
                crate::effects::replacement::complete_deferred_replacement_programs_with_bindings(
                    game,
                    ctx,
                    original,
                    programs,
                    |_, context, _| bindings_for_program(context),
                )
            },
        )
    }
}
