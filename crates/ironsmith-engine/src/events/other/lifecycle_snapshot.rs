//! Freeze one completed permanent-change operation before any later instruction.
use super::{
    BecameMonstrousEvent, ConvertedEvent, KeywordActionEvent, KeywordActionKind, LandPlayedEvent,
    MutatedEvent, TransformedEvent, TurnedFaceUpEvent,
};
use crate::effects::ExecutionError;
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
use crate::triggers::TriggerEvent;

pub(crate) fn freeze_completed_lifecycle_events(
    game: &GameState,
    events: &mut [TriggerEvent],
) -> Result<(), ExecutionError> {
    if !events.iter().any(needs_snapshot) {
        return Ok(());
    }
    let observed = game
        .continuous_query_snapshot()
        .map_err(ExecutionError::ContinuousDiscovery)?;
    let effects = observed
        .try_all_continuous_effects_arc()
        .map_err(ExecutionError::ContinuousDiscovery)?;
    for event in events.iter_mut().filter(|event| needs_snapshot(event)) {
        let snapshot = event
            .object_id()
            .and_then(|id| observed.object(id))
            .filter(|object| object.zone == crate::zone::Zone::Battlefield)
            .map(|object| {
                ObjectSnapshot::try_from_object_with_calculated_characteristics_and_effects(
                    object, &observed, &effects,
                )
            })
            .transpose()?;
        if let Some(inner) = event.downcast::<crate::events::EnterBattlefieldEvent>() {
            let mut entry = inner.clone();
            entry.completed_snapshot = snapshot;
            if entry.from == crate::zone::Zone::Stack {
                entry.emerge_sacrifice = observed
                    .object(entry.object)
                    .filter(|object| object.optional_costs_paid.was_paid_label("Emerge"))
                    .and_then(|object| {
                        object
                            .cast_tagged_objects
                            .get(crate::tag::SOURCE_EMERGE_SACRIFICE_TAG)
                    })
                    .filter(|receipts| receipts.len() <= 1)
                    .cloned();
            }
            *event = event.with_inner_event(entry);
        } else {
            attach_snapshot(event, snapshot);
        }
    }
    Ok(())
}

fn needs_snapshot(event: &TriggerEvent) -> bool {
    event
        .downcast::<crate::events::EnterBattlefieldEvent>()
        .is_some_and(|event| event.completed_snapshot.is_none())
        || event
            .downcast::<ConvertedEvent>()
            .is_some_and(|event| event.snapshot.is_none())
        || event
            .downcast::<LandPlayedEvent>()
            .is_some_and(|event| event.snapshot.is_none())
        || event
            .downcast::<TransformedEvent>()
            .is_some_and(|event| event.snapshot.is_none())
        || event
            .downcast::<MutatedEvent>()
            .is_some_and(|event| event.snapshot.is_none())
        || event
            .downcast::<TurnedFaceUpEvent>()
            .is_some_and(|event| event.snapshot.is_none())
        || event
            .downcast::<BecameMonstrousEvent>()
            .is_some_and(|event| event.snapshot.is_none())
        || event.downcast::<KeywordActionEvent>().is_some_and(|event| {
            matches!(
                event.action,
                KeywordActionKind::Renown | KeywordActionKind::GainClassLevel
            ) && event.snapshot.is_none()
        })
}

fn attach_snapshot(event: &mut TriggerEvent, snapshot: Option<ObjectSnapshot>) {
    if let Some(inner) = event.downcast::<crate::events::EnterBattlefieldEvent>() {
        let mut entry = inner.clone();
        entry.completed_snapshot = snapshot;
        *event = event.with_inner_event(entry);
    } else if let Some(inner) = event.downcast::<ConvertedEvent>() {
        *event = event.with_inner_event(inner.clone().with_snapshot(snapshot));
    } else if let Some(inner) = event.downcast::<LandPlayedEvent>() {
        *event = event.with_inner_event(inner.clone().with_snapshot(snapshot));
    } else if let Some(inner) = event.downcast::<TransformedEvent>() {
        *event = event.with_inner_event(inner.clone().with_snapshot(snapshot));
    } else if let Some(inner) = event.downcast::<MutatedEvent>() {
        *event = event.with_inner_event(inner.clone().with_snapshot(snapshot));
    } else if let Some(inner) = event.downcast::<TurnedFaceUpEvent>() {
        *event = event.with_inner_event(inner.clone().with_snapshot(snapshot));
    } else if let Some(inner) = event.downcast::<BecameMonstrousEvent>() {
        *event = event.with_inner_event(inner.clone().with_snapshot(snapshot));
    } else if let Some(inner) = event.downcast::<KeywordActionEvent>() {
        *event = event.with_inner_event(inner.clone().with_snapshot(snapshot));
    }
}

/// Prefer live completed characteristics. If the original incarnation departed
/// during the observed programs, use its exact immutable battlefield departure
/// receipt, never a stable-card lookup or a replacement-owned successor.
pub(crate) fn retain_departed_lifecycle_snapshots(
    game: &GameState,
    events: &mut [TriggerEvent],
    observations: &[TriggerEvent],
) {
    for event in events.iter_mut().filter(|event| needs_snapshot(event)) {
        let Some(subject) = event.object_id() else {
            continue;
        };
        if game
            .object(subject)
            .is_some_and(|object| object.zone == crate::zone::Zone::Battlefield)
        {
            continue;
        }
        // These are actual sibling-original receipts supplied by the group
        // owner, not a lookup in later mutable history or the trigger queue.
        let snapshot = observations
            .iter()
            .rev()
            .chain(
                game.effect_store
                    .retained_original_observation_scopes
                    .iter()
                    .rev()
                    .flat_map(|scope| scope.iter().rev()),
            )
            .filter_map(|observation| observation.downcast::<crate::events::ZoneChangeEvent>())
            .filter(|change| change.from == crate::zone::Zone::Battlefield)
            .flat_map(|change| change.snapshots())
            .find(|snapshot| {
                snapshot.object_id == subject && snapshot.zone == crate::zone::Zone::Battlefield
            })
            .cloned();
        if snapshot.is_some() {
            attach_snapshot(event, snapshot);
        }
    }
}
