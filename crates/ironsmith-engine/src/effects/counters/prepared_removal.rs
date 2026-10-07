//! Stage removal replacements without running their appended programs before
//! sibling originals. Shared by standalone removals and counter transfers.
use crate::effect::EffectOutcome;
use crate::effects::{ExecutionContext, ExecutionError, ResolvedTarget,
    SimultaneousEffectCommit, SimultaneousEffectCompletion};
use crate::events::processing::{PreparedReplacementProgram, TraitEventResult,
    process_trait_event_with_execution_context};
use crate::events::{Event, RemoveCountersEvent, downcast_event};
use crate::game_state::GameState;

pub(super) struct PreparedCounterRemoval {
    result: TraitEventResult,
    before: GameState,
}

pub(super) fn prepare_counter_removal(game: &mut GameState, ctx: &mut ExecutionContext,
    event: Event) -> Result<PreparedCounterRemoval, ExecutionError> {
    let before = game.clone();
    let removal = downcast_event::<RemoveCountersEvent>(event.inner()).ok_or_else(||
        ExecutionError::InternalError("counter-removal owner requires a removal event".into()))?;
    let count = removal.count.min(game.counter_count(removal.target, removal.counter_type));
    if game.object(removal.target).is_none() || game.is_phased_out(removal.target) || count == 0 {
        return Ok(PreparedCounterRemoval { result: TraitEventResult::Prevented, before });
    }
    let parent = event.provenance();
    let proposal = if game.provenance_graph().node(parent).is_some() {
        game.alloc_child_event_provenance(parent, crate::events::EventKind::RemoveCounters)
    } else {
        game.provenance_graph_mut().alloc_root_event(crate::events::EventKind::RemoveCounters)
    };
    let event = event.rewrap(removal.with_count(count)).with_provenance(proposal);
    let result = process_trait_event_with_execution_context(game, event, ctx)?;
    Ok(PreparedCounterRemoval { result, before })
}

fn targets(context: &crate::events::processing::ReplacementEventContext)
    -> Result<Option<Vec<ResolvedTarget>>, ExecutionError> {
    let removal = downcast_event::<RemoveCountersEvent>(context.event.inner()).ok_or_else(||
        ExecutionError::InternalError("counter-removal completion lost its event".into()))?;
    Ok(Some(vec![ResolvedTarget::Object(removal.target)]))
}

struct CounterRemovalCompletion {
    original: Option<Box<dyn SimultaneousEffectCompletion>>,
    programs: Vec<PreparedReplacementProgram>,
}
impl SimultaneousEffectCompletion for CounterRemovalCompletion {
    fn freeze(&mut self, game: &mut GameState) -> Result<(), ExecutionError> {
        if let Some(original) = &mut self.original { original.freeze(game)?; }
        Ok(())
    }
    fn complete(self: Box<Self>, game: &mut GameState, ctx: &mut ExecutionContext,
        outcome: EffectOutcome) -> Result<EffectOutcome, ExecutionError> {
        let outcome = if let Some(original) = self.original {
            original.complete(game, ctx, outcome)?
        } else { outcome };
        if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
        crate::effects::replacement::execute_deferred_replacement_programs_with_targets(
            game, ctx, outcome, self.programs, |_, context, _| targets(context))
    }
}

pub(super) fn commit_prepared_counter_removal_original(game: &mut GameState,
    ctx: &mut ExecutionContext, prepared: PreparedCounterRemoval)
    -> Result<SimultaneousEffectCommit, ExecutionError> {
    let (original, programs) = prepared.result.into_expansion();
    let snapshot = if let TraitEventResult::Replaced { source, .. } = &original {
        let before = prepared.before.continuous_query_snapshot()
            .map_err(ExecutionError::ContinuousDiscovery)?;
        before.object(*source).map(|object|
            crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(object, &before))
    } else { None };
    let deferred = if let TraitEventResult::Replaced { effects, source, controller, context, .. } = &original {
        crate::effects::replacement::prepare_draw_continuation_with_bindings(game, ctx,
            effects, *source, *controller, context, snapshot.clone(),
            crate::effects::replacement::ReplacementProgramBindings {
                targets: targets(context)?, object_tags: Vec::new(),
            })?
    } else { None };
    let (outcome, continuation) = if let Some(receipt) = deferred {
        (receipt.outcome, receipt.completion)
    } else {
        let outcome = if let TraitEventResult::Replaced { effects, source, controller, context, .. } = &original {
            let payload = crate::effects::replacement::execute_replacement_payload_with_snapshot(
                game, ctx, effects, *source, *controller, context, targets(context)?, snapshot, Vec::new())?;
            let mut receipt = EffectOutcome::replaced();
            receipt.set_value(crate::effect::OutcomeValue::Count(0));
            EffectOutcome::aggregate_replacement_outcomes(receipt, [payload])
        } else {
            super::remove_counters::commit_counter_removal(game, ctx, original)?
        };
        (outcome, None)
    };
    Ok(SimultaneousEffectCommit { outcome,
        completion: if programs.is_empty() && continuation.is_none() { None } else {
            Some(Box::new(CounterRemovalCompletion { original: continuation, programs }))
        },
    })
}
