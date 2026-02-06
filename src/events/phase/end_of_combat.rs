//! End of combat event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;

/// End of combat event.
///
/// Triggered at the end of combat step.
#[derive(Debug, Clone)]
pub struct EndOfCombatEvent;

impl EndOfCombatEvent {
    /// Create a new end of combat event.
    pub fn new() -> Self {
        Self
    }
}

impl Default for EndOfCombatEvent {
    fn default() -> Self {
        Self::new()
    }
}

impl GameEventType for EndOfCombatEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::EndOfCombat
    }

    fn clone_box(&self) -> Box<dyn GameEventType> {
        Box::new(self.clone())
    }

    fn affected_player(&self, game: &GameState) -> PlayerId {
        game.turn.active_player
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }

    fn display(&self) -> String {
        "End of combat".to_string()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn object_id(&self) -> Option<ObjectId> {
        None
    }

    fn player(&self) -> Option<PlayerId> {
        None
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
    fn test_end_of_combat_event_creation() {
        let event = EndOfCombatEvent::new();
        assert_eq!(event.event_kind(), EventKind::EndOfCombat);
    }
}
