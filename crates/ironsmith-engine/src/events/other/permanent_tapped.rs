//! Permanent tapped event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;

/// A permanent became tapped event.
///
/// Triggered when a permanent becomes tapped.
#[derive(Debug, Clone)]
pub struct PermanentTappedEvent {
    /// Optional additional cost whose payment caused this exact transition.
    pub cost_kind: Option<ironsmith_core::OptionalCostKind>,
    /// The permanent that became tapped
    pub permanent: ObjectId,
    /// Characteristics when the state transition completed.
    pub snapshot: Option<ObjectSnapshot>,
    /// Player performing the action, if the producer explicitly knows it.
    pub actor: Option<PlayerId>,
    /// Recipient before the action; active "you tap an untapped creature"
    /// clauses qualify this state, while passive clauses use `snapshot`.
    pub before_snapshot: Option<ObjectSnapshot>,
    /// The tap is the CR 508.1f tap of a creature being declared as an
    /// attacker ("if it isn't being declared as an attacker", Verity Circle).
    pub attack_declaration: bool,
}

impl PermanentTappedEvent {
    /// Create a new permanent tapped event.
    pub fn new(permanent: ObjectId) -> Self {
        Self {
            cost_kind: None,
            permanent,
            snapshot: None,
            actor: None,
            before_snapshot: None,
            attack_declaration: false,
        }
    }

    /// Capture a completed transition at its producer, before later effects
    /// can change its characteristics or controller.
    pub fn capture(game: &GameState, permanent: ObjectId, actor: Option<PlayerId>) -> Self {
        Self {
            cost_kind: None,
            permanent,
            snapshot: game.object(permanent).and_then(|object| {
                ObjectSnapshot::capture_for_execution(object, game)
            }),
            actor,
            before_snapshot: None,
            attack_declaration: false,
        }
    }

    pub fn with_cost_kind(mut self, kind: ironsmith_core::OptionalCostKind) -> Self {
        self.cost_kind = Some(kind);
        self
    }

    pub fn with_before_snapshot(mut self, snapshot: ObjectSnapshot) -> Self {
        self.before_snapshot = Some(snapshot);
        self
    }

    pub fn with_snapshot(mut self, snapshot: ObjectSnapshot) -> Self {
        self.snapshot = Some(snapshot);
        self
    }

    pub fn with_actor(mut self, actor: PlayerId) -> Self {
        self.actor = Some(actor);
        self
    }
}

impl GameEventType for PermanentTappedEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::PermanentTapped
    }

    fn affected_player(&self, game: &GameState) -> PlayerId {
        game.object(self.permanent)
            .map(|o| game.controller_of(o))
            .unwrap_or(game.turn.active_player)
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }

    fn display(&self) -> String {
        "Permanent became tapped".to_string()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn object_id(&self) -> Option<ObjectId> {
        Some(self.permanent)
    }

    fn player(&self) -> Option<PlayerId> {
        self.actor
    }

    fn controller(&self) -> Option<PlayerId> {
        None // Determined from game state
    }

    fn snapshot(&self) -> Option<&ObjectSnapshot> {
        self.snapshot.as_ref()
    }
}

/// Snapshot the common pre-instruction battlefield before any member changes
/// state. Redirected recipients and chosen-cost recipients share the same view.
pub(crate) fn before_tap_state_snapshots(
    game: &GameState,
) -> Result<std::collections::HashMap<ObjectId, ObjectSnapshot>, crate::effects::ExecutionError> {
    game.battlefield
        .iter()
        .filter_map(|id| game.object(*id))
        .map(|object| {
            ObjectSnapshot::try_from_object_with_calculated_characteristics(object, game)
                .map(|snapshot| (object.id, snapshot))
        })
        .collect()
}

pub(crate) fn bind_before_tap_state_snapshots(
    events: &mut [crate::triggers::TriggerEvent],
    snapshots: &std::collections::HashMap<ObjectId, ObjectSnapshot>,
) {
    for event in events {
        // Replacement/composed instructions may already have captured their
        // own complete occurrence; don't reinterpret their earlier state.
        if event.simultaneous_batch().is_some() {
            continue;
        }
        if let Some(tapped) = event.downcast::<PermanentTappedEvent>() {
            if let Some(before) = snapshots.get(&tapped.permanent) {
                let mut tapped = tapped.clone();
                tapped.before_snapshot = Some(before.clone());
                *event = event.with_inner_event(tapped);
            }
        } else if let Some(untapped) = event.downcast::<super::PermanentUntappedEvent>() {
            if let Some(before) = snapshots.get(&untapped.permanent) {
                let mut untapped = untapped.clone();
                untapped.before_snapshot = Some(before.clone());
                *event = event.with_inner_event(untapped);
            }
        }
    }
}

/// Stamp the actual state changes made by one complete tap/untap instruction.
/// Every call owns its batch: a broader cost/zone-change scope may contain
/// distinct tap instructions and must not merge them. Convoke/improvise and
/// the turn-based untap collect their whole instruction before calling here.
/// Already-tapped/untapped objects produce no notification.
pub(crate) fn group_tap_state_events(
    game: &mut GameState,
    events: &mut [crate::triggers::TriggerEvent],
    provenance: crate::provenance::ProvNodeId,
) -> Result<(), crate::effects::ExecutionError> {
    let state_change = |event: &crate::triggers::TriggerEvent| {
        matches!(
            event.kind(),
            EventKind::PermanentTapped | EventKind::PermanentUntapped
        )
    };
    let Some(kind) = events
        .iter()
        .find(|event| state_change(event) && event.simultaneous_batch().is_none())
        .map(|event| event.kind())
    else {
        return Ok(());
    };
    // Freeze the complete final world before allocating a batch or editing
    // any event. An incomplete member cannot publish a partial receipt.
    let snapshots = events.iter().filter(|event| state_change(event) && event.simultaneous_batch().is_none())
        .filter_map(|event| event.downcast::<PermanentTappedEvent>().map(|event| event.permanent)
            .or_else(|| event.downcast::<super::PermanentUntappedEvent>().map(|event| event.permanent)))
        .filter_map(|id| game.object(id))
        .map(|object| ObjectSnapshot::try_from_object_with_calculated_characteristics(object, game)
            .map(|snapshot| (object.id, snapshot)))
        .collect::<Result<std::collections::HashMap<_, _>, _>>()?;
    let batch = game.alloc_child_event_provenance(provenance, kind);
    for event in events.iter_mut().filter(|event| state_change(event)) {
        if event.simultaneous_batch().is_some() {
            continue;
        }
        // All members of one instruction see the completed simultaneous state,
        // rather than a partially tapped/untapped intermediate battlefield.
        if let Some(tapped) = event.downcast::<PermanentTappedEvent>() {
            let mut tapped = tapped.clone();
            if let Some(snapshot) = snapshots.get(&tapped.permanent) {
                tapped.snapshot = Some(snapshot.clone());
            }
            *event = event.with_inner_event(tapped);
        } else if let Some(untapped) = event.downcast::<super::PermanentUntappedEvent>() {
            let mut untapped = untapped.clone();
            if let Some(snapshot) = snapshots.get(&untapped.permanent) {
                untapped.snapshot = Some(snapshot.clone());
            }
            *event = event.with_inner_event(untapped);
        }
        if event.simultaneous_batch().is_none() {
            *event = event.clone().with_simultaneous_batch(batch);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permanent_tapped_event_creation() {
        let event = PermanentTappedEvent::new(ObjectId::from_raw(1));
        assert_eq!(event.permanent, ObjectId::from_raw(1));
    }

    #[test]
    fn test_permanent_tapped_event_kind() {
        let event = PermanentTappedEvent::new(ObjectId::from_raw(1));
        assert_eq!(event.event_kind(), EventKind::PermanentTapped);
    }
}
