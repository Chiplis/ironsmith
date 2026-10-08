//! Put counters effect implementation.

use crate::decision::FallbackStrategy;
use crate::decisions::{DistributeSpec, NumberSpec, make_decision_with_fallback};
use crate::effect::{ChoiceCount, EffectOutcome, ExecutionFact, Value};
use crate::effects::helpers::{resolve_nonnegative_u32, resolve_objects_for_effect};
use crate::effects::{CostExecutableEffect, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::filter::{FilterContext, ObjectFilterExt as _};
use crate::game_state::{GameState, Target};
use crate::ids::ObjectId;
use crate::target::ChooseSpec;
pub use ironsmith_core::PutCountersEffect;

/// Effect that puts counters on a target permanent.
///
/// Supports replacement effects like Doubling Season and Hardened Scales.
///
/// # Fields
///
/// * `counter_type` - The type of counter to put
/// * `count` - How many counters to put
/// * `target` - Which permanent to target
/// * `target_count` - How many targets (for "up to" effects)
/// * `distributed` - If true, distribute total counters among chosen targets
///
/// # Example
///
/// ```ignore
/// // Put two +1/+1 counters on target creature
/// let effect = PutCountersEffect::new(
///     CounterType::PlusOnePlusOne,
///     2,
///     ChooseSpec::creature(),
/// );
/// ```
impl EffectExecutor for PutCountersEffect {
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
        Ok(Box::new(CounterInstructionProposal {
            effect: self.clone(),
            input: None,
            prepared: Vec::new(),
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
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        game.clear_pending_decision_controllers();
        let result = crate::effects::composition::execute_transaction(
            game,
            ctx,
            || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                let (mut requests, count) = match resolve_counter_inputs(self, game, ctx)? {
                    CounterInputPlan::Pending => {
                        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::count(0),
                        ));
                    }
                    CounterInputPlan::Finished { outcome, count } => {
                        return counter_action_completed_outputs(
                            self,
                            game,
                            ctx,
                            crate::effects::CompletedEffectOutputs::aggregate_only(outcome),
                            count,
                        );
                    }
                    CounterInputPlan::Placements { requests, count } => (requests, count),
                };
                // Freeze every original before deferred additions. Grouping
                // notifications alone does not make sequential mutations simultaneous.
                let outcomes = if requests.len() > 1 {
                    super::placement::execute_counter_batch_with_limit_outputs(
                        game,
                        ctx,
                        requests,
                        self.maximum_total,
                    )?
                } else if let Some(event) = requests.pop() {
                    vec![
                        super::placement::execute_counter_placement_with_limit_outputs(
                            game,
                            ctx,
                            event,
                            self.maximum_total,
                        )?,
                    ]
                } else {
                    Vec::new()
                };
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }

                let outcome = EffectOutcome::aggregate_summing_counts(
                    outcomes.iter().map(|outputs| outputs.outcome.clone()),
                );
                let mut outputs = crate::effects::CompletedEffectOutputs::aggregate_only(outcome);
                for child in outcomes {
                    outputs.retain_owned_child(child);
                }
                counter_action_completed_outputs(self, game, ctx, outputs, count)
            },
        );
        // Suspension is neutral only on success; failed discovery stays failed.
        if result.is_ok() && ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        result
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "target for counters"
    }

    fn get_target_count(&self) -> Option<ChoiceCount> {
        self.target_count
    }

    fn get_target_distribution_value(&self) -> Option<&Value> {
        // CR 601.2d: "distribute N counters among ... targets" is divided
        // when the targets are announced, at least one per target.
        (self.distributed && self.target.is_target()).then_some(&self.amount)
    }

    fn cost_description(&self) -> Option<String> {
        if self.completion_action == Some(crate::events::KeywordActionKind::Blight)
            && let Value::Fixed(count) = self.amount
        {
            return Some(format!("Blight {count}"));
        }
        if matches!(self.target.base(), ChooseSpec::Source)
            && let Value::Fixed(count) = self.amount
        {
            return Some(if count == 1 {
                format!(
                    "Put a {} counter on this source",
                    self.counter_type.description()
                )
            } else {
                format!(
                    "Put {} {} counters on this source",
                    count,
                    self.counter_type.description()
                )
            });
        }
        None
    }
}

enum CounterInputPlan {
    Pending,
    Finished {
        outcome: EffectOutcome,
        count: u32,
    },
    Placements {
        requests: Vec<crate::events::Event>,
        count: u32,
    },
}

/// Resolve authored choices and allocations once, before placement originals.
fn resolve_counter_inputs(
    effect: &PutCountersEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Result<CounterInputPlan, ExecutionError> {
    let zero_amount = matches!(effect.amount, Value::Fixed(0));
    // Handle Source target specially (for abilities like level-up that target themselves).
    let target_ids = match effect.target.base() {
        ChooseSpec::Object(filter)
            if ctx.cause.cause_type == crate::events::cause::CauseType::Cost
                || (ctx.optional_action
                    && effect.completion_action
                        == Some(crate::events::KeywordActionKind::Blight)) =>
        {
            let filter_ctx = game.filter_context_for(ctx.controller, Some(ctx.source));
            let candidates: Vec<_> = game
                .battlefield
                .iter()
                .copied()
                .filter(|id| {
                    !game.is_phased_out(*id)
                        && (zero_amount
                            || game.can_have_counter_type_placed(*id, effect.counter_type))
                        && game
                            .object(*id)
                            .is_some_and(|object| filter.matches(object, &filter_ctx, game))
                })
                .collect();
            if candidates.is_empty() {
                return Err(ExecutionError::Impossible(
                    "no valid object for counter cost".into(),
                ));
            }
            let spec = crate::decisions::specs::ChooseObjectsSpec::new(
                ctx.source,
                "Choose a creature to receive counters",
                candidates,
                1,
                Some(1),
            );
            crate::decisions::make_decision(
                game,
                ctx.decision_maker,
                ctx.controller,
                Some(ctx.source),
                spec,
            )
        }
        ChooseSpec::Source => vec![ctx.source],
        _ => match resolve_objects_for_effect(game, ctx, &effect.target) {
            Ok(objects) if !objects.is_empty() => objects,
            Ok(_) | Err(ExecutionError::InvalidTarget) | Err(ExecutionError::TagNotFound(_)) => {
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(CounterInputPlan::Pending);
                }
                // No target chosen (valid for "up to" effects).
                let count = resolve_nonnegative_u32(game, &effect.amount, ctx)?;
                return Ok(CounterInputPlan::Finished {
                    outcome: EffectOutcome::resolved(),
                    count,
                });
            }
            Err(error) => return Err(error),
        },
    };

    if ctx.decision_maker.awaiting_choice() {
        return Ok(CounterInputPlan::Pending);
    }
    // The authored/shared choice is independent of each recipient's headroom.
    let max_count = resolve_nonnegative_u32(game, &effect.amount, ctx)?;
    let amount_is_up_to = effect
        .amount
        .has_surface_hint(ironsmith_core::ValueSurfaceHint::UpTo);
    let count = if amount_is_up_to {
        let description = format!(
            "Choose how many {} counters to put",
            effect.counter_type.description()
        );
        let spec = NumberSpec::up_to(ctx.source, max_count, description);
        let chosen = make_decision_with_fallback(
            game,
            &mut ctx.decision_maker,
            ctx.controller,
            Some(ctx.source),
            spec,
            FallbackStrategy::Maximum,
        );
        if ctx.decision_maker.awaiting_choice() {
            return Ok(CounterInputPlan::Pending);
        }
        chosen.min(max_count)
    } else {
        max_count
    };
    if count == 0 {
        let outcome = if amount_is_up_to {
            EffectOutcome::count(0).with_execution_fact(ExecutionFact::ChosenNumber(0))
        } else {
            EffectOutcome::count(0)
        };
        return Ok(CounterInputPlan::Finished { outcome, count: 0 });
    }

    // CR 601.2d / 603.3d: the controller announces how distributed
    // counters are divided (at least one per target) when the targets
    // are chosen. A share announced for a target that has since become
    // illegal is lost (CR 608.2b). Without an announced division (a
    // non-targeted "distribute among" or a trigger whose division wasn't
    // announced) the controller divides them now.
    let distributed_counts: Option<Vec<(ObjectId, u32)>> = if effect.distributed {
        let announced = if effect.target.is_target() {
            ctx.take_target_distribution(&effect.target)
        } else {
            None
        };
        let division: Vec<(Target, u32)> = if let Some(announced) = announced {
            announced.allocations
        } else if target_ids.len() == 1 {
            vec![(Target::Object(target_ids[0]), count)]
        } else {
            let min_per_target = if effect.target.is_target() && count as usize >= target_ids.len()
            {
                1
            } else {
                0
            };
            let spec = DistributeSpec::new(
                ctx.source,
                count,
                target_ids.iter().copied().map(Target::Object).collect(),
                min_per_target,
            );
            let division = make_decision_with_fallback(
                game,
                &mut ctx.decision_maker,
                ctx.controller,
                Some(ctx.source),
                spec,
                FallbackStrategy::Maximum,
            );
            if ctx.decision_maker.awaiting_choice() {
                return Ok(CounterInputPlan::Pending);
            }
            division
        };
        // Keep the allocations in target order so counters land in the
        // same order on every peer; never place more than the total.
        let mut remaining = count;
        let mut allocations: Vec<(ObjectId, u32)> = Vec::new();
        for target_id in &target_ids {
            let share: u32 = division
                .iter()
                .filter(|(target, _)| *target == Target::Object(*target_id))
                .map(|(_, amount)| *amount)
                .sum();
            let share = share.min(remaining);
            remaining -= share;
            allocations.push((*target_id, share));
        }
        Some(allocations)
    } else {
        None
    };

    let mut requests = Vec::with_capacity(target_ids.len());
    for target_id in target_ids {
        let assigned_count = distributed_counts
            .as_ref()
            .map(|allocations| {
                allocations
                    .iter()
                    .find(|(id, _)| *id == target_id)
                    .map_or(0, |(_, amount)| *amount)
            })
            .unwrap_or(count);
        // A full recipient contributes no proposal and cannot consume a
        // replacement intended for a later eligible recipient. Modifiers are
        // still checked against this same recipient's ceiling at commitment.
        let assigned_count = effect.maximum_total.map_or(assigned_count, |maximum| {
            assigned_count
                .min(maximum.saturating_sub(game.counter_count(target_id, effect.counter_type)))
        });
        if assigned_count == 0 {
            continue;
        }
        let event = crate::events::Event::put_counters(
            target_id,
            effect.counter_type,
            assigned_count,
            ctx.cause.clone(),
        )
        .with_provenance(ctx.provenance);
        requests.push(event);
    }
    Ok(CounterInputPlan::Placements { requests, count })
}

struct CounterInstructionProposal {
    effect: PutCountersEffect,
    input: Option<CounterInputPlan>,
    prepared: Vec<Option<super::prepared_placement::PreparedCounterPlacement>>,
}

impl std::fmt::Debug for CounterInstructionProposal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CounterInstructionProposal")
            .field("effect", &self.effect)
            .finish_non_exhaustive()
    }
}

struct CounterInstructionCompletion {
    effect: PutCountersEffect,
    count: u32,
    batch: Option<crate::provenance::ProvNodeId>,
    inner: Option<Box<dyn crate::effects::SimultaneousEffectCompletion>>,
}

impl crate::effects::SimultaneousEffectCompletion for CounterInstructionCompletion {
    fn original_phase_status(&self) -> crate::effects::OriginalPhaseStatus {
        self.inner
            .as_ref()
            .map_or(crate::effects::OriginalPhaseStatus::Complete, |inner| {
                inner.original_phase_status()
            })
    }

    fn complete_original_phase_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        ExecutionError,
    > {
        let Self {
            effect,
            count,
            batch,
            inner,
        } = *self;
        let mut receipt = if let Some(inner) = inner {
            inner.complete_original_phase_with_outputs(game, ctx, original)?
        } else {
            crate::effects::SimultaneousEffectCommit::finished(
                crate::effects::CompletedEffectOutputs::aggregate_only(original),
            )
        };
        if ctx.decision_maker.awaiting_choice() {
            return Ok(receipt);
        }
        // The instruction completion notification follows all inner additions.
        // Retain that owner even when the original phase has no further child.
        receipt.completion = Some(Box::new(Self {
            effect,
            count,
            batch,
            inner: receipt.completion,
        }));
        Ok(receipt)
    }

    fn complete_original_phase_from_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: crate::effects::CompletedEffectOutputs,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        ExecutionError,
    > {
        let Self {
            effect,
            count,
            batch,
            inner,
        } = *self;
        let mut receipt = if let Some(inner) = inner {
            inner.complete_original_phase_from_outputs(game, ctx, original)?
        } else {
            crate::effects::SimultaneousEffectCommit::finished(original)
        };
        if ctx.decision_maker.awaiting_choice() {
            return Ok(receipt);
        }
        // The instruction completion notification follows all inner additions.
        // Retain that owner even when the original phase has no further child.
        receipt.completion = Some(Box::new(Self {
            effect,
            count,
            batch,
            inner: receipt.completion,
        }));
        Ok(receipt)
    }

    fn prepare_draw_boundary_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        ExecutionError,
    > {
        let Self {
            effect,
            count,
            batch,
            inner,
        } = *self;
        let mut receipt = if let Some(inner) = inner {
            inner.prepare_draw_boundary_with_outputs(game, ctx, original)?
        } else {
            crate::effects::SimultaneousEffectCommit::finished(
                crate::effects::CompletedEffectOutputs::aggregate_only(original),
            )
        };
        if ctx.decision_maker.awaiting_choice() {
            return Ok(receipt);
        }
        if let Some(inner) = receipt.completion.take() {
            receipt.completion = Some(Box::new(Self {
                effect,
                count,
                batch,
                inner: Some(inner),
            }));
            return Ok(receipt);
        }
        let outputs = counter_action_completed_outputs_in_group(
            &effect,
            game,
            ctx,
            receipt.outcome,
            count,
            batch,
        )?;
        Ok(crate::effects::SimultaneousEffectCommit::finished(outputs))
    }

    fn prepare_draw_boundary_from_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: crate::effects::CompletedEffectOutputs,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        ExecutionError,
    > {
        let Self {
            effect,
            count,
            batch,
            inner,
        } = *self;
        let mut receipt = if let Some(inner) = inner {
            inner.prepare_draw_boundary_from_outputs(game, ctx, original)?
        } else {
            crate::effects::SimultaneousEffectCommit::finished(original)
        };
        if ctx.decision_maker.awaiting_choice() {
            return Ok(receipt);
        }
        if let Some(inner) = receipt.completion.take() {
            receipt.completion = Some(Box::new(Self {
                effect,
                count,
                batch,
                inner: Some(inner),
            }));
            return Ok(receipt);
        }
        receipt.outcome.projections_complete = false;
        let outputs = counter_action_completed_outputs_in_group(
            &effect,
            game,
            ctx,
            receipt.outcome,
            count,
            batch,
        )?;
        Ok(crate::effects::SimultaneousEffectCommit::finished(outputs))
    }

    fn observe_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut crate::effects::ExecutionContext,
        original: &mut EffectOutcome,
    ) -> Result<(), crate::effects::ExecutionError> {
        if let Some(inner) = &mut self.inner {
            inner.observe_original(game, ctx, original)?;
        }
        Ok(())
    }

    fn freeze(&mut self, game: &mut GameState) -> Result<(), ExecutionError> {
        if let Some(inner) = &mut self.inner {
            inner.freeze(game)?;
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
        let outputs = if let Some(inner) = self.inner {
            inner.complete_with_outputs(game, ctx, original)?
        } else {
            crate::effects::CompletedEffectOutputs::aggregate_only(original)
        };
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        counter_action_completed_outputs_in_group(
            &self.effect,
            game,
            ctx,
            outputs,
            self.count,
            self.batch,
        )
    }

    fn complete_from_original_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: crate::effects::CompletedEffectOutputs,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        let mut outputs = if let Some(inner) = self.inner {
            inner.complete_from_original_outputs(game, ctx, original)?
        } else {
            original
        };
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        // This instruction has not declared complete authored projections for
        // its original plus final notification. Retain the actual child data
        // while keeping the enclosing projection coverage conservative.
        outputs.projections_complete = false;
        counter_action_completed_outputs_in_group(
            &self.effect,
            game,
            ctx,
            outputs,
            self.count,
            self.batch,
        )
    }
}

impl crate::effects::SimultaneousEffectProposal for CounterInstructionProposal {
    fn has_simultaneous_originals(&self) -> bool {
        matches!(&self.input, Some(CounterInputPlan::Placements { requests, .. }) if requests.len() > 1)
    }

    fn prepare_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(());
        }
        if self.input.is_none() || matches!(self.input, Some(CounterInputPlan::Pending)) {
            self.input = Some(resolve_counter_inputs(&self.effect, game, ctx)?);
            if let Some(CounterInputPlan::Placements { requests, .. }) = &self.input {
                self.prepared = requests.iter().map(|_| None).collect();
            }
        }
        if let Some(CounterInputPlan::Placements { requests, .. }) = &self.input {
            for (request, prepared) in requests.iter().zip(&mut self.prepared) {
                if !prepared
                    .as_ref()
                    .is_some_and(|original| !original.requires_replacement_input())
                {
                    *prepared = Some(
                        super::prepared_placement::prepare_counter_placement_with_limit(
                            game,
                            ctx,
                            request.clone(),
                            self.effect.maximum_total,
                        )?,
                    );
                }
                if ctx.decision_maker.awaiting_choice() {
                    break;
                }
            }
        }
        Ok(())
    }

    fn commit_original(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::SimultaneousEffectCommit, ExecutionError> {
        self.commit_original_with_outputs(game, ctx)
            .map(crate::effects::SimultaneousEffectCommit::into_aggregate)
    }

    fn commit_original_with_outputs(
        mut self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        ExecutionError,
    > {
        self.prepare_original(game, ctx)?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::SimultaneousEffectCommit::finished(
                crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            ));
        }
        let (mut receipt, count) = match self.input.take() {
            Some(CounterInputPlan::Finished { outcome, count }) => (
                crate::effects::SimultaneousEffectCommit::finished(
                    crate::effects::CompletedEffectOutputs::aggregate_only(outcome),
                ),
                count,
            ),
            Some(CounterInputPlan::Placements { requests, count }) => {
                let grouped = requests.len() > 1;
                let mut batch = None;
                let mut receipts = Vec::with_capacity(self.prepared.len());
                for prepared in self.prepared {
                    let prepared = prepared.ok_or_else(|| {
                        ExecutionError::InternalError("counter original was not prepared".into())
                    })?;
                    let mut receipt =
                        super::commit_prepared_counter_original_with_outputs(game, ctx, prepared)?;
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(crate::effects::SimultaneousEffectCommit::finished(
                            crate::effects::CompletedEffectOutputs::aggregate_only(
                                EffectOutcome::count(0),
                            ),
                        ));
                    }
                    if grouped {
                        super::placement::group_counter_placement_events(
                            game,
                            ctx,
                            &mut receipt.outcome.outcome.events,
                            &mut batch,
                        );
                    }
                    receipts.push(receipt);
                }
                (
                    crate::effects::composition::compose_original_commits_with_projection_outputs(
                        receipts,
                        Box::new(EffectOutcome::aggregate_summing_counts),
                    ),
                    count,
                )
            }
            _ => {
                return Err(ExecutionError::InternalError(
                    "counter inputs were not prepared".into(),
                ));
            }
        };
        if self.effect.completion_action.is_some() {
            receipt.completion = Some(Box::new(CounterInstructionCompletion {
                effect: self.effect,
                count,
                batch: game.simultaneous_action_batch(),
                inner: receipt.completion.take(),
            }));
        }
        Ok(receipt)
    }
    fn commit(
        mut self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        crate::effects::composition::execute_compound(game, ctx, |game, ctx| {
            self.prepare_original(game, ctx)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            let simultaneous = matches!(&self.input, Some(CounterInputPlan::Placements { requests, .. }) if requests.len() > 1);
            crate::effects::composition::complete_prepared_original_with_grouping(
                self,
                game,
                ctx,
                simultaneous,
            )
        })
    }
}

fn counter_action_completed_outputs(
    effect: &PutCountersEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    outputs: crate::effects::CompletedEffectOutputs,
    amount: u32,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    let batch = game.simultaneous_action_batch();
    counter_action_completed_outputs_in_group(effect, game, ctx, outputs, amount, batch)
}

fn counter_action_completed_in_group(
    effect: &PutCountersEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    outcome: EffectOutcome,
    amount: u32,
    batch: Option<crate::provenance::ProvNodeId>,
) -> Result<EffectOutcome, ExecutionError> {
    counter_action_completed_outputs_in_group(
        effect,
        game,
        ctx,
        crate::effects::CompletedEffectOutputs::aggregate_only(outcome),
        amount,
        batch,
    )
    .map(crate::effects::CompletedEffectOutputs::into_outcome)
}

fn counter_action_completed_outputs_in_group(
    effect: &PutCountersEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    outputs: crate::effects::CompletedEffectOutputs,
    amount: u32,
    batch: Option<crate::provenance::ProvNodeId>,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    if let Some(action) = effect.completion_action {
        let mut event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::KeywordActionEvent::new(
                action,
                ctx.iteration.iterated_player.unwrap_or(ctx.controller),
                ctx.source,
                amount,
            ),
            ctx.provenance,
        );
        if let Some(batch) = batch {
            event = event.with_simultaneous_batch(batch);
        }
        let completion = crate::effects::composition::publish_keyword_action_completion_receipt(
            game, ctx, event,
        )?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        Ok(outputs.append_batch_completion_outputs(completion))
    } else {
        Ok(outputs)
    }
}

impl CostExecutableEffect for PutCountersEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
    ) -> Result<(), crate::effects::CostValidationError> {
        if self.target.is_target() {
            return Err(crate::effects::CostValidationError::Other(
                "a cost object must be chosen, not targeted".to_string(),
            ));
        }
        let zero_amount = matches!(self.amount, Value::Fixed(0));
        match self.target.base() {
            ChooseSpec::Source => {
                if game
                    .object(source)
                    .is_some_and(|obj| obj.zone == crate::zone::Zone::Battlefield)
                    && !game.is_phased_out(source)
                    && (zero_amount || game.can_have_counter_type_placed(source, self.counter_type))
                {
                    Ok(())
                } else {
                    Err(crate::effects::CostValidationError::Other(
                        "source must be on the battlefield".to_string(),
                    ))
                }
            }
            ChooseSpec::Object(filter) => {
                let filter_ctx = FilterContext::new(controller).with_source(source);
                if game.battlefield.iter().copied().any(|object_id| {
                    !game.is_phased_out(object_id)
                        && (zero_amount
                            || game.can_have_counter_type_placed(object_id, self.counter_type))
                        && game
                            .object(object_id)
                            .is_some_and(|object| filter.matches(object, &filter_ctx, game))
                }) {
                    Ok(())
                } else {
                    Err(crate::effects::CostValidationError::Other(
                        "no valid object for the counter cost".to_string(),
                    ))
                }
            }
            _ => Err(crate::effects::CostValidationError::Other(
                "put-counters cost supports only source or one chosen object".to_string(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::test_prelude::*;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn create_creature_on_battlefield(
        game: &mut GameState,
        name: &str,
        controller: PlayerId,
    ) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let object = Object::from_card(id, &card, controller, Zone::Battlefield);
        game.add_object(object);
        id
    }

    fn permanent_counter_replacement_case(
        action: crate::replacement::ReplacementAction,
    ) -> (GameState, ObjectId, ObjectId, ObjectId, PlayerId) {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = create_creature_on_battlefield(&mut game, "Counter replacement", alice);
        let original = create_creature_on_battlefield(&mut game, "Original recipient", alice);
        let redirected = create_creature_on_battlefield(&mut game, "Redirected recipient", alice);
        let mut replacement = crate::static_abilities::StaticAbility::double_counters_replacement(
            crate::target::ObjectFilter::creature(),
            Some(CounterType::PlusOnePlusOne),
            "Replace counter placement".into(),
        )
        .generate_replacement_effect(source, alice)
        .unwrap();
        replacement.replacement = action;
        game.effect_store
            .replacement_effects
            .add_resolution_effect(replacement);
        (game, source, original, redirected, alice)
    }

    #[test]
    fn permanent_counter_application_executes_instead_without_original_placement() {
        let (mut game, source, original, _, alice) =
            permanent_counter_replacement_case(crate::replacement::ReplacementAction::Instead(
                vec![crate::effect::Effect::gain_life(2)],
            ));
        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.targets = vec![crate::effects::ResolvedTarget::Object(original)];
        let outcome = PutCountersEffect::plus_one_counters(2, ChooseSpec::target_creature())
            .execute(&mut game, &mut ctx)
            .unwrap();
        assert_eq!(game.counter_count(original, CounterType::PlusOnePlusOne), 0);
        assert_eq!(game.player(alice).unwrap().life, 22);
        assert_eq!(outcome.status, crate::effect::OutcomeStatus::Replaced);
        assert_eq!(outcome.count_or_zero(), 0);
        assert_eq!(outcome.events.len(), 1);
        assert!(
            outcome.events[0]
                .downcast::<crate::events::LifeGainEvent>()
                .is_some()
        );
    }

    #[test]
    fn permanent_counter_application_commits_redirected_recipient() {
        let (mut game, source, original, redirected, alice) =
            permanent_counter_replacement_case(crate::replacement::ReplacementAction::Prevent);
        game.effect_store.replacement_effects = crate::replacement::ReplacementEffectManager::new();
        let mut replacement = crate::static_abilities::StaticAbility::double_counters_replacement(
            crate::target::ObjectFilter::creature(),
            Some(CounterType::PlusOnePlusOne),
            "Redirect counters".into(),
        )
        .generate_replacement_effect(source, alice)
        .unwrap();
        replacement.replacement = crate::replacement::ReplacementAction::Redirect {
            target: crate::replacement::RedirectTarget::ToObject(redirected),
            which: crate::replacement::RedirectWhich::First,
        };
        game.effect_store
            .replacement_effects
            .add_resolution_effect(replacement);
        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.targets = vec![crate::effects::ResolvedTarget::Object(original)];
        let outcome = PutCountersEffect::plus_one_counters(2, ChooseSpec::target_creature())
            .execute(&mut game, &mut ctx)
            .unwrap();
        assert_eq!(game.counter_count(original, CounterType::PlusOnePlusOne), 0);
        assert_eq!(
            game.counter_count(redirected, CounterType::PlusOnePlusOne),
            2
        );
        assert_eq!(outcome.affected_objects(), Some([redirected].as_slice()));
        assert_eq!(outcome.count_or_zero(), 2);
        let marker = outcome.events[0]
            .downcast::<crate::events::MarkersChangedEvent>()
            .unwrap();
        assert_eq!(
            marker.location,
            crate::marker::MarkerLocation::Object(redirected)
        );
    }

    #[test]
    fn permanent_counter_application_propagates_payload_error_and_restores_state() {
        let (mut game, source, original, _, alice) = permanent_counter_replacement_case(
            crate::replacement::ReplacementAction::Instead(vec![
                crate::effect::Effect::gain_life(2),
                crate::effect::Effect::gain_life(Value::X),
            ]),
        );
        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.targets = vec![crate::effects::ResolvedTarget::Object(original)];
        let outcome = PutCountersEffect::plus_one_counters(2, ChooseSpec::target_creature())
            .execute(&mut game, &mut ctx);
        assert!(
            outcome.is_err(),
            "replacement payload errors must propagate"
        );
        assert_eq!(game.counter_count(original, CounterType::PlusOnePlusOne), 0);
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert!(game.take_pending_trigger_events().is_empty());
    }

    struct CounterPayloadDecisions {
        recipient: ObjectId,
        check_unrenowned: Option<ObjectId>,
        pause: bool,
        pending: bool,
        calls: usize,
    }

    impl crate::decision::DecisionMaker for CounterPayloadDecisions {
        fn awaiting_choice(&self) -> bool {
            self.pending
        }
        fn decide_boolean(
            &mut self,
            game: &GameState,
            _: &crate::decisions::context::BooleanContext,
        ) -> bool {
            assert!(
                !self.pending,
                "asked another payload question while pending"
            );
            if let Some(source) = self.check_unrenowned {
                assert!(
                    !game.is_renowned(source),
                    "counter replacement sees the pre-renown state"
                );
            }
            self.calls += 1;
            self.pending = self.pause;
            !self.pause
        }
        fn decide_objects(
            &mut self,
            _: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            assert!(!self.pending);
            ctx.candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .take(ctx.min)
                .map(|candidate| candidate.id)
                .collect()
        }
        fn decide_counters(
            &mut self,
            _: &GameState,
            ctx: &crate::decisions::context::CountersContext,
        ) -> Vec<(CounterType, u32)> {
            assert!(!self.pending);
            assert!(
                ctx.available_counters
                    .iter()
                    .any(|(kind, count)| *kind == CounterType::PlusOnePlusOne && *count > 0)
            );
            vec![(CounterType::PlusOnePlusOne, 1)]
        }
        fn decide_proliferate(
            &mut self,
            _: &GameState,
            _: &crate::decisions::context::ProliferateContext,
        ) -> crate::decisions::specs::ProliferateResponse {
            assert!(!self.pending);
            crate::decisions::specs::ProliferateResponse {
                permanents: vec![self.recipient],
                players: Vec::new(),
            }
        }
    }

    fn counter_instruction(operation: usize) -> Box<dyn EffectExecutor> {
        match operation {
            0 => Box::new(PutCountersEffect::plus_one_counters(
                1,
                ChooseSpec::target_creature(),
            )),
            1 => Box::new(crate::effects::DoubleCountersEffect::new(
                None,
                ChooseSpec::target_creature(),
            )),
            2 => Box::new(crate::effects::ProliferateEffect::new(1)),
            3 => Box::new(crate::effects::MoveCountersEffect::new(
                CounterType::PlusOnePlusOne,
                1,
                ChooseSpec::creature(),
                ChooseSpec::creature(),
            )),
            4 => Box::new(crate::effects::MoveOneCounterEffect::new(
                ChooseSpec::creature(),
                ChooseSpec::creature(),
            )),
            5 => Box::new(crate::effects::MoveAllCountersEffect::new(
                ChooseSpec::creature(),
                ChooseSpec::creature(),
            )),
            6 => Box::new(crate::effects::RenownEffect::new(1)),
            7 => Box::new(crate::effects::EvolveEffect::new()),
            8 => Box::new(crate::effects::ExploreEffect::new(
                ChooseSpec::target_creature(),
            )),
            _ => Box::new(crate::effects::ConniveEffect::new(
                ChooseSpec::target_creature(),
            )),
        }
    }

    #[test]
    fn permanent_counter_consumers_preserve_instead_pause_error_and_replay() {
        for operation in 0..10 {
            for pending_case in [false, true] {
                let mut payload = vec![crate::effect::Effect::gain_life(2)];
                if pending_case {
                    payload.push(crate::effect::Effect::may(vec![
                        crate::effect::Effect::gain_life(1),
                    ]));
                    payload.push(crate::effect::Effect::may(vec![
                        crate::effect::Effect::gain_life(3),
                    ]));
                } else {
                    payload.push(crate::effect::Effect::gain_life(Value::X));
                }
                let (mut game, source, original, entered, alice) =
                    permanent_counter_replacement_case(
                        crate::replacement::ReplacementAction::Prevent,
                    );
                game.effect_store.replacement_effects =
                    crate::replacement::ReplacementEffectManager::new();
                game.object_mut(source)
                    .unwrap()
                    .add_counters(CounterType::PlusOnePlusOne, 1);
                game.object_mut(original)
                    .unwrap()
                    .add_counters(CounterType::PlusOnePlusOne, 1);
                let mut replacement =
                    crate::static_abilities::StaticAbility::double_counters_replacement(
                        crate::target::ObjectFilter::creature(),
                        Some(CounterType::PlusOnePlusOne),
                        "Instead of placing counters".into(),
                    )
                    .generate_replacement_effect(source, alice)
                    .unwrap();
                replacement.replacement = crate::replacement::ReplacementAction::Instead(payload);
                let one_shot = game
                    .effect_store
                    .replacement_effects
                    .add_one_shot_effect(replacement);
                let targets = if (3..=5).contains(&operation) {
                    vec![
                        crate::effects::ResolvedTarget::Object(source),
                        crate::effects::ResolvedTarget::Object(original),
                    ]
                } else {
                    vec![crate::effects::ResolvedTarget::Object(original)]
                };
                if operation == 7 {
                    game.object_mut(entered)
                        .unwrap()
                        .add_counters(CounterType::PlusOnePlusOne, 3);
                }
                let library_card = if operation == 9 {
                    Some(
                        game.create_object_from_card(
                            &CardBuilder::new(CardId::new(), "Connive draw card")
                                .card_types(vec![CardType::Instant])
                                .build(),
                            alice,
                            Zone::Library,
                        ),
                    )
                } else {
                    None
                };
                let instruction = counter_instruction(operation);
                let mut dm = CounterPayloadDecisions {
                    recipient: original,
                    check_unrenowned: (operation == 6).then_some(source),
                    pause: pending_case,
                    pending: false,
                    calls: 0,
                };
                let mut ctx = ExecutionContext::new(source, alice, &mut dm);
                ctx.targets = targets.clone();
                if operation == 7 {
                    ctx.triggering_event =
                        Some(crate::triggers::TriggerEvent::new_with_provenance(
                            crate::events::EnterBattlefieldEvent::new(entered, Zone::Hand),
                            crate::provenance::ProvNodeId::default(),
                        ));
                }
                ctx.set_tagged_players("retained", vec![alice]);
                let result = instruction.execute(&mut game, &mut ctx);
                assert_eq!(result.is_err(), !pending_case, "operation {operation}");
                assert_eq!(ctx.get_tagged_players("retained"), Some(&vec![alice]));
                if pending_case {
                    let outcome = result.unwrap();
                    assert_eq!(outcome.count_or_zero(), 0);
                    assert!(outcome.events.is_empty());
                }
                assert_eq!(
                    game.counter_count(source, CounterType::PlusOnePlusOne),
                    1,
                    "operation {operation}"
                );
                assert_eq!(
                    game.counter_count(original, CounterType::PlusOnePlusOne),
                    1,
                    "operation {operation}"
                );
                assert_eq!(game.player(alice).unwrap().life, 20);
                if let Some(card) = library_card {
                    let player = game.player(alice).unwrap();
                    assert!(player.library.contains(&card));
                    assert!(player.hand.is_empty());
                    assert!(player.graveyard.is_empty());
                }

                assert!(
                    game.effect_store
                        .replacement_effects
                        .get_effect(one_shot)
                        .is_some()
                );
                assert!(game.take_pending_trigger_events().is_empty());
                if operation == 6 {
                    assert!(!game.is_renowned(source));
                }
                if pending_case {
                    assert_eq!(dm.calls, 1);
                    let mut dm = CounterPayloadDecisions {
                        recipient: original,
                        check_unrenowned: (operation == 6).then_some(source),
                        pause: false,
                        pending: false,
                        calls: 0,
                    };
                    let mut ctx = ExecutionContext::new(source, alice, &mut dm);
                    ctx.targets = targets;
                    if operation == 7 {
                        ctx.triggering_event =
                            Some(crate::triggers::TriggerEvent::new_with_provenance(
                                crate::events::EnterBattlefieldEvent::new(entered, Zone::Hand),
                                crate::provenance::ProvNodeId::default(),
                            ));
                    }
                    let replay = instruction.execute(&mut game, &mut ctx).unwrap();
                    assert_eq!(dm.calls, 2);
                    assert_eq!(game.player(alice).unwrap().life, 26);
                    if library_card.is_some() {
                        let player = game.player(alice).unwrap();
                        assert!(player.library.is_empty());
                        assert!(player.hand.is_empty());
                        assert_eq!(player.graveyard.len(), 1);
                    }

                    assert_eq!(game.counter_count(original, CounterType::PlusOnePlusOne), 1);
                    assert_eq!(
                        game.counter_count(source, CounterType::PlusOnePlusOne),
                        if (3..=5).contains(&operation) { 0 } else { 1 }
                    );
                    assert_eq!(
                        replay
                            .events
                            .iter()
                            .filter(|event| event.kind() == crate::events::EventKind::LifeGain)
                            .count(),
                        3
                    );
                    assert_eq!(
                        replay
                            .events
                            .iter()
                            .filter_map(
                                |event| event.downcast::<crate::events::MarkersChangedEvent>()
                            )
                            .filter(|marker| marker.is_added())
                            .count(),
                        0
                    );
                    assert!(
                        game.effect_store
                            .replacement_effects
                            .get_effect(one_shot)
                            .is_none()
                    );
                    let queued = game
                        .turn_store
                        .turn_history
                        .projected_records()
                        .map(|record| record.event.clone())
                        .collect::<Vec<_>>();
                    if operation == 9 {
                        let mut observed = std::collections::HashSet::new();
                        let actual_events: Vec<_> = replay
                            .events
                            .iter()
                            .chain(queued.iter())
                            .filter(|event| observed.insert(event.occurrence_key()))
                            .collect();
                        assert_eq!(
                            actual_events
                                .iter()
                                .filter_map(
                                    |event| event.downcast::<crate::events::CardsDrawnEvent>()
                                )
                                .map(|draw| draw.cards.len())
                                .sum::<usize>(),
                            1
                        );
                        assert_eq!(
                            actual_events
                                .iter()
                                .filter(
                                    |event| event.kind() == crate::events::EventKind::CardDiscarded
                                )
                                .count(),
                            1
                        );
                        for (from, to) in
                            [(Zone::Library, Zone::Hand), (Zone::Hand, Zone::Graveyard)]
                        {
                            assert_eq!(
                                actual_events
                                    .iter()
                                    .filter_map(
                                        |event| event.downcast::<crate::events::ZoneChangeEvent>()
                                    )
                                    .filter(|change| change.from == from && change.to == to)
                                    .count(),
                                1
                            );
                        }
                        assert_eq!(
                            actual_events
                                .iter()
                                .filter(|event| event.kind() == crate::events::EventKind::LifeGain)
                                .count(),
                            3
                        );
                        assert!(!actual_events.iter().any(|event| {
                            event
                                .downcast::<crate::events::MarkersChangedEvent>()
                                .is_some_and(|marker| marker.is_added())
                        }));
                    } else {
                        assert_eq!(
                            queued
                                .iter()
                                .filter(|event| event.kind() == crate::events::EventKind::LifeGain)
                                .count(),
                            3
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_put_counters_emits_affected_result_memory() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let target = create_creature_on_battlefield(&mut game, "Bear", alice);

        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.targets = vec![crate::effects::ResolvedTarget::Object(target)];

        let effect = PutCountersEffect::plus_one_counters(2, ChooseSpec::target_creature());
        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("effect should resolve");

        assert_eq!(result.affected_objects(), Some([target].as_slice()));
        let memory = result
            .affected_object_memory()
            .expect("counter target memory should be recorded");
        assert_eq!(memory.len(), 1);
        assert_eq!(memory[0].object_id, target);
        assert_eq!(game.counter_count(target, CounterType::PlusOnePlusOne), 2);
        assert_eq!(result.events.len(), 1);
    }

    #[test]
    fn put_up_to_counters_uses_chosen_amount() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let target = create_creature_on_battlefield(&mut game, "Saga Token", alice);

        let mut dm = crate::decision::NumericInputDecisionMaker::from_strs(&["1"]);
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);
        ctx.targets = vec![crate::effects::ResolvedTarget::Object(target)];

        let effect = PutCountersEffect::new(
            CounterType::Lore,
            Value::Fixed(3).with_surface_hint(ironsmith_core::ValueSurfaceHint::UpTo),
            ChooseSpec::target_creature(),
        );
        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("up-to counter amount should resolve");

        assert_eq!(result.as_count(), Some(1));
        assert_eq!(game.counter_count(target, CounterType::Lore), 1);
    }

    #[test]
    fn chosen_object_counter_cost_is_legal_and_does_not_collapse_to_source() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source_card = CardBuilder::new(CardId::from_raw(700), "Hatchet")
            .card_types(vec![CardType::Artifact])
            .build();
        let source = game.create_object_from_card(&source_card, alice, Zone::Battlefield);
        let target = create_creature_on_battlefield(&mut game, "Cost Bear", alice);
        let effect = PutCountersEffect::new(
            CounterType::MinusOneMinusOne,
            1,
            ChooseSpec::Object(
                crate::filter::ObjectFilter::creature()
                    .you_control()
                    .in_zone(Zone::Battlefield),
            ),
        );

        assert!(
            CostExecutableEffect::can_execute_as_cost(&effect, &game, source, alice).is_ok(),
            "the chosen creature, not the source, should make the cost legal"
        );
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);
        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("chosen counter cost should execute");

        assert_eq!(outcome.as_count(), Some(1));
        assert_eq!(game.counter_count(target, CounterType::MinusOneMinusOne), 1);
        assert_eq!(game.counter_count(source, CounterType::MinusOneMinusOne), 0);
    }
}

#[cfg(test)]
mod prepared_ceiling_tests {
    use super::*;
    use crate::ids::{CardId, PlayerId};
    use crate::object::CounterType;
    use crate::replacement::{EventModification, ReplacementAction, ReplacementEffect};
    use crate::zone::Zone;

    #[test]
    fn prepared_and_direct_placements_retain_each_original_recipient_ceiling() {
        for (initial_first, initial_second, doubled, one_shot, expected_first, expected_second, expected_total) in [
            (1, 2, true, false, 4, 4, 5),
            (0, 4, false, false, 3, 4, 3),
            (1, 3, false, false, 4, 4, 4),
            (4, 0, true, true, 4, 4, 4),
        ] {
        for prepared in [false, true] {
            let mut game = crate::tests::test_helpers::setup_two_player_game();
            let alice = PlayerId::from_index(0);
            let card = crate::card::CardBuilder::new(CardId::new(), "Ceiling recipient")
                .card_types(vec![crate::types::CardType::Creature])
                .power_toughness(crate::card::PowerToughness::fixed(2, 2))
                .build();
            let first = game.create_object_from_card(&card, alice, Zone::Battlefield);
            let second = game.create_object_from_card(&card, alice, Zone::Battlefield);
            game.object_mut(first).unwrap().counters.insert(CounterType::Charge, initial_first);
            game.object_mut(second).unwrap().counters.insert(CounterType::Charge, initial_second);
            if doubled {
                let replacement = ReplacementEffect::with_matcher(
                    first, alice,
                    crate::events::counters::matchers::WouldPutCountersMatcher::any(),
                    ReplacementAction::Modify(EventModification::Multiply(2)),
                );
                if one_shot {
                    game.effect_store.replacement_effects.add_one_shot_effect(replacement);
                } else {
                    game.effect_store.replacement_effects.add_resolution_effect(replacement);
                }
            }
            let mut effect = PutCountersEffect::new(
                CounterType::Charge,
                3,
                ChooseSpec::All(crate::filter::ObjectFilter::creature()),
            );
            effect.maximum_total = Some(4);
            let mut ctx = ExecutionContext::new_default(first, alice);
            let result = if prepared {
                effect.prepare_simultaneous_player_action(&game, &mut ctx).unwrap()
                    .commit(&mut game, &mut ctx).unwrap()
            } else {
                effect.execute(&mut game, &mut ctx).unwrap()
            };
            assert_eq!(game.counter_count(first, CounterType::Charge), expected_first);
            assert_eq!(game.counter_count(second, CounterType::Charge), expected_second);
            assert_eq!(result.count_or_zero(), expected_total);
            let counters = result.events_of_type::<crate::events::MarkersChangedEvent>().collect::<Vec<_>>();
            assert_eq!(counters.len(), usize::from(initial_first < 4) + usize::from(initial_second < 4));
        }
        }
    }
}

#[cfg(test)]
mod retained_counter_draw_boundary_tests {
    use super::*;
    use crate::ids::{CardId, PlayerId};
    use crate::object::CounterType;
    use crate::replacement::{ReplacementAction, ReplacementEffect};
    use crate::zone::Zone;

    #[test]
    fn native_counter_owner_retains_replacement_draw_after_all_original_placements() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let card = crate::card::CardBuilder::new(CardId::new(), "Counter draw witness")
            .card_types(vec![crate::types::CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2)).build();
        let first = game.create_object_from_card(&card, alice, Zone::Battlefield);
        let second = game.create_object_from_card(&card, alice, Zone::Battlefield);
        game.create_object_from_card(&card, alice, Zone::Library);
        game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
            first, alice, crate::events::counters::matchers::WouldPutCountersMatcher::any(),
            ReplacementAction::Additionally(vec![crate::effect::Effect::gain_life(2),
                crate::effect::Effect::draw(1), crate::effect::Effect::gain_life(4)]),
        ));
        let effect = PutCountersEffect::new(CounterType::Charge, 1,
            ChooseSpec::All(crate::filter::ObjectFilter::creature().in_zone(Zone::Battlefield)));
        let mut ctx = ExecutionContext::new_default(first, alice);
        let boundary = effect.prepare_replacement_draw_continuation_with_outputs(&mut game, &mut ctx).unwrap();
        assert!(boundary.completion.is_some());
        assert_eq!(game.counter_count(first, CounterType::Charge), 1);
        assert_eq!(game.counter_count(second, CounterType::Charge), 1);
        assert_eq!(game.player(alice).unwrap().life, 22);
        assert!(game.player(alice).unwrap().hand.is_empty());
        let completed = crate::effects::composition::complete_committed_original_with_outputs(
            &mut game, &mut ctx, boundary,
        ).unwrap();
        assert_eq!(game.player(alice).unwrap().life, 26);
        assert_eq!(game.player(alice).unwrap().hand.len(), 1);
        assert_eq!(completed.outcome.count_or_zero(), 2);
        assert_eq!(completed.outcome.events_of_type::<crate::events::MarkersChangedEvent>().count(), 2);
        assert_eq!(completed.outcome.events_of_type::<crate::events::CardsDrawnEvent>().count(), 1);
    }
}
