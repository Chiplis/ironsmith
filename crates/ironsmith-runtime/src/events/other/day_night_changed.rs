use std::any::Any;

use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::GameState;
use crate::ids::PlayerId;

/// Event emitted when the game's day/night designation changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DayNightChangedEvent {
    /// True when the new designation is day, false when it is night.
    pub is_daytime: bool,
}

impl DayNightChangedEvent {
    pub fn new(is_daytime: bool) -> Self {
        Self { is_daytime }
    }
}

impl GameEventType for DayNightChangedEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::DayNightChanged
    }

    fn affected_player(&self, game: &GameState) -> PlayerId {
        game.turn.active_player
    }

    fn display(&self) -> String {
        if self.is_daytime {
            "It becomes day".to_string()
        } else {
            "It becomes night".to_string()
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
