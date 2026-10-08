//! Shared completion boundary for already prepared simultaneous actions.

use crate::effect::EffectOutcome;
use crate::effects::{
    ExecutionContext, ExecutionError, OriginalEffectOutput, SimultaneousEffectCommit,
};
use crate::game_state::GameState;

pub(crate) enum OriginalTriggerObservation {
    /// Match the originals at the ordinary shared instruction boundary.
    Capture,
    /// The action owner has already published/captured every original. Its
    /// specialized observer may deliberately leave unrelated held events alone.
    OwnerPublished,
}

/// The caller prepares all choices before this boundary and owns rollback of
/// the enclosing instruction. The callback commits only originals, retaining
/// each action owner's continuation. No appended program runs until every
/// original has committed and all receipts have frozen in that completed world.
pub(crate) fn execute_simultaneous_originals<'a>(
    game: &mut GameState,
    ctx: &mut ExecutionContext<'a>,
    simultaneous: bool,
    originals: impl FnOnce(
        &mut GameState,
        &mut ExecutionContext<'a>,
    ) -> Result<Vec<SimultaneousEffectCommit>, ExecutionError>,
) -> Result<Vec<EffectOutcome>, ExecutionError> {
    execute_simultaneous_originals_with_default_outputs(game, ctx, simultaneous, originals).map(
        |outputs| {
            outputs
                .into_iter()
                .map(crate::effects::CompletedEffectOutputs::into_outcome)
                .collect()
        },
    )
}

/// Retained outputs with the ordinary original-life-payment observer.
/// Specialized observers use `execute_simultaneous_originals_with_outputs`.
pub(crate) fn execute_simultaneous_originals_with_default_outputs<'a, O: OriginalEffectOutput>(
    game: &mut GameState,
    ctx: &mut ExecutionContext<'a>,
    simultaneous: bool,
    originals: impl FnOnce(
        &mut GameState,
        &mut ExecutionContext<'a>,
    ) -> Result<Vec<SimultaneousEffectCommit<O>>, ExecutionError>,
) -> Result<Vec<crate::effects::CompletedEffectOutputs>, ExecutionError> {
    execute_simultaneous_originals_with_outputs(
        game,
        ctx,
        simultaneous,
        originals,
        |game, ctx, receipts| {
            game.observe_prepared_life_payment_originals(
                ctx,
                receipts
                    .iter_mut()
                    .flat_map(|receipt| receipt.outcome.aggregate_mut().events.iter_mut()),
            )
        },
    )
}

/// Retain owner-supplied outputs through the same group freeze, observation,
/// completion and scope-closing boundary used by the aggregate adapter.
pub(crate) fn execute_simultaneous_originals_with_outputs<'a, O: OriginalEffectOutput>(
    game: &mut GameState,
    ctx: &mut ExecutionContext<'a>,
    simultaneous: bool,
    originals: impl FnOnce(
        &mut GameState,
        &mut ExecutionContext<'a>,
    ) -> Result<Vec<SimultaneousEffectCommit<O>>, ExecutionError>,
    observe: impl FnOnce(
        &mut GameState,
        &mut ExecutionContext<'a>,
        &mut [SimultaneousEffectCommit<O>],
    ) -> Result<OriginalTriggerObservation, ExecutionError>,
) -> Result<Vec<crate::effects::CompletedEffectOutputs>, ExecutionError> {
    if ctx.decision_maker.awaiting_choice() {
        return Ok(Vec::new());
    }
    let mut opened = simultaneous.then(|| game.open_simultaneous_action());
    let result = (|| {
        let (mut receipts, observations) =
            crate::effects::with_action_observations(game, |game| originals(game, ctx))?;
        super::original_observations::retain_original_observations(
            receipts.iter_mut(),
            observations,
        );
        if ctx.decision_maker.awaiting_choice() {
            return Ok(Vec::new());
        }
        if let Some(opened) = opened.take() {
            game.close_simultaneous_action(opened);
        }
        finish_simultaneous_originals_with_outputs(game, ctx, receipts, observe)
    })();
    // A suspended/failed original still closes the grouping scope. Its caller
    // restores the compound checkpoint without leaking a simultaneous frame.
    if let Some(opened) = opened {
        game.close_simultaneous_action(opened);
    }
    result
}

/// Finalize originals after their owner closes grouping and lookback scopes.
/// The observer sees the original receipt representation; retain rich packets
/// once afterward so completed originals never pass through a scalar projection.
fn finish_simultaneous_originals_with_outputs<'a, O: OriginalEffectOutput>(
    game: &mut GameState,
    ctx: &mut ExecutionContext<'a>,
    receipts: Vec<SimultaneousEffectCommit<O>>,
    observe: impl FnOnce(
        &mut GameState,
        &mut ExecutionContext<'a>,
        &mut [SimultaneousEffectCommit<O>],
    ) -> Result<OriginalTriggerObservation, ExecutionError>,
) -> Result<Vec<crate::effects::CompletedEffectOutputs>, ExecutionError> {
    let receipts = prepare_simultaneous_originals_with_participants(
        game,
        ctx,
        receipts,
        |receipt| receipt,
        observe,
    )?;
    let receipts = receipts
        .into_iter()
        .map(|receipt| SimultaneousEffectCommit {
            outcome: receipt.outcome.into_outputs(),
            completion: receipt.completion,
        })
        .collect::<Vec<_>>();
    let receipts = if original_cohort_phase_status(&receipts)
        != crate::effects::OriginalPhaseStatus::Combined
    {
        let Some(receipts) = complete_original_cohort_phase_with_outputs(game, ctx, receipts)?
        else {
            return Ok(Vec::new());
        };
        receipts
    } else {
        // Combined owners remain on their existing route until migrated.
        // Their presence prevents claiming this cohort's originals are ready.
        receipts
    };
    let mut outcomes = Vec::with_capacity(receipts.len());
    for receipt in receipts {
        outcomes.push(complete_committed_original_with_outputs(
            game, ctx, receipt,
        )?);
        if ctx.decision_maker.awaiting_choice() {
            return Ok(Vec::new());
        }
    }
    Ok(outcomes)
}

/// Capability belongs to every original in the cohort, never its first child.
fn original_cohort_phase_status(
    receipts: &[SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>],
) -> crate::effects::OriginalPhaseStatus {
    original_cohort_phase_status_from_receipts(receipts)
}

pub(crate) fn original_cohort_phase_status_from_receipts<'a>(
    receipts: impl IntoIterator<
        Item = &'a SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
    >,
) -> crate::effects::OriginalPhaseStatus {
    let mut status = crate::effects::OriginalPhaseStatus::Complete;
    for receipt in receipts {
        if let Some(completion) = &receipt.completion {
            match completion.original_phase_status() {
                crate::effects::OriginalPhaseStatus::Combined => {
                    return crate::effects::OriginalPhaseStatus::Combined;
                }
                crate::effects::OriginalPhaseStatus::Retained => {
                    status = crate::effects::OriginalPhaseStatus::Retained;
                }
                crate::effects::OriginalPhaseStatus::Complete => {}
            }
        }
    }
    status
}

/// Participant metadata is retained; the accessor only selects its receipt.
pub(crate) fn original_cohort_phase_status_with_participants<P>(
    participants: &mut [P],
    receipt: fn(&mut P) -> &mut SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
) -> crate::effects::OriginalPhaseStatus {
    original_cohort_phase_status_from_receipts(
        participants
            .iter_mut()
            .map(|participant| &*receipt(participant)),
    )
}

/// Finish one separated cohort with its owner's participant context callback.
/// The callback advances only a Retained original and captures its resulting
/// context even when no continuation remains. It must retain the same participant.
pub(crate) fn complete_original_cohort_phase_with_participants<P>(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    mut participants: Vec<P>,
    receipt: fn(&mut P) -> &mut SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
    mut advance: impl FnMut(&mut GameState, &mut ExecutionContext, P) -> Result<P, ExecutionError>,
) -> Result<Option<Vec<P>>, ExecutionError> {
    if ctx.decision_maker.awaiting_choice() {
        return Ok(None);
    }
    if original_cohort_phase_status_with_participants(&mut participants, receipt)
        == crate::effects::OriginalPhaseStatus::Combined
    {
        return Err(ExecutionError::InternalError(
            "original cohort contains an unseparated child".into(),
        ));
    }
    let mut retained = Vec::with_capacity(participants.len());
    for mut participant in participants {
        if ctx.resolution_stopped() {
            receipt(&mut participant).completion = None;
        } else if receipt(&mut participant)
            .completion
            .as_ref()
            .is_some_and(|completion| {
                completion.original_phase_status() == crate::effects::OriginalPhaseStatus::Retained
            })
        {
            participant = advance(game, ctx, participant)?;
        }
        if ctx.decision_maker.awaiting_choice() {
            return Ok(None);
        }
        if !ctx.resolution_stopped()
            && receipt(&mut participant)
                .completion
                .as_ref()
                .is_some_and(|completion| {
                    completion.original_phase_status()
                        != crate::effects::OriginalPhaseStatus::Complete
                })
        {
            return Err(ExecutionError::InternalError(
                "original cohort left an unresolved child original".into(),
            ));
        }
        retained.push(participant);
    }
    if ctx.resolution_stopped() {
        for participant in &mut retained {
            receipt(participant).completion = None;
        }
    }
    Ok(Some(retained))
}

/// Ordinary receipts use the same cohort owner without another context policy.
fn complete_original_cohort_phase_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    receipts: Vec<SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>>,
) -> Result<
    Option<Vec<SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>>>,
    ExecutionError,
> {
    complete_original_cohort_phase_with_participants(
        game,
        ctx,
        receipts,
        |receipt| receipt,
        complete_retained_original_phase_with_outputs,
    )
}

/// Freeze and observe a complete original group without running a completion.
/// Resumable owners retain this exact prepared group across their draw boundary.
pub(crate) fn prepare_simultaneous_originals_with_participants<'a, P, O: OriginalEffectOutput>(
    game: &mut GameState,
    ctx: &mut ExecutionContext<'a>,
    mut participants: Vec<P>,
    receipt: fn(&mut P) -> &mut SimultaneousEffectCommit<O>,
    observe: impl FnOnce(
        &mut GameState,
        &mut ExecutionContext<'a>,
        &mut [P],
    ) -> Result<OriginalTriggerObservation, ExecutionError>,
) -> Result<Vec<P>, ExecutionError> {
    if ctx.decision_maker.awaiting_choice() {
        return Ok(Vec::new());
    }
    // Include queued entry observations even for originals with no additions.
    // Freeze the complete group before any participant's continuation freezes.
    game.freeze_completed_entry_events(participants.iter_mut().flat_map(|participant| {
        receipt(participant)
            .outcome
            .aggregate_mut()
            .events
            .iter_mut()
    }))?;
    for participant in &mut participants {
        let original = receipt(participant);
        crate::effects::outcome_recording::complete_outcome(
            game,
            None,
            Some(ctx.controller),
            original.outcome.aggregate_mut(),
            Vec::new(),
        );
        if let Some(completion) = &mut original.completion {
            completion.freeze(game)?;
        }
    }
    // Every original is frozen before any specialized original observer runs.
    // Complete every observer before the first participant's added program.
    for participant in &mut participants {
        let original = receipt(participant);
        if let Some(completion) = &mut original.completion {
            observe_original_completion(
                game,
                ctx,
                completion.as_mut(),
                original.outcome.aggregate_mut(),
            )?;
        }
        if ctx.decision_maker.awaiting_choice() {
            return Ok(Vec::new());
        }
    }
    let observation = observe(game, ctx, &mut participants)?;
    if ctx.decision_maker.awaiting_choice() {
        return Ok(Vec::new());
    }
    if matches!(observation, OriginalTriggerObservation::Capture)
        && participants
            .iter_mut()
            .any(|participant| receipt(participant).completion.is_some())
    {
        crate::effects::runtime::capture_triggers_before_added_program(
            game,
            ctx,
            None,
            participants.iter_mut().flat_map(|participant| {
                receipt(participant)
                    .outcome
                    .aggregate_mut()
                    .events
                    .iter_mut()
            }),
        )?;
    }
    Ok(participants)
}

/// Shared dispatch for an already frozen continuation's original observer.
/// Batch owners call this for every participant before completing any of them.
pub(crate) fn observe_original_completion(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    completion: &mut dyn crate::effects::SimultaneousEffectCompletion,
    original: &mut EffectOutcome,
) -> Result<(), ExecutionError> {
    if ctx.decision_maker.awaiting_choice() {
        return Ok(());
    }
    completion.observe_original(game, ctx, original)
}

/// The standalone freeze/observation/completion owner retains child outputs.
pub(crate) fn complete_standalone_original_with_outputs<O: OriginalEffectOutput>(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    receipt: SimultaneousEffectCommit<O>,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    let Some(receipt) = prepare_standalone_completion_with_outputs(game, ctx, receipt)? else {
        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
            EffectOutcome::count(0),
        ));
    };
    complete_committed_original_with_outputs(game, ctx, receipt)
}

/// Retain the actual original after its shared freeze/observation boundary.
/// Native adapters can consume original results before dispatching additions.
/// Simultaneous cohorts retain their separate whole-cohort preparation owner.
pub(crate) fn prepare_standalone_completion_with_outputs<O: OriginalEffectOutput>(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    mut receipt: SimultaneousEffectCommit<O>,
) -> Result<Option<SimultaneousEffectCommit<O>>, ExecutionError> {
    if ctx.decision_maker.awaiting_choice() {
        return Ok(None);
    }
    if let Some(completion) = &mut receipt.completion {
        prepare_standalone_original_completion(
            game,
            ctx,
            receipt.outcome.aggregate_mut(),
            completion.as_mut(),
        )?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(None);
        }
    }
    Ok(Some(receipt))
}

/// Borrowed original receipts use the same standalone freeze/observation owner.
/// Whole-cohort preparation retains its separate all-originals timing boundary.
pub(crate) fn prepare_standalone_original_completion(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    original: &mut EffectOutcome,
    completion: &mut dyn crate::effects::SimultaneousEffectCompletion,
) -> Result<(), ExecutionError> {
    game.freeze_completed_entry_events(original.events.iter_mut())?;
    completion.freeze(game)?;
    observe_original_completion(game, ctx, completion, original)
}

/// Finish only an explicitly separated original phase. Combined owners stay
/// on their existing route; Complete owners retain their unexecuted additions.
/// This does not prove a whole cohort is ready while any owner is Combined.
pub(crate) fn complete_retained_original_phase_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    mut receipt: SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
) -> Result<SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>, ExecutionError> {
    let Some(completion) = receipt.completion.take() else {
        return Ok(receipt);
    };
    if completion.original_phase_status() != crate::effects::OriginalPhaseStatus::Retained {
        receipt.completion = Some(completion);
        return Ok(receipt);
    }
    let original = receipt.outcome;
    let completed = completion.complete_original_phase_from_outputs(game, ctx, original)?;
    if ctx.decision_maker.awaiting_choice() {
        return Ok(SimultaneousEffectCommit::finished(
            crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
        ));
    }
    Ok(completed)
}

/// A retained authored instruction is one original unit of its enclosing
/// replacement program. Its cursor owns internal action/cost ordering; this
/// boundary applies only to owners with no enclosing added-program queue.
/// It must not be used to reclassify arbitrary compound completions.
pub(crate) fn complete_authored_original_subtree_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    completion: Box<dyn crate::effects::SimultaneousEffectCompletion>,
    original: crate::effects::CompletedEffectOutputs,
) -> Result<SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>, ExecutionError> {
    let outputs = completion.complete_from_original_outputs(game, ctx, original)?;
    if ctx.decision_maker.awaiting_choice() {
        return Ok(SimultaneousEffectCommit::finished(
            crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
        ));
    }
    Ok(SimultaneousEffectCommit::finished(outputs))
}

/// Dispatch an already frozen/observed receipt. Finished outputs need no
/// synthetic continuation; deferred additions retain the original packet's
/// metadata alongside their own output without duplicating aggregate history.
pub(crate) fn complete_committed_original_with_outputs<O: OriginalEffectOutput>(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    receipt: SimultaneousEffectCommit<O>,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    let original = receipt.outcome.into_outputs();
    if ctx.resolution_stopped() {
        return Ok(original);
    }
    match receipt.completion {
        Some(completion) => {
            let outputs = completion.complete_from_original_outputs(game, ctx, original)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::count(0),
                ));
            }
            Ok(outputs)
        }
        None => Ok(original),
    }
}

/// Complete an ordered stream of already frozen/observed originals once.
/// Callers own scope, observation inheritance and projection. The post-child
/// hook can freeze an exact arrival before a later sibling moves it. Pending
/// input returns no completed stream; the enclosing transaction owns replay.
pub(crate) fn complete_retained_originals_with_outputs<O: OriginalEffectOutput>(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    receipts: impl IntoIterator<Item = SimultaneousEffectCommit<O>>,
    mut after_original: impl FnMut(
        &mut GameState,
        &mut ExecutionContext,
        usize,
        &mut crate::effects::CompletedEffectOutputs,
    ) -> Result<(), ExecutionError>,
) -> Result<Option<Vec<crate::effects::CompletedEffectOutputs>>, ExecutionError> {
    let mut completed = Vec::new();
    for (index, receipt) in receipts.into_iter().enumerate() {
        let mut outputs = complete_committed_original_with_outputs(game, ctx, receipt)?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(None);
        }
        after_original(game, ctx, index, &mut outputs)?;
        completed.push(outputs);
    }
    Ok(Some(completed))
}

/// A transparent composition wrapper decorates a completed child receipt.
/// It receives failures and suspension as well, so result-slot restoration
/// follows the same contract as ordinary execution.
pub(crate) trait OriginalOutcomeAdapter: Send {
    /// Release metadata reserved by an unstarted authored child when its
    /// resolution stops. This is not an instruction result or an error.
    fn cancel(self: Box<Self>, _game: &mut GameState, _ctx: &mut ExecutionContext) {}

    fn finish(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        result: Result<EffectOutcome, ExecutionError>,
    ) -> Result<EffectOutcome, ExecutionError>;

    /// Owners may project a captured participant from the retained packet.
    /// Scalar decorators keep their existing result and restoration policy.
    fn finish_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        result: Result<crate::effects::CompletedEffectOutputs, ExecutionError>,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        match result {
            Ok(mut outputs) => {
                outputs.outcome = self.finish(game, ctx, Ok(outputs.outcome))?;
                outputs.synchronize_observations();
                Ok(outputs)
            }
            Err(error) => self
                .finish(game, ctx, Err(error))
                .map(crate::effects::CompletedEffectOutputs::aggregate_only),
        }
    }
}

struct AdaptedOriginalCompletion {
    inner: Box<dyn crate::effects::SimultaneousEffectCompletion>,
    adapter: Box<dyn OriginalOutcomeAdapter>,
}

impl crate::effects::SimultaneousEffectCompletion for AdaptedOriginalCompletion {
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
        let result = self
            .inner
            .complete_original_phase_with_outputs(game, ctx, original);
        match result {
            Ok(receipt) => adapt_original_outcome_with_outputs(receipt, self.adapter, game, ctx),
            Err(error) => self
                .adapter
                .finish_with_outputs(game, ctx, Err(error))
                .map(crate::effects::SimultaneousEffectCommit::finished),
        }
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
        let result = self
            .inner
            .complete_original_phase_from_outputs(game, ctx, original);
        match result {
            Ok(receipt) => adapt_original_outcome_with_outputs(receipt, self.adapter, game, ctx),
            Err(error) => self
                .adapter
                .finish_with_outputs(game, ctx, Err(error))
                .map(crate::effects::SimultaneousEffectCommit::finished),
        }
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
        let result = self
            .inner
            .prepare_draw_boundary_with_outputs(game, ctx, original);
        match result {
            Ok(receipt) => adapt_original_outcome_with_outputs(receipt, self.adapter, game, ctx),
            Err(error) => self
                .adapter
                .finish_with_outputs(game, ctx, Err(error))
                .map(crate::effects::SimultaneousEffectCommit::finished),
        }
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
        let result = self
            .inner
            .prepare_draw_boundary_from_outputs(game, ctx, original);
        match result {
            Ok(receipt) => adapt_original_outcome_with_outputs(receipt, self.adapter, game, ctx),
            Err(error) => self
                .adapter
                .finish_with_outputs(game, ctx, Err(error))
                .map(crate::effects::SimultaneousEffectCommit::finished),
        }
    }

    fn observe_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut crate::effects::ExecutionContext,
        original: &mut EffectOutcome,
    ) -> Result<(), crate::effects::ExecutionError> {
        self.inner.observe_original(game, ctx, original)
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
        let result = self.inner.complete_with_outputs(game, ctx, original);
        self.adapter.finish_with_outputs(game, ctx, result)
    }

    fn complete_from_original_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: crate::effects::CompletedEffectOutputs,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        let result = self
            .inner
            .complete_from_original_outputs(game, ctx, original);
        self.adapter.finish_with_outputs(game, ctx, result)
    }
}

/// Preserve the child's prepared original and deferred programs. Decorations
/// run once after completion, rather than forcing the full child to execute
/// while its siblings are still committing their originals.
pub(crate) fn adapt_original_outcome_with_outputs(
    mut receipt: SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
    adapter: Box<dyn OriginalOutcomeAdapter>,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Result<SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>, ExecutionError> {
    if ctx.decision_maker.awaiting_choice() {
        // Restore decorator state now; a suspended receipt will never reach
        // its deferred completion. The enclosing transaction owns replay.
        adapter.finish(game, ctx, Ok(receipt.outcome.outcome))?;
        return Ok(SimultaneousEffectCommit::finished(
            crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
        ));
    }
    if let Some(inner) = receipt.completion.take() {
        receipt.completion = Some(Box::new(AdaptedOriginalCompletion { inner, adapter }));
    } else {
        receipt.outcome = adapter.finish_with_outputs(game, ctx, Ok(receipt.outcome))?;
    }
    Ok(receipt)
}

/// Keep a prepared participant's complete context for its deferred programs.
/// The enclosing batch context is restored on success, suspension and error.
struct ScopedOriginalCompletion {
    context: crate::effects::ExecutionContextCheckpoint,
    inner: Box<dyn crate::effects::SimultaneousEffectCompletion>,
}

impl crate::effects::SimultaneousEffectCompletion for ScopedOriginalCompletion {
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
        let parent = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        self.context.restore_ref_preserving_resolution_control(ctx);
        let result = self
            .inner
            .complete_original_phase_with_outputs(game, ctx, original)
            .map(|receipt| with_original_execution_context(receipt, ctx));
        if result.is_ok() && !ctx.decision_maker.awaiting_choice() {
            parent.restore_preserving_resolution_control(ctx);
        } else {
            parent.restore(ctx);
        }
        result
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
        let parent = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        self.context.restore_ref_preserving_resolution_control(ctx);
        let result = self
            .inner
            .complete_original_phase_from_outputs(game, ctx, original)
            .map(|receipt| with_original_execution_context(receipt, ctx));
        if result.is_ok() && !ctx.decision_maker.awaiting_choice() {
            parent.restore_preserving_resolution_control(ctx);
        } else {
            parent.restore(ctx);
        }
        result
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
        let parent = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        self.context.restore_ref_preserving_resolution_control(ctx);
        let result = self
            .inner
            .prepare_draw_boundary_with_outputs(game, ctx, original)
            .map(|receipt| with_original_execution_context(receipt, ctx));
        if result.is_ok() && !ctx.decision_maker.awaiting_choice() {
            parent.restore_preserving_resolution_control(ctx);
        } else {
            parent.restore(ctx);
        }
        result
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
        let parent = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        self.context.restore_ref_preserving_resolution_control(ctx);
        let result = self
            .inner
            .prepare_draw_boundary_from_outputs(game, ctx, original)
            .map(|receipt| with_original_execution_context(receipt, ctx));
        if result.is_ok() && !ctx.decision_maker.awaiting_choice() {
            parent.restore_preserving_resolution_control(ctx);
        } else {
            parent.restore(ctx);
        }
        result
    }

    fn observe_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut crate::effects::ExecutionContext,
        original: &mut EffectOutcome,
    ) -> Result<(), crate::effects::ExecutionError> {
        let parent = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        self.context.restore_ref(ctx);
        let result = self.inner.observe_original(game, ctx, original);
        parent.restore(ctx);
        result
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
        let parent = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        self.context.restore_ref_preserving_resolution_control(ctx);
        let result = self.inner.complete_with_outputs(game, ctx, original);
        if result.is_ok() && !ctx.decision_maker.awaiting_choice() {
            parent.restore_preserving_resolution_control(ctx);
        } else {
            parent.restore(ctx);
        }
        result
    }

    fn complete_from_original_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: crate::effects::CompletedEffectOutputs,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        let parent = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        self.context.restore_ref_preserving_resolution_control(ctx);
        let result = self
            .inner
            .complete_from_original_outputs(game, ctx, original);
        if result.is_ok() && !ctx.decision_maker.awaiting_choice() {
            parent.restore_preserving_resolution_control(ctx);
        } else {
            parent.restore(ctx);
        }
        result
    }
}

pub(crate) fn with_original_execution_context<Output>(
    mut receipt: SimultaneousEffectCommit<Output>,
    ctx: &ExecutionContext,
) -> SimultaneousEffectCommit<Output> {
    receipt.completion = receipt.completion.map(|inner| {
        Box::new(ScopedOriginalCompletion {
            context: crate::effects::ExecutionContextCheckpoint::capture(ctx),
            inner,
        }) as Box<dyn crate::effects::SimultaneousEffectCompletion>
    });
    receipt
}

/// Hold original trigger publication without allocating a new action identity.
/// Damage already has an occurrence batch and observes the damage before its
/// consequences; its original consequences must retain that existing identity.
pub(crate) fn with_held_original_triggers<T>(
    game: &mut GameState,
    body: impl FnOnce(&mut GameState) -> Result<T, ExecutionError>,
) -> Result<T, ExecutionError> {
    game.effect_store.trigger_matching_holds += 1;
    let result = body(game);
    game.effect_store.trigger_matching_holds -= 1;
    result
}

/// Compose prepared receipts without letting any child's additions run before
/// the enclosing batch commits all originals. Each completion receives only
/// its own original observations and the batch observer's publication flags.
struct GroupedOriginalCompletion {
    receipts: Vec<SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>>,
    project: Box<dyn Fn(Vec<EffectOutcome>) -> Result<EffectOutcome, ExecutionError> + Send>,
}

impl crate::effects::SimultaneousEffectCompletion for GroupedOriginalCompletion {
    fn original_phase_status(&self) -> crate::effects::OriginalPhaseStatus {
        original_cohort_phase_status(&self.receipts)
    }

    fn complete_original_phase_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>, ExecutionError>
    {
        let Self { receipts, project } = *self;
        let receipts = receipts
            .into_iter()
            .map(|mut receipt| {
                inherit_original_observations(&mut receipt.outcome.outcome, &original.events);
                receipt
            })
            .collect();
        let Some(retained) = complete_original_cohort_phase_with_outputs(game, ctx, receipts)?
        else {
            return Ok(SimultaneousEffectCommit::finished(
                crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            ));
        };
        compose_original_commits_with_fallible_projection_outputs(retained, project)
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
        let Self { receipts, project } = *self;
        let mut retained = Vec::new();
        let mut paused = false;
        for mut receipt in receipts {
            inherit_original_observations(&mut receipt.outcome.outcome, &original.events);
            if !paused && !ctx.resolution_stopped() {
                if let Some(completion) = receipt.completion.take() {
                    let original = receipt.outcome;
                    receipt = completion.prepare_draw_boundary_from_outputs(game, ctx, original)?;
                }
                paused = receipt.completion.is_some();
            }
            retained.push(receipt);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(crate::effects::SimultaneousEffectCommit::finished(
                    crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
                ));
            }
        }
        if ctx.resolution_stopped() {
            for receipt in &mut retained {
                receipt.completion = None;
            }
        }
        compose_original_commits_with_fallible_projection_outputs(retained, project)
    }

    fn observe_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut crate::effects::ExecutionContext,
        original: &mut EffectOutcome,
    ) -> Result<(), crate::effects::ExecutionError> {
        for receipt in &mut self.receipts {
            inherit_original_observations(&mut receipt.outcome.outcome, &original.events);
            if let Some(completion) = &mut receipt.completion {
                completion.observe_original(game, ctx, &mut receipt.outcome.outcome)?;
            }
            inherit_original_observations(original, &receipt.outcome.outcome.events);
            if ctx.decision_maker.awaiting_choice() {
                break;
            }
        }
        Ok(())
    }

    fn freeze(&mut self, game: &mut GameState) -> Result<(), ExecutionError> {
        for receipt in &mut self.receipts {
            crate::effects::outcome_recording::complete_outcome(
                game,
                None,
                None,
                &mut receipt.outcome.outcome,
                Vec::new(),
            );
            if let Some(completion) = &mut receipt.completion {
                completion.freeze(game)?;
            }
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
        if self.original_phase_status() == crate::effects::OriginalPhaseStatus::Retained {
            let receipt = self.complete_original_phase_with_outputs(game, ctx, original)?;
            return complete_committed_original_with_outputs(game, ctx, receipt);
        }

        let receipts = self.receipts.into_iter().map(|mut receipt| {
            inherit_original_observations(&mut receipt.outcome.outcome, &original.events);
            receipt
        });
        let Some(children) =
            complete_retained_originals_with_outputs(game, ctx, receipts, |_, _, _, _| Ok(()))?
        else {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        };
        let projections_complete =
            !children.is_empty() && children.iter().all(|outputs| outputs.projections_complete);
        let outcome = (self.project)(children.iter().map(|child| child.outcome.clone()).collect())?;
        let mut outputs = crate::effects::CompletedEffectOutputs::aggregate_only(outcome);
        outputs.projections_complete = projections_complete;
        // A grouped projection owns these child receipts, but declares no
        // participant association across their separate semantic actions.
        outputs.retain_batch_children(children);
        Ok(outputs)
    }
}

pub(crate) fn inherit_original_observations(
    outcome: &mut EffectOutcome,
    observed: &[crate::triggers::TriggerEvent],
) {
    inherit_observed_events(&mut outcome.events, observed);
    if let Some(original) = &mut outcome.instruction_result {
        inherit_original_observations(original, observed);
    }
}

pub(crate) fn inherit_observed_events(
    events: &mut [crate::triggers::TriggerEvent],
    observed: &[crate::triggers::TriggerEvent],
) {
    for event in events {
        if let Some(observed) = observed
            .iter()
            .find(|observed| observed.occurrence_key() == event.occurrence_key())
        {
            *event = observed.clone();
        }
    }
}

pub(crate) fn compose_original_commits_with_outputs(
    receipts: Vec<SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>>,
) -> SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs> {
    compose_original_commits_with_projection_outputs(receipts, Box::new(EffectOutcome::aggregate))
}

/// Result projection is metadata owned by the enclosing compound. It never
/// changes child action receipts or advances a deferred program early.
pub(crate) fn compose_original_commits_with_projection_outputs(
    receipts: Vec<SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>>,
    project: Box<dyn Fn(Vec<EffectOutcome>) -> EffectOutcome + Send>,
) -> SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs> {
    let outcome = project(
        receipts
            .iter()
            .map(|receipt| receipt.outcome.outcome.clone())
            .collect(),
    );
    compose_projected_original_commits(
        receipts,
        outcome,
        Box::new(move |outcomes| Ok(project(outcomes))),
    )
}

/// Checked result projection shares the same grouped original/completion owner.
/// Errors propagate through the enclosing action's resource transaction.
pub(crate) fn compose_original_commits_with_fallible_projection_outputs(
    receipts: Vec<SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>>,
    project: Box<dyn Fn(Vec<EffectOutcome>) -> Result<EffectOutcome, ExecutionError> + Send>,
) -> Result<SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>, ExecutionError> {
    let outcome = project(
        receipts
            .iter()
            .map(|receipt| receipt.outcome.outcome.clone())
            .collect(),
    )?;
    Ok(compose_projected_original_commits(
        receipts, outcome, project,
    ))
}

fn compose_projected_original_commits(
    receipts: Vec<SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>>,
    outcome: EffectOutcome,
    project: Box<dyn Fn(Vec<EffectOutcome>) -> Result<EffectOutcome, ExecutionError> + Send>,
) -> SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs> {
    let mut outputs = crate::effects::CompletedEffectOutputs::aggregate_only(outcome);
    if receipts.iter().any(|receipt| receipt.completion.is_some()) {
        SimultaneousEffectCommit {
            outcome: outputs,
            completion: Some(Box::new(GroupedOriginalCompletion { receipts, project })),
        }
    } else {
        outputs.projections_complete = !receipts.is_empty();
        for receipt in receipts {
            outputs.projections_complete &= receipt.outcome.projections_complete;
            outputs.retain_batch_children([receipt.outcome]);
        }
        SimultaneousEffectCommit::finished(outputs)
    }
}

/// Ordinary commitment of a prepared compound uses the same original and
/// completion coordinator as an enclosing simultaneous action.
pub(crate) fn complete_prepared_original(
    proposal: Box<dyn crate::effects::SimultaneousEffectProposal>,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Result<EffectOutcome, ExecutionError> {
    complete_prepared_original_with_grouping(proposal, game, ctx, true)
}

/// Complete a retained proposal through the same transaction and observer
/// boundary, without requiring a single original to invent simultaneity.
pub(crate) fn complete_prepared_original_with_grouping(
    proposal: Box<dyn crate::effects::SimultaneousEffectProposal>,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    simultaneous: bool,
) -> Result<EffectOutcome, ExecutionError> {
    complete_prepared_original_with_outputs(proposal, game, ctx, simultaneous)
        .map(crate::effects::CompletedEffectOutputs::into_outcome)
}

/// Retain the proposal owner's outputs through the same resource transaction,
/// affordability checks, original preparation and observation coordinator.
pub(crate) fn complete_prepared_original_with_outputs(
    mut proposal: Box<dyn crate::effects::SimultaneousEffectProposal>,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    simultaneous: bool,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    crate::effects::tokens::execute_resource_transaction_with_pending_value(
        game,
        ctx,
        || {
            crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::with_objects(
                Vec::new(),
            ))
        },
        |game, ctx| {
            if ctx.decision_maker.awaiting_choice() {
                return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::count(0),
                ));
            }
            if !crate::effects::can_pay_declared_resources(
                game,
                &proposal.declared_payment_resources(),
            ) {
                return Err(ExecutionError::Impossible(
                    "prepared compound payments exceed available resources".into(),
                ));
            }
            proposal.prepare_selection(game, ctx)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::count(0),
                ));
            }
            proposal.prepare_original(game, ctx)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::count(0),
                ));
            }
            if !crate::effects::can_pay_declared_resources(
                game,
                &proposal.declared_payment_resources(),
            ) {
                return Err(ExecutionError::Impossible(
                    "prepared compound payments exceed available resources".into(),
                ));
            }
            // Deferred branch preparation can reveal an intrinsically
            // simultaneous child. Ask its owner after preparation, before
            // opening the original observation and look-back boundary.
            let simultaneous = simultaneous || proposal.has_simultaneous_originals();
            let outcomes = execute_simultaneous_originals_with_default_outputs(
                game,
                ctx,
                simultaneous,
                |game, ctx| {
                    let pinned = simultaneous.then(|| {
                        crate::effects::helpers::begin_simultaneous_zone_change_lookback(game)
                    });
                    let result = (|| {
                        proposal.seal_original(game, ctx)?;
                        if ctx.decision_maker.awaiting_choice() {
                            return Ok(crate::effects::SimultaneousEffectCommit::finished(
                                crate::effects::CompletedEffectOutputs::aggregate_only(
                                    EffectOutcome::count(0),
                                ),
                            ));
                        }
                        if !crate::effects::can_pay_declared_resources(
                            game,
                            &proposal.declared_payment_resources(),
                        ) {
                            return Err(ExecutionError::Impossible(
                                "prepared compound payments exceed available resources".into(),
                            ));
                        }
                        proposal.commit_original_with_outputs(game, ctx)
                    })();
                    if let Some(pinned) = pinned {
                        crate::effects::helpers::end_simultaneous_zone_change_lookback(
                            game, pinned,
                        );
                    }
                    result.map(|receipt| vec![receipt])
                },
            )?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::count(0),
                ));
            }
            let aggregate =
                EffectOutcome::aggregate(outcomes.iter().map(|outputs| outputs.outcome.clone()));
            // This coordinator submitted exactly one prepared owner. Preserve
            // its direct participant routing instead of inventing a parent.
            let mut outcomes = outcomes.into_iter();
            let outputs = outcomes.next().ok_or_else(|| {
                ExecutionError::InternalError("missing standalone prepared owner".into())
            })?;
            if outcomes.next().is_some() {
                return Err(ExecutionError::InternalError(
                    "multiple standalone prepared owners".into(),
                ));
            }
            Ok(outputs.project_aggregate(aggregate))
        },
    )
}
