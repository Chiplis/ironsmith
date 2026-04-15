//! Permanent converted event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};

/// A permanent converted.
#[derive(Debug, Clone)]
pub struct ConvertedEvent {
    /// The permanent that converted.
    pub permanent: ObjectId,
}

impl ConvertedEvent {
    /// Create a new converted event.
    pub fn new(permanent: ObjectId) -> Self {
        Self { permanent }
    }
}

impl GameEventType for ConvertedEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::Converted
    }

    fn affected_player(&self, game: &GameState) -> PlayerId {
        game.object(self.permanent)
            .map(|o| o.controller)
            .unwrap_or(game.turn.active_player)
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }

    fn display(&self) -> String {
        "Permanent converted".to_string()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn object_id(&self) -> Option<ObjectId> {
        Some(self.permanent)
    }
}
