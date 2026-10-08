//! Remove counters effect implementation.

use crate::effect::{EffectOutcome, Value};
use crate::effects::helpers::{resolve_bounded_nonnegative_u32, resolve_single_object_for_effect};
use crate::effects::{CompletedEffectOutputs, CostExecutableEffect, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;
pub use ironsmith_core::RemoveCountersEffect;

/// Effect that removes counters from a target permanent.
///
/// # Fields
///
/// * `counter_type` - The type of counter to remove
/// * `count` - How many counters to remove
/// * `target` - Which permanent to target
///
/// # Example
///
/// ```ignore
/// // Remove two +1/+1 counters from target creature
/// let effect = RemoveCountersEffect::new(
///     CounterType::PlusOnePlusOne,
///     2,
///     ChooseSpec::creature(),
/// );
/// ```
impl EffectExecutor for RemoveCountersEffect {
    fn supports_replacement_draw_continuation(&self) -> bool {
        true
    }

    fn prepare_replacement_draw_continuation_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        ExecutionError,
    > {
        crate::effects::replacement::prepare_native_draw_continuation_with_outputs(self, game, ctx)
    }

    fn supports_simultaneous_player_action(&self) -> bool {
        true
    }

    fn prepare_simultaneous_player_action(
        &self,
        _game: &GameState,
        _ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        Ok(Box::new(CounterRemovalInstructionProposal {
            effect: self.clone(),
            prepared: None,
        }))
    }

    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

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
        if ctx.decision_maker.awaiting_choice() {
            return Ok(CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        game.clear_pending_decision_controllers();
        let result = crate::effects::composition::execute_transaction(
            game,
            ctx,
            || CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                let Some(event) = counter_removal_instruction_event(self, game, ctx)? else {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                };
                super::execute_counter_removal_with_outputs(game, ctx, event)
            },
        );
        // Preserve the authored adapter's existing neutral suspension policy.
        // The shared transaction owns restoration of the complete action.
        if result.is_ok() && ctx.decision_maker.awaiting_choice() {
            return Ok(CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        result
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "target to remove counters from"
    }

    fn cost_description(&self) -> Option<String> {
        if matches!(self.count.unhinted(), Value::CountersOn(spec, Some(kind))
            if *kind == self.counter_type && spec.base() == self.target.base())
        {
            return Some(format!(
                "Remove all {} counters from that permanent",
                self.counter_type.description()
            ));
        }
        if matches!(self.target.base(), ChooseSpec::Source)
            && let Some(count) = self.count.constant_integer()
        {
            let label = self.counter_type.description();
            return Some(if count == 1 {
                format!("Remove a {label} counter from this source")
            } else {
                format!("Remove {} {label} counters from this source", count)
            });
        }
        None
    }
}

/// Freeze the authored target and quantity before sibling originals mutate.
/// Both ordinary and simultaneous instructions use this same input resolver.
fn counter_removal_instruction_event(
    effect: &RemoveCountersEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Result<Option<crate::events::Event>, ExecutionError> {
    let target_id = resolve_single_object_for_effect(game, ctx, &effect.target)?;
    let requested = resolve_bounded_nonnegative_u32(
        game,
        &effect.count,
        ctx,
        game.counter_count(target_id, effect.counter_type),
    )?;
    if ctx.decision_maker.awaiting_choice() {
        return Ok(None);
    }
    Ok(Some(
        crate::events::Event::remove_counters(target_id, effect.counter_type, requested)
            .with_provenance(ctx.provenance),
    ))
}

/// A counter instruction owns preparation only. The existing removal owner
/// commits its original and retains additions for the enclosing coordinator.
struct CounterRemovalInstructionProposal {
    effect: RemoveCountersEffect,
    prepared: Option<PreparedCounterRemoval>,
}

impl std::fmt::Debug for CounterRemovalInstructionProposal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CounterRemovalInstructionProposal")
            .field("effect", &self.effect)
            .field("prepared", &self.prepared.is_some())
            .finish()
    }
}

impl crate::effects::SimultaneousEffectProposal for CounterRemovalInstructionProposal {
    fn prepare_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        if self.prepared.is_some() || ctx.decision_maker.awaiting_choice() {
            return Ok(());
        }
        let Some(event) = counter_removal_instruction_event(&self.effect, game, ctx)? else {
            return Ok(());
        };
        let prepared = prepare_counter_removal(game, ctx, event)?;
        if !ctx.decision_maker.awaiting_choice() {
            self.prepared = Some(prepared);
        }
        Ok(())
    }

    fn commit_original_with_outputs(
        mut self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError>
    {
        self.prepare_original(game, ctx)?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::SimultaneousEffectCommit::finished(
                CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            ));
        }
        let prepared = self.prepared.take().ok_or_else(|| {
            ExecutionError::InternalError("counter instruction lost its prepared removal".into())
        })?;
        commit_prepared_counter_removal_original_with_outputs(game, ctx, prepared)
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
        crate::effects::composition::complete_prepared_original_with_outputs(self, game, ctx, false)
            .map(CompletedEffectOutputs::into_outcome)
    }
}

pub(crate) struct PreparedCounterRemoval {
    requested: u32,
    result: Option<crate::events::processing::TraitEventResult>,
    skipped: EffectOutcome,
    replacement_source_snapshot: Option<crate::snapshot::ObjectSnapshot>,
}

impl std::fmt::Debug for PreparedCounterRemoval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreparedCounterRemoval")
            .field("requested", &self.requested)
            .field("replacement_prepared", &self.result.is_some())
            .finish()
    }
}

impl crate::effects::SimultaneousEffectProposal for PreparedCounterRemoval {
    fn commit_original_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError>
    {
        commit_prepared_counter_removal_original_with_outputs(game, ctx, *self)
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
        crate::effects::composition::complete_prepared_original_with_outputs(self, game, ctx, false)
            .map(CompletedEffectOutputs::into_outcome)
    }
}

pub(crate) fn prepare_counter_removal(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    event: crate::events::Event,
) -> Result<PreparedCounterRemoval, ExecutionError> {
    let event = match crate::events::CounterRemovalEvent::from_event(event.inner()) {
        Some(crate::events::CounterRemovalEvent::Object(removal)) => event.rewrap(
            removal
                .clone()
                .with_attribution(Some(ctx.source), Some(ctx.controller))
                .with_cause(ctx.cause.clone()),
        ),
        Some(crate::events::CounterRemovalEvent::Player(removal)) => {
            event.rewrap(removal.clone().with_cause(ctx.cause.clone()))
        }
        None => {
            return Err(ExecutionError::InternalError(
                "counter-removal preparation requires a removal event".into(),
            ));
        }
    };
    prepare_counter_removal_event(game, ctx, event)
}

fn prepare_counter_removal_event(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    event: crate::events::Event,
) -> Result<PreparedCounterRemoval, ExecutionError> {
    let requested = crate::events::CounterRemovalEvent::from_event(event.inner())
        .ok_or_else(|| {
            ExecutionError::InternalError(
                "counter-removal preparation requires a removal event".into(),
            )
        })?
        .count();
    let removal =
        crate::events::CounterRemovalEvent::from_event(event.inner()).ok_or_else(|| {
            ExecutionError::InternalError("counter-removal owner requires a removal event".into())
        })?;
    let count = match &removal {
        crate::events::CounterRemovalEvent::Object(removal) => {
            if game.object(removal.target).is_none() {
                return Ok(PreparedCounterRemoval {
                    requested,
                    result: None,
                    skipped: EffectOutcome::target_invalid(),
                    replacement_source_snapshot: None,
                });
            }
            if game.is_phased_out(removal.target) {
                return Ok(PreparedCounterRemoval {
                    requested,
                    result: None,
                    skipped: EffectOutcome::count(0),
                    replacement_source_snapshot: None,
                });
            }
            removal
                .count
                .min(game.counter_count(removal.target, removal.counter_type))
        }
        crate::events::CounterRemovalEvent::Player(removal) => removal.count.min(
            game.player(removal.player)
                .map_or(0, |player| player.counter_count(removal.counter_type)),
        ),
    };
    if count == 0 {
        return Ok(PreparedCounterRemoval {
            requested,
            result: None,
            skipped: EffectOutcome::count(0),
            replacement_source_snapshot: None,
        });
    }
    // The instruction's provenance is a causal parent, not this proposal's identity.
    // Several counter groups can be removed by one instruction.
    let parent = event.provenance();
    let proposal = if game.provenance_graph().node(parent).is_some() {
        game.alloc_child_event_provenance(parent, crate::events::EventKind::RemoveCounters)
    } else {
        game.provenance_graph_mut()
            .alloc_root_event(crate::events::EventKind::RemoveCounters)
    };
    let event = removal.with_count(&event, count).with_provenance(proposal);
    let processed =
        crate::events::processing::process_trait_event_with_execution_context(game, event, ctx)?;

    let (original, programs) = processed.into_expansion();
    let replacement_source_snapshot =
        if let crate::events::processing::TraitEventResult::Replaced { source, .. } = &original {
            let observed = game
                .continuous_query_snapshot()
                .map_err(ExecutionError::ContinuousDiscovery)?;
            observed
                .object(*source)
                .map(|object| {
                    crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                        object, &observed,
                    )
                })
                .or_else(|| game.source_last_known_snapshot(*source).cloned())
        } else {
            None
        };
    let result = if programs.is_empty() {
        original
    } else {
        crate::events::processing::TraitEventResult::Expanded {
            original: Box::new(original),
            programs,
        }
    };
    Ok(PreparedCounterRemoval {
        requested,
        result: Some(result),
        skipped: EffectOutcome::count(0),
        replacement_source_snapshot,
    })
}

/// State-based removals are not controlled by a player (CR 704.2).
pub(crate) fn prepare_game_rule_counter_removal(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    event: crate::events::Event,
) -> Result<PreparedCounterRemoval, ExecutionError> {
    let event = match crate::events::CounterRemovalEvent::from_event(event.inner()) {
        Some(crate::events::CounterRemovalEvent::Object(removal)) => event.rewrap(
            removal
                .clone()
                .with_attribution(None, None)
                .with_cause(ctx.cause.clone()),
        ),
        Some(crate::events::CounterRemovalEvent::Player(removal)) => event.rewrap(
            removal
                .clone()
                .with_attribution(None, None)
                .with_cause(ctx.cause.clone()),
        ),
        None => {
            return Err(ExecutionError::InternalError(
                "game-rule counter removal requires a removal event".into(),
            ));
        }
    };
    prepare_counter_removal_event(game, ctx, event)
}

fn removal_bindings(
    context: &crate::events::processing::ReplacementEventContext,
) -> Result<Option<Vec<crate::effects::ResolvedTarget>>, ExecutionError> {
    let removal = crate::events::CounterRemovalEvent::from_event(context.event.inner())
        .ok_or_else(|| {
            ExecutionError::InternalError("counter-removal continuation lost its event".into())
        })?;
    Ok(Some(vec![removal_target(removal.target())]))
}

pub(crate) fn commit_prepared_counter_removal_original_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    prepared: PreparedCounterRemoval,
) -> Result<crate::effects::SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
    let Some(result) = prepared.result else {
        return Ok(crate::effects::SimultaneousEffectCommit::finished(
            CompletedEffectOutputs::aggregate_only(
                prepared.skipped.with_requested_amount(prepared.requested),
            ),
        ));
    };
    let (original, programs) = result.into_expansion();
    let deferred = if let crate::events::processing::TraitEventResult::Replaced {
        effects,
        source,
        controller,
        context,
        ..
    } = &original
    {
        crate::effects::replacement::prepare_draw_continuation_with_bindings_and_outputs(
            game,
            ctx,
            effects,
            *source,
            *controller,
            context,
            prepared.replacement_source_snapshot.clone(),
            crate::effects::replacement::ReplacementProgramBindings {
                targets: removal_bindings(context)?,
                object_tags: Vec::new(),
            },
        )?
    } else {
        None
    };
    let (outcome, continuation) = if let Some(receipt) = deferred {
        (receipt.outcome, receipt.completion)
    } else {
        (
            commit_counter_removal_with_outputs(
                game,
                ctx,
                original,
                prepared.replacement_source_snapshot,
            )?,
            None,
        )
    };
    Ok(
        crate::effects::replacement::defer_replacement_programs_with_outputs(
            crate::effects::SimultaneousEffectCommit {
                outcome: {
                    let aggregate = outcome
                        .outcome
                        .clone()
                        .with_requested_amount(prepared.requested);
                    outcome.project_aggregate(aggregate)
                },
                completion: continuation,
            },
            programs,
            |context| {
                Ok(crate::effects::replacement::ReplacementProgramBindings {
                    targets: removal_bindings(context)?,
                    object_tags: Vec::new(),
                })
            },
        ),
    )
}

pub(crate) fn execute_counter_removal_event_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    event: crate::events::Event,
) -> Result<CompletedEffectOutputs, ExecutionError> {
    let outcomes = execute_counter_removal_originals_with_outputs(game, ctx, vec![event], false)?;
    if ctx.decision_maker.awaiting_choice() {
        return Ok(CompletedEffectOutputs::aggregate_only(
            EffectOutcome::count(0),
        ));
    }
    outcomes.into_iter().next().ok_or_else(|| {
        ExecutionError::InternalError("counter-removal owner lost its receipt".into())
    })
}

/// Selection has no original mutations. Recorded envelopes preserve composed
/// instruction evidence while the selected groups share the parent's phases.
pub(super) enum SelectedCounterRemovalPlan {
    Single(crate::events::Event),
    Finished(CompletedEffectOutputs),
    Groups {
        events: Vec<crate::events::Event>,
        requested: u64,
    },
    Recorded(Box<SelectedCounterRemovalPlan>),
}

#[derive(Debug, Clone)]
pub(super) enum CounterRemovalSelector {
    Among(ironsmith_core::RemoveAnyCountersAmongEffect),
    UpTo(ironsmith_core::RemoveUpToAnyCountersEffect),
    UpToKind(super::remove_up_to_counters::RemoveUpToCountersEffect),
}

impl CounterRemovalSelector {
    fn select(
        self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<SelectedCounterRemovalPlan, ExecutionError> {
        match self {
            Self::Among(effect) => {
                super::remove_any_counters_among::select_distributed_counter_removal(
                    &effect, game, ctx,
                )
            }
            Self::UpTo(effect) => {
                super::remove_up_to_any_counters::select_up_to_any_counter_removal(
                    &effect, game, ctx,
                )
            }
            Self::UpToKind(effect) => {
                super::remove_up_to_counters::select_up_to_counter_removal(&effect, game, ctx)
            }
        }
    }
}

#[derive(Debug)]
struct SelectedCounterRemovalProposal {
    selector: CounterRemovalSelector,
    prepared: Option<(Box<dyn crate::effects::SimultaneousEffectProposal>, bool)>,
}

pub(super) fn selected_counter_removal_proposal(
    selector: CounterRemovalSelector,
) -> Box<dyn crate::effects::SimultaneousEffectProposal> {
    Box::new(SelectedCounterRemovalProposal {
        selector,
        prepared: None,
    })
}

impl crate::effects::SimultaneousEffectProposal for SelectedCounterRemovalProposal {
    fn prepare_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        if self.prepared.is_some() || ctx.decision_maker.awaiting_choice() {
            return Ok(());
        }
        let plan = self.selector.clone().select(game, ctx)?;
        if !ctx.decision_maker.awaiting_choice() {
            let prepared = prepare_selected_counter_removal_plan(game, ctx, plan)?;
            if !ctx.decision_maker.awaiting_choice() {
                self.prepared = Some(prepared);
            }
        }
        Ok(())
    }

    fn commit_original_with_outputs(
        mut self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError>
    {
        self.prepare_original(game, ctx)?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::SimultaneousEffectCommit::finished(
                CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            ));
        }
        let (proposal, _) = self.prepared.take().ok_or_else(|| {
            ExecutionError::InternalError(
                "selected counter instruction lost its prepared groups".into(),
            )
        })?;
        proposal.commit_original_with_outputs(game, ctx)
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
        mut self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        crate::effects::composition::execute_transaction(
            game,
            ctx,
            || EffectOutcome::count(0),
            |game, ctx| {
                self.prepare_original(game, ctx)?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(EffectOutcome::count(0));
                }
                let (proposal, simultaneous) = self.prepared.take().ok_or_else(|| {
                    ExecutionError::InternalError(
                        "selected counter instruction lost its prepared groups".into(),
                    )
                })?;
                crate::effects::composition::complete_prepared_original_with_outputs(
                    proposal,
                    game,
                    ctx,
                    simultaneous,
                )
                .map(CompletedEffectOutputs::into_outcome)
            },
        )
    }
}

enum PreparedSelectedCounterRemoval {
    Finished(CompletedEffectOutputs),
    Groups {
        children: Vec<Box<dyn crate::effects::SimultaneousEffectProposal>>,
        requested: u64,
    },
}

impl std::fmt::Debug for PreparedSelectedCounterRemoval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Finished(_) => f.write_str("FinishedCounterRemoval"),
            Self::Groups {
                children,
                requested,
            } => f
                .debug_struct("PreparedCounterRemovalGroups")
                .field("groups", &children.len())
                .field("requested", requested)
                .finish(),
        }
    }
}

impl crate::effects::SimultaneousEffectProposal for PreparedSelectedCounterRemoval {
    fn commit_original_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError>
    {
        match *self {
            Self::Finished(outputs) => {
                Ok(crate::effects::SimultaneousEffectCommit::finished(outputs))
            }
            Self::Groups {
                children,
                requested,
            } => {
                let mut receipts = Vec::with_capacity(children.len());
                for child in children {
                    receipts.push(child.commit_original_with_outputs(game, ctx)?);
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(crate::effects::SimultaneousEffectCommit::finished(
                            CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
                        ));
                    }
                }
                crate::effects::composition::compose_original_commits_with_fallible_projection_outputs(
                    receipts, Box::new(move |outcomes| project_selected_counter_removal(outcomes, requested)))
            }
        }
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
        crate::effects::composition::complete_prepared_original_with_outputs(self, game, ctx, false)
            .map(CompletedEffectOutputs::into_outcome)
    }
}

fn prepare_selected_counter_removal_plan(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    plan: SelectedCounterRemovalPlan,
) -> Result<(Box<dyn crate::effects::SimultaneousEffectProposal>, bool), ExecutionError> {
    match plan {
        SelectedCounterRemovalPlan::Single(event) => {
            let children = prepare_counter_removal_proposals(game, ctx, vec![event], true)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok((
                    Box::new(PreparedSelectedCounterRemoval::Finished(
                        CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
                    )),
                    false,
                ));
            }
            let proposal = children.into_iter().next().ok_or_else(|| {
                ExecutionError::InternalError(
                    "selected counter instruction lost its single proposal".into(),
                )
            })?;
            Ok((proposal, false))
        }

        SelectedCounterRemovalPlan::Finished(outputs) => Ok((
            Box::new(PreparedSelectedCounterRemoval::Finished(outputs)),
            false,
        )),
        SelectedCounterRemovalPlan::Groups { events, requested } => {
            let simultaneous = events.len() > 1;
            let children = prepare_counter_removal_proposals(game, ctx, events, true)?;
            Ok((
                Box::new(PreparedSelectedCounterRemoval::Groups {
                    children,
                    requested,
                }),
                simultaneous,
            ))
        }
        SelectedCounterRemovalPlan::Recorded(plan) => {
            let (proposal, simultaneous) = prepare_selected_counter_removal_plan(game, ctx, *plan)?;
            Ok((
                crate::effects::outcome_recording::record_proposal(proposal, None),
                simultaneous,
            ))
        }
    }
}

fn project_selected_counter_removal(
    outcomes: Vec<EffectOutcome>,
    requested: u64,
) -> Result<EffectOutcome, ExecutionError> {
    let mut removed_total = 0u64;
    for outcome in &outcomes {
        let removed = u32::try_from(outcome.count_or_zero())
            .map_err(|_| ExecutionError::InternalError("invalid counter-removal count".into()))?;
        removed_total = removed_total
            .checked_add(u64::from(removed))
            .ok_or_else(|| {
                ExecutionError::InternalError("counter-removal total overflow".into())
            })?;
    }
    let count = i64::try_from(removed_total).map_err(|_| {
        ExecutionError::InternalError("counter-removal total exceeds outcome range".into())
    })?;
    let mut outcome = EffectOutcome::aggregate(outcomes);
    outcome.set_value(crate::effect::OutcomeValue::Count(count));
    Ok(outcome.with_requested_amount(requested))
}

pub(super) fn complete_selected_counter_removal_plan(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    plan: SelectedCounterRemovalPlan,
) -> Result<CompletedEffectOutputs, ExecutionError> {
    if let SelectedCounterRemovalPlan::Finished(outputs) = plan {
        return Ok(outputs);
    }
    crate::effects::composition::execute_transaction(
        game,
        ctx,
        || CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
        |game, ctx| {
            let (proposal, simultaneous) = prepare_selected_counter_removal_plan(game, ctx, plan)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::count(0),
                ));
            }
            crate::effects::composition::complete_prepared_original_with_outputs(
                proposal,
                game,
                ctx,
                simultaneous,
            )
        },
    )
}

pub(super) fn prepare_counter_removal_proposals(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    events: Vec<crate::events::Event>,
    record_children: bool,
) -> Result<Vec<Box<dyn crate::effects::SimultaneousEffectProposal>>, ExecutionError> {
    let mut children = Vec::with_capacity(events.len());
    for event in events {
        let original = prepare_counter_removal(game, ctx, event)?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(Vec::new());
        }
        let proposal: Box<dyn crate::effects::SimultaneousEffectProposal> = Box::new(original);
        children.push(if record_children {
            crate::effects::outcome_recording::record_proposal(proposal, None)
        } else {
            proposal
        });
    }
    Ok(children)
}

/// Ordinary single removals already have a recorded child gateway. A compound
/// batch records each prepared child instead, without recording a single twice.
fn execute_counter_removal_originals_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    events: Vec<crate::events::Event>,
    record_children: bool,
) -> Result<Vec<CompletedEffectOutputs>, ExecutionError> {
    crate::effects::composition::execute_transaction(game, ctx, Vec::new, |game, ctx| {
        game.clear_pending_decision_controllers();
        let simultaneous = events.len() > 1;
        let prepared = prepare_counter_removal_proposals(game, ctx, events, record_children)?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(Vec::new());
        }
        crate::effects::composition::execute_simultaneous_originals_with_default_outputs(
            game,
            ctx,
            simultaneous,
            |game, ctx| {
                let mut receipts = Vec::with_capacity(prepared.len());
                for original in prepared {
                    receipts.push(original.commit_original_with_outputs(game, ctx)?);
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(Vec::new());
                    }
                }
                Ok(receipts)
            },
        )
    })
}

fn commit_counter_removal_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    processed: crate::events::processing::TraitEventResult,
    replacement_source_snapshot: Option<crate::snapshot::ObjectSnapshot>,
) -> Result<CompletedEffectOutputs, ExecutionError> {
    use crate::events::CounterRemovalEvent;
    use crate::events::processing::TraitEventResult;
    match processed {
        TraitEventResult::Expanded { .. } => Err(ExecutionError::InternalError(
            "counter removal received an unflattened original".into(),
        )),
        TraitEventResult::Proceed(event) | TraitEventResult::Modified(event) => {
            commit_resolved_counter_removal(game, ctx, event)
                .map(CompletedEffectOutputs::aggregate_only)
        }
        TraitEventResult::Replaced {
            effects,
            source,
            controller,
            context,
            ..
        } => {
            let removal =
                CounterRemovalEvent::from_event(context.event.inner()).ok_or_else(|| {
                    ExecutionError::InternalError(
                        "counter-removal replacement lost its event".into(),
                    )
                })?;
            let mut original = EffectOutcome::replaced();
            original.set_value(crate::effect::OutcomeValue::Count(0));
            crate::effects::replacement::execute_replacement_original_payload_with_outputs(
                game,
                ctx,
                &effects,
                source,
                controller,
                &context,
                crate::effects::replacement::ReplacementProgramBindings {
                    targets: Some(vec![removal_target(removal.target())]),
                    object_tags: Vec::new(),
                },
                replacement_source_snapshot,
                original,
            )
        }
        TraitEventResult::Prevented => {
            let mut outcome = EffectOutcome::prevented();
            outcome.set_value(crate::effect::OutcomeValue::Count(0));
            Ok(CompletedEffectOutputs::aggregate_only(outcome))
        }
        TraitEventResult::NeedsChoice { .. } | TraitEventResult::NeedsInteraction { .. } => {
            Err(ExecutionError::InternalError(
                "counter removal suspended without a captured decision".into(),
            ))
        }
    }
}

fn commit_resolved_counter_removal(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    event: crate::events::Event,
) -> Result<EffectOutcome, ExecutionError> {
    use crate::events::CounterRemovalEvent;
    let removal = CounterRemovalEvent::from_event(event.inner()).ok_or_else(|| {
        ExecutionError::InternalError(
            "counter-removal replacement returned an incompatible event".into(),
        )
    })?;
    let mut outcome = match removal {
        CounterRemovalEvent::Object(removal) => {
            if game.object(removal.target).is_none() {
                return Ok(EffectOutcome::target_invalid());
            }
            commit_object_counter_removal(game, ctx, removal)?
        }
        CounterRemovalEvent::Player(removal) => commit_player_counter_removal(game, ctx, removal)?,
    };
    for notification in &mut outcome.events {
        let observation = game.alloc_child_event_provenance(
            event.provenance(),
            crate::events::EventKind::MarkersChanged,
        );
        *notification = notification.clone().with_provenance(observation);
    }
    Ok(outcome)
}

fn removal_target(target: crate::game_state::Target) -> crate::effects::ResolvedTarget {
    match target {
        crate::game_state::Target::Object(object) => crate::effects::ResolvedTarget::Object(object),
        crate::game_state::Target::Player(player) => crate::effects::ResolvedTarget::Player(player),
    }
}

/// Player and object commits retain their target-specific notifications but
/// share preparation, amount replacements, redirects and deferred programs.
fn commit_player_counter_removal(
    game: &mut GameState,
    ctx: &ExecutionContext,
    removal: &crate::events::RemovePlayerCountersEvent,
) -> Result<EffectOutcome, ExecutionError> {
    let actual = removal.count.min(
        game.player(removal.player)
            .map_or(0, |player| player.counter_count(removal.counter_type)),
    );
    match game.remove_player_counters_with_source(
        removal.player,
        removal.counter_type,
        removal.count,
        removal.source,
        removal.actor,
    ) {
        Some((removed, mut notification)) => {
            if removed != actual {
                return Err(ExecutionError::InternalError(
                    "player counter-removal commit disagrees with its resolved amount".into(),
                ));
            }
            if removal.source == Some(ctx.source)
                && game.object(ctx.source).is_none()
                && let Some(snapshot) = &ctx.source_snapshot
            {
                notification = notification.with_source_snapshot(snapshot.clone());
            }
            Ok(EffectOutcome::count(i64::from(removed)).with_event(notification))
        }
        None => Ok(EffectOutcome::count(0)),
    }
}

/// Commit an already accepted removal. Replacement-aware proposals and
/// validated cost payments share mutation and observation capture here.
fn commit_object_counter_removal(
    game: &mut GameState,
    ctx: &ExecutionContext,
    removal: &crate::events::RemoveCountersEvent,
) -> Result<EffectOutcome, ExecutionError> {
    let target = removal.target;
    let actual = removal
        .count
        .min(game.counter_count(target, removal.counter_type));
    match game.remove_counters(
        target,
        removal.counter_type,
        removal.count,
        removal.source,
        removal.actor,
    ) {
        Some((removed, mut notification)) => {
            if removed != actual {
                return Err(ExecutionError::InternalError(
                    "counter-removal commit disagrees with its resolved amount".into(),
                ));
            }
            if removal.source == Some(ctx.source)
                && game.object(ctx.source).is_none()
                && let Some(snapshot) = &ctx.source_snapshot
            {
                notification = notification.with_source_snapshot(snapshot.clone());
            }
            Ok(EffectOutcome::count(i64::from(removed))
                .with_event(notification)
                .with_affected_objects_from_game(game, vec![target]))
        }
        None => Ok(EffectOutcome::count(0)),
    }
}

/// One quantity decoder for counter-cost families. The enclosing instruction
/// owns this receipt even when replacements prevent, redirect or add actions.
/// Pending selection has not committed a cost and exports no new X.
pub(crate) fn counter_cost_x_from_outcome(
    outcome: &EffectOutcome,
    execution: &ExecutionContext,
) -> Result<Option<u32>, crate::effects::CostValidationError> {
    if execution.decision_maker.awaiting_choice() {
        return Ok(None);
    }
    let requested = outcome.requested_amount().ok_or_else(|| {
        crate::effects::CostValidationError::Other(
            "counter cost has no original quantity receipt".into(),
        )
    })?;
    u32::try_from(requested).map(Some).map_err(|_| {
        crate::effects::CostValidationError::Other(
            "counter cost X exceeds the supported range".into(),
        )
    })
}

/// Both validation and captured payment bind the same exact object and kind.
fn counter_removal_cost_input(
    effect: &RemoveCountersEffect,
    game: &GameState,
    source: crate::ids::ObjectId,
) -> Result<(crate::ids::ObjectId, u32), ExecutionError> {
    let target = match effect.target.base() {
        ChooseSpec::Source => source,
        ChooseSpec::SpecificObject(id) => {
            if !game.battlefield.contains(id)
                || !game
                    .object(*id)
                    .is_some_and(|object| object.zone == crate::zone::Zone::Battlefield)
                || game.is_phased_out(*id)
            {
                return Err(ExecutionError::Impossible(
                    "counter-cost object is unavailable".into(),
                ));
            }
            *id
        }
        _ => {
            return Err(ExecutionError::Impossible(
                "counter cost requires an exact source or granting object".into(),
            ));
        }
    };
    let available = game.counter_count(target, effect.counter_type);
    let count = if matches!(effect.count.unhinted(), Value::CountersOn(spec, Some(kind))
        if *kind == effect.counter_type && spec.base() == effect.target.base())
    {
        available
    } else {
        let quantity = effect.count.constant_integer().ok_or_else(|| {
            ExecutionError::Impossible(
                "counter cost requires fixed arithmetic or its exact object's matching counters"
                    .into(),
            )
        })?;
        u32::try_from(quantity.max(0)).map_err(|_| ExecutionError::ResourceLimitExceeded {
            resource: "counter-cost quantity",
            requested: quantity.max(0) as u128,
            maximum: u32::MAX as u128,
        })?
    };
    if available < count {
        return Err(ExecutionError::Impossible("not enough counters".into()));
    }
    Ok((target, count))
}

impl CostExecutableEffect for RemoveCountersEffect {
    fn payment_x_from_prepared_payment(
        &self,
        proposal: &dyn crate::effects::SimultaneousEffectProposal,
        _execution: &ExecutionContext,
    ) -> Result<Option<u32>, crate::effects::CostValidationError> {
        super::prepared_payment::counter_quantity_payment_x(proposal, Some(self.counter_type))
    }

    fn supports_prepared_payment(&self) -> bool {
        matches!(
            self.target.base(),
            ChooseSpec::Source | ChooseSpec::SpecificObject(_)
        ) && (self
            .count
            .constant_integer()
            .is_some_and(|quantity| u32::try_from(quantity.max(0)).is_ok())
            || matches!(self.count.unhinted(), Value::CountersOn(spec, Some(kind))
                    if *kind == self.counter_type && spec.base() == self.target.base()))
    }

    fn prepare_simultaneous_payment(
        &self,
        game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        let (target, count) = counter_removal_cost_input(self, game, ctx.source)?;
        let event = crate::events::Event::remove_counters(target, self.counter_type, count)
            .with_provenance(ctx.provenance);
        super::capture_counter_payment_with_quantity(game, ctx, vec![event])
    }

    fn accepts_prepared_payment(
        &self,
        proposal: &dyn crate::effects::SimultaneousEffectProposal,
    ) -> bool {
        super::prepared_payment::accepts_counter_quantity_payment(proposal, Some(self.counter_type))
    }

    fn validate_payment_outcome(
        &self,
        outcome: &EffectOutcome,
    ) -> Result<(), crate::effects::CostValidationError> {
        super::prepared_payment::validate_counter_quantity_payment(outcome)
    }

    fn payment_x_from_outcome(
        &self,
        outcome: &EffectOutcome,
        execution: &ExecutionContext,
    ) -> Result<Option<u32>, crate::effects::CostValidationError> {
        if matches!(self.target.base(), ChooseSpec::Source) {
            counter_cost_x_from_outcome(outcome, execution)
        } else {
            Ok(None)
        }
    }

    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        _controller: crate::ids::PlayerId,
    ) -> Result<(), crate::effects::CostValidationError> {
        counter_removal_cost_input(self, game, source)
            .map(|_| ())
            .map_err(|error| match error {
                ExecutionError::Impossible(message) => {
                    crate::effects::CostValidationError::Other(message)
                }
                error => crate::effects::CostValidationError::ExecutionFailed(error),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effects::ResolvedTarget;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::test_prelude::*;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn make_creature_card(card_id: u32, name: &str) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    }

    fn create_creature_with_counters(
        game: &mut GameState,
        name: &str,
        controller: PlayerId,
        counter_type: CounterType,
        count: u32,
    ) -> ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name);
        let mut obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        obj.counters.insert(counter_type, count);
        game.add_object(obj);
        id
    }

    #[test]
    fn test_remove_counters() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature_with_counters(
            &mut game,
            "Hangarback Walker",
            alice,
            CounterType::PlusOnePlusOne,
            5,
        );
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = RemoveCountersEffect::plus_one_counters(2, ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        let obj = game.object(creature_id).unwrap();
        assert_eq!(obj.counters.get(&CounterType::PlusOnePlusOne), Some(&3)); // 5 - 2
    }

    #[test]
    fn remove_counters_records_affected_result_memory() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature_with_counters(
            &mut game,
            "Hangarback Walker",
            alice,
            CounterType::PlusOnePlusOne,
            5,
        );
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let result = RemoveCountersEffect::plus_one_counters(2, ChooseSpec::creature())
            .execute(&mut game, &mut ctx)
            .expect("remove counters should resolve");

        assert_eq!(result.affected_objects(), Some([creature_id].as_slice()));
        let memory = result
            .affected_object_memory()
            .expect("counter target memory should be recorded");
        assert_eq!(memory.len(), 1);
        assert_eq!(memory[0].object_id, creature_id);
    }

    #[test]
    fn test_remove_more_than_available() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature_with_counters(
            &mut game,
            "Hangarback Walker",
            alice,
            CounterType::PlusOnePlusOne,
            2,
        );
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = RemoveCountersEffect::plus_one_counters(5, ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Only 2 counters were available to remove
        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        // When all counters are removed, the entry is removed from the HashMap
        assert_eq!(
            game.counter_count(creature_id, CounterType::PlusOnePlusOne),
            0
        );
    }

    #[test]
    fn test_remove_from_no_counters() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature_with_counters(
            &mut game,
            "Grizzly Bears",
            alice,
            CounterType::PlusOnePlusOne,
            0,
        );
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = RemoveCountersEffect::plus_one_counters(1, ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // No counters to remove
        assert_eq!(result.value, crate::effect::OutcomeValue::Count(0));
    }

    #[test]
    fn test_remove_counters_no_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = RemoveCountersEffect::plus_one_counters(1, ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx);

        assert!(result.is_err());
    }

    #[test]
    fn test_remove_counters_from_source_spec() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature_with_counters(
            &mut game,
            "Echo Host",
            alice,
            CounterType::PlusOnePlusOne,
            2,
        );
        let mut ctx = ExecutionContext::new_default(creature_id, alice);

        let effect = RemoveCountersEffect::plus_one_counters(1, ChooseSpec::Source);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1));
        assert_eq!(
            game.counter_count(creature_id, CounterType::PlusOnePlusOne),
            1
        );
    }

    #[test]
    fn test_remove_counters_clone_box() {
        let effect = RemoveCountersEffect::plus_one_counters(1, ChooseSpec::creature());
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("RemoveCountersEffect"));
    }

    #[test]
    fn test_remove_counters_get_target_spec() {
        let effect = RemoveCountersEffect::plus_one_counters(1, ChooseSpec::creature());
        assert!(effect.get_target_spec().is_some());
    }
}

#[cfg(test)]
mod counter_removal_replacement_owner_tests {
    use super::*;

    fn check_replaced_removal(instead: bool) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = crate::ids::PlayerId::from_index(0);
        let definition = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Counter removal source")
            .card_types(vec![crate::types::CardType::Artifact]).build();
        let source = game.create_object_from_definition(&definition, alice, crate::zone::Zone::Battlefield);
        let counter_type = crate::object::CounterType::Charge;
        game.object_mut(source).unwrap().counters.insert(counter_type, 3);
        let action = if instead {
            crate::replacement::ReplacementAction::Instead(vec![crate::effect::Effect::gain_life(2)])
        } else { crate::replacement::ReplacementAction::Prevent };
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(
            crate::replacement::ReplacementEffect::with_matcher(source, alice,
                crate::events::counters::matchers::WouldRemoveCountersMatcher::any(), action));
        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = crate::effect::Effect::new(RemoveCountersEffect::new(counter_type, 2, ChooseSpec::Source));
        let outcome = crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
        assert_eq!(game.counter_count(source, counter_type), 3,
            "counter removal must honor the declared replacement proposal before mutating counters");
        assert_eq!(outcome.count_or_zero(), 0);
        assert_eq!(game.player(alice).unwrap().life, if instead { 22 } else { 20 });
        assert_eq!(outcome.events_of_type::<crate::events::MarkersChangedEvent>().count(), 0);
        assert_eq!(outcome.events_of_type::<crate::events::LifeGainEvent>().count(), usize::from(instead));
        assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
        let next = crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
        assert_eq!(next.count_or_zero(), 2);
        assert_eq!(game.counter_count(source, counter_type), 1);
        assert_eq!(next.events_of_type::<crate::events::MarkersChangedEvent>().count(), 1);
        assert_eq!(game.player(alice).unwrap().life, if instead { 22 } else { 20 });
    }

    #[test]
    fn counter_removal_prevention_reaches_actual_effect_owner() { check_replaced_removal(false); }
    #[test]
    fn counter_removal_instead_program_reaches_actual_effect_owner() { check_replaced_removal(true); }
}

#[cfg(test)]
mod removed_counter_removal_selected_api_tests {
    use super::*;
    #[test]
    fn selected_zero_counter_removal_preserves_one_shot_until_positive_proposal() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = crate::ids::PlayerId::from_index(0);
        let definition = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Selected counter removal source")
            .card_types(vec![crate::types::CardType::Artifact]).build();
        let source = game.create_object_from_definition(&definition, alice, crate::zone::Zone::Battlefield);
        let counter_type = crate::object::CounterType::Charge;
        game.object_mut(source).unwrap().counters.insert(counter_type, 3);
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(
            crate::replacement::ReplacementEffect::with_matcher(source, alice,
                crate::events::counters::matchers::WouldRemoveCountersMatcher::any(),
                crate::replacement::ReplacementAction::Prevent));
        for count in [0, 1] {
            let result = crate::events::processing::process_event_with_chosen_replacement_trait(
                &mut game, crate::events::Event::remove_counters(source, counter_type, count), shield).unwrap();
            if count == 0 {
                let event = result.resolved_event().expect("zero removal retains its proposal without applying prevention");
                let removal = crate::events::downcast_event::<crate::events::RemoveCountersEvent>(event.inner()).unwrap();
                assert_eq!(removal.count, 0);
                assert_eq!(removal.target, source);
                assert_eq!(removal.counter_type, counter_type);
            } else {
                assert!(matches!(result, crate::events::processing::TraitEventResult::Prevented));
            }
            assert_eq!(game.effect_store.replacement_effects.get_effect(shield).is_some(), count == 0);
            assert_eq!(game.counter_count(source, counter_type), 3, "proposal APIs do not commit removal");
            assert!(game.take_pending_trigger_events().is_empty());
        }
    }
}

#[cfg(test)]
mod removed_counter_removal_quantity_tests {
    use super::*;
    fn check_removed_operation(modification: crate::replacement::EventModification) {
        struct PreferSource(crate::ids::ObjectId);
        impl crate::decision::DecisionMaker for PreferSource {
            fn decide_options(&mut self, _game: &GameState, ctx: &crate::decisions::context::SelectOptionsContext) -> Vec<usize> {
                let option = ctx.options.iter().find(|option| option.legal && option.object_id == Some(self.0))
                    .or_else(|| ctx.options.iter().find(|option| option.legal)).unwrap();
                vec![option.index]
            }
        }
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = crate::ids::PlayerId::from_index(0);
        let definition = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Counter removal quantity source")
            .card_types(vec![crate::types::CardType::Artifact]).build();
        let source = game.create_object_from_definition(&definition, alice, crate::zone::Zone::Battlefield);
        let adder = game.create_object_from_definition(&definition, alice, crate::zone::Zone::Battlefield);
        let counter_type = crate::object::CounterType::Charge;
        game.object_mut(source).unwrap().counters.insert(counter_type, 3);
        let register = |source, modification| crate::replacement::ReplacementEffect::with_matcher(source, alice,
            crate::events::counters::matchers::WouldRemoveCountersMatcher::any(),
            crate::replacement::ReplacementAction::Modify(modification));
        let removed = game.effect_store.replacement_effects.add_one_shot_effect(register(source, modification));
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(register(adder, crate::replacement::EventModification::Add(1)));
        let effect = crate::effect::Effect::new(RemoveCountersEffect::new(counter_type, 1, ChooseSpec::Source));
        let mut chooser = PreferSource(source);
        for positive in [false, true] {
            game.take_pending_trigger_events();
            let mut ctx = ExecutionContext::new(source, alice, &mut chooser);
            let outcome = crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
            assert_eq!(outcome.count_or_zero(), if positive { 2 } else { 0 });
            assert_eq!(game.counter_count(source, counter_type), if positive { 1 } else { 3 });
            assert_eq!(outcome.events_of_type::<crate::events::MarkersChangedEvent>().count(), usize::from(positive));
            assert!(game.effect_store.replacement_effects.get_effect(removed).is_none());
            assert_eq!(game.effect_store.replacement_effects.get_effect(shield).is_some(), !positive);
            if !positive { assert!(game.take_pending_trigger_events().is_empty()); }
        }
    }
    #[test]
    fn counter_removal_subtraction_to_zero_preserves_later_increase() {
        check_removed_operation(crate::replacement::EventModification::Subtract(1));
    }
    #[test]
    fn counter_removal_set_to_zero_preserves_later_increase() {
        check_removed_operation(crate::replacement::EventModification::SetTo(0));
    }
}

#[cfg(test)]
mod unsigned_counter_quantity_contract_tests {
    use super::*;
    use crate::card::{CardBuilder,PowerToughness};
    use crate::effect::{Effect,EffectId};
    use crate::effects::{execute_effect,PutCountersEffect,DealDamageEffect};
    use crate::ids::{CardId,PlayerId};
    use crate::object::CounterType;
    use crate::types::CardType;
    use crate::zone::Zone;
    fn fixture()->(GameState,crate::ids::ObjectId,PlayerId) {
        let mut game=crate::tests::test_helpers::setup_two_player_game();let alice=PlayerId::from_index(0);
        let source=game.create_object_from_card(&CardBuilder::new(CardId::new(),"Unsigned counter source").card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(1,1)).build(),alice,Zone::Battlefield);
        (game,source,alice)
    }
    #[test]
    fn unsigned_literal_constructor_preserves_full_counter_quantity() {
        for amount in [i32::MAX as u32,i32::MAX as u32+1,u32::MAX] {
            let (mut game,source,alice)=fixture();let mut ctx=ExecutionContext::new_default(source,alice);
            let outcome=PutCountersEffect::new(CounterType::Charge,amount,ChooseSpec::SpecificObject(source)).execute(&mut game,&mut ctx).unwrap();
            assert_eq!(game.counter_count(source,CounterType::Charge),amount,"unsigned literal must preserve its value");
            assert_eq!(outcome.as_count(),Some(i64::from(amount)));
        }
    }
    #[test]
    fn unsigned_prior_counter_receipt_drives_full_removal() {
        for amount in [i32::MAX as u32,i32::MAX as u32+1,u32::MAX] {
            let (mut game,source,alice)=fixture();let mut ctx=ExecutionContext::new_default(source,alice);ctx.x_value=Some(amount);
            let placed=execute_effect(&mut game,&Effect::with_id(21,Effect::new(PutCountersEffect::new(CounterType::Charge,Value::X,ChooseSpec::SpecificObject(source)))),&mut ctx).unwrap();
            assert_eq!(placed.as_count(),Some(i64::from(amount)));
            let removed=RemoveCountersEffect::new(CounterType::Charge,Value::EffectValue(EffectId(21)),ChooseSpec::SpecificObject(source)).execute(&mut game,&mut ctx).expect("representable unsigned removal must resolve");
            assert_eq!(game.counter_count(source,CounterType::Charge),0);
            assert_eq!(removed.as_count(),Some(i64::from(amount)));
        }
    }
    #[test]
    fn source_counter_prevention_consumes_full_unsigned_follow_up_quantity() {
        for amount in [i32::MAX as u32,i32::MAX as u32+1,u32::MAX] {
            let (mut game,shield,alice)=fixture();
            let attacker=game.create_object_from_card(&CardBuilder::new(CardId::new(),"Damage source").card_types(vec![CardType::Artifact]).build(),alice,Zone::Battlefield);
            game.object_mut(shield).unwrap().counters.insert(CounterType::Charge,amount);
            game.effect_store.replacement_effects.add_resolution_effect(crate::replacement::ReplacementEffect::with_matcher(shield,alice,crate::events::DamageToSelfMatcher::new(),
                crate::replacement::ReplacementAction::PreventDamageByRemovingSourceCounters {counter_type:CounterType::Charge}));
            let mut ctx=ExecutionContext::new_default(attacker,alice);ctx.x_value=Some(amount);
            let outcome=DealDamageEffect::new(Value::X,ChooseSpec::SpecificObject(shield)).execute(&mut game,&mut ctx).unwrap();
            assert_eq!(game.damage_on(shield),0,"shield prevents the unsigned amount");
            assert_eq!(game.counter_count(shield,CounterType::Charge),0,"prevention consumes exactly its unsigned counter budget");
            assert_eq!(outcome.as_count(),Some(0),"prevented original damage remains zero");
            let next=DealDamageEffect::new(1,ChooseSpec::SpecificObject(shield)).execute(&mut game,&mut ctx).unwrap();
            assert_eq!(game.damage_on(shield),1,"spent shield cannot prevent the next damage");
            assert_eq!(next.as_count(),Some(1));
        }
    }
}

#[cfg(test)]
mod wide_bounded_counter_request_tests {
    use super::*;
    use crate::effect::{Effect,EffectId};
    use crate::effects::{execute_effect,MoveAllCountersEffect,MoveCountersEffect};
    use crate::object::CounterType;
    use crate::ids::{CardId,PlayerId};
    use crate::types::CardType;
    use crate::zone::Zone;
    fn object(game:&mut GameState,alice:PlayerId)->crate::ids::ObjectId {
        let card=crate::card::CardBuilder::new(CardId::new(),"Bounded quantity recipient").card_types(vec![CardType::Artifact]).build();
        game.create_object_from_card(&card,alice,Zone::Battlefield)
    }
    fn check(movement:bool) {
        let mut game=crate::tests::test_helpers::setup_two_player_game();let alice=PlayerId::from_index(0);
        let source=object(&mut game,alice);let collected=object(&mut game,alice);let following=object(&mut game,alice);
        for kind in [CounterType::Charge,CounterType::PlusOnePlusOne] {game.object_mut(source).unwrap().counters.insert(kind,u32::MAX);}
        let mut ctx=ExecutionContext::new_default(source,alice);
        let out=execute_effect(&mut game,&Effect::with_id(27,Effect::new(MoveAllCountersEffect::new(ChooseSpec::SpecificObject(source),ChooseSpec::SpecificObject(collected)))),&mut ctx).unwrap();
        assert_eq!(out.as_count(),Some(2*i64::from(u32::MAX)),"real mixed movement produces a wider-than-event receipt");
        let amount=Value::EffectValue(EffectId(27));
        // The targeted transfer consumes its announced pair, as real casting does.
        if movement {ctx.targets=vec![crate::effects::ResolvedTarget::Object(collected),crate::effects::ResolvedTarget::Object(following)];}
        let out=if movement {
            MoveCountersEffect::new(CounterType::Charge,amount,ChooseSpec::SpecificObject(collected),ChooseSpec::SpecificObject(following)).execute(&mut game,&mut ctx)
        } else {
            RemoveCountersEffect::new(CounterType::Charge,amount,ChooseSpec::SpecificObject(collected)).execute(&mut game,&mut ctx)
        }.expect("available counters bound a realizable operation even when requested total exceeds one event range");
        assert_eq!(out.as_count(),Some(i64::from(u32::MAX)));
        assert_eq!(game.counter_count(collected,CounterType::Charge),0);
        assert_eq!(game.counter_count(collected,CounterType::PlusOnePlusOne),u32::MAX,"unrequested kind remains untouched");
        assert_eq!(game.counter_count(following,CounterType::Charge),if movement {u32::MAX} else {0});
        for kind in [CounterType::Charge,CounterType::PlusOnePlusOne] {assert_eq!(game.counter_count(source,kind),0);}
    }
    #[test] fn wide_actual_prior_total_removes_all_available_counters() {check(false);}
    #[test] fn wide_actual_prior_total_moves_all_available_counters() {check(true);}
}
