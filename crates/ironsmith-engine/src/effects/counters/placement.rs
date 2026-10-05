//! One composable counter placement request for objects and players.

use crate::effect::EffectOutcome;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::events::{Event, PutCountersEvent};
use crate::game_state::{GameState, Target};

#[derive(Debug, Clone)]
struct CounterPlacement(Event);

impl EffectExecutor for CounterPlacement {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let recipient = crate::events::downcast_event::<PutCountersEvent>(self.0.inner())
            .ok_or_else(|| {
                ExecutionError::InternalError("counter request has no placement event".into())
            })?
            .target;
        match recipient {
            Target::Object(_) => super::execute_object_counter_placement(game, ctx, self.0.clone()),
            Target::Player(_) => super::execute_player_counter_placement(game, ctx, self.0.clone()),
        }
    }
}

pub(crate) fn execute_counter_placement(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    event: Event,
) -> Result<EffectOutcome, ExecutionError> {
    CounterPlacement(event).execute_child(game, ctx)
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
pub(crate) fn execute_counter_batch(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    events: Vec<Event>,
) -> Result<Vec<EffectOutcome>, ExecutionError> {
    let checkpoint = game.clone();
    let context = crate::effects::ExecutionContextCheckpoint::capture(ctx);
    let result = (|| {
        let mut prepared = Vec::with_capacity(events.len());
        for event in events {
            prepared.push(super::prepare_counter_placement(game, ctx, event)?);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(Vec::new());
            }
        }
        let opened = game.open_simultaneous_action();
        let mut originals = Vec::with_capacity(prepared.len());
        let mut batch = None;
        for request in prepared {
            let mut original = super::commit_prepared_counter_original(game, ctx, request)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(Vec::new());
            }
            group_counter_placement_events(game, ctx, &mut original.outcome.events, &mut batch);
            originals.push(original);
        }
        game.close_simultaneous_action(opened);
        for original in &mut originals {
            crate::effects::outcome_recording::complete_outcome(
                game,
                None,
                Some(ctx.controller),
                &mut original.outcome,
                Vec::new(),
            );
            if let Some(completion) = &mut original.completion {
                completion.freeze(game)?;
            }
        }
        let mut outcomes = Vec::with_capacity(originals.len());
        for original in originals {
            let outcome = match original.completion {
                Some(completion) => completion.complete(game, ctx, original.outcome)?,
                None => original.outcome,
            };
            if ctx.decision_maker.awaiting_choice() {
                return Ok(Vec::new());
            }
            outcomes.push(outcome);
        }
        Ok(outcomes)
    })();
    if result.is_err() || ctx.decision_maker.awaiting_choice() {
        game.restore_execution_checkpoint(
            checkpoint,
            result.is_ok() && ctx.decision_maker.awaiting_choice(),
        );
        context.restore(ctx);
    }
    result
}

#[derive(Debug, Clone)]
struct CounterRemoval(Event);
impl EffectExecutor for CounterRemoval {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        super::remove_counters::execute_counter_removal_event(game, ctx, self.0.clone())
    }
}
pub(crate) fn execute_counter_removal(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    event: Event,
) -> Result<EffectOutcome, ExecutionError> {
    CounterRemoval(event).execute_child(game, ctx)
}
