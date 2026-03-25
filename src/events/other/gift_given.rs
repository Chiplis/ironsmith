//! Gift-given event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;

/// A player gave a gift as a Gift ability resolved.
#[derive(Debug, Clone)]
pub struct GiftGivenEvent {
    /// The player who gave the gift.
    pub player: PlayerId,
    /// The opponent who received the gift.
    pub recipient: PlayerId,
    /// The spell or permanent whose Gift ability resolved.
    pub source: ObjectId,
}

impl GiftGivenEvent {
    pub fn new(player: PlayerId, recipient: PlayerId, source: ObjectId) -> Self {
        Self {
            player,
            recipient,
            source,
        }
    }
}

impl GameEventType for GiftGivenEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::GiftGiven
    }

    fn affected_player(&self, _game: &GameState) -> PlayerId {
        self.player
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }

    fn source_object(&self) -> Option<ObjectId> {
        Some(self.source)
    }

    fn object_id(&self) -> Option<ObjectId> {
        Some(self.source)
    }

    fn player(&self) -> Option<PlayerId> {
        Some(self.player)
    }

    fn controller(&self) -> Option<PlayerId> {
        Some(self.player)
    }

    fn snapshot(&self) -> Option<&ObjectSnapshot> {
        None
    }

    fn display(&self) -> String {
        "Player gave a gift".to_string()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gift_given_event_kind_and_display() {
        let event = GiftGivenEvent::new(
            PlayerId::from_index(0),
            PlayerId::from_index(1),
            ObjectId::from_raw(7),
        );
        assert_eq!(event.event_kind(), EventKind::GiftGiven);
        assert_eq!(event.display(), "Player gave a gift");
        assert_eq!(event.player(), Some(PlayerId::from_index(0)));
        assert_eq!(event.source_object(), Some(ObjectId::from_raw(7)));
    }
}
