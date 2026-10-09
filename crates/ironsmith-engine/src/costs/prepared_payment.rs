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
        self.run_value(ctx, body)
    }

    /// Queries and actions share one binding lifetime. The fallible adapter
    /// retains the existing error-inference contract for action callers.
    pub(crate) fn run_value<T>(
        &self,
        ctx: &mut ExecutionContext,
        body: impl FnOnce(&mut ExecutionContext) -> T,
    ) -> T {
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
    acknowledge_program: bool,
    scope: PaymentScope,
    components: Vec<PreparedCostComponent>,
}

struct PaymentCompletion {
    scope: PaymentScope,
    inner: Box<dyn SimultaneousEffectCompletion>,
}
impl PaymentCompletion {
    fn advance(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        phase: crate::effects::composition::CompletionPhase,
    ) -> Result<SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>, ExecutionError>
    {
        let Self { scope, inner } = *self;
        let mut receipt = scope.run(ctx, |ctx| phase.dispatch(inner, game, ctx))?;
        receipt.completion = receipt.completion.map(|inner| {
            Box::new(PaymentCompletion { scope, inner }) as Box<dyn SimultaneousEffectCompletion>
        });
        Ok(receipt)
    }

    fn finish(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: crate::effects::composition::CompletionInput,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        let Self { scope, inner } = *self;
        scope.run(ctx, |ctx| original.dispatch(inner, game, ctx))
    }
}
impl SimultaneousEffectCompletion for PaymentCompletion {
    fn original_phase_status(&self) -> crate::effects::OriginalPhaseStatus {
        self.inner.original_phase_status()
    }

    fn complete_original_phase_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>, ExecutionError>
    {
        self.advance(
            game,
            ctx,
            crate::effects::composition::CompletionPhase::OriginalOutcome(original),
        )
    }

    fn complete_original_phase_from_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: crate::effects::CompletedEffectOutputs,
    ) -> Result<SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>, ExecutionError>
    {
        self.advance(
            game,
            ctx,
            crate::effects::composition::CompletionPhase::OriginalOutputs(original),
        )
    }

    fn prepare_draw_boundary_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>, ExecutionError>
    {
        self.advance(
            game,
            ctx,
            crate::effects::composition::CompletionPhase::DrawOutcome(original),
        )
    }

    fn prepare_draw_boundary_from_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: crate::effects::CompletedEffectOutputs,
    ) -> Result<SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>, ExecutionError>
    {
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
        self.finish(
            game,
            ctx,
            crate::effects::composition::CompletionInput::Outcome(original),
        )
    }

    fn complete_from_original_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: crate::effects::CompletedEffectOutputs,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        self.finish(
            game,
            ctx,
            crate::effects::composition::CompletionInput::Outputs(original),
        )
    }
}
impl SimultaneousEffectProposal for PreparedPayment {
    fn prepare_selection(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        self.scope.run(ctx, |ctx| {
            for component in &mut self.components {
                component.proposal.prepare_selection(game, ctx)?;
                if ctx.decision_maker.awaiting_choice() {
                    break;
                }
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
        let Self {
            scope,
            components,
            acknowledge_program,
        } = *self;
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
        if acknowledge_program && !ctx.decision_maker.awaiting_choice() {
            // This is the total-cost owner's acknowledgement, not an inferred
            // quantity or a physical action. Component receipts retain history.
            receipt = crate::effects::composition::compose_original_commits_with_projection_outputs(
                vec![receipt],
                Box::new(|outcomes| {
                    EffectOutcome::aggregate_with_primary_result(
                        EffectOutcome::resolved(),
                        outcomes,
                    )
                }),
            );
        }
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
    prepare_selected_total_cost(cost, game, ctx, payer, reason, false)
}

/// The program forwards a selected cost to its existing native owner. Its
/// successful acknowledgement is independent of replaced physical results.
pub(crate) fn prepare_total_cost_program_action(
    cost: &crate::cost::TotalCost,
    game: &GameState,
    ctx: &mut ExecutionContext,
    payer: PlayerId,
    reason: PaymentReason,
) -> Result<Option<Box<dyn SimultaneousEffectProposal>>, ExecutionError> {
    prepare_selected_total_cost(cost, game, ctx, payer, reason, true)
}

fn prepare_selected_total_cost(
    cost: &crate::cost::TotalCost,
    game: &GameState,
    ctx: &mut ExecutionContext,
    payer: PlayerId,
    reason: PaymentReason,
    acknowledge_program: bool,
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
        Box::new(PreparedPayment {
            scope,
            components,
            acknowledge_program,
        }) as Box<dyn SimultaneousEffectProposal>
    }))
}

/// Execution errors cannot establish that a cost was unaffordable. Both offer
/// queries and actual payment requests retain the cost owner's typed result.
pub(crate) fn acknowledged_total_cost<T>(
    result: Result<T, crate::cost::CostPaymentError>,
) -> Result<Option<T>, ExecutionError> {
    match result {
        Ok(receipt) => Ok(Some(receipt)),
        Err(crate::cost::CostPaymentError::ExecutionFailed(error)) => Err(error),
        Err(_) => Ok(None),
    }
}

/// Ordinary requests use the sequential payment owner, rather than preparing
/// every component early. It retains funding, choices, X, rollback and the
/// publication boundary of each component before the next one is selected.
pub(crate) fn execute_total_cost_program_action(
    cost: &crate::cost::TotalCost,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    payer: PlayerId,
    reason: PaymentReason,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    let paid = acknowledged_total_cost(
        crate::special_actions::pay_total_cost_with_choice_in_context_with_outputs(
            game, payer, ctx.source, cost, reason, ctx,
        ),
    )?;
    if ctx.decision_maker.awaiting_choice() {
        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
            EffectOutcome::count(0),
        ));
    }
    let Some(children) = paid else {
        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
            EffectOutcome::declined(),
        ));
    };
    let mut outputs =
        crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::resolved());
    // The actual cost gateway already queued these events. This packet keeps
    // its acknowledgement and child views without publishing them again.
    outputs.retain_published_children(children);
    Ok(outputs)
}

pub(crate) fn total_cost_supports_prepared_program(cost: &crate::cost::TotalCost) -> bool {
    match cost.kind() {
        ironsmith_core::TotalCostKind::All(costs) => {
            costs.iter().all(|cost| cost.0.supports_prepared_payment())
        }
        ironsmith_core::TotalCostKind::OneOf(branches) => {
            branches.iter().all(total_cost_supports_prepared_program)
        }
    }
}

/// Select a real payable alternative before native preparation. Selection
/// performs no payment; acknowledgement remains with original commitment.
pub(crate) fn select_payable_total_cost(
    game: &GameState,
    payer: PlayerId,
    source: crate::ids::ObjectId,
    cost: &crate::cost::TotalCost,
    reason: PaymentReason,
    ctx: &mut ExecutionContext,
) -> Result<Option<crate::cost::TotalCost>, ExecutionError> {
    Ok(match cost.kind() {
        ironsmith_core::TotalCostKind::All(_) => Some(cost.clone()),
        ironsmith_core::TotalCostKind::OneOf(branches) => {
            let mut payable = Vec::new();
            for branch in branches {
                if acknowledged_total_cost(
                    crate::special_actions::can_pay_total_cost_with_reason_in_context(
                        game, payer, source, branch, reason, ctx,
                    ),
                )?
                .is_some()
                {
                    payable.push(branch.clone());
                }
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(None);
                }
            }
            let selected = match payable.as_slice() {
                [] => None,
                [only] => Some(only.clone()),
                _ => {
                    let options = payable
                        .into_iter()
                        .map(|branch| (branch.display(), branch))
                        .collect::<Vec<_>>();
                    crate::decisions::ask_choose_one(
                        game,
                        &mut ctx.decision_maker,
                        payer,
                        source,
                        &options,
                    )
                }
            };
            // OneOf may contain another OneOf. Resolve only the selected path;
            // the native total owner receives the actual All component list.
            match selected {
                Some(selected) if !ctx.decision_maker.awaiting_choice() => {
                    select_payable_total_cost(game, payer, source, &selected, reason, ctx)?
                }
                _ => None,
            }
        }
    })
}
