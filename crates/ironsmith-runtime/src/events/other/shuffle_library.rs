//! Shuffle library event implementation.

use std::any::Any;

use crate::events::cause::EventCause;
use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;

/// A player shuffled their library.
#[derive(Debug, Clone)]
pub struct ShuffleLibraryEvent {
    /// The player whose library was shuffled.
    pub player: PlayerId,
    /// What caused the shuffle.
    pub cause: EventCause,
}

impl ShuffleLibraryEvent {
    pub fn new(player: PlayerId, cause: EventCause) -> Self {
        Self { player, cause }
    }
}

impl GameEventType for ShuffleLibraryEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::ShuffleLibrary
    }

    fn affected_player(&self, _game: &GameState) -> PlayerId {
        self.player
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }

    fn object_id(&self) -> Option<ObjectId> {
        self.cause.source
    }

    fn player(&self) -> Option<PlayerId> {
        Some(self.player)
    }

    fn controller(&self) -> Option<PlayerId> {
        self.cause.source_controller.or(Some(self.player))
    }

    fn snapshot(&self) -> Option<&ObjectSnapshot> {
        None
    }

    fn display(&self) -> String {
        "Player shuffled their library".to_string()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
