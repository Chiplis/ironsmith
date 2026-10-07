//! One composable counter placement request for objects and players.

use crate::effect::EffectOutcome;
use crate::effects::{CompletedEffectOutputs, EffectExecutor, ExecutionContext, ExecutionError};
use crate::events::{Event, PutCountersEvent};
use crate::game_state::{GameState, Target};

#[derive(Debug, Clone)]
struct CounterPlacement(Event, Option<u32>);

impl EffectExecutor for CounterPlacement {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        let recipient = crate::events::downcast_event::<PutCountersEvent>(self.0.inner())
            .ok_or_else(|| {
                ExecutionError::InternalError("counter request has no placement event".into())
            })?
            .target;
        match recipient {
            Target::Object(_) => {
                super::object_counter_placement::execute_object_counter_placement_with_limit_outputs(
                    game,
                    ctx,
                    self.0.clone(),
                    self.1,
                )
            }
            Target::Player(_) => {
                super::player_counter_placement::execute_player_counter_placement_with_outputs(
                    game,
                    ctx,
                    self.0.clone(),
                )
            }
        }
    }
}

pub(crate) fn execute_counter_placement_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    event: Event,
) -> Result<CompletedEffectOutputs, ExecutionError> {
    execute_counter_placement_with_limit_outputs(game, ctx, event, None)
}

pub(super) fn execute_counter_placement_with_limit_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    event: Event,
    maximum_total: Option<u32>,
) -> Result<CompletedEffectOutputs, ExecutionError> {
    CounterPlacement(event, maximum_total).execute_child_with_outputs(game, ctx)
}

/// Share one grouping identity without moving counter replacements to a new
/// timing boundary. Each request retains its own original and additions.
pub(crate) fn group_counter_placement_events(
    game: &mut GameState,
    ctx: &ExecutionContext,
    events: &mut [crate::triggers::TriggerEvent],
    batch: &mut Option<crate::provenance::ProvNodeId>,
) {
    for event in events {
        if event.kind() == crate::events::EventKind::MarkersChanged {
            let batch = *batch.get_or_insert_with(|| {
                game.alloc_child_event_provenance(
                    ctx.provenance,
                    crate::events::EventKind::MarkersChanged,
                )
            });
            *event = event.clone().with_simultaneous_batch(batch);
        }
    }
}

/// Prepare every placement in one pre-mutation world, commit all originals,
/// freeze them, then execute additions. The returned order matches requests.
pub(crate) fn execute_counter_batch_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    events: Vec<Event>,
) -> Result<Vec<CompletedEffectOutputs>, ExecutionError> {
    execute_counter_batch_with_limit_outputs(game, ctx, events, None)
}

pub(super) fn execute_counter_batch_with_limit_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    events: Vec<Event>,
    maximum_total: Option<u32>,
) -> Result<Vec<CompletedEffectOutputs>, ExecutionError> {
    crate::effects::composition::execute_transaction(game, ctx, Vec::new, |game, ctx| {
        let mut prepared = Vec::with_capacity(events.len());
        for event in events {
            prepared.push(super::prepared_placement::prepare_counter_placement_with_limit(
                game, ctx, event, maximum_total,
            )?);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(Vec::new());
            }
        }
        crate::effects::composition::execute_simultaneous_originals_with_default_outputs(
            game,
            ctx,
            true,
            |game, ctx| {
                let mut originals = Vec::with_capacity(prepared.len());
                let mut batch = None;
                for request in prepared {
                    let mut original =
                        super::commit_prepared_counter_original_with_outputs(game, ctx, request)?;
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(Vec::new());
                    }
                    group_counter_placement_events(
                        game,
                        ctx,
                        &mut original.outcome.outcome.events,
                        &mut batch,
                    );
                    originals.push(original);
                }
                Ok(originals)
            },
        )
    })
}

#[derive(Debug, Clone)]
struct CounterRemoval(Event);

impl EffectExecutor for CounterRemoval {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(CompletedEffectOutputs::into_outcome)
    }
    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        super::remove_counters::execute_counter_removal_event_with_outputs(
            game,
            ctx,
            self.0.clone(),
        )
    }
}

pub(crate) fn execute_counter_removal_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    event: Event,
) -> Result<CompletedEffectOutputs, ExecutionError> {
    CounterRemoval(event).execute_child_with_outputs(game, ctx)
}

pub(crate) fn execute_player_counter_removal_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    player: crate::ids::PlayerId,
    counter_type: crate::object::CounterType,
    count: u32,
    source: Option<crate::ids::ObjectId>,
    actor: Option<crate::ids::PlayerId>,
) -> Result<CompletedEffectOutputs, ExecutionError> {
    let event = Event::new_with_provenance(
        crate::events::RemovePlayerCountersEvent::new(player, counter_type, count, source, actor),
        ctx.provenance,
    );
    execute_counter_removal_with_outputs(game, ctx, event)
}

/// Acknowledged payment is distinct from the action used to pay it (CR 118.11).
/// Validate the full authored quantity before replacements, then compose the
/// ordinary removal owner and retain its complete actual-action observations.
/// Validate one authored counter payment before replacements or mutations.
/// Repeated requests within that payment share a nominal resource budget.
fn counter_removal_cost_inputs(
    game: &GameState,
    events: &[Event],
) -> Result<Option<(i64, Vec<crate::effects::PaymentResourceClaim>)>, ExecutionError> {
    let (resources, total) = super::prepared_payment::counter_payment_resources(events)?;
    if !crate::effects::can_pay_declared_resources(game, &resources) {
        return Ok(None);
    }
    let total = i64::try_from(total).map_err(|_| {
        ExecutionError::Impossible("counter cost exceeds the outcome count range".into())
    })?;
    Ok(Some((total, resources)))
}

#[derive(Debug)]
pub(crate) enum PreparedCounterCost {
    Finished(EffectOutcome),
    Originals {
        total: i64,
        physical_count: bool,
        simultaneous: bool,
        resources: Vec<crate::effects::PaymentResourceClaim>,
        children: Vec<Box<dyn crate::effects::SimultaneousEffectProposal>>,
    },
}

fn counter_cost_outcome(
    total: i64,
    physical_count: bool,
    outcomes: Vec<EffectOutcome>,
) -> Result<EffectOutcome, ExecutionError> {
    // Acceptance owns the nominal quantity, while "removed this way" owns
    // the original physical actions. Replacement additions cannot contribute
    // their unrelated counts to either quantity.
    let mut removed = 0u64;
    for outcome in &outcomes {
        let count = u32::try_from(outcome.instruction_result().count_or_zero()).map_err(|_| {
            ExecutionError::InternalError("counter payment has an invalid removal count".into())
        })?;
        removed = removed.checked_add(u64::from(count)).ok_or_else(|| {
            ExecutionError::InternalError("counter payment removal total overflow".into())
        })?;
    }
    let removed = i64::try_from(removed).map_err(|_| {
        ExecutionError::InternalError("counter payment removal total exceeds outcome range".into())
    })?;
    let mut payment = EffectOutcome::aggregate(
        outcomes.iter().map(|outcome| outcome.instruction_result().clone()),
    );
    payment.status = crate::effect::OutcomeStatus::Succeeded;
    // Quantity-producing counter costs expose actual removals. Existing
    // direct payment owners (notably energy paid) retain their nominal result;
    // their complete physical child receipts remain owned by the composition.
    payment.value = crate::effect::OutcomeValue::Count(if physical_count { removed } else { total });
    // Child removals may be prevented or replaced while this cost is accepted.
    // Their terminal facts stay in the complete observations and owned child
    // packets, rather than becoming the payment instruction's acknowledgement.
    payment.execution_facts.retain(|fact| !matches!(fact,
        crate::effect::ExecutionFact::Declined | crate::effect::ExecutionFact::TargetInvalid
        | crate::effect::ExecutionFact::Prevented | crate::effect::ExecutionFact::Protected
        | crate::effect::ExecutionFact::Impossible | crate::effect::ExecutionFact::Replaced));
    let payment = payment.with_execution_fact(crate::effect::ExecutionFact::Accepted);
    Ok(payment.with_authoritative_observations(EffectOutcome::aggregate(outcomes))
        .with_requested_amount(total as u64))
}

impl crate::effects::SimultaneousEffectProposal for PreparedCounterCost {
    fn declared_payment_resources(&self) -> Vec<crate::effects::PaymentResourceClaim> {
        match self {
            Self::Finished(_) => Vec::new(),
            Self::Originals { resources, .. } => resources.clone(),
        }
    }

    fn commit_original_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError>
    {
        let (total, physical_count, children) = match *self {
            Self::Finished(outcome) => {
                return Ok(crate::effects::SimultaneousEffectCommit::finished(
                    CompletedEffectOutputs::aggregate_only(outcome),
                ));
            }
            Self::Originals {
                total, physical_count, children, ..
            } => (total, physical_count, children),
        };
        let mut originals = Vec::new();
        for child in children {
            originals.push(child.commit_original_with_outputs(game, ctx)?);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(crate::effects::SimultaneousEffectCommit::finished(
                    CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
                ));
            }
        }
        crate::effects::composition::compose_original_commits_with_fallible_projection_outputs(
            originals,
            Box::new(move |outcomes| counter_cost_outcome(total, physical_count, outcomes)),
        )
    }

    fn commit_original(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::SimultaneousEffectCommit, ExecutionError> {
        self.commit_original_with_outputs(game, ctx)
            .map(crate::effects::SimultaneousEffectCommit::into_aggregate)
    }

    fn commit(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let simultaneous = matches!(
            &*self,
            Self::Originals {
                simultaneous: true,
                ..
            }
        );
        crate::effects::composition::complete_prepared_original_with_grouping(
            self,
            game,
            ctx,
            simultaneous,
        )
    }
}

/// Prepare the counter payment owner without committing any original or added
/// program. The enclosing payment retains its cause/context and rollback scope.
/// Independent cost components still need dependency-aware affordability.
pub(crate) fn prepare_counter_removal_cost(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    events: Vec<Event>,
    physical_count: bool,
) -> Result<PreparedCounterCost, ExecutionError> {
    let Some((total, resources)) = counter_removal_cost_inputs(game, &events)? else {
        return Ok(PreparedCounterCost::Finished(EffectOutcome::impossible()));
    };
    game.clear_pending_decision_controllers();
    let simultaneous = events.len() > 1;
    let children =
        super::remove_counters::prepare_counter_removal_proposals(game, ctx, events, false)?;
    if ctx.decision_maker.awaiting_choice() {
        return Ok(PreparedCounterCost::Finished(EffectOutcome::count(0)));
    }
    Ok(PreparedCounterCost::Originals {
        total,
        physical_count,
        simultaneous,
        resources,
        children,
    })
}

#[derive(Debug, Clone)]
struct CounterRemovalCost(Vec<Event>);

impl EffectExecutor for CounterRemovalCost {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(CompletedEffectOutputs::into_outcome)
    }
    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        crate::effects::composition::execute_transaction(
            game,
            ctx,
            || CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                let plan = prepare_counter_removal_cost(game, ctx, self.0.clone(), false)?;
                let plan: Box<dyn crate::effects::SimultaneousEffectProposal> = match plan {
                    PreparedCounterCost::Finished(outcome) => {
                        return Ok(CompletedEffectOutputs::aggregate_only(outcome));
                    }
                    plan @ PreparedCounterCost::Originals { .. } => Box::new(plan),
                };
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                let mut outcomes = crate::effects::composition::execute_simultaneous_originals_with_default_outputs(
                    game, ctx, self.0.len() > 1,
                    |game, ctx| {
                        let original = plan.commit_original_with_outputs(game, ctx)?;
                        if ctx.decision_maker.awaiting_choice() {
                            return Ok(Vec::new());
                        }
                        Ok(vec![original])
                    },
                )?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                let mut outputs = outcomes.pop().ok_or_else(|| {
                    ExecutionError::InternalError("counter payment lost its completed owner".into())
                })?;
                outputs.projections_complete = false;
                Ok(outputs)
            },
        )
    }
}

pub(crate) fn execute_counter_removal_cost(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    event: Event,
) -> Result<EffectOutcome, ExecutionError> {
    execute_counter_removal_cost_with_outputs(game, ctx, event)
        .map(CompletedEffectOutputs::into_outcome)
}

/// Retain actual child receipts independently of the nominal payment result.
pub(crate) fn execute_counter_removal_cost_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    event: Event,
) -> Result<CompletedEffectOutputs, ExecutionError> {
    execute_counter_removal_cost_batch_with_outputs(game, ctx, vec![event])
}

/// Accepted payment over several counter groups is one authored action.
pub(crate) fn execute_counter_removal_cost_batch(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    events: Vec<Event>,
) -> Result<EffectOutcome, ExecutionError> {
    execute_counter_removal_cost_batch_with_outputs(game, ctx, events)
        .map(CompletedEffectOutputs::into_outcome)
}

fn execute_counter_removal_cost_batch_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    events: Vec<Event>,
) -> Result<CompletedEffectOutputs, ExecutionError> {
    let cause = crate::events::cause::EventCause::from_cost(ctx.source, ctx.controller);
    let previous = std::mem::replace(&mut ctx.cause, cause);
    let result = CounterRemovalCost(events).execute_child_with_outputs(game, ctx);
    ctx.cause = previous;
    result
}
