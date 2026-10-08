//! Retain an already-bound compound zone replacement across its first draw.
use super::ReplacementProgramBindings;
use crate::effect::{Effect, EffectOutcome};
use crate::effects::{
    CompletedEffectOutputs, ExecutionContext, ExecutionContextCheckpoint, ExecutionError,
    SimultaneousEffectCommit, SimultaneousEffectCompletion,
};
use crate::events::processing::PreparedReplacementProgram;
use crate::game_state::GameState;

type BoundProgram = (PreparedReplacementProgram, ReplacementProgramBindings);

struct ZoneTail {
    before: CompletedEffectOutputs,
    first: SimultaneousEffectCommit<CompletedEffectOutputs>,
    programs: Vec<BoundProgram>,
    followups: Vec<Effect>,
    scope: ExecutionContextCheckpoint,
}

fn append(before: CompletedEffectOutputs, child: CompletedEffectOutputs) -> CompletedEffectOutputs {
    let child_coverage = child.projections_complete;
    let mut outputs = before.append_replacement_outputs([child]);
    outputs.projections_complete &= child_coverage;
    outputs
}

impl SimultaneousEffectCompletion for ZoneTail {
    fn original_phase_status(&self) -> crate::effects::OriginalPhaseStatus {
        // This continuation is constructed only from added programs/follow-ups.
        // Their internal pending draws are additions to the enclosing action.
        crate::effects::OriginalPhaseStatus::Complete
    }

    fn freeze(&mut self, game: &mut GameState) -> Result<(), ExecutionError> {
        game.freeze_completed_entry_events(
            self.before
                .outcome
                .events
                .iter_mut()
                .chain(self.first.outcome.outcome.events.iter_mut()),
        )?;
        if let Some(first) = &mut self.first.completion {
            first.freeze(game)?;
        }
        Ok(())
    }
    fn observe_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: &mut EffectOutcome,
    ) -> Result<(), ExecutionError> {
        crate::effects::composition::inherit_original_observations(
            &mut self.before.outcome,
            &original.events,
        );
        crate::effects::composition::inherit_original_observations(
            &mut self.first.outcome.outcome,
            &original.events,
        );
        if let Some(completion) = &mut self.first.completion {
            let parent = ExecutionContextCheckpoint::capture(ctx);
            self.scope.restore_ref(ctx);
            let result = completion.observe_original(game, ctx, &mut self.first.outcome.outcome);
            parent.restore(ctx);
            result?;
        }
        Ok(())
    }
    fn prepare_draw_boundary_with_outputs(
        self: Box<Self>,
        _game: &mut GameState,
        _ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
        Ok(SimultaneousEffectCommit {
            outcome: CompletedEffectOutputs::aggregate_only(original),
            completion: Some(self),
        })
    }

    fn prepare_draw_boundary_from_outputs(
        self: Box<Self>,
        _game: &mut GameState,
        _ctx: &mut ExecutionContext,
        original: crate::effects::CompletedEffectOutputs,
    ) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
        Ok(SimultaneousEffectCommit {
            outcome: original,
            completion: Some(self),
        })
    }
    fn complete(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        prefix: EffectOutcome,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.complete_with_outputs(game, ctx, prefix)
            .map(CompletedEffectOutputs::into_outcome)
    }
    fn complete_with_outputs(
        mut self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        prefix: EffectOutcome,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        crate::effects::composition::inherit_original_observations(
            &mut self.before.outcome,
            &prefix.events,
        );
        crate::effects::composition::inherit_original_observations(
            &mut self.first.outcome.outcome,
            &prefix.events,
        );
        let parent = ExecutionContextCheckpoint::capture(ctx);
        self.scope.restore_ref(ctx);
        let result = (|| {
            let first = crate::effects::composition::complete_committed_original_with_outputs(
                game, ctx, self.first,
            )?;
            let mut outputs = append(self.before, first);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::count(0),
                ));
            }
            let programs = self.programs;
            outputs = super::complete_replacement_programs_with_original_outputs(
                game,
                ctx,
                outputs,
                |game, ctx, original| {
                    super::complete_bound_replacement_programs_with_outputs(
                        game, ctx, original, programs,
                    )
                },
            )?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::count(0),
                ));
            }
            for effect in self.followups {
                if ctx.resolution_stopped() {
                    break;
                }
                crate::effects::capture_triggers_before_added_program(
                    game,
                    ctx,
                    Some(&effect),
                    outputs.outcome.events.iter_mut(),
                )?;
                let added = crate::effects::execute_effect_with_outputs(game, &effect, ctx)?;
                outputs = append(outputs, added);
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
            }
            outputs.synchronize_observations();
            Ok(outputs)
        })();
        parent.restore(ctx);
        result
    }
}

pub(crate) fn prepare_zone_draw_tail(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    before: EffectOutcome,
    programs: Vec<BoundProgram>,
    followups: &[Effect],
) -> Result<SimultaneousEffectCommit, ExecutionError> {
    prepare_zone_draw_tail_with_outputs(game, ctx, before, programs, followups)
        .map(SimultaneousEffectCommit::into_aggregate)
}

/// Every program binding is frozen by the action owner before this boundary.
/// A paused draw owns its suffix; later programs cannot overtake it.
pub(crate) fn prepare_zone_draw_tail_with_outputs<O: crate::effects::OriginalEffectOutput>(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    before: O,
    programs: Vec<BoundProgram>,
    followups: &[Effect],
) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
    let mut before = before.into_outputs();
    if !programs.is_empty() || !followups.is_empty() {
        // Added instructions have not declared complete enclosing projections.
        before.projections_complete = false;
    }
    let mut programs = programs.into_iter();
    while let Some((program, bindings)) = programs.next() {
        crate::effects::capture_triggers_before_added_program(
            game,
            ctx,
            program.effects.first(),
            before.outcome.events.iter_mut(),
        )?;
        let first = super::with_replacement_child(
            game,
            ctx,
            program.source,
            program.controller,
            &program.context,
            bindings.targets,
            program.source_snapshot,
            bindings.object_tags,
            |game, child| {
                super::prepare_scoped_program_draw_boundary_with_outputs(
                    game,
                    child,
                    &program.effects,
                )
            },
        )?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(SimultaneousEffectCommit::finished(
                CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            ));
        }
        if first.completion.is_some() {
            let prefix = EffectOutcome::aggregate_replacement_outcomes(
                before.outcome.clone(),
                [first.outcome.outcome.clone()],
            );
            return Ok(SimultaneousEffectCommit {
                // This routing view preserves identities and contains no continuation.
                outcome: before
                    .clone_projection()
                    .project_aggregate(prefix)
                    .append_owned_child(first.outcome.clone_projection()),
                completion: Some(Box::new(ZoneTail {
                    before,
                    first,
                    programs: programs.collect(),
                    followups: followups.to_vec(),
                    scope: ExecutionContextCheckpoint::capture(ctx),
                })),
            });
        }
        before = append(before, first.outcome);
    }
    crate::effects::capture_triggers_before_added_program(
        game,
        ctx,
        followups.first(),
        before.outcome.events.iter_mut(),
    )?;
    if !followups.is_empty() {
        let first = super::prepare_scoped_program_draw_boundary_with_outputs(game, ctx, followups)?;
        if first.completion.is_some() {
            let prefix = EffectOutcome::aggregate_replacement_outcomes(
                before.outcome.clone(),
                [first.outcome.outcome.clone()],
            );
            return Ok(SimultaneousEffectCommit {
                // This routing view preserves identities and contains no continuation.
                outcome: before
                    .clone_projection()
                    .project_aggregate(prefix)
                    .append_owned_child(first.outcome.clone_projection()),
                completion: Some(Box::new(ZoneTail {
                    before,
                    first,
                    programs: Vec::new(),
                    followups: Vec::new(),
                    scope: ExecutionContextCheckpoint::capture(ctx),
                })),
            });
        }
        return Ok(SimultaneousEffectCommit::finished(append(
            before,
            first.outcome,
        )));
    }

    Ok(SimultaneousEffectCommit::finished(before))
}
