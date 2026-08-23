//! Creature became blocked event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;
use crate::triggers::AttackEventTarget;

/// A creature became blocked event.
///
/// Triggered when an attacking creature becomes blocked.
/// Distinct from CreatureBlockedEvent which fires for the blocker.
#[derive(Debug, Clone)]
pub struct CreatureBecameBlockedEvent {
    /// The attacking creature that became blocked
    pub attacker: ObjectId,
    /// Number of creatures currently blocking the attacker.
    pub blocker_count: u32,
    /// The creatures currently blocking the attacker.
    pub blockers: Vec<ObjectId>,
    /// What the attacker is attacking, if known at trigger generation time.
    pub attack_target: Option<AttackEventTarget>,
    /// Snapshot of the attacker at declaration time.
    pub attacker_snapshot: Option<ObjectSnapshot>,
    /// Snapshots of the blockers at declaration time.
    pub blocker_snapshots: Vec<ObjectSnapshot>,
}

impl CreatureBecameBlockedEvent {
    /// Create a new creature became blocked event.
    pub fn new(attacker: ObjectId, blocker_count: u32) -> Self {
        Self {
            attacker,
            blocker_count,
            blockers: Vec::new(),
            attack_target: None,
            attacker_snapshot: None,
            blocker_snapshots: Vec::new(),
        }
    }

    pub fn with_target(
        attacker: ObjectId,
        blocker_count: u32,
        attack_target: AttackEventTarget,
    ) -> Self {
        Self {
            attacker,
            blocker_count,
            blockers: Vec::new(),
            attack_target: Some(attack_target),
            attacker_snapshot: None,
            blocker_snapshots: Vec::new(),
        }
    }

    pub fn with_target_and_blockers(
        attacker: ObjectId,
        blockers: Vec<ObjectId>,
        attack_target: Option<AttackEventTarget>,
        attacker_snapshot: Option<ObjectSnapshot>,
        blocker_snapshots: Vec<ObjectSnapshot>,
    ) -> Self {
        Self {
            attacker,
            blocker_count: blockers.len() as u32,
            blockers,
            attack_target,
            attacker_snapshot,
            blocker_snapshots,
        }
    }
}

impl GameEventType for CreatureBecameBlockedEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::CreatureBecameBlocked
    }

    fn affected_player(&self, game: &GameState) -> PlayerId {
        game.object(self.attacker)
            .map(|o| game.controller_of(o))
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
        match self.attack_target {
            Some(AttackEventTarget::Player(player_id)) => Some(player_id),
            _ => None,
        }
    }

    fn controller(&self) -> Option<PlayerId> {
        None // Will be filled in when game state is available
    }

    fn snapshot(&self) -> Option<&ObjectSnapshot> {
        self.attacker_snapshot.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_creature_became_blocked_event_creation() {
        let event = CreatureBecameBlockedEvent::new(ObjectId::from_raw(1), 2);
        assert_eq!(event.attacker, ObjectId::from_raw(1));
        assert_eq!(event.blocker_count, 2);
        assert!(event.blockers.is_empty());
        assert_eq!(event.attack_target, None);
    }

    #[test]
    fn test_creature_became_blocked_event_kind() {
        let event = CreatureBecameBlockedEvent::new(ObjectId::from_raw(1), 1);
        assert_eq!(event.event_kind(), EventKind::CreatureBecameBlocked);
    }
}
