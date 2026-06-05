//! Die roll event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};

/// A player rolled a die and got a result.
#[derive(Debug, Clone)]
pub struct DieRolledEvent {
    pub player: PlayerId,
    pub source: ObjectId,
    pub natural_result: u32,
    pub result: u32,
    pub sides: u32,
}

impl DieRolledEvent {
    pub fn new(player: PlayerId, source: ObjectId, result: u32, sides: u32) -> Self {
        Self::new_with_natural_result(player, source, result, result, sides)
    }

    pub fn new_with_natural_result(
        player: PlayerId,
        source: ObjectId,
        natural_result: u32,
        result: u32,
        sides: u32,
    ) -> Self {
        Self {
            player,
            source,
            natural_result,
            result,
            sides,
        }
    }
}

impl GameEventType for DieRolledEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::DieRolled
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

    fn display(&self) -> String {
        format!("Player rolled a {} on a d{}", self.result, self.sides)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
