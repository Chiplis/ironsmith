//! Freeze one completed permanent-change operation before any later instruction.
use super::{
    KeywordActionEvent, KeywordActionKind, MutatedEvent, TransformedEvent, TurnedFaceUpEvent,
};
use crate::effects::ExecutionError;
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
use crate::triggers::TriggerEvent;

pub(crate) fn freeze_completed_lifecycle_events(
    game: &GameState,
    events: &mut [TriggerEvent],
) -> Result<(), ExecutionError> {
    let pending = |event: &TriggerEvent| {
        event
            .downcast::<TransformedEvent>()
            .is_some_and(|event| event.snapshot.is_none())
            || event
                .downcast::<MutatedEvent>()
                .is_some_and(|event| event.snapshot.is_none())
            || event
                .downcast::<TurnedFaceUpEvent>()
                .is_some_and(|event| event.snapshot.is_none())
            || event.downcast::<KeywordActionEvent>().is_some_and(|event| {
                event.action == KeywordActionKind::Renown && event.snapshot.is_none()
            })
    };
    if !events.iter().any(pending) {
        return Ok(());
    }
    let observed = game
        .continuous_query_snapshot()
        .map_err(ExecutionError::ContinuousDiscovery)?;
    let effects = observed
        .try_all_continuous_effects_arc()
        .map_err(ExecutionError::ContinuousDiscovery)?;
    for event in events.iter_mut().filter(|event| pending(event)) {
        let snapshot = event
            .object_id()
            .and_then(|id| observed.object(id))
            .filter(|object| object.zone == crate::zone::Zone::Battlefield)
            .map(|object| {
                ObjectSnapshot::from_object_with_calculated_characteristics_and_effects(
                    object, &observed, &effects,
                )
            });
        if let Some(inner) = event.downcast::<TransformedEvent>() {
            *event = event.with_inner_event(inner.clone().with_snapshot(snapshot));
        } else if let Some(inner) = event.downcast::<MutatedEvent>() {
            *event = event.with_inner_event(inner.clone().with_snapshot(snapshot));
        } else if let Some(inner) = event.downcast::<TurnedFaceUpEvent>() {
            *event = event.with_inner_event(inner.clone().with_snapshot(snapshot));
        } else if let Some(inner) = event.downcast::<KeywordActionEvent>() {
            *event = event.with_inner_event(inner.clone().with_snapshot(snapshot));
        }
    }
    Ok(())
}
