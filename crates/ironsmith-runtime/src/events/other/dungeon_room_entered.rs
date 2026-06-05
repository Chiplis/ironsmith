//! Event emitted when a player moves into a dungeon room.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};

#[derive(Debug, Clone)]
pub struct DungeonRoomEnteredEvent {
    pub owner: PlayerId,
    pub source: ObjectId,
    pub dungeon_name: String,
    pub room_name: String,
}

impl DungeonRoomEnteredEvent {
    pub fn new(
        owner: PlayerId,
        source: ObjectId,
        dungeon_name: impl Into<String>,
        room_name: impl Into<String>,
    ) -> Self {
        Self {
            owner,
            source,
            dungeon_name: dungeon_name.into(),
            room_name: room_name.into(),
        }
    }
}

impl GameEventType for DungeonRoomEnteredEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::DungeonRoomEntered
    }

    fn affected_player(&self, _game: &GameState) -> PlayerId {
        self.owner
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
        Some(self.owner)
    }

    fn controller(&self) -> Option<PlayerId> {
        Some(self.owner)
    }

    fn display(&self) -> String {
        format!(
            "Player entered dungeon room '{}' in '{}'",
            self.room_name, self.dungeon_name
        )
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
