//! Structural movement of a player's marker into a dungeon room.
use std::any::Any;
use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
#[derive(Debug, Clone)]
pub struct DungeonRoomEnteredEvent {
    pub source: ObjectId,
    pub player: PlayerId,
    pub dungeon: String,
    pub room: String,
}
impl GameEventType for DungeonRoomEnteredEvent {
    fn event_kind(&self) -> EventKind { EventKind::DungeonRoomEntered }
    fn affected_player(&self, _: &GameState) -> PlayerId { self.player }
    fn with_target_replaced(&self, _: &Target, _: &Target) -> Option<Box<dyn GameEventType>> { None }
    fn display(&self) -> String { format!("Entered {} in {}", self.room, self.dungeon) }
    fn as_any(&self) -> &dyn Any { self }
    fn player(&self) -> Option<PlayerId> { Some(self.player) }
    fn controller(&self) -> Option<PlayerId> { Some(self.player) }
    fn source_object(&self) -> Option<ObjectId> { Some(self.source) }
}
