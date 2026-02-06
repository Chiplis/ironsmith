//! Beginning of end step event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;

/// Beginning of end step event.
///
/// Triggered at the beginning of a player's end step.
#[derive(Debug, Clone)]
pub struct BeginningOfEndStepEvent {
    /// The player whose end step it is
    pub player: PlayerId,
}

impl BeginningOfEndStepEvent {
    /// Create a new beginning of end step event.
    pub fn new(player: PlayerId) -> Self {
        Self { player }
    }
}

impl GameEventType for BeginningOfEndStepEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::BeginningOfEndStep
    }

    fn clone_box(&self) -> Box<dyn GameEventType> {
        Box::new(self.clone())
    }

    fn affected_player(&self, _game: &GameState) -> PlayerId {
        self.player
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }

    fn display(&self) -> String {
        "Beginning of end step".to_string()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn object_id(&self) -> Option<ObjectId> {
        None
    }

    fn player(&self) -> Option<PlayerId> {
        Some(self.player)
    }

    fn controller(&self) -> Option<PlayerId> {
        None
    }

    fn snapshot(&self) -> Option<&ObjectSnapshot> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_beginning_of_end_step_event_creation() {
        let event = BeginningOfEndStepEvent::new(PlayerId::from_index(0));
        assert_eq!(event.player, PlayerId::from_index(0));
    }

    #[test]
    fn test_beginning_of_end_step_event_kind() {
        let event = BeginningOfEndStepEvent::new(PlayerId::from_index(0));
        assert_eq!(event.event_kind(), EventKind::BeginningOfEndStep);
    }
}
