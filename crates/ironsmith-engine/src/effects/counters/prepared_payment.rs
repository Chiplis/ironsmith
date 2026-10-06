//! Captured nominal counter payments compose the existing removal owner.

use crate::effect::EffectOutcome;
use crate::effects::{
    CompletedEffectOutputs, ExecutionContext, ExecutionError, PaymentResourceClaim,
    SimultaneousEffectCommit, SimultaneousEffectCompletion, SimultaneousEffectProposal,
};
use crate::events::cause::EventCause;
use crate::events::{CounterRemovalEvent, Event};
use crate::game_state::GameState;

fn with_payment_cause<T>(
    ctx: &mut ExecutionContext,
    cause: &EventCause,
    body: impl FnOnce(&mut ExecutionContext) -> Result<T, ExecutionError>,
) -> Result<T, ExecutionError> {
    let previous = std::mem::replace(&mut ctx.cause, cause.clone());
    let result = body(ctx);
    ctx.cause = previous;
    result
}

/// Capture inputs without replacement choices, original mutation or additions.
/// The enclosing total owns shared affordability and transaction rollback.
pub(crate) fn capture_counter_payment(
    game: &GameState,
    ctx: &ExecutionContext,
    events: Vec<Event>,
) -> Result<Box<dyn SimultaneousEffectProposal>, ExecutionError> {
    capture_counter_payment_with_projection(game, ctx, events, false)
}

/// Counter-quantity costs export zero as well as positive nominal quantity.
/// Energy retains its existing count-only zero receipt through the adapter above.
pub(crate) fn capture_counter_payment_with_quantity(
    game: &GameState,
    ctx: &ExecutionContext,
    events: Vec<Event>,
) -> Result<Box<dyn SimultaneousEffectProposal>, ExecutionError> {
    let proposal = capture_counter_payment_with_projection(game, ctx, events, true)?;
    if proposal.nominal_payment_quantity().is_none() {
        return Err(ExecutionError::Impossible(
            "chosen counter payment exceeds available counters".into(),
        ));
    }
    Ok(proposal)
}

/// Quantity acknowledgement belongs to the captured payment, not physical
/// replacement results or a sum of unrelated compound resource units.
pub(super) fn accepts_counter_quantity_payment(
    proposal: &dyn SimultaneousEffectProposal,
    restriction: Option<crate::object::CounterType>,
) -> bool {
    proposal.nominal_payment_quantity().is_some()
        && proposal.declared_payment_resources().iter().all(|claim| {
            matches!(claim,
                PaymentResourceClaim::Counters {
                    target: crate::game_state::Target::Object(_), counter_type, ..
                } if restriction.is_none_or(|expected| expected == *counter_type)
            )
        })
}

pub(super) fn counter_quantity_payment_x(
    proposal: &dyn SimultaneousEffectProposal,
    restriction: Option<crate::object::CounterType>,
) -> Result<Option<u32>, crate::effects::CostValidationError> {
    if !accepts_counter_quantity_payment(proposal, restriction) {
        return Ok(None);
    }
    proposal
        .nominal_payment_quantity()
        .map(|quantity| {
            u32::try_from(quantity).map_err(|_| {
                crate::effects::CostValidationError::Other(
                    "counter cost X exceeds the supported range".into(),
                )
            })
        })
        .transpose()
}

pub(super) fn validate_counter_quantity_payment(
    outcome: &EffectOutcome,
) -> Result<(), crate::effects::CostValidationError> {
    if outcome.status == crate::effect::OutcomeStatus::Impossible {
        Err(crate::effects::CostValidationError::Other(
            "counter payment was not acknowledged".into(),
        ))
    } else {
        Ok(())
    }
}

/// Decode one nominal counter-payment instruction for every payment path.
/// Quantity sums its own removal events; resource affordability remains with
/// the common subject/type budget owner.
pub(super) fn counter_payment_resources(
    events: &[Event],
) -> Result<(Vec<PaymentResourceClaim>, u64), ExecutionError> {
    let mut resources = Vec::with_capacity(events.len());
    let mut quantity = 0u64;
    for event in events {
        let removal = CounterRemovalEvent::from_event(event.inner()).ok_or_else(|| {
            ExecutionError::InternalError("counter cost requires removal events".into())
        })?;
        resources.push(PaymentResourceClaim::Counters {
            target: removal.target(),
            counter_type: removal.counter_type(),
            count: removal.count(),
        });
        quantity = quantity
            .checked_add(u64::from(removal.count()))
            .ok_or_else(|| {
                ExecutionError::Impossible(
                    "counter cost exceeds the supported quantity range".into(),
                )
            })?;
    }
    Ok((resources, quantity))
}

fn capture_counter_payment_with_projection(
    game: &GameState,
    ctx: &ExecutionContext,
    events: Vec<Event>,
    record_zero_quantity: bool,
) -> Result<Box<dyn SimultaneousEffectProposal>, ExecutionError> {
    let (resources, nominal_quantity) = counter_payment_resources(&events)?;
    let payable = crate::effects::can_pay_declared_resources(game, &resources);
    Ok(Box::new(CapturedCounterPayment {
        events,
        resources,
        payable,
        record_zero_quantity,
        nominal_quantity,
        cause: EventCause::from_cost(ctx.source, ctx.controller),
        prepared: None,
    }))
}

#[derive(Debug)]
struct CapturedCounterPayment {
    events: Vec<Event>,
    resources: Vec<PaymentResourceClaim>,
    payable: bool,
    record_zero_quantity: bool,
    nominal_quantity: u64,
    cause: EventCause,
    prepared: Option<super::placement::PreparedCounterCost>,
}
impl SimultaneousEffectProposal for CapturedCounterPayment {
    fn has_simultaneous_originals(&self) -> bool {
        self.events.len() > 1
    }

    fn nominal_payment_quantity(&self) -> Option<u64> {
        (self.payable && self.record_zero_quantity).then_some(self.nominal_quantity)
    }

    fn declared_payment_resources(&self) -> Vec<PaymentResourceClaim> {
        if self.payable {
            self.resources.clone()
        } else {
            Vec::new()
        }
    }
    fn prepare_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        if self.prepared.is_some() || ctx.decision_maker.awaiting_choice() {
            return Ok(());
        }
        if !self.payable {
            self.prepared = Some(super::placement::PreparedCounterCost::Finished(
                EffectOutcome::impossible(),
            ));
            return Ok(());
        }
        // A zero payment acknowledges acceptance without proposing a removal.
        if self
            .resources
            .iter()
            .all(|claim| matches!(claim, PaymentResourceClaim::Counters { count: 0, .. }))
        {
            let outcome = if self.record_zero_quantity {
                EffectOutcome::count(0).with_requested_amount(0u32)
            } else {
                EffectOutcome::count(0)
            };
            self.prepared = Some(super::placement::PreparedCounterCost::Finished(outcome));
            return Ok(());
        }
        let prepared = with_payment_cause(ctx, &self.cause, |ctx| {
            super::placement::prepare_counter_removal_cost(game, ctx, self.events.clone())
        })?;
        if !ctx.decision_maker.awaiting_choice() {
            self.prepared = Some(prepared);
        }
        Ok(())
    }
    fn commit_original_with_outputs(
        mut self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
        self.prepare_original(game, ctx)?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(SimultaneousEffectCommit::finished(
                CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            ));
        }
        let prepared = self.prepared.take().ok_or_else(|| {
            ExecutionError::InternalError("counter payment lost its prepared owner".into())
        })?;
        let mut receipt = with_payment_cause(ctx, &self.cause, |ctx| {
            Box::new(prepared).commit_original_with_outputs(game, ctx)
        })?;
        receipt.completion = receipt.completion.map(|inner| {
            Box::new(PaymentCompletion {
                cause: self.cause,
                inner,
            }) as Box<dyn SimultaneousEffectCompletion>
        });
        Ok(receipt)
    }
    fn commit_original(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<SimultaneousEffectCommit, ExecutionError> {
        self.commit_original_with_outputs(game, ctx)
            .map(SimultaneousEffectCommit::into_aggregate)
    }
    fn commit(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let simultaneous = self.has_simultaneous_originals();
        crate::effects::composition::complete_prepared_original_with_grouping(
            self,
            game,
            ctx,
            simultaneous,
        )
    }
}

struct PaymentCompletion {
    cause: EventCause,
    inner: Box<dyn SimultaneousEffectCompletion>,
}
impl SimultaneousEffectCompletion for PaymentCompletion {
    fn observe_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: &mut EffectOutcome,
    ) -> Result<(), ExecutionError> {
        with_payment_cause(ctx, &self.cause, |ctx| {
            self.inner.observe_original(game, ctx, original)
        })
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
            .map(CompletedEffectOutputs::into_outcome)
    }
    fn complete_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        let Self { cause, inner } = *self;
        with_payment_cause(ctx, &cause, |ctx| {
            inner.complete_with_outputs(game, ctx, original)
        })
    }
}
