//! Card-revealed event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;
use crate::zone::Zone;

/// A player revealed a card.
#[derive(Debug, Clone)]
pub struct CardRevealedEvent {
    /// The player who revealed the card.
    pub player: PlayerId,
    /// The revealed card.
    pub card: ObjectId,
    /// The zone the card was revealed from.
    pub zone: Zone,
    /// The source object whose effect or ability caused the reveal, when known.
    pub source: Option<ObjectId>,
    /// Snapshot of the revealed card at reveal time.
    pub snapshot: Option<ObjectSnapshot>,
}

impl CardRevealedEvent {
    pub fn new(
        player: PlayerId,
        card: ObjectId,
        zone: Zone,
        source: Option<ObjectId>,
        snapshot: Option<ObjectSnapshot>,
    ) -> Self {
        Self {
            player,
            card,
            zone,
            source,
            snapshot,
        }
    }
}

impl GameEventType for CardRevealedEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::CardRevealed
    }

    fn affected_player(&self, _game: &GameState) -> PlayerId {
        self.player
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }

    fn source_object(&self) -> Option<ObjectId> {
        self.source
    }

    fn object_id(&self) -> Option<ObjectId> {
        Some(self.card)
    }

    fn player(&self) -> Option<PlayerId> {
        Some(self.player)
    }

    fn controller(&self) -> Option<PlayerId> {
        Some(self.player)
    }

    fn snapshot(&self) -> Option<&ObjectSnapshot> {
        self.snapshot.as_ref()
    }

    fn display(&self) -> String {
        "Player revealed a card".to_string()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
