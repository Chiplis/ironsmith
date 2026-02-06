//! Markers changed event implementation.
//!
//! This event fires when markers (counters, etc.) are added to or removed from
//! an object or player. It unifies counter placement and removal into a single
//! event type for cleaner handling.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::marker::{Marker, MarkerLocation};

/// The type of marker change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkerChangeType {
    /// Markers were added.
    Added,
    /// Markers were removed.
    Removed,
}

/// A marker change event.
///
/// Fires when markers are added to or removed from an object or player.
/// This allows effects to react to marker changes and track counts for
/// "for each counter removed this way" effects.
#[derive(Debug, Clone)]
pub struct MarkersChangedEvent {
    /// The type of change (added or removed).
    pub change_type: MarkerChangeType,
    /// The marker that changed.
    pub marker: Marker,
    /// Where the marker is/was located.
    pub location: MarkerLocation,
    /// The number of markers added/removed.
    pub amount: u32,
    /// The source that caused this change (if any).
    pub source: Option<ObjectId>,
    /// The player who controlled the source (if any).
    pub source_controller: Option<PlayerId>,
}

impl MarkersChangedEvent {
    /// Create a new markers added event.
    pub fn added(
        marker: impl Into<Marker>,
        location: impl Into<MarkerLocation>,
        amount: u32,
        source: Option<ObjectId>,
        source_controller: Option<PlayerId>,
    ) -> Self {
        Self {
            change_type: MarkerChangeType::Added,
            marker: marker.into(),
            location: location.into(),
            amount,
            source,
            source_controller,
        }
    }

    /// Create a new markers removed event.
    pub fn removed(
        marker: impl Into<Marker>,
        location: impl Into<MarkerLocation>,
        amount: u32,
        source: Option<ObjectId>,
        source_controller: Option<PlayerId>,
    ) -> Self {
        Self {
            change_type: MarkerChangeType::Removed,
            marker: marker.into(),
            location: location.into(),
            amount,
            source,
            source_controller,
        }
    }

    /// Check if this is an add event.
    pub fn is_added(&self) -> bool {
        self.change_type == MarkerChangeType::Added
    }

    /// Check if this is a remove event.
    pub fn is_removed(&self) -> bool {
        self.change_type == MarkerChangeType::Removed
    }

    /// Get the object ID if the location is an object.
    pub fn object(&self) -> Option<ObjectId> {
        self.location.as_object()
    }

    /// Get the player ID if the location is a player.
    pub fn player(&self) -> Option<PlayerId> {
        self.location.as_player()
    }
}

impl GameEventType for MarkersChangedEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::MarkersChanged
    }

    fn clone_box(&self) -> Box<dyn GameEventType> {
        Box::new(self.clone())
    }

    fn affected_player(&self, game: &GameState) -> PlayerId {
        match &self.location {
            MarkerLocation::Player(id) => *id,
            MarkerLocation::Object(id) => game
                .object(*id)
                .map(|o| o.controller)
                .unwrap_or(game.turn.active_player),
        }
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }

    fn source_object(&self) -> Option<ObjectId> {
        self.source
    }

    fn display(&self) -> String {
        let action = match self.change_type {
            MarkerChangeType::Added => "added to",
            MarkerChangeType::Removed => "removed from",
        };
        format!("{} {:?} {}", self.amount, self.marker, action)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn object_id(&self) -> Option<ObjectId> {
        self.location.as_object()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::CounterType;

    #[test]
    fn test_markers_added_event() {
        let event = MarkersChangedEvent::added(
            CounterType::PlusOnePlusOne,
            ObjectId::from_raw(1),
            3,
            Some(ObjectId::from_raw(99)),
            Some(PlayerId::from_index(0)),
        );

        assert!(event.is_added());
        assert!(!event.is_removed());
        assert_eq!(event.amount, 3);
        assert_eq!(event.object(), Some(ObjectId::from_raw(1)));
        assert_eq!(event.source, Some(ObjectId::from_raw(99)));
    }

    #[test]
    fn test_markers_removed_event() {
        let event =
            MarkersChangedEvent::removed(CounterType::Charge, ObjectId::from_raw(2), 5, None, None);

        assert!(event.is_removed());
        assert!(!event.is_added());
        assert_eq!(event.amount, 5);
        assert_eq!(event.object(), Some(ObjectId::from_raw(2)));
    }

    #[test]
    fn test_player_location() {
        let event =
            MarkersChangedEvent::added(CounterType::Poison, PlayerId::from_index(1), 2, None, None);

        assert_eq!(event.player(), Some(PlayerId::from_index(1)));
        assert_eq!(event.object(), None);
    }

    #[test]
    fn test_event_kind() {
        let event =
            MarkersChangedEvent::added(CounterType::Loyalty, ObjectId::from_raw(1), 1, None, None);
        assert_eq!(event.event_kind(), EventKind::MarkersChanged);
    }
}
