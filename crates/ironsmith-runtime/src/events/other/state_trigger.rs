//! Synthetic event used for state-triggered abilities.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};

/// A synthetic event representing a state trigger becoming true.
#[derive(Debug, Clone)]
pub struct StateTriggerEvent {
    /// The object whose state-triggered ability fired.
    pub source: ObjectId,
}

impl StateTriggerEvent {
    pub fn new(source: ObjectId) -> Self {
        Self { source }
    }
}

impl GameEventType for StateTriggerEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::StateTrigger
    }

    fn affected_player(&self, game: &GameState) -> PlayerId {
        game.object(self.source)
            .map(|obj| obj.controller)
            .unwrap_or(game.turn.active_player)
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }

    fn display(&self) -> String {
        "State trigger".to_string()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn object_id(&self) -> Option<ObjectId> {
        Some(self.source)
    }
}
