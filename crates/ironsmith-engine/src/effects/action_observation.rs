//! Metadata owner for one completed semantic action observation.

use crate::effects::ExecutionError;
use crate::game_state::GameState;
use crate::provenance::ProvNodeId;
use crate::triggers::TriggerEvent;

/// Retain the authored occurrence while allocating its own causal history row.
/// An instruction parent belongs to the proposal; repeated completions each
/// receive a child. Root actions use a root event without inventing a proposal.
/// Action-specific subject construction remains with each semantic owner.
pub(crate) fn observe_action_completion(
    game: &mut GameState,
    event: TriggerEvent,
    parent: Option<ProvNodeId>,
) -> Result<TriggerEvent, ExecutionError> {
    let mut events = observe_action_completions(game, vec![(event, parent)])?;
    Ok(events.remove(0))
}

/// Allocate and project the entire completed group before any characteristic
/// query. Snapshot-dependent history is then refreshed from the same frozen
/// world. Projection does not match triggers or run continuations.
pub(crate) fn observe_action_completions(
    game: &mut GameState,
    completions: Vec<(TriggerEvent, Option<ProvNodeId>)>,
) -> Result<Vec<TriggerEvent>, ExecutionError> {
    observe_action_completions_with_grouping(game, completions, true)
}

/// A retained receipt queue can contain several authored actions. Complete its
/// occurrences through the same owner without inventing a simultaneous group.
pub(crate) fn observe_action_completions_retaining_groups(
    game: &mut GameState,
    completions: Vec<(TriggerEvent, Option<ProvNodeId>)>,
) -> Result<Vec<TriggerEvent>, ExecutionError> {
    observe_action_completions_with_grouping(game, completions, false)
}

fn observe_action_completions_with_grouping(
    game: &mut GameState,
    completions: Vec<(TriggerEvent, Option<ProvNodeId>)>,
    group_fresh: bool,
) -> Result<Vec<TriggerEvent>, ExecutionError> {
    use std::collections::{HashMap, HashSet};

    // History retains completed receipts, so even a clone made before its
    // first completion can recover the original frozen occurrence from history.
    let requested = completions
        .iter()
        .map(|(event, _)| event.occurrence_key())
        .collect::<HashSet<_>>();
    let mut completed = game
        .retained_action_observations()
        .filter(|event| requested.contains(&event.occurrence_key()))
        .filter(|event| event.completed_action_provenance().is_some())
        .map(|event| (event.occurrence_key(), event.clone()))
        .collect::<HashMap<_, _>>();
    for (event, _) in &completions {
        if event.completed_action_provenance().is_some() {
            validate_completed_receipt(game, event)?;
            completed
                .entry(event.occurrence_key())
                .or_insert_with(|| event.clone());
        }
    }
    let mut fresh = Vec::<(TriggerEvent, Option<ProvNodeId>)>::new();
    let mut indices = HashMap::<usize, usize>::new();
    for (event, parent) in &completions {
        if let Some(receipt) = completed.get(&event.occurrence_key()) {
            validate_occurrence_alias(event, receipt)?;
            continue;
        }
        if let Some(&index) = indices.get(&event.occurrence_key()) {
            let (previous, _) = &mut fresh[index];
            validate_occurrence_alias(event, previous)?;
            // An explicit subject receipt on any alias belongs to this one
            // original occurrence. Do not refreeze a poorer alias separately.
            if previous.snapshot().is_none() && event.snapshot().is_some() {
                *previous = event.clone();
            }
        } else {
            indices.insert(event.occurrence_key(), fresh.len());
            fresh.push((event.clone(), *parent));
        }
    }
    if !fresh.is_empty() {
        // Completion errors must not leave projected rows or provenance behind.
        let result =
            crate::effects::composition::execute_world_checkpoint_transaction(game, |game| {
                let opened = group_fresh && fresh.len() > 1 && game.open_simultaneous_action();
                let result = (|| {
                    let mut events = Vec::with_capacity(fresh.len());
                    for (mut event, parent) in fresh {
                        let provenance = match parent {
                            Some(parent) => game.alloc_child_event_provenance(parent, event.kind()),
                            None => game.provenance_graph_mut().alloc_root_event(event.kind()),
                        };
                        event.set_provenance(provenance);
                        if event.simultaneous_batch().is_none()
                            && let Some(batch) = game.simultaneous_action_batch()
                        {
                            event = event.with_simultaneous_batch(batch);
                        }
                        events.push(event);
                    }
                    for event in &events {
                        game.stage_turn_history_event(event);
                    }
                    crate::events::other::freeze_completed_lifecycle_events(game, &mut events)?;
                    for event in &mut events {
                        event.mark_action_completed(
                            game.provenance_graph().node(event.provenance()).unwrap(),
                        );
                        game.stage_turn_history_event(event);
                    }
                    Ok::<_, ExecutionError>(events)
                })();
                game.close_simultaneous_action(opened);
                result
            });
        match result {
            Ok(events) => {
                for event in events {
                    completed.insert(event.occurrence_key(), event);
                }
            }
            Err(error) => {
                return Err(error);
            }
        }
    }
    // Preserve input order and aliases, but every alias shares the one frozen
    // payload, causal row and original simultaneous identity. Reuse neither
    // stages history nor opens a new action group.
    completions
        .into_iter()
        .map(|(event, _)| {
            let receipt = &completed[&event.occurrence_key()];
            Ok(event.with_completed_action_receipt(receipt))
        })
        .collect()
}

fn validate_occurrence_alias(
    event: &TriggerEvent,
    receipt: &TriggerEvent,
) -> Result<(), ExecutionError> {
    if !event.ptr_eq(receipt)
        || event.kind() != receipt.kind()
        || event.object_id() != receipt.object_id()
    {
        return Err(ExecutionError::InternalError(
            "completed action alias changed its occurrence or semantic subject".into(),
        ));
    }
    Ok(())
}

fn validate_completed_receipt(
    game: &GameState,
    event: &TriggerEvent,
) -> Result<(), ExecutionError> {
    let valid = event.completed_action_provenance() == Some(event.provenance())
        && game
            .provenance_graph()
            .node(event.provenance())
            .is_some_and(|node| {
                event.action_completion_matches_node(node)
                    && matches!(node.kind,
                crate::provenance::ProvenanceNodeKind::RootEvent { kind }
                | crate::provenance::ProvenanceNodeKind::DerivedEvent { kind }
                if kind == event.kind())
            })
        && !game
            .turn_store
            .turn_history
            .projected_records()
            .any(|record| {
                record.event.provenance() == event.provenance() && !record.event.ptr_eq(event)
            });
    if !valid {
        return Err(ExecutionError::InternalError(
            "completed action receipt does not belong to the current provenance history".into(),
        ));
    }
    Ok(())
}

/// Lifecycle adapters already authored their event and instruction parent.
/// They share occurrence allocation and completion projection, retaining the
/// original publication/transaction boundary and every event alias.
pub(crate) fn observe_lifecycle_completions(
    game: &mut GameState,
    events: &mut Vec<TriggerEvent>,
) -> Result<(), ExecutionError> {
    let completions = events
        .iter()
        .cloned()
        .map(|event| {
            let parent = event.provenance();
            (event, Some(parent))
        })
        .collect();
    *events = observe_action_completions(game, completions)?;
    Ok(())
}

/// Observe child receipts without changing their trigger publication or result
/// bindings. The body's action owner retains rollback and suspension policy.
/// Every active observer receives each occurrence once, including enrichment.
pub(crate) fn with_action_observations<T>(
    game: &mut GameState,
    body: impl FnOnce(&mut GameState) -> Result<T, ExecutionError>,
) -> Result<(T, Vec<TriggerEvent>), ExecutionError> {
    let depth = game.effect_store.action_observation_records.len();
    game.effect_store
        .action_observation_records
        .push(Vec::new());
    let result = body(game);
    if game.effect_store.action_observation_records.len() != depth + 1 {
        return Err(ExecutionError::InternalError(
            "action observation scope was not restored by its nested owner".into(),
        ));
    }
    let observations = game.effect_store.action_observation_records.pop().unwrap();
    result.map(|value| (value, observations))
}

/// Complete the exact original subjects after authored programs. A departure
/// receipt belongs to the old incarnation; a later return cannot replace it.
pub(crate) fn observe_lifecycle_completions_with_observations(
    game: &mut GameState,
    events: &mut Vec<TriggerEvent>,
    observations: &[TriggerEvent],
) -> Result<(), ExecutionError> {
    crate::events::other::retain_departed_lifecycle_snapshots(game, events, observations);
    observe_lifecycle_completions(game, events)
}
