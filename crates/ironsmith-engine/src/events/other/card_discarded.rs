//! Card discarded event implementation.

use std::any::Any;

use crate::events::cause::EventCause;
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
    /// What caused the discard, if known.
    pub cause: Option<EventCause>,
    /// Last-known information for the card as it existed in hand before the discard.
    pub snapshot: Option<ObjectSnapshot>,
    /// Cards discarded by the same discard action, in event order.
    pub batch_cards: Vec<ObjectId>,
    /// Last-known information for cards discarded by the same discard action.
    pub batch_snapshots: Vec<ObjectSnapshot>,
    /// This card's index in `batch_cards` when the event came from a batch discard.
    pub batch_index: Option<usize>,
}

impl CardDiscardedEvent {
    /// Create a new card discarded event.
    pub fn new(player: PlayerId, card: ObjectId) -> Self {
        Self {
            player,
            card,
            cause: None,
            snapshot: None,
            batch_cards: vec![card],
            batch_snapshots: Vec::new(),
            batch_index: Some(0),
        }
    }

    pub fn with_cause(player: PlayerId, card: ObjectId, cause: EventCause) -> Self {
        Self {
            player,
            card,
            cause: Some(cause),
            snapshot: None,
            batch_cards: vec![card],
            batch_snapshots: Vec::new(),
            batch_index: Some(0),
        }
    }

    pub fn with_snapshot(mut self, snapshot: ObjectSnapshot) -> Self {
        self.snapshot = Some(snapshot);
        self
    }

    pub fn with_batch(
        mut self,
        batch_cards: Vec<ObjectId>,
        batch_snapshots: Vec<ObjectSnapshot>,
        batch_index: usize,
    ) -> Self {
        self.batch_cards = batch_cards;
        self.batch_snapshots = batch_snapshots;
        self.batch_index = Some(batch_index);
        self
    }
}

impl GameEventType for CardDiscardedEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::CardDiscarded
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
        self.cause
            .as_ref()
            .and_then(|cause| cause.source_controller)
    }

    fn snapshot(&self) -> Option<&ObjectSnapshot> {
        self.snapshot.as_ref()
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
