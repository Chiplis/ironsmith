//! Zone change event implementation.

use std::any::Any;

use crate::events::cause::EventCause;
use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;
use crate::zone::Zone;

/// A zone change event that can be processed through the replacement effect system.
///
/// This is the primitive event for all zone changes. Higher-level concepts like
/// "dies", "discard", "exile", "ETB" are all just filtered views of zone changes.
#[derive(Debug, Clone)]
pub struct ZoneChangeEvent {
    /// The objects changing zones. Usually one, but can be multiple for batch
    /// operations like "discard your hand" or "mill 3".
    pub objects: Vec<ObjectId>,
    /// The zone the objects are leaving
    pub from: Zone,
    /// The zone the objects are entering
    pub to: Zone,
    /// What caused this zone change (effect, cost, SBA, game rule, etc.)
    pub cause: EventCause,
    /// Snapshot of the object's state before the zone change (for LKI).
    /// For batch events, this is the snapshot of the first/primary object.
    pub snapshot: Option<ObjectSnapshot>,
}

impl ZoneChangeEvent {
    /// Create a new zone change event for a single object.
    pub fn new(object: ObjectId, from: Zone, to: Zone, snapshot: Option<ObjectSnapshot>) -> Self {
        Self {
            objects: vec![object],
            from,
            to,
            cause: EventCause::default(),
            snapshot,
        }
    }

    /// Create a zone change event with a specific cause.
    pub fn with_cause(
        object: ObjectId,
        from: Zone,
        to: Zone,
        cause: EventCause,
        snapshot: Option<ObjectSnapshot>,
    ) -> Self {
        Self {
            objects: vec![object],
            from,
            to,
            cause,
            snapshot,
        }
    }

    /// Create a batch zone change event for multiple objects.
    pub fn batch(objects: Vec<ObjectId>, from: Zone, to: Zone, cause: EventCause) -> Self {
        Self {
            objects,
            from,
            to,
            cause,
            snapshot: None,
        }
    }

    /// Get the number of objects in this zone change.
    pub fn count(&self) -> usize {
        self.objects.len()
    }

    /// Return a new event with a different destination zone.
    pub fn with_destination(&self, to: Zone) -> Self {
        Self { to, ..self.clone() }
    }

    /// Check if this is a "dies" event (battlefield to graveyard).
    pub fn is_dies(&self) -> bool {
        self.from == Zone::Battlefield && self.to == Zone::Graveyard
    }

    /// Check if this is a "discard" event (hand to graveyard).
    pub fn is_discard(&self) -> bool {
        self.from == Zone::Hand && self.to == Zone::Graveyard
    }

    /// Check if this is a "mill" event (library to graveyard).
    pub fn is_mill(&self) -> bool {
        self.from == Zone::Library && self.to == Zone::Graveyard
    }

    /// Check if this is entering the battlefield.
    pub fn is_etb(&self) -> bool {
        self.to == Zone::Battlefield
    }

    /// Check if this is leaving the battlefield.
    pub fn is_ltb(&self) -> bool {
        self.from == Zone::Battlefield
    }

    /// Check if this is being exiled.
    pub fn is_exile(&self) -> bool {
        self.to == Zone::Exile
    }

    /// Check if this is entering a graveyard.
    pub fn is_to_graveyard(&self) -> bool {
        self.to == Zone::Graveyard
    }
}

impl GameEventType for ZoneChangeEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::ZoneChange
    }

    fn clone_box(&self) -> Box<dyn GameEventType> {
        Box::new(self.clone())
    }

    fn affected_player(&self, game: &GameState) -> PlayerId {
        // Use the first object's controller, or fall back to active player
        self.objects
            .first()
            .and_then(|&id| game.object(id))
            .map(|o| o.controller)
            .unwrap_or(game.turn.active_player)
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        // Zone changes don't have redirectable targets
        None
    }

    fn source_object(&self) -> Option<ObjectId> {
        self.cause.source
    }

    fn object_id(&self) -> Option<ObjectId> {
        self.objects.first().copied()
    }

    fn display(&self) -> String {
        if self.objects.len() == 1 {
            format!("Move object from {:?} to {:?}", self.from, self.to)
        } else {
            format!(
                "Move {} objects from {:?} to {:?}",
                self.objects.len(),
                self.from,
                self.to
            )
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn snapshot(&self) -> Option<&ObjectSnapshot> {
        self.snapshot.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::cause::CauseType;

    #[test]
    fn test_zone_change_event_creation() {
        let event =
            ZoneChangeEvent::new(ObjectId::from_raw(1), Zone::Hand, Zone::Battlefield, None);

        assert_eq!(event.from, Zone::Hand);
        assert_eq!(event.to, Zone::Battlefield);
        assert_eq!(event.objects.first().copied(), Some(ObjectId::from_raw(1)));
        assert_eq!(event.count(), 1);
    }

    #[test]
    fn test_zone_change_with_cause() {
        let cause = EventCause::from_effect(ObjectId::from_raw(99), PlayerId::from_index(0));
        let event = ZoneChangeEvent::with_cause(
            ObjectId::from_raw(1),
            Zone::Hand,
            Zone::Graveyard,
            cause,
            None,
        );

        assert_eq!(event.cause.cause_type, CauseType::Effect);
        assert_eq!(event.cause.source, Some(ObjectId::from_raw(99)));
    }

    #[test]
    fn test_zone_change_batch() {
        let objects = vec![
            ObjectId::from_raw(1),
            ObjectId::from_raw(2),
            ObjectId::from_raw(3),
        ];
        let event = ZoneChangeEvent::batch(
            objects.clone(),
            Zone::Library,
            Zone::Graveyard,
            EventCause::from_effect(ObjectId::from_raw(99), PlayerId::from_index(0)),
        );

        assert_eq!(event.objects, objects);
        assert_eq!(event.count(), 3);
        assert_eq!(event.objects.first().copied(), Some(ObjectId::from_raw(1)));
        assert!(event.is_mill());
    }

    #[test]
    fn test_zone_change_is_dies() {
        let dies_event = ZoneChangeEvent::new(
            ObjectId::from_raw(1),
            Zone::Battlefield,
            Zone::Graveyard,
            None,
        );
        assert!(dies_event.is_dies());
        assert!(dies_event.is_ltb());
        assert!(dies_event.is_to_graveyard());

        let not_dies_event =
            ZoneChangeEvent::new(ObjectId::from_raw(1), Zone::Hand, Zone::Graveyard, None);
        assert!(!not_dies_event.is_dies());
        assert!(not_dies_event.is_discard());
    }

    #[test]
    fn test_zone_change_is_etb() {
        let etb_event =
            ZoneChangeEvent::new(ObjectId::from_raw(1), Zone::Hand, Zone::Battlefield, None);
        assert!(etb_event.is_etb());

        let not_etb_event = ZoneChangeEvent::new(
            ObjectId::from_raw(1),
            Zone::Battlefield,
            Zone::Graveyard,
            None,
        );
        assert!(!not_etb_event.is_etb());
    }

    #[test]
    fn test_zone_change_is_exile() {
        let exile_event =
            ZoneChangeEvent::new(ObjectId::from_raw(1), Zone::Battlefield, Zone::Exile, None);
        assert!(exile_event.is_exile());
        assert!(exile_event.is_ltb());
    }

    #[test]
    fn test_zone_change_with_destination() {
        let event = ZoneChangeEvent::new(
            ObjectId::from_raw(1),
            Zone::Battlefield,
            Zone::Graveyard,
            None,
        );

        let changed = event.with_destination(Zone::Exile);
        assert_eq!(changed.to, Zone::Exile);
        assert_eq!(changed.from, Zone::Battlefield);
    }

    #[test]
    fn test_zone_change_event_kind() {
        let event =
            ZoneChangeEvent::new(ObjectId::from_raw(1), Zone::Hand, Zone::Battlefield, None);
        assert_eq!(event.event_kind(), EventKind::ZoneChange);
    }

    #[test]
    fn test_zone_change_display() {
        let single =
            ZoneChangeEvent::new(ObjectId::from_raw(1), Zone::Hand, Zone::Battlefield, None);
        assert!(single.display().contains("Move object"));

        let batch = ZoneChangeEvent::batch(
            vec![ObjectId::from_raw(1), ObjectId::from_raw(2)],
            Zone::Library,
            Zone::Graveyard,
            EventCause::default(),
        );
        assert!(batch.display().contains("2 objects"));
    }
}
