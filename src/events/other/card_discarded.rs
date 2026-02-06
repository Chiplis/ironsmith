//! Card discarded event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;

/// A player discarded a card event.
///
/// Triggered when a player discards a card. Distinct from the Discard event
/// used in the replacement effect system - this is for triggers only.
#[derive(Debug, Clone)]
pub struct CardDiscardedEvent {
    /// The player who discarded the card
    pub player: PlayerId,
    /// The card that was discarded
    pub card: ObjectId,
}

impl CardDiscardedEvent {
    /// Create a new card discarded event.
    pub fn new(player: PlayerId, card: ObjectId) -> Self {
        Self { player, card }
    }
}

impl GameEventType for CardDiscardedEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::CardDiscarded
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
        "Player discarded a card".to_string()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn object_id(&self) -> Option<ObjectId> {
        Some(self.card)
    }

    fn player(&self) -> Option<PlayerId> {
        Some(self.player)
    }

    fn controller(&self) -> Option<PlayerId> {
        None
    }

    fn snapshot(&self) -> Option<&ObjectSnapshot> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_card_discarded_event_creation() {
        let event = CardDiscardedEvent::new(PlayerId::from_index(0), ObjectId::from_raw(42));
        assert_eq!(event.player, PlayerId::from_index(0));
        assert_eq!(event.card, ObjectId::from_raw(42));
    }

    #[test]
    fn test_card_discarded_event_kind() {
        let event = CardDiscardedEvent::new(PlayerId::from_index(0), ObjectId::from_raw(1));
        assert_eq!(event.event_kind(), EventKind::CardDiscarded);
    }
}
