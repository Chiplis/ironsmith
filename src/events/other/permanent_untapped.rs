//! Permanent untapped event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;

/// A permanent became untapped event.
///
/// Triggered when a permanent becomes untapped.
#[derive(Debug, Clone)]
pub struct PermanentUntappedEvent {
    /// The permanent that became untapped
    pub permanent: ObjectId,
}

impl PermanentUntappedEvent {
    /// Create a new permanent untapped event.
    pub fn new(permanent: ObjectId) -> Self {
        Self { permanent }
    }
}

impl GameEventType for PermanentUntappedEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::PermanentUntapped
    }

    fn clone_box(&self) -> Box<dyn GameEventType> {
        Box::new(self.clone())
    }

    fn affected_player(&self, game: &GameState) -> PlayerId {
        game.object(self.permanent)
            .map(|o| o.controller)
            .unwrap_or(game.turn.active_player)
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }

    fn display(&self) -> String {
        "Permanent became untapped".to_string()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn object_id(&self) -> Option<ObjectId> {
        Some(self.permanent)
    }

    fn player(&self) -> Option<PlayerId> {
        None
    }

    fn controller(&self) -> Option<PlayerId> {
        None // Determined from game state
    }

    fn snapshot(&self) -> Option<&ObjectSnapshot> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permanent_untapped_event_creation() {
        let event = PermanentUntappedEvent::new(ObjectId::from_raw(1));
        assert_eq!(event.permanent, ObjectId::from_raw(1));
    }

    #[test]
    fn test_permanent_untapped_event_kind() {
        let event = PermanentUntappedEvent::new(ObjectId::from_raw(1));
        assert_eq!(event.event_kind(), EventKind::PermanentUntapped);
    }
}
