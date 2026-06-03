//! Event emitted when a player loses the game.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::PlayerId;

#[derive(Debug, Clone)]
pub struct PlayerLostGameEvent {
    pub player: PlayerId,
}

impl PlayerLostGameEvent {
    pub fn new(player: PlayerId) -> Self {
        Self { player }
    }
}

impl GameEventType for PlayerLostGameEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::PlayerLostGame
    }

    fn affected_player(&self, _game: &GameState) -> PlayerId {
        self.player
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }

    fn display(&self) -> String {
        "Player lost the game".to_string()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
