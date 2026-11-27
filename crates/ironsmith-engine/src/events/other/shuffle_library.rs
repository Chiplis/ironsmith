//! Shuffle library event implementation.

use std::any::Any;

use crate::events::cause::EventCause;
use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;

/// A player shuffled a library.
#[derive(Debug, Clone)]
pub struct ShuffleLibraryEvent {
    /// The player whose library was shuffled.
    pub player: PlayerId,
    /// The player instructed to perform the shuffle.
    pub shuffler: PlayerId,
    /// What caused the shuffle.
    pub cause: EventCause,
}

impl ShuffleLibraryEvent {
    pub fn new(player: PlayerId, cause: EventCause) -> Self {
        Self::by(player, player, cause)
    }

    pub fn by(shuffler: PlayerId, library_owner: PlayerId, cause: EventCause) -> Self {
        Self {
            player: library_owner,
            shuffler,
            cause,
        }
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
        Some(self.shuffler)
    }

    fn controller(&self) -> Option<PlayerId> {
        self.cause.source_controller.or(Some(self.shuffler))
    }

    fn snapshot(&self) -> Option<&ObjectSnapshot> {
        None
    }

    fn display(&self) -> String {
        if self.shuffler == self.player {
            "Player shuffled their library"
        } else {
            "Player shuffled another player's library"
        }
        .to_string()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
