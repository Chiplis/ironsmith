//! Staged counter consequences for a simultaneous damage result operation.
//! Selection is separated from original commitment and appended programs.
use crate::effect::EffectOutcome;
use crate::effects::{
    ExecutionContext, ExecutionError, ResolvedTarget, SimultaneousEffectCommit,
    SimultaneousEffectCompletion,
};
use crate::events::processing::{
    PreparedReplacementProgram, TraitEventResult, process_trait_event_with_execution_context,
};
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

pub(crate) struct PreparedCounterPlacement {
    result: TraitEventResult,
    original_is_object: bool,
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
    let before = game.clone();
    let counter = downcast_event::<PutCountersEvent>(event.inner()).ok_or_else(|| {
        ExecutionError::InternalError("counter preparation requires a counter event".into())
    })?;
    let original_is_object = matches!(counter.target, Target::Object(_));
    let allowed = match counter.target {
        Target::Object(object) => game.can_have_counter_type_placed(object, counter.counter_type),
        Target::Player(player) => {
            if game.player(player).is_none() {
                return Err(ExecutionError::PlayerNotFound(player));
            }
            !game
                .turn_store
                .turn_history
                .player_counter_is_locked_this_turn(player, counter.counter_type)
                && (counter.counter_type != crate::CounterType::Poison
                    || game.can_get_poison_counters(player))
        }
    };
    if counter.count == 0 || !allowed {
        return Ok(PreparedCounterPlacement {
            result: TraitEventResult::Prevented,
            original_is_object,
            before,
        });
    }
    let event = if original_is_object {
        let parent = event.provenance();
        let id = if game.provenance_graph().node(parent).is_some() {
            game.alloc_child_event_provenance(parent, crate::events::EventKind::PutCounters)
        } else {
            game.provenance_graph_mut()
                .alloc_root_event(crate::events::EventKind::PutCounters)
        };
        event.with_provenance(id)
    } else {
        event
    };
    let result=if original_is_object
        && downcast_event::<PutCountersEvent>(event.inner()).is_some_and(|counter|matches!(counter.target,Target::Object(object) if ctx.replacement.entry_counter_source==Some(object))) {
        TraitEventResult::Proceed(event)
    }else{process_trait_event_with_execution_context(game,event,ctx)?};
    Ok(PreparedCounterPlacement {
        result,
        original_is_object,
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
struct CounterPlacementCompletion {
    original: Option<Box<dyn SimultaneousEffectCompletion>>,
    programs: Vec<PreparedReplacementProgram>,
}
impl SimultaneousEffectCompletion for CounterPlacementCompletion {
    fn observe_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        outcome: &mut EffectOutcome,
    ) -> Result<(), ExecutionError> {
        if let Some(original) = &mut self.original {
            original.observe_original(game, ctx, outcome)?;
        }
        Ok(())
    }

    fn freeze(&mut self, game: &mut GameState) -> Result<(), ExecutionError> {
        if let Some(original) = &mut self.original {
            original.freeze(game)?;
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
        let outputs = if let Some(inner) = self.original {
            inner.complete_with_outputs(game, ctx, original)?
        } else {
            crate::effects::CompletedEffectOutputs::aggregate_only(original)
        };
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        let completed =
            crate::effects::replacement::complete_deferred_replacement_programs_with_targets(
                game,
                ctx,
                outputs.outcome.clone(),
                self.programs,
                |_, context, _| target(context),
            )?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        Ok(outputs.append_batch_program_outputs(completed))
    }
}

pub(crate) fn commit_prepared_counter_original_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    prepared: PreparedCounterPlacement,
) -> Result<SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>, ExecutionError> {
    let (original, programs) = prepared.result.into_expansion();
    let replacement_source_snapshot = if let TraitEventResult::Replaced { source, .. } = &original {
        let before = prepared
            .before
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
    let deferred = if let TraitEventResult::Replaced {
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
            replacement_source_snapshot.clone(),
            crate::effects::replacement::ReplacementProgramBindings {
                targets: target(context)?,
                object_tags: Vec::new(),
            },
        )?
    } else {
        None
    };
    let (outcome, continuation) = if let Some(receipt) = deferred {
        (receipt.outcome, receipt.completion)
    } else {
        let outcome = if let TraitEventResult::Replaced {
            effects,
            source,
            controller,
            context,
            ..
        } = &original
        {
            let payload = crate::effects::replacement::execute_replacement_payload_with_outputs(
                game,
                ctx,
                effects,
                *source,
                *controller,
                context,
                target(context)?,
                replacement_source_snapshot,
                Vec::new(),
            )?;
            let mut original = EffectOutcome::replaced();
            original.set_value(crate::effect::OutcomeValue::Count(0));
            {
                let aggregate = EffectOutcome::aggregate_replacement_outcomes(
                    original,
                    [payload.outcome.clone()],
                );
                payload.project_aggregate(aggregate)
            }
        } else if prepared.original_is_object {
            super::object_counter_placement::commit_object_counter_placement_with_frame_outputs(
                game,
                ctx,
                original,
                Some(&prepared.before),
            )?
        } else {
            super::player_counter_placement::commit_player_counter_placement_with_outputs(
                game,
                ctx,
                original,
                &prepared.before,
            )?
        };
        (outcome, None)
    };
    Ok(SimultaneousEffectCommit {
        outcome,
        completion: if programs.is_empty() && continuation.is_none() {
            None
        } else {
            Some(Box::new(CounterPlacementCompletion {
                original: continuation,
                programs,
            }))
        },
    })
}
