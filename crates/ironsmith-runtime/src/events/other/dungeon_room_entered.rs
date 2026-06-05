//! Dungeon room entry event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::PlayerId;

/// Event emitted when a player enters a room of a dungeon they are venturing through.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DungeonRoomEnteredEvent {
    pub player: PlayerId,
    pub dungeon_name: String,
    pub room_name: String,
}

impl DungeonRoomEnteredEvent {
    pub fn new(
        player: PlayerId,
        dungeon_name: impl Into<String>,
        room_name: impl Into<String>,
    ) -> Self {
        Self {
            player,
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
        self.player
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }

    fn player(&self) -> Option<PlayerId> {
        Some(self.player)
    }

    fn controller(&self) -> Option<PlayerId> {
        Some(self.player)
    }

    fn display(&self) -> String {
        format!(
            "{:?} entered {} of {}",
            self.player, self.room_name, self.dungeon_name
        )
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
