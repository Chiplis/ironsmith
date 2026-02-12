//! Turned-face-up event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};

/// A face-down permanent was turned face up.
#[derive(Debug, Clone)]
pub struct TurnedFaceUpEvent {
    /// The permanent that was turned face up.
    pub permanent: ObjectId,
    /// The player who turned it face up.
    pub player: PlayerId,
}

impl TurnedFaceUpEvent {
    /// Create a new turned-face-up event.
    pub fn new(permanent: ObjectId, player: PlayerId) -> Self {
        Self { permanent, player }
    }
}

impl GameEventType for TurnedFaceUpEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::TurnedFaceUp
    }

    fn clone_box(&self) -> Box<dyn GameEventType> {
        Box::new(self.clone())
    }

    fn affected_player(&self, _game: &GameState) -> PlayerId {
        self.player
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }

    fn display(&self) -> String {
        "Permanent turned face up".to_string()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn object_id(&self) -> Option<ObjectId> {
        Some(self.permanent)
    }

    fn player(&self) -> Option<PlayerId> {
        Some(self.player)
    }

    fn controller(&self) -> Option<PlayerId> {
        Some(self.player)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turned_face_up_event_kind() {
        let event = TurnedFaceUpEvent::new(ObjectId::from_raw(1), PlayerId::from_index(0));
        assert_eq!(event.event_kind(), EventKind::TurnedFaceUp);
    }
}
