//! Replacement continuations retain the native selected iteration cursor.
//! The cursor owns iteration bindings, result projection, targets and postludes;
//! this adapter only pauses its next actual child at a draw boundary.
#[cfg(test)]
use crate::effect::Effect;
use crate::effect::{EffectOutcome, ExecutionFact};
use crate::effects::replacement::ReplacementResume;
use crate::effects::{
    ActionProgramCursor, CompletedEffectOutputs, ExecutionContext, ExecutionContextCheckpoint,
    ExecutionError, ProgramActionScope, SimultaneousEffectCommit, SimultaneousEffectCompletion,
};
use crate::game_state::GameState;
#[cfg(test)]
use crate::ids::{ObjectId, PlayerId};
#[cfg(test)]
use crate::snapshot::ObjectSnapshot;
#[cfg(test)]
use crate::tag::TagKey;

pub(super) fn correlated_player_count(outcomes: &[EffectOutcome]) -> i64 {
    let summary = EffectOutcome::aggregate_summing_counts(outcomes.iter().cloned());
    let count = summary.as_count().unwrap_or(0);
    if count != 0 {
        return count;
    }
    i64::from(
        summary
            .execution_facts
            .iter()
            .any(|fact| matches!(fact, ExecutionFact::Accepted)),
    )
}

pub(super) fn prepare_iteration_continuation(
    cursor: Option<Box<dyn ActionProgramCursor>>,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
    let Some(cursor) = cursor else {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(SimultaneousEffectCommit::finished(
                CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            ));
        }
        return Err(ExecutionError::InternalError(
            "selected iterator lost its cursor".into(),
        ));
    };
    IterationContinuation {
        cursor,
        history: Vec::new(),
        pending: None,
        context: ExecutionContextCheckpoint::capture(ctx),
    }
    .run(game, ctx, true)
}

/// Bind the authored iteration only after its one physical damage owner
/// completes. The view changes the primary result without duplicating history.
struct IterationDamageBinding(Box<dyn crate::effects::SimultaneousEffectProposal>);
impl super::OriginalOutcomeAdapter for IterationDamageBinding {
    fn finish(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        result: Result<EffectOutcome, ExecutionError>,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.finish_with_outputs(
            game,
            ctx,
            result.map(CompletedEffectOutputs::aggregate_only),
        )
        .map(CompletedEffectOutputs::into_outcome)
    }
    fn finish_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        result: Result<CompletedEffectOutputs, ExecutionError>,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        let mut outputs = result?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(outputs);
        }
        let binding = self.0.bind_damage_action(game, ctx, &outputs)?;
        let primary = binding.transfer_owned_outputs(&mut outputs);
        let observations = outputs.outcome.clone();
        Ok(outputs.project_aggregate(primary.with_authoritative_observations(observations)))
    }
}

struct IterationContinuation {
    cursor: Box<dyn ActionProgramCursor>,
    history: Vec<EffectOutcome>,
    pending: Option<(ProgramActionScope, Box<dyn ReplacementResume>)>,
    context: ExecutionContextCheckpoint,
}
impl IterationContinuation {
    fn run(
        mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        defer_draws: bool,
    ) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
        if let Some((scope, pending)) = self.pending.take() {
            let outputs = scope.run(game, ctx, |game, ctx| {
                crate::effects::replacement::resume_replacement_child_with_outputs(
                    game, ctx, pending,
                )
            })?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(SimultaneousEffectCommit::finished(
                    CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
                ));
            }
            self.history.push(outputs.outcome.clone());
            self.cursor.accept_action(outputs)?;
        }
        loop {
            if ctx.resolution_stopped() {
                let mut outputs = self.cursor.finish_stopped(game, ctx)?.outputs;
                let observed = self
                    .history
                    .iter()
                    .flat_map(|outcome| outcome.events.iter().cloned())
                    .collect::<Vec<_>>();
                super::inherit_original_observations(&mut outputs.outcome, &observed);
                outputs.synchronize_observations();
                return Ok(SimultaneousEffectCommit::finished(outputs));
            }

            let next = self.cursor.next_action(game, ctx)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(SimultaneousEffectCommit::finished(
                    CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
                ));
            }
            let preparations = self.cursor.take_preparations();
            let declared = !preparations.is_empty();
            for preparation in preparations {
                preparation.prepare(game)?;
            }
            let Some(mut action) = next else {
                if declared {
                    continue;
                }
                let mut outputs = self.cursor.finish()?.outputs;
                let observed = self
                    .history
                    .iter()
                    .flat_map(|outcome| outcome.events.iter().cloned())
                    .collect::<Vec<_>>();
                super::inherit_original_observations(&mut outputs.outcome, &observed);
                outputs.synchronize_observations();
                return Ok(SimultaneousEffectCommit::finished(outputs));
            };
            crate::effects::capture_triggers_before_added_program(
                game,
                ctx,
                Some(&action.effect),
                self.history
                    .iter_mut()
                    .flat_map(|outcome| outcome.events.iter_mut()),
            )?;
            let native = action.native.take();
            let purpose = action
                .scope
                .execution_purpose(crate::effects::EffectExecutionPurpose::Action);
            let prepared = action.scope.run(game, ctx, |game, ctx| match native {
                Some(super::action_program::NativeProgramAction::SharedDamage(mut proposal)) => {
                    if !defer_draws {
                        return crate::effects::damage::complete_prepared_damage_action(
                            game, ctx, proposal,
                        )
                        .map(|prefix| {
                            crate::effects::replacement::PreparedReplacementChild {
                                prefix,
                                resume: None,
                            }
                        });
                    }
                    proposal.prepare_selection(game, ctx)?;
                    proposal.prepare_original(game, ctx)?;
                    let inputs = proposal.damage_action_inputs().ok_or_else(|| {
                        ExecutionError::InternalError(
                            "prepared iteration damage lost its shared inputs".into(),
                        )
                    })?;
                    let opened = game.open_simultaneous_action();
                    let receipt = (|| {
                        let owner = inputs.seal(game, ctx)?;
                        if ctx.decision_maker.awaiting_choice() {
                            return Ok(SimultaneousEffectCommit::finished(
                                CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
                            ));
                        }
                        owner.commit_original_with_outputs(game, ctx)
                    })();
                    game.close_simultaneous_action(opened);
                    let receipt = super::adapt_original_outcome_with_outputs(
                        receipt?,
                        Box::new(IterationDamageBinding(proposal)),
                        game,
                        ctx,
                    )?;
                    crate::effects::replacement::prepare_committed_draw_boundary(game, ctx, receipt)
                }
                Some(super::action_program::NativeProgramAction::TotalCost {
                    cost,
                    payer,
                    reason,
                }) => {
                    crate::costs::execute_total_cost_program_action(&cost, game, ctx, payer, reason)
                        .map(
                            |prefix| crate::effects::replacement::PreparedReplacementChild {
                                prefix,
                                resume: None,
                            },
                        )
                }
                None if defer_draws
                    && matches!(purpose, crate::effects::EffectExecutionPurpose::Action) =>
                {
                    crate::effects::replacement::prepare_replacement_child(
                        game,
                        ctx,
                        &action.effect,
                    )
                }
                None => purpose.execute(game, &action.effect, ctx).map(|prefix| {
                    crate::effects::replacement::PreparedReplacementChild {
                        prefix,
                        resume: None,
                    }
                }),
            })?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(SimultaneousEffectCommit::finished(
                    CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
                ));
            }
            if let Some(pending) = prepared.resume {
                let prefix = EffectOutcome::aggregate(
                    self.history
                        .iter()
                        .cloned()
                        .chain(std::iter::once(prepared.prefix.outcome.clone())),
                );
                self.pending = Some((action.scope, pending));
                self.context = ExecutionContextCheckpoint::capture(ctx);
                return Ok(SimultaneousEffectCommit {
                    outcome: prepared.prefix.project_aggregate(prefix),
                    completion: Some(Box::new(self)),
                });
            }
            self.history.push(prepared.prefix.outcome.clone());
            self.cursor.accept_action(prepared.prefix)?;
        }
    }
}
impl SimultaneousEffectCompletion for IterationContinuation {
    // The cursor retains the rest of one authored instruction, including its
    // internal payment and postlude order. It has no enclosing addition queue.
    fn original_phase_status(&self) -> crate::effects::OriginalPhaseStatus {
        crate::effects::OriginalPhaseStatus::Retained
    }

    fn complete_original_phase_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
        self.complete_original_phase_from_outputs(
            game,
            ctx,
            CompletedEffectOutputs::aggregate_only(original),
        )
    }

    fn complete_original_phase_from_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: CompletedEffectOutputs,
    ) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
        super::complete_authored_original_subtree_with_outputs(game, ctx, self, original)
    }

    fn freeze(&mut self, game: &mut GameState) -> Result<(), ExecutionError> {
        game.freeze_completed_entry_events(
            self.history
                .iter_mut()
                .flat_map(|outcome| outcome.events.iter_mut()),
        )?;
        if let Some((_, pending)) = &mut self.pending {
            pending.freeze(game)?;
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
    fn complete(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.complete_with_outputs(game, ctx, original)
            .map(CompletedEffectOutputs::into_outcome)
    }
    fn complete_with_outputs(
        mut self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        for outcome in &mut self.history {
            super::inherit_original_observations(outcome, &original.events);
        }
        if let Some((_, pending)) = &mut self.pending {
            pending.observe_prefix(&original.events);
        }
        let parent = ExecutionContextCheckpoint::capture(ctx);
        let executing_effect = ctx.executing_effect;
        self.context.restore_ref(ctx);
        let result = (*self).run(game, ctx, false).map(|receipt| receipt.outcome);
        if result.is_err() || ctx.decision_maker.awaiting_choice() {
            parent.restore(ctx);
        }
        ctx.executing_effect = executing_effect;
        result
    }
}

#[cfg(test)]
#[path = "object_iteration_tests.rs"]
mod tests;
