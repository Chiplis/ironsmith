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

struct PaymentOriginalCompletion {
    inner: Box<dyn crate::effects::SimultaneousEffectCompletion>,
    context: crate::effects::ExecutionContextCheckpoint,
}
impl crate::effects::SimultaneousEffectCompletion for PaymentOriginalCompletion {
    fn freeze(&mut self, game: &mut GameState) -> Result<(), ExecutionError> {
        self.inner.freeze(game)
    }
    fn complete(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut crate::effects::ExecutionContext,
        original: EffectOutcome,
    ) -> Result<EffectOutcome, ExecutionError> {
        let parent = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        self.context.restore(ctx);
        let outcome = self.inner.complete(game, ctx, original);
        parent.restore(ctx);
        outcome
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
        if effect
            .downcast_ref::<crate::effects::GainLifeEffect>()
            .is_none()
            && effect
                .downcast_ref::<crate::effects::LoseLifeEffect>()
                .is_none()
        {
            return Err(ExecutionError::UnresolvableValue(
                "simultaneous life-payment Instead action has no prepared original boundary".into(),
            ));
        }
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
                let event = if let Some(gain) =
                    effect.downcast_ref::<crate::effects::GainLifeEffect>()
                {
                    let player = crate::effects::helpers::resolve_player_from_spec(
                        game,
                        &gain.player,
                        child,
                    )?;
                    let amount = crate::effects::helpers::resolve_value(game, &gain.amount, child)?
                        .max(0) as u32;
                    TriggerEvent::new_with_provenance(
                        crate::events::LifeGainEvent::new(player, amount).with_source(child.source),
                        child.provenance,
                    )
                } else {
                    let loss = effect
                        .downcast_ref::<crate::effects::LoseLifeEffect>()
                        .expect("checked life action");
                    let player = crate::effects::helpers::resolve_player_from_spec(
                        game,
                        &loss.player,
                        child,
                    )?;
                    let amount = crate::effects::helpers::resolve_value(game, &loss.amount, child)?
                        .max(0) as u32;
                    TriggerEvent::new_with_provenance(
                        LifeLossEvent::from_effect(player, amount),
                        child.provenance,
                    )
                };
                let next =
                    crate::effects::life::life_change::prepare_life_change(game, child, crate::events::Event::from_raw(event))?;
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
        crate::game_loop::queue_triggers_from_reported_events(self, &mut matched, events, true);
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

    pub(crate) fn commit_life_payment_original(
        &mut self,
        prepared: PreparedLifePayment,
        ctx: &mut crate::effects::ExecutionContext,
    ) -> Result<crate::effects::SimultaneousEffectCommit, ExecutionError> {
        let parent = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        if let Some(scope) = &prepared.original_context {
            scope.restore_ref(ctx);
        }
        let committed = crate::effects::life::life_change::commit_prepared_life_original(
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
                EffectOutcome::count(0),
            ));
        }
        // A replacement that prevents/replaces the action still fulfills the
        // accepted cost. Outcome life metrics read the actual loss events.
        receipt.outcome.status = crate::effect::OutcomeStatus::Succeeded;
        receipt.outcome.value = crate::effect::OutcomeValue::Count(i64::from(prepared.amount));
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
        receipt.outcome.events.push(event);
        Ok(receipt)
    }

    /// One acknowledged payment, with the real caller's replacement choices,
    /// temporary replacement scope, and source context retained.
    pub(crate) fn pay_life_with_context(
        &mut self,
        player: PlayerId,
        amount: u32,
        ctx: &mut crate::effects::ExecutionContext,
    ) -> Result<Option<EffectOutcome>, ExecutionError> {
        self.pay_life_receipts_simultaneously_with_context(&[(player, amount)], ctx)
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
                let mut receipts = Vec::new();
                for proposal in prepared {
                    receipts.push(self.commit_life_payment_original(proposal, ctx)?);
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(None);
                    }
                }
                // Freeze every completion before the first added program acts.
                for receipt in &mut receipts {
                    if let Some(completion) = &mut receipt.completion {
                        completion.freeze(self)?;
                    }
                }
                let mut outcomes = receipts
                    .iter()
                    .map(|receipt| receipt.outcome.clone())
                    .collect::<Vec<_>>();
                self.publish_life_payment_receipts(&mut outcomes)?;
                let mut completed = Vec::new();
                for (receipt, outcome) in receipts.into_iter().zip(outcomes) {
                    let outcome = if let Some(completion) = receipt.completion {
                        completion.complete(self, ctx, outcome)?
                    } else {
                        outcome
                    };
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(None);
                    }
                    completed.push(outcome);
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
