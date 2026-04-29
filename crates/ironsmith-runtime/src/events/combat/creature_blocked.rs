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
    /// Snapshot of the blocking creature at declaration time.
    pub blocker_snapshot: Option<ObjectSnapshot>,
    /// Snapshot of the attacking creature at declaration time.
    pub attacker_snapshot: Option<ObjectSnapshot>,
}

impl CreatureBlockedEvent {
    /// Create a new creature blocked event.
    pub fn new(blocker: ObjectId, attacker: ObjectId) -> Self {
        Self {
            blocker,
            attacker,
            blocker_snapshot: None,
            attacker_snapshot: None,
        }
    }

    pub fn with_snapshots(
        blocker: ObjectId,
        attacker: ObjectId,
        blocker_snapshot: ObjectSnapshot,
        attacker_snapshot: ObjectSnapshot,
    ) -> Self {
        Self {
            blocker,
            attacker,
            blocker_snapshot: Some(blocker_snapshot),
            attacker_snapshot: Some(attacker_snapshot),
        }
    }
}

impl GameEventType for CreatureBlockedEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::CreatureBlocked
    }

    fn affected_player(&self, game: &GameState) -> PlayerId {
        game.object(self.blocker)
            .map(|o| game.controller_of(o))
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
        self.blocker_snapshot.as_ref()
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
