//! Creature blocked event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;

/// A creature blocked event.
///
/// Triggered when a creature is declared as a blocker during the declare blockers step.
#[derive(Debug, Clone)]
pub struct CreatureBlockedEvent {
    /// The blocking creature
    pub blocker: ObjectId,
    /// The creature being blocked
    pub attacker: ObjectId,
}

impl CreatureBlockedEvent {
    /// Create a new creature blocked event.
    pub fn new(blocker: ObjectId, attacker: ObjectId) -> Self {
        Self { blocker, attacker }
    }
}

impl GameEventType for CreatureBlockedEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::CreatureBlocked
    }

    fn clone_box(&self) -> Box<dyn GameEventType> {
        Box::new(self.clone())
    }

    fn affected_player(&self, game: &GameState) -> PlayerId {
        game.object(self.blocker)
            .map(|o| o.controller)
            .unwrap_or(game.turn.active_player)
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }

    fn display(&self) -> String {
        "Creature blocks".to_string()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn object_id(&self) -> Option<ObjectId> {
        Some(self.blocker)
    }

    fn player(&self) -> Option<PlayerId> {
        None
    }

    fn controller(&self) -> Option<PlayerId> {
        None // Will be filled in when game state is available
    }

    fn snapshot(&self) -> Option<&ObjectSnapshot> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_creature_blocked_event_creation() {
        let event = CreatureBlockedEvent::new(ObjectId::from_raw(1), ObjectId::from_raw(2));
        assert_eq!(event.blocker, ObjectId::from_raw(1));
        assert_eq!(event.attacker, ObjectId::from_raw(2));
    }

    #[test]
    fn test_creature_blocked_event_kind() {
        let event = CreatureBlockedEvent::new(ObjectId::from_raw(1), ObjectId::from_raw(2));
        assert_eq!(event.event_kind(), EventKind::CreatureBlocked);
    }
}
