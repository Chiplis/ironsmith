//! Creature became blocked event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;

/// A creature became blocked event.
///
/// Triggered when an attacking creature becomes blocked.
/// Distinct from CreatureBlockedEvent which fires for the blocker.
#[derive(Debug, Clone)]
pub struct CreatureBecameBlockedEvent {
    /// The attacking creature that became blocked
    pub attacker: ObjectId,
}

impl CreatureBecameBlockedEvent {
    /// Create a new creature became blocked event.
    pub fn new(attacker: ObjectId) -> Self {
        Self { attacker }
    }
}

impl GameEventType for CreatureBecameBlockedEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::CreatureBecameBlocked
    }

    fn clone_box(&self) -> Box<dyn GameEventType> {
        Box::new(self.clone())
    }

    fn affected_player(&self, game: &GameState) -> PlayerId {
        game.object(self.attacker)
            .map(|o| o.controller)
            .unwrap_or(game.turn.active_player)
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }

    fn display(&self) -> String {
        "Creature became blocked".to_string()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn object_id(&self) -> Option<ObjectId> {
        Some(self.attacker)
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
    fn test_creature_became_blocked_event_creation() {
        let event = CreatureBecameBlockedEvent::new(ObjectId::from_raw(1));
        assert_eq!(event.attacker, ObjectId::from_raw(1));
    }

    #[test]
    fn test_creature_became_blocked_event_kind() {
        let event = CreatureBecameBlockedEvent::new(ObjectId::from_raw(1));
        assert_eq!(event.event_kind(), EventKind::CreatureBecameBlocked);
    }
}
