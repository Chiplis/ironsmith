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
        original.completion = Some(Box::new(DeferredReplacementPrograms {
            original: original.completion.take(),
            programs,
            bindings: Box::new(bindings),
        }));
    }
    original
}

struct DeferredReplacementPrograms {
    original: Option<Box<dyn crate::effects::SimultaneousEffectCompletion>>,
    programs: Vec<crate::events::processing::PreparedReplacementProgram>,
    bindings: ProgramBindings,
}

impl crate::effects::SimultaneousEffectCompletion for DeferredReplacementPrograms {
    fn prepare_draw_boundary_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        ExecutionError,
    > {
        // A retained replacement original is already stopped at its real draw.
        if self.original.is_some() {
            return Ok(crate::effects::SimultaneousEffectCommit {
                outcome: crate::effects::CompletedEffectOutputs::aggregate_only(original),
                completion: Some(self),
            });
        }
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

    fn observe_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        outcome: &mut EffectOutcome,
    ) -> Result<(), ExecutionError> {
        if let Some(original) = &mut self.original {
            original.observe_original(game, ctx, outcome)?;
        }
        Ok(())
    }

    fn freeze(&mut self, game: &mut GameState) -> Result<(), ExecutionError> {
        if let Some(original) = &mut self.original {
            original.freeze(game)?;
        }
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
        let outputs = if let Some(inner) = self.original {
            inner.complete_with_outputs(game, ctx, original)?
        } else {
            crate::effects::CompletedEffectOutputs::aggregate_only(original)
        };
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        let bindings_for_program = self.bindings;
        let completed =
            crate::effects::replacement::complete_deferred_replacement_programs_with_bindings(
                game,
                ctx,
                outputs.outcome.clone(),
                self.programs,
                |_, context, _| bindings_for_program(context),
            )?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        Ok(outputs.append_batch_program_outputs(completed))
    }
}
