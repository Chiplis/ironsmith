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
                    // An exile-until duration ends on the actual completed
                    // departure even when this owner publishes it directly.
                    // Queue its one-shot return; the normal duration owner
                    // still performs entry choices after prior events match.
                    let departed = events.iter().filter_map(|event|
                        event.downcast::<crate::events::ZoneChangeEvent>())
                        .filter(|change| matches!(change.from, crate::zone::Zone::Battlefield | crate::zone::Zone::Stack)
                            && change.to != crate::zone::Zone::Battlefield)
                        .flat_map(|change|change.original_objects()).collect::<Vec<_>>();
                    if !departed.is_empty() {game.return_exiled_for_sources_leave(&departed);}
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
    let outputs = observe_lifecycle_completions_with_outputs(game, events.clone())?;
    *events = outputs.outcome.events;
    Ok(())
}

/// The lifecycle producer retains its actual frozen occurrences as one rich
/// completion packet. Compatibility callers project only after successful
/// observation, preserving their original input list on errors.
pub(crate) fn observe_lifecycle_completions_with_outputs(
    game: &mut GameState,
    events: Vec<TriggerEvent>,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    let completions = events
        .into_iter()
        .map(|event| {
            let parent = event.provenance();
            (event, Some(parent))
        })
        .collect();
    let events = observe_action_completions(game, completions)?;
    Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
        crate::effect::EffectOutcome::resolved().with_events(events),
    ))
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

#[cfg(test)]
mod completed_departure_duration_tests {
    use super::*;
    use crate::{CardId,CardType,PlayerId,Zone};
    fn object(game:&mut GameState,name:&str,owner:PlayerId,zone:Zone)->crate::ObjectId {
        let card=crate::card::CardBuilder::new(CardId::new(),name).card_types(vec![CardType::Artifact]).build();
        game.create_object_from_card(&card,owner,zone)
    }
    #[test]
    fn directly_completed_source_departure_ends_duration_without_pending_event_republication() {
        let a=PlayerId(0);let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        let source=object(&mut game,"Independent duration source",a,Zone::Stack);
        let exiled=object(&mut game,"Independent duration member",a,Zone::Exile);
        let stable=game.object(exiled).unwrap().stable_id;
        game.add_exiled_with_source_link_returning_to(source,exiled,Zone::Battlefield);
        game.mark_return_exiled_when_source_leaves(source);
        game.take_pending_trigger_events();
        game.move_object_by_effect(source,Zone::Graveyard).unwrap();
        let events=game.take_pending_trigger_events();
        assert!(events.iter().any(|event|event.downcast::<crate::events::ZoneChangeEvent>().is_some_and(|event|event.original_objects().contains(&source))));
        observe_lifecycle_completions_with_outputs(&mut game,events).unwrap();
        assert!(game.take_pending_trigger_events().is_empty(),"completion does not republish the physical departure");
        assert!(game.has_pending_duration_end_returns());
        game.process_pending_duration_end_returns(&mut crate::decision::SelectFirstDecisionMaker).unwrap();
        let returned=game.find_object_by_stable_id(stable).unwrap();assert_ne!(returned,exiled);assert_eq!(game.object(returned).unwrap().zone,Zone::Battlefield);
    }
    #[test]
    fn published_return_child_is_visible_inside_exact_action_observation_scope() {
        let a=PlayerId(0);let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        let source=object(&mut game,"Independent return source",a,Zone::Battlefield);
        let old=object(&mut game,"Independent returned member",a,Zone::Graveyard);
        let mut ctx=crate::effects::ExecutionContext::new_default(source,a);
        let (_,observations)=with_action_observations(&mut game,|game|crate::effects::execute_effect(game,
            &crate::Effect::new(crate::effects::ReturnFromGraveyardToBattlefieldEffect::new(crate::ChooseSpec::SpecificObject(old),false)),&mut ctx)).unwrap();
        let movement=observations.iter().find_map(|event|event.downcast::<crate::events::ZoneChangeEvent>()
            .filter(|movement|movement.original_objects().contains(&old)&&movement.to==Zone::Battlefield)).expect("completed return retains the exact original target");
        assert!(!movement.objects.contains(&old),"the real publisher names the destination, not the old graveyard object");
        assert!(!movement.result_objects.is_empty());assert!(movement.result_objects.iter().all(|id|game.object(*id).is_some_and(|object|object.zone==Zone::Battlefield)));
    }
}
