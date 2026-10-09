//! Checked original life payments, loss companions and event-time observers.
use super::*;
use crate::effect::EffectOutcome;
use crate::effects::ExecutionError;
use crate::events::{EventKind, LifeLossEvent, LifePaidEvent};
use crate::triggers::{TriggerEvent, TriggerQueue};
pub(crate) struct PreparedLifePayment {
    player: PlayerId,
    amount: u32,
    original: crate::events::processing::TraitEventResult,
    original_context: Option<crate::effects::ExecutionContextCheckpoint>,
}

impl PreparedLifePayment {
    pub(crate) fn requires_replacement_input(&self) -> bool {
        self.original.requires_replacement_input()
    }
}

struct PaymentOriginalCompletion {
    inner: Box<dyn crate::effects::SimultaneousEffectCompletion>,
    context: crate::effects::ExecutionContextCheckpoint,
}
impl PaymentOriginalCompletion {
    /// Forward one actual phase through the retained scope and continuation.
    fn advance(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut crate::effects::ExecutionContext,
        phase: crate::effects::composition::CompletionPhase,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        ExecutionError,
    > {
        let Self { inner, context } = *self;
        let parent = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        context.restore_ref(ctx);
        let result = phase.dispatch(inner, game, ctx);
        let context = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        parent.restore(ctx);
        let mut receipt = result?;
        receipt.completion = receipt.completion.map(|inner| {
            Box::new(PaymentOriginalCompletion { inner, context })
                as Box<dyn crate::effects::SimultaneousEffectCompletion>
        });
        Ok(receipt)
    }
}

impl crate::effects::SimultaneousEffectCompletion for PaymentOriginalCompletion {
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
        ExecutionError,
    > {
        self.advance(
            game,
            ctx,
            crate::effects::composition::CompletionPhase::OriginalOutcome(original),
        )
    }

    fn complete_original_phase_from_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut crate::effects::ExecutionContext,
        original: crate::effects::CompletedEffectOutputs,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        ExecutionError,
    > {
        self.advance(
            game,
            ctx,
            crate::effects::composition::CompletionPhase::OriginalOutputs(original),
        )
    }

    fn prepare_draw_boundary_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut crate::effects::ExecutionContext,
        original: EffectOutcome,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        ExecutionError,
    > {
        self.advance(
            game,
            ctx,
            crate::effects::composition::CompletionPhase::DrawOutcome(original),
        )
    }

    fn prepare_draw_boundary_from_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut crate::effects::ExecutionContext,
        original: crate::effects::CompletedEffectOutputs,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        ExecutionError,
    > {
        self.advance(
            game,
            ctx,
            crate::effects::composition::CompletionPhase::DrawOutputs(original),
        )
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
        ctx: &mut crate::effects::ExecutionContext,
        original: EffectOutcome,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.complete_with_outputs(game, ctx, original)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }
    fn complete_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut crate::effects::ExecutionContext,
        original: EffectOutcome,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        let parent = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        self.context.restore(ctx);
        let outputs = self.inner.complete_with_outputs(game, ctx, original);
        parent.restore(ctx);
        outputs
    }

    fn complete_from_original_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut crate::effects::ExecutionContext,
        original: crate::effects::CompletedEffectOutputs,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        let parent = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        self.context.restore(ctx);
        let outputs = self
            .inner
            .complete_from_original_outputs(game, ctx, original);
        parent.restore(ctx);
        outputs
    }
}

/// A simultaneous replacement may substitute one prepared life action. The
/// replacement's bindings are retained while its actual recipient/amount is
/// evaluated before any other original. Compound action schedules need their
/// own shared iterator; reject them during preparation rather than executing
/// a prefix while another payer's original is still pending.
fn prepare_simultaneous_payment_original(
    game: &mut GameState,
    ctx: &mut crate::effects::ExecutionContext,
    prepared: crate::events::processing::TraitEventResult,
) -> Result<
    (
        crate::events::processing::TraitEventResult,
        Option<crate::effects::ExecutionContextCheckpoint>,
    ),
    ExecutionError,
> {
    use crate::events::processing::TraitEventResult;
    let (original, programs) = prepared.into_expansion();
    let (original, scope) = if let TraitEventResult::Replaced {
        effects,
        source,
        controller,
        context,
        ..
    } = &original
    {
        let [effect] = effects.as_slice() else {
            return Err(ExecutionError::UnresolvableValue("simultaneous life-payment replacement requires a prepared single action; compound Instead program is not represented".into()));
        };
        crate::effects::replacement::with_replacement_child(
            game,
            ctx,
            *source,
            *controller,
            context,
            None,
            None,
            vec![],
            |game, child| {
                let event = effect
                    .0
                    .replacement_original_event(game, child)?
                    .ok_or_else(|| ExecutionError::UnresolvableValue(
                        "simultaneous life-payment Instead action has no prepared original boundary".into(),
                    ))?;
                let next =
                    crate::effects::life::life_change::prepare_life_change(game, child, event)?;
                let (next, scope) = prepare_simultaneous_payment_original(game, child, next)?;
                Ok((
                    next,
                    scope.or_else(|| {
                        Some(crate::effects::ExecutionContextCheckpoint::capture(child))
                    }),
                ))
            },
        )?
    } else {
        (original, None)
    };
    let original = if programs.is_empty() {
        original
    } else {
        TraitEventResult::Expanded {
            original: Box::new(original),
            programs,
        }
    };
    Ok((original, scope))
}
impl GameState {
    /// Prepared compounds retain the enclosing action identity while sharing
    /// the payment observer: nominal acknowledgements and actual actions enter
    /// history together before any replacement-added program runs.
    pub(crate) fn observe_prepared_life_payment_originals<'a>(
        &mut self,
        ctx: &crate::effects::ExecutionContext,
        events: impl IntoIterator<Item = &'a mut TriggerEvent>,
    ) -> Result<crate::effects::composition::OriginalTriggerObservation, ExecutionError> {
        use crate::effects::composition::OriginalTriggerObservation;
        let mut events = events.into_iter().collect::<Vec<_>>();
        if !events
            .iter()
            .any(|event| event.downcast::<LifePaidEvent>().is_some())
        {
            return Ok(OriginalTriggerObservation::Capture);
        }
        crate::events::damage::validate_damage_history_amounts(
            self,
            events.iter().map(|event| &**event),
        )?;
        for event in &events {
            self.stage_turn_history_event(event);
        }
        self.refresh_continuous_state()
            .map_err(ExecutionError::ContinuousDiscovery)?;
        crate::effects::capture_triggers_before_added_program(
            self,
            ctx,
            None,
            events.iter_mut().map(|event| &mut **event),
        )?;
        Ok(OriginalTriggerObservation::OwnerPublished)
    }

    fn publish_life_payment_receipts(
        &mut self,
        outcomes: &mut [EffectOutcome],
    ) -> Result<(), ExecutionError> {
        if outcomes.iter().all(|outcome| outcome.events.is_empty()) {
            return Ok(());
        }
        let batch = self
            .provenance_graph_mut()
            .alloc_root_event(EventKind::LifePaid);
        let mut events = Vec::new();
        for outcome in outcomes.iter_mut() {
            for event in &mut outcome.events {
                *event = event.clone().with_simultaneous_batch(batch);
                events.push(event.clone());
            }
        }
        crate::events::damage::validate_damage_history_amounts(self, events.iter())?;
        for event in &events {
            self.stage_turn_history_event(event);
        }
        self.refresh_continuous_state()
            .map_err(ExecutionError::ContinuousDiscovery)?;
        // Record both the actual loss and the payment before evaluating any
        // observer. Do not drain other held original events or resolve triggers
        // while a containing cast/activation is still paying its other costs.
        let mut matched = TriggerQueue::new();
        crate::game_loop::try_queue_triggers_from_reported_events(
            self,
            &mut matched,
            events,
            true,
        )?;
        self.defer_trigger_entries(matched.take_all());
        for outcome in outcomes {
            for event in &mut outcome.events {
                event.mark_triggers_captured();
                self.queue_trigger_event(event.provenance(), event.clone());
            }
        }
        Ok(())
    }

    /// Prepare the action used to pay the authored amount (CR 118.11, 119.4).
    /// Replacement processing changes the actual action, never the accepted
    /// nominal payment. All simultaneous proposals prepare before originals.
    pub(crate) fn prepare_life_payment(
        &mut self,
        player: PlayerId,
        amount: u32,
        ctx: &mut crate::effects::ExecutionContext,
        simultaneous: bool,
    ) -> Result<Option<PreparedLifePayment>, ExecutionError> {
        self.refresh_continuous_state()
            .map_err(ExecutionError::ContinuousDiscovery)?;
        if !self.can_pay_life(player, amount) {
            return Ok(None);
        }
        crate::events::damage::checked_damage_count(u128::from(amount), "life payment result")?;
        let provenance = self.alloc_child_event_provenance(ctx.provenance, EventKind::LifeLoss);
        let original = crate::effects::life::life_change::prepare_life_change(
            self,
            ctx,
            crate::events::Event::new_with_provenance(
                LifeLossEvent::from_effect(player, amount),
                provenance,
            ),
        )?;
        let (original, original_context) = if simultaneous {
            prepare_simultaneous_payment_original(self, ctx, original)?
        } else {
            (original, None)
        };
        Ok(Some(PreparedLifePayment {
            player,
            amount,
            original,
            original_context,
        }))
    }

    pub(crate) fn commit_life_payment_original_with_outputs(
        &mut self,
        prepared: PreparedLifePayment,
        ctx: &mut crate::effects::ExecutionContext,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        ExecutionError,
    > {
        let parent = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        if let Some(scope) = &prepared.original_context {
            scope.restore_ref(ctx);
        }
        let committed =
            crate::effects::life::life_change::commit_prepared_life_original_with_outputs(
                self,
                ctx,
                prepared.original,
            );
        parent.restore(ctx);
        let mut receipt = committed?;
        if let Some(context) = prepared.original_context {
            if let Some(inner) = receipt.completion.take() {
                receipt.completion = Some(Box::new(PaymentOriginalCompletion { inner, context }));
            }
        }
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::SimultaneousEffectCommit::finished(
                crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            ));
        }
        // A replacement that prevents/replaces the action still fulfills the
        // accepted cost. Outcome life metrics read the actual loss events.
        let provenance = self.alloc_child_event_provenance(ctx.provenance, EventKind::LifePaid);
        let mut event = TriggerEvent::new_with_provenance(
            LifePaidEvent {
                player: prepared.player,
                amount: prepared.amount,
            },
            provenance,
        );
        if let Some(batch) = self.simultaneous_action_batch() {
            event = event.with_simultaneous_batch(batch);
        }
        let actual_action = receipt.outcome;
        let acknowledgement = EffectOutcome::aggregate_with_primary_result(
            EffectOutcome::count(i64::from(prepared.amount)),
            [
                actual_action.outcome.clone(),
                EffectOutcome::resolved().with_event(event),
            ],
        );
        receipt.outcome = actual_action.project_aggregate(acknowledgement);
        Ok(receipt)
    }

    /// Commit retained accepted payments through the same observer as fresh
    /// payments. Caller owns affordability, transaction rollback and replay.
    pub(crate) fn complete_life_payment_originals_with_outputs(
        &mut self,
        prepared: Vec<PreparedLifePayment>,
        ctx: &mut crate::effects::ExecutionContext,
        simultaneous: bool,
    ) -> Result<Vec<crate::effects::CompletedEffectOutputs>, ExecutionError> {
        crate::effects::composition::execute_simultaneous_originals_with_outputs(
            self,
            ctx,
            simultaneous,
            |game, ctx| {
                let mut receipts = Vec::with_capacity(prepared.len());
                for proposal in prepared {
                    receipts.push(game.commit_life_payment_original_with_outputs(proposal, ctx)?);
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(Vec::new());
                    }
                }
                Ok(receipts)
            },
            |game, _, receipts| {
                let mut outcomes = receipts
                    .iter()
                    .map(|receipt| receipt.outcome.outcome.clone())
                    .collect::<Vec<_>>();
                game.publish_life_payment_receipts(&mut outcomes)?;
                for (receipt, outcome) in receipts.iter_mut().zip(outcomes) {
                    receipt.outcome = std::mem::replace(
                        &mut receipt.outcome,
                        crate::effects::CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::count(0),
                        ),
                    )
                    .project_aggregate(outcome);
                }
                Ok(crate::effects::composition::OriginalTriggerObservation::OwnerPublished)
            },
        )
    }

    /// One acknowledged payment, with the real caller's replacement choices,
    /// temporary replacement scope, and source context retained.
    pub(crate) fn pay_life_with_context(
        &mut self,
        player: PlayerId,
        amount: u32,
        ctx: &mut crate::effects::ExecutionContext,
    ) -> Result<Option<EffectOutcome>, ExecutionError> {
        self.pay_life_with_context_and_outputs(player, amount, ctx)
            .map(|outputs| outputs.map(crate::effects::CompletedEffectOutputs::into_outcome))
    }

    pub(crate) fn pay_life_with_context_and_outputs(
        &mut self,
        player: PlayerId,
        amount: u32,
        ctx: &mut crate::effects::ExecutionContext,
    ) -> Result<Option<crate::effects::CompletedEffectOutputs>, ExecutionError> {
        self.pay_life_outputs_simultaneously_with_context(&[(player, amount)], ctx)
            .map(|outcomes| outcomes.and_then(|mut outcomes| outcomes.pop()))
    }
    /// Choice-free convenience entry. Actual cost/effect producers retain their
    /// caller's decision maker via pay_life_with_context.
    pub fn pay_life(&mut self, player: PlayerId, amount: u32) -> Result<bool, ExecutionError> {
        let mut ctx = crate::effects::ExecutionContext::new_default(ObjectId::from_raw(0), player);
        self.pay_life_with_context(player, amount, &mut ctx)
            .map(|receipt| receipt.is_some())
    }
    pub fn pay_life_simultaneously(
        &mut self,
        payments: &[(PlayerId, u32)],
    ) -> Result<bool, ExecutionError> {
        let player = payments
            .first()
            .map_or(self.turn.active_player, |payment| payment.0);
        let mut ctx = crate::effects::ExecutionContext::new_default(ObjectId::from_raw(0), player);
        self.pay_life_receipts_simultaneously_with_context(payments, &mut ctx)
            .map(|receipts| receipts.is_some())
    }
    fn pay_life_receipts_simultaneously_with_context(
        &mut self,
        payments: &[(PlayerId, u32)],
        ctx: &mut crate::effects::ExecutionContext,
    ) -> Result<Option<Vec<EffectOutcome>>, ExecutionError> {
        self.pay_life_outputs_simultaneously_with_context(payments, ctx)
            .map(|outputs| {
                outputs.map(|outputs| {
                    outputs
                        .into_iter()
                        .map(crate::effects::CompletedEffectOutputs::into_outcome)
                        .collect()
                })
            })
    }

    fn pay_life_outputs_simultaneously_with_context(
        &mut self,
        payments: &[(PlayerId, u32)],
        ctx: &mut crate::effects::ExecutionContext,
    ) -> Result<Option<Vec<crate::effects::CompletedEffectOutputs>>, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(None);
        }
        let (root, meter) = self.begin_token_resource_scope();
        let checkpoint = self.clone();
        let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        let mut result = match self.token_resource_failure() {
            Some(error) => Err(error),
            None => (|| {
                self.refresh_continuous_state()
                    .map_err(ExecutionError::ContinuousDiscovery)?;
                if !self.can_pay_life_simultaneously(payments) {
                    return Ok(None);
                }
                let apnap = self.team_apnap_player_order();
                let mut ordered = payments.iter().copied().enumerate().collect::<Vec<_>>();
                ordered.sort_by_key(|(index, (player, _))| {
                    (
                        apnap
                            .iter()
                            .position(|candidate| candidate == player)
                            .unwrap_or(usize::MAX),
                        *index,
                    )
                });
                let input_indices = ordered.iter().map(|(index, _)| *index).collect::<Vec<_>>();
                let mut prepared = Vec::new();
                for (_, (player, amount)) in ordered {
                    let Some(proposal) =
                        self.prepare_life_payment(player, amount, ctx, payments.len() > 1)?
                    else {
                        return Ok(None);
                    };
                    prepared.push(proposal);
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(None);
                    }
                }
                let completed = self.complete_life_payment_originals_with_outputs(
                    prepared,
                    ctx,
                    payments.len() > 1,
                )?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(None);
                }
                let mut mapped = input_indices.into_iter().zip(completed).collect::<Vec<_>>();
                mapped.sort_by_key(|(index, _)| *index);
                Ok(Some(
                    mapped.into_iter().map(|(_, outcome)| outcome).collect(),
                ))
            })(),
        };
        if let Err(error) = &result {
            self.record_token_resource_failure(error);
        }
        if let Some(error) = self.token_resource_failure() {
            result = Err(error);
        }
        if !matches!(result, Ok(Some(_))) || ctx.decision_maker.awaiting_choice() {
            self.restore_execution_checkpoint(
                checkpoint,
                result.is_ok() && ctx.decision_maker.awaiting_choice(),
            );
            context_checkpoint.restore(ctx);
        }
        self.end_token_resource_scope(root, &meter);
        result
    }
}

#[cfg(test)]
#[path = "life_payment_tests.rs"]
mod tests;
