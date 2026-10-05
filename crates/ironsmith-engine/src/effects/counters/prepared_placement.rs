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

pub(crate) struct PreparedCounterPlacement {
    result: TraitEventResult,
    original_is_object: bool,
    before: GameState,
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
        outcome: EffectOutcome,
    ) -> Result<EffectOutcome, ExecutionError> {
        let outcome = if let Some(original) = self.original {
            original.complete(game, ctx, outcome)?
        } else {
            outcome
        };
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        crate::effects::replacement::execute_deferred_replacement_programs_with_targets(
            game,
            ctx,
            outcome,
            self.programs,
            |_, context, _| target(context),
        )
    }
}

pub(crate) fn commit_prepared_counter_original(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    prepared: PreparedCounterPlacement,
) -> Result<SimultaneousEffectCommit, ExecutionError> {
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
        crate::effects::replacement::prepare_draw_continuation_with_bindings(
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
            let payload = crate::effects::replacement::execute_replacement_payload_with_snapshot(
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
            EffectOutcome::aggregate_replacement_outcomes(original, [payload])
        } else if prepared.original_is_object {
            super::object_counter_placement::commit_object_counter_placement_with_frame(
                game,
                ctx,
                original,
                Some(&prepared.before),
            )?
        } else {
            super::player_counter_placement::commit_player_counter_placement(
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
