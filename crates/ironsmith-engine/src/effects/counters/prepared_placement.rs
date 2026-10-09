//! Staged counter consequences for a simultaneous damage result operation.
//! Selection is separated from original commitment and appended programs.
use crate::effect::EffectOutcome;
use crate::effects::{ExecutionContext, ExecutionError, ResolvedTarget, SimultaneousEffectCommit};
use crate::events::processing::{TraitEventResult, process_trait_event_with_execution_context};
use crate::events::{Event, PutCountersEvent, downcast_event};
use crate::game_state::{GameState, Target};

/// Compose placements from captured participant contexts. Every proposal is
/// prepared before placement originals, and every original freezes before any
/// additions. Requesting instructions retain the complete child outcomes.
pub(crate) fn execute_scoped_counter_placements(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    programs: Vec<(crate::effects::ExecutionContextCheckpoint, Event)>,
    observe: impl FnOnce(&mut GameState, &mut ExecutionContext) -> Result<(), ExecutionError>,
) -> Result<EffectOutcome, ExecutionError> {
    execute_scoped_counter_placements_with_outputs(game, ctx, programs, observe)
        .map(crate::effects::CompletedEffectOutputs::into_outcome)
}

pub(crate) fn execute_scoped_counter_placements_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    programs: Vec<(crate::effects::ExecutionContextCheckpoint, Event)>,
    observe: impl FnOnce(&mut GameState, &mut ExecutionContext) -> Result<(), ExecutionError>,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    if ctx.decision_maker.awaiting_choice() {
        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
            EffectOutcome::count(0),
        ));
    }
    if programs.is_empty() {
        observe(game, ctx)?;
        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
            EffectOutcome::count(0),
        ));
    }
    crate::effects::composition::execute_transaction(
        game,
        ctx,
        || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
        |game, ctx| {
            let parent = crate::effects::ExecutionContextCheckpoint::capture(ctx);
            let result = (|| {
                let mut prepared = Vec::with_capacity(programs.len());
                for (context, event) in programs {
                    context.restore_ref(ctx);
                    let proposal = prepare_counter_placement(game, ctx, event)?;
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::count(0),
                        ));
                    }
                    prepared.push((context, proposal));
                }
                parent.restore_ref(ctx);
                let outcomes =
                    crate::effects::composition::execute_simultaneous_originals_with_outputs(
                        game,
                        ctx,
                        prepared.len() > 1,
                        |game, ctx| {
                            let mut receipts = Vec::with_capacity(prepared.len());
                            for (context, proposal) in prepared {
                                context.restore_ref(ctx);
                                let receipt = commit_prepared_counter_original_with_outputs(
                                    game, ctx, proposal,
                                )?;
                                if ctx.decision_maker.awaiting_choice() {
                                    return Ok(Vec::new());
                                }
                                receipts.push(
                                    crate::effects::composition::with_original_execution_context(
                                        receipt, ctx,
                                    ),
                                );
                            }
                            parent.restore_ref(ctx);
                            Ok(receipts)
                        },
                        |game, ctx, receipts| {
                            observe(game, ctx)?;
                            game.observe_prepared_life_payment_originals(
                                ctx,
                                receipts
                                    .iter_mut()
                                    .flat_map(|receipt| receipt.outcome.outcome.events.iter_mut()),
                            )
                        },
                    )?;
                let aggregate = EffectOutcome::aggregate(
                    outcomes.iter().map(|outputs| outputs.outcome.clone()),
                );
                let mut outputs = crate::effects::CompletedEffectOutputs::aggregate_only(aggregate);
                outputs.retain_batch_children(outcomes);
                Ok(outputs)
            })();
            parent.restore(ctx);
            result
        },
    )
}

/// Prepare all scoped counter requests and commit their originals without
/// freezing, observing, or completing any added program. The enclosing action
/// owns grouping and rollback, and can compose zone bindings alongside this
/// retained packet before freezing the complete simultaneous original world.
pub(crate) fn commit_scoped_counter_placement_originals_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    programs: Vec<(crate::effects::ExecutionContextCheckpoint, Event)>,
) -> Result<SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>, ExecutionError> {
    let empty = || {
        SimultaneousEffectCommit::finished(crate::effects::CompletedEffectOutputs::aggregate_only(
            EffectOutcome::count(0),
        ))
    };
    if programs.is_empty() || ctx.decision_maker.awaiting_choice() {
        return Ok(empty());
    }
    let parent = crate::effects::ExecutionContextCheckpoint::capture(ctx);
    let result = (|| {
        let mut prepared = Vec::with_capacity(programs.len());
        for (context, event) in programs {
            context.restore_ref(ctx);
            let proposal = prepare_counter_placement(game, ctx, event)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(empty());
            }
            prepared.push((context, proposal));
        }
        let mut receipts = Vec::with_capacity(prepared.len());
        for (context, proposal) in prepared {
            context.restore_ref(ctx);
            let receipt = commit_prepared_counter_original_with_outputs(game, ctx, proposal)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(empty());
            }
            receipts
                .push(crate::effects::composition::with_original_execution_context(receipt, ctx));
        }
        Ok(crate::effects::composition::compose_original_commits_with_outputs(receipts))
    })();
    parent.restore(ctx);
    result
}

pub(crate) struct PreparedCounterPlacement {
    result: TraitEventResult,
    original_is_object: bool,
    limit: Option<(crate::ids::ObjectId, crate::object::CounterType, u32)>,
    before: GameState,
}

impl PreparedCounterPlacement {
    pub(crate) fn requires_replacement_input(&self) -> bool {
        self.result.requires_replacement_input()
    }
}

pub(crate) fn prepare_counter_placement(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    event: Event,
) -> Result<PreparedCounterPlacement, ExecutionError> {
    prepare_counter_placement_with_limit(game, ctx, event, None)
}

pub(super) fn prepare_counter_placement_with_limit(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    event: Event,
    maximum_total: Option<u32>,
) -> Result<PreparedCounterPlacement, ExecutionError> {
    let before = game.clone();
    let counter = downcast_event::<PutCountersEvent>(event.inner()).ok_or_else(|| {
        ExecutionError::InternalError("counter preparation requires a counter event".into())
    })?;
    let original_is_object = matches!(counter.target, Target::Object(_));
    let limit = match counter.target {
        Target::Object(object) => {
            maximum_total.map(|maximum| (object, counter.counter_type, maximum))
        }
        Target::Player(_) => None,
    };
    let allowed = match counter.target {
        Target::Object(object) => game.can_have_counter_type_placed(object, counter.counter_type),
        Target::Player(_) => {
            super::player_counter_placement::player_counter_request_outcome(game, &event)?.is_none()
        }
    };
    if counter.count == 0 || !allowed {
        return Ok(PreparedCounterPlacement {
            result: TraitEventResult::Prevented,
            original_is_object,
            limit,
            before,
        });
    }
    let result = match counter.target {
        Target::Object(object) => {
            super::object_counter_placement::prepare_object_counter_replacement(
                game, ctx, event, object,
            )?
        }
        Target::Player(_) => process_trait_event_with_execution_context(game, event, ctx)?,
    };
    Ok(PreparedCounterPlacement {
        result,
        original_is_object,
        limit,
        before,
    })
}

fn target(
    context: &crate::events::processing::ReplacementEventContext,
) -> Result<Option<Vec<ResolvedTarget>>, ExecutionError> {
    let counter = downcast_event::<PutCountersEvent>(context.event.inner()).ok_or_else(|| {
        ExecutionError::InternalError("counter completion lost its captured event".into())
    })?;
    Ok(Some(vec![match counter.target {
        Target::Object(id) => ResolvedTarget::Object(id),
        Target::Player(id) => ResolvedTarget::Player(id),
    }]))
}

/// Execute the selected counter replacement original once, preserving the
/// caller's recipient binding and captured source frame. Counter additions
/// remain the enclosing expansion's responsibility.
#[allow(clippy::too_many_arguments)]
pub(super) fn execute_counter_replacement_original_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    effects: &[crate::effect::Effect],
    source: crate::ids::ObjectId,
    controller: crate::ids::PlayerId,
    context: &crate::events::processing::ReplacementEventContext,
    targets: Option<Vec<ResolvedTarget>>,
    source_snapshot: Option<crate::snapshot::ObjectSnapshot>,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    let mut original = EffectOutcome::replaced();
    original.set_value(crate::effect::OutcomeValue::Count(0));
    crate::effects::replacement::execute_replacement_original_payload_with_outputs(
        game,
        ctx,
        effects,
        source,
        controller,
        context,
        crate::effects::replacement::ReplacementProgramBindings {
            targets,
            object_tags: Vec::new(),
        },
        source_snapshot,
        original,
    )
}

pub(crate) fn commit_prepared_counter_original_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    prepared: PreparedCounterPlacement,
) -> Result<SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>, ExecutionError> {
    commit_counter_original_from_result_with_outputs(
        game,
        ctx,
        prepared.result,
        prepared.original_is_object,
        Some(prepared.before),
        prepared.limit,
    )
}

pub(super) fn commit_counter_original_from_result_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    result: TraitEventResult,
    original_is_object: bool,
    before: Option<GameState>,
    limit: Option<(crate::ids::ObjectId, crate::object::CounterType, u32)>,
) -> Result<SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>, ExecutionError> {
    let (original, programs) = result.into_expansion();
    let replacement_source_snapshot = if let TraitEventResult::Replaced { source, .. } = &original {
        let before = before
            .as_ref()
            .unwrap_or(game)
            .continuous_query_snapshot()
            .map_err(ExecutionError::ContinuousDiscovery)?;
        before.object(*source).map(|object| {
            crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                object, &before,
            )
        })
    } else {
        None
    };
    let receipt = match original {
        TraitEventResult::Replaced {
            effects,
            source,
            controller,
            context,
            ..
        } => {
            let bindings = crate::effects::replacement::ReplacementProgramBindings {
                targets: target(&context)?,
                object_tags: Vec::new(),
            };
            crate::effects::replacement::commit_bound_replacement_program_original_with_outputs(
                game,
                ctx,
                crate::events::processing::PreparedReplacementProgram {
                    effects,
                    source,
                    controller,
                    context,
                    source_snapshot: replacement_source_snapshot,
                },
                bindings,
            )?
        }
        original => {
            let outcome = if original_is_object {
                super::object_counter_placement::commit_object_counter_placement_with_limit_outputs(
                    game,
                    ctx,
                    original,
                    before.as_ref(),
                    limit,
                )?
            } else {
                super::player_counter_placement::commit_player_counter_placement_with_outputs(
                    game,
                    ctx,
                    original,
                    before.as_ref().ok_or_else(|| {
                        ExecutionError::InternalError(
                            "player counter original requires its captured before frame".into(),
                        )
                    })?,
                )?
            };
            SimultaneousEffectCommit::finished(outcome)
        }
    };
    let (outcome, continuation) = (receipt.outcome, receipt.completion);
    Ok(
        crate::effects::replacement::defer_replacement_programs_with_outputs(
            SimultaneousEffectCommit {
                outcome,
                completion: continuation,
            },
            programs,
            |context| {
                Ok(crate::effects::replacement::ReplacementProgramBindings {
                    targets: target(context)?,
                    object_tags: Vec::new(),
                })
            },
        ),
    )
}
