//! Search library event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;

/// A player searched a library.
#[derive(Debug, Clone)]
pub struct SearchLibraryEvent {
    /// The player who performed the search.
    pub player: PlayerId,
    /// Whose library was searched, when known.
    pub library_owner: Option<PlayerId>,
}

impl SearchLibraryEvent {
    pub fn new(player: PlayerId, library_owner: Option<PlayerId>) -> Self {
        Self {
            player,
            library_owner,
        }
    }
}

impl GameEventType for SearchLibraryEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::SearchLibrary
    }

    fn affected_player(&self, _game: &GameState) -> PlayerId {
        self.player
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }

    fn object_id(&self) -> Option<ObjectId> {
        None
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
        "Player searched a library".to_string()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
