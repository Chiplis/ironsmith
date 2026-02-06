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
    /// The permanent that became tapped
    pub permanent: ObjectId,
}

impl PermanentTappedEvent {
    /// Create a new permanent tapped event.
    pub fn new(permanent: ObjectId) -> Self {
        Self { permanent }
    }
}

impl GameEventType for PermanentTappedEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::PermanentTapped
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
        "Permanent became tapped".to_string()
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
