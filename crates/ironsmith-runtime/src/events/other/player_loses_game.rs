//! Player-loses-game event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::PlayerId;

#[derive(Debug, Clone)]
pub struct PlayerLosesGameEvent {
    pub player: PlayerId,
}

impl PlayerLosesGameEvent {
    pub fn new(player: PlayerId) -> Self {
        Self { player }
    }
}

impl GameEventType for PlayerLosesGameEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::PlayerLosesGame
    }

    fn affected_player(&self, _game: &GameState) -> PlayerId {
        self.player
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }

    fn display(&self) -> String {
        format!("Player {} loses the game", self.player.0)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn player(&self) -> Option<PlayerId> {
        Some(self.player)
    }
}
