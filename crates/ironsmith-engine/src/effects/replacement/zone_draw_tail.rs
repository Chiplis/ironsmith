//! Retain an already-bound compound zone replacement across its first draw.
use crate::effect::{Effect, EffectOutcome};
use crate::effects::{ExecutionContext, ExecutionContextCheckpoint, ExecutionError,
    SimultaneousEffectCommit, SimultaneousEffectCompletion};
use crate::events::processing::PreparedReplacementProgram;
use crate::game_state::GameState;
use super::ReplacementProgramBindings;

type BoundProgram = (PreparedReplacementProgram, ReplacementProgramBindings);

struct ZoneTail {
    before: EffectOutcome,
    first: SimultaneousEffectCommit,
    programs: Vec<BoundProgram>,
    followups: Vec<Effect>,
    scope: ExecutionContextCheckpoint,
}
impl SimultaneousEffectCompletion for ZoneTail {
    fn freeze(&mut self, game: &mut GameState) -> Result<(), ExecutionError> {
        if let Some(first) = &mut self.first.completion { first.freeze(game)?; }
        Ok(())
    }
    fn complete(self: Box<Self>, game: &mut GameState, ctx: &mut ExecutionContext,
        _prefix: EffectOutcome) -> Result<EffectOutcome, ExecutionError> {
        let parent = ExecutionContextCheckpoint::capture(ctx);
        self.scope.restore(ctx);
        let result = (|| {
            let first = if let Some(completion) = self.first.completion {
                completion.complete(game, ctx, self.first.outcome)?
            } else { self.first.outcome };
            let mut outcome = EffectOutcome::aggregate_replacement_outcomes(self.before, [first]);
            if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
            for (program, bindings) in self.programs {
                crate::effects::runtime::capture_triggers_before_added_program(
                    game, ctx, program.effects.first(), outcome.events.iter_mut(),
                )?;
                let added = super::execute_replacement_payload_with_snapshot(
                    game, ctx, &program.effects, program.source, program.controller,
                    &program.context, bindings.targets, program.source_snapshot, bindings.object_tags,
                )?;
                outcome = EffectOutcome::aggregate_replacement_outcomes(outcome, [added]);
                if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
            }
            for effect in self.followups {
                crate::effects::runtime::capture_triggers_before_added_program(
                    game, ctx, Some(&effect), outcome.events.iter_mut(),
                )?;
                let added = crate::effects::execute_effect(game, &effect, ctx)?;
                outcome = EffectOutcome::aggregate_replacement_outcomes(outcome, [added]);
                if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
            }
            Ok(outcome)
        })();
        parent.restore(ctx);
        result
    }
}

/// All bindings are captured before any program runs. Programs after a paused
/// draw belong to that continuation; they cannot overtake it during preparation.
pub(crate) fn prepare_zone_draw_tail(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    mut before: EffectOutcome,
    programs: Vec<BoundProgram>,
    followups: &[Effect],
) -> Result<SimultaneousEffectCommit, ExecutionError> {
    let mut programs = programs.into_iter();
    while let Some((program, bindings)) = programs.next() {
        crate::effects::runtime::capture_triggers_before_added_program(
            game, ctx, program.effects.first(), before.events.iter_mut(),
        )?;
        let prepared = super::prepare_draw_continuation_with_bindings(
            game, ctx, &program.effects, program.source, program.controller, &program.context,
            program.source_snapshot.clone(), bindings.clone(),
        )?;
        let first = if let Some(first) = prepared { first } else {
            SimultaneousEffectCommit::finished(super::execute_replacement_payload_with_snapshot(
                game, ctx, &program.effects, program.source, program.controller, &program.context,
                bindings.targets, program.source_snapshot, bindings.object_tags,
            )?)
        };
        if ctx.decision_maker.awaiting_choice() {
            return Ok(SimultaneousEffectCommit::finished(EffectOutcome::count(0)));
        }
        if first.completion.is_some() {
            return Ok(SimultaneousEffectCommit {
                outcome: EffectOutcome::aggregate_replacement_outcomes(before.clone(), [first.outcome.clone()]),
                completion: Some(Box::new(ZoneTail {
                    before, first, programs: programs.collect(), followups: followups.to_vec(),
                    scope: ExecutionContextCheckpoint::capture(ctx),
                })),
            });
        }
        before = EffectOutcome::aggregate_replacement_outcomes(before, [first.outcome]);
    }
    crate::effects::runtime::capture_triggers_before_added_program(
        game, ctx, followups.first(), before.events.iter_mut(),
    )?;
    if let Some(first) = super::prepare_scoped_draw_continuation(game, ctx, followups)? {
        let outcome = EffectOutcome::aggregate_replacement_outcomes(before.clone(), [first.outcome.clone()]);
        if first.completion.is_some() {
            return Ok(SimultaneousEffectCommit {
                outcome,
                completion: Some(Box::new(ZoneTail {
                    before, first, programs: Vec::new(), followups: Vec::new(),
                    scope: ExecutionContextCheckpoint::capture(ctx),
                })),
            });
        }
        return Ok(SimultaneousEffectCommit::finished(outcome));
    }
    for effect in followups {
        let added = crate::effects::execute_effect(game, effect, ctx)?;
        before = EffectOutcome::aggregate_replacement_outcomes(before, [added]);
        if ctx.decision_maker.awaiting_choice() { return Ok(SimultaneousEffectCommit::finished(EffectOutcome::count(0))); }
        crate::effects::runtime::capture_triggers_before_added_program(game, ctx, None, before.events.iter_mut())?;
    }
    Ok(SimultaneousEffectCommit::finished(before))
}
