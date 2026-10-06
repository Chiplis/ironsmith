//! Prepared cost components compose their action owners and retain receipts.

use super::PaymentReason;
use crate::effect::EffectOutcome;
use crate::effects::{
    ExecutionContext, ExecutionError, SimultaneousEffectCommit, SimultaneousEffectCompletion,
    SimultaneousEffectProposal,
};
use crate::game_state::GameState;
use crate::ids::PlayerId;

#[derive(Debug, Clone)]
pub(crate) struct PaymentScope {
    payer: PlayerId,
    reason: PaymentReason,
    cause: crate::events::cause::EventCause,
}

impl PaymentScope {
    pub(crate) fn new(ctx: &ExecutionContext, payer: PlayerId, reason: PaymentReason) -> Self {
        Self {
            payer,
            reason,
            cause: super::payer_trait::payment_event_cause(
                ctx.source,
                payer,
                reason,
                Some(&ctx.cause),
            ),
        }
    }

    pub(crate) fn run<T>(
        &self,
        ctx: &mut ExecutionContext,
        body: impl FnOnce(&mut ExecutionContext) -> Result<T, ExecutionError>,
    ) -> Result<T, ExecutionError> {
        let controller = ctx.controller;
        let cause = std::mem::replace(&mut ctx.cause, self.cause.clone());
        let reason = ctx.mana.payment_reason.replace(self.reason);
        ctx.controller = self.payer;
        let result = body(ctx);
        ctx.controller = controller;
        ctx.cause = cause;
        ctx.mana.payment_reason = reason;
        result
    }
}

/// Retain the actual cost owner alongside its action proposal. A total owns
/// composition and scope; component acceptance remains with the cost owner.
#[derive(Debug)]
struct PreparedCostComponent {
    cost: super::Cost,
    proposal: Box<dyn SimultaneousEffectProposal>,
}

fn original_payment_error(error: crate::cost::CostPaymentError) -> ExecutionError {
    match error {
        crate::cost::CostPaymentError::ExecutionFailed(error) => error,
        other => {
            ExecutionError::Impossible(format!("prepared payment was not acknowledged: {other}"))
        }
    }
}

#[derive(Debug)]
struct PreparedPayment {
    scope: PaymentScope,
    components: Vec<PreparedCostComponent>,
}

struct PaymentCompletion {
    scope: PaymentScope,
    inner: Box<dyn SimultaneousEffectCompletion>,
}
impl SimultaneousEffectCompletion for PaymentCompletion {
    fn prepare_draw_boundary_with_outputs(
        self: Box<Self>, game: &mut GameState, ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>, ExecutionError> {
        let Self { scope, inner } = *self;
        let mut receipt = scope.run(ctx, |ctx| inner.prepare_draw_boundary_with_outputs(game, ctx, original))?;
        receipt.completion = receipt.completion.map(|inner|
            Box::new(PaymentCompletion { scope, inner }) as Box<dyn SimultaneousEffectCompletion>);
        Ok(receipt)
    }

    fn observe_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut crate::effects::ExecutionContext,
        original: &mut EffectOutcome,
    ) -> Result<(), crate::effects::ExecutionError> {
        self.scope
            .run(ctx, |ctx| self.inner.observe_original(game, ctx, original))
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
        let Self { scope, inner } = *self;
        scope.run(ctx, |ctx| inner.complete_with_outputs(game, ctx, original))
    }
}
impl SimultaneousEffectProposal for PreparedPayment {
    fn prepare_selection(
        &mut self, game: &mut GameState, ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        self.scope.run(ctx, |ctx| {
            for component in &mut self.components {
                component.proposal.prepare_selection(game, ctx)?;
                if ctx.decision_maker.awaiting_choice() { break; }
            }
            Ok(())
        })
    }

    fn has_simultaneous_originals(&self) -> bool {
        self.components.len() > 1
            || self
                .components
                .iter()
                .any(|component| component.proposal.has_simultaneous_originals())
    }

    fn nominal_payment_quantity(&self) -> Option<u64> {
        // A singleton preserves its owner's units; totals of unrelated costs
        // have no one nominal quantity to export.
        let [component] = self.components.as_slice() else {
            return None;
        };
        component.proposal.nominal_payment_quantity()
    }

    fn declared_payment_resources(&self) -> Vec<crate::effects::PaymentResourceClaim> {
        self.components
            .iter()
            .flat_map(|component| component.proposal.declared_payment_resources())
            .collect()
    }

    fn declared_life_payments(&self) -> Vec<(PlayerId, u32)> {
        self.components
            .iter()
            .flat_map(|component| component.proposal.declared_life_payments())
            .collect()
    }
    fn prepare_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        self.scope.run(ctx, |ctx| {
            for component in &mut self.components {
                component.proposal.prepare_original(game, ctx)?;
                if ctx.decision_maker.awaiting_choice() {
                    break;
                }
            }
            Ok(())
        })
    }

    fn seal_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        self.scope.run(ctx, |ctx| {
            for component in &mut self.components {
                component.proposal.seal_original(game, ctx)?;
                if ctx.decision_maker.awaiting_choice() {
                    break;
                }
            }
            Ok(())
        })
    }
    fn commit_original(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<SimultaneousEffectCommit, ExecutionError> {
        self.commit_original_with_outputs(game, ctx)
            .map(SimultaneousEffectCommit::into_aggregate)
    }

    fn commit_original_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>, ExecutionError>
    {
        let Self { scope, components } = *self;
        let mut receipt = scope.run(ctx, |ctx| {
            let mut receipts = Vec::new();
            for PreparedCostComponent { cost, proposal } in components {
                let receipt = proposal.commit_original_with_outputs(game, ctx)?;
                if !ctx.decision_maker.awaiting_choice() {
                    cost.0
                        .validate_payment_outcome(&receipt.outcome.outcome)
                        .map_err(original_payment_error)?;
                    if ctx.x_value.is_none() {
                        ctx.x_value = cost
                            .0
                            .payment_x_from_outcome(&receipt.outcome.outcome, ctx)
                            .map_err(original_payment_error)?;
                    }
                    let payment_x = ctx.x_value;
                    cost.0
                        .finalize_payment_bindings(game, &receipt.outcome.outcome, ctx, payment_x)
                        .map_err(original_payment_error)?;
                }
                receipts.push(receipt);
                if ctx.decision_maker.awaiting_choice() {
                    break;
                }
            }
            Ok(crate::effects::composition::compose_original_commits_with_outputs(receipts))
        })?;
        receipt.completion = receipt.completion.map(|inner| {
            Box::new(PaymentCompletion { scope, inner }) as Box<dyn SimultaneousEffectCompletion>
        });
        Ok(receipt)
    }
    fn commit(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        crate::effects::composition::complete_prepared_original(self, game, ctx)
    }
}

/// Compose a selected total from the preparation contracts of its actual cost
/// payers. Check capability for the whole program before any component chooses
/// inputs, so an unsupported later component cannot leak an earlier choice.
pub(crate) fn prepare_total_cost(
    cost: &crate::cost::TotalCost,
    game: &GameState,
    ctx: &mut ExecutionContext,
    payer: PlayerId,
    reason: PaymentReason,
) -> Result<Option<Box<dyn SimultaneousEffectProposal>>, ExecutionError> {
    let ironsmith_core::TotalCostKind::All(costs) = cost.kind() else {
        return Ok(None);
    };
    if !costs.iter().all(|cost| cost.0.supports_prepared_payment()) {
        return Ok(None);
    }
    let scope = PaymentScope::new(ctx, payer, reason);
    let original_x = ctx.x_value;
    let components = scope.run(ctx, |ctx| {
        let mut components = Vec::with_capacity(costs.len());
        for cost in costs {
            let Some(proposal) = cost.0.prepare_simultaneous_payment(game, ctx)? else {
                return Ok(None);
            };
            if ctx.decision_maker.awaiting_choice() {
                return Ok(None);
            }
            if ctx.x_value.is_none() {
                ctx.x_value = cost
                    .0
                    .payment_x_from_prepared_payment(proposal.as_ref(), ctx)
                    .map_err(original_payment_error)?;
            }
            components.push(PreparedCostComponent {
                cost: cost.clone(),
                proposal,
            });
        }
        Ok(Some(components))
    });
    // Nominal inputs may depend on earlier payments, but preparation publishes
    // no live X. Original commitment exports the acknowledged value as before.
    ctx.x_value = original_x;
    let components = components?;
    Ok(components.map(|components| {
        Box::new(PreparedPayment { scope, components }) as Box<dyn SimultaneousEffectProposal>
    }))
}
