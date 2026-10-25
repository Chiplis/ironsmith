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
    /// The combat phase in which this declaration happened. Older/manual
    /// event constructors may omit this; production declarations retain it.
    pub combat_phase: Option<u32>,
    /// Snapshot of the blocking creature at declaration time.
    pub blocker_snapshot: Option<ObjectSnapshot>,
    /// Snapshot of the attacking creature at declaration time.
    pub attacker_snapshot: Option<ObjectSnapshot>,
    /// CR 509.3c-d / 509.4: the blocker was put onto the battlefield
    /// blocking. The attacker becomes blocked by it, but the blocker never
    /// blocked, so blocker-side "blocks" triggers don't fire.
    pub entered_blocking: bool,
    /// CR 509.3a-e: an effect made an existing creature block after blockers
    /// were declared; `blocker_was_blocking` says whether it was already a
    /// blocking creature (then generic "blocks" triggers don't fire again).
    pub added_by_effect: bool,
    pub blocker_was_blocking: bool,
}

impl CreatureBlockedEvent {
    /// Create a new creature blocked event.
    pub fn new(blocker: ObjectId, attacker: ObjectId) -> Self {
        Self {
            blocker,
            attacker,
            combat_phase: None,
            blocker_snapshot: None,
            attacker_snapshot: None,
            entered_blocking: false,
            added_by_effect: false,
            blocker_was_blocking: false,
        }
    }

    pub fn with_combat_phase(mut self, combat_phase: u32) -> Self {
        self.combat_phase = Some(combat_phase);
        self
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
            combat_phase: None,
            blocker_snapshot: Some(blocker_snapshot),
            attacker_snapshot: Some(attacker_snapshot),
            entered_blocking: false,
            added_by_effect: false,
            blocker_was_blocking: false,
        }
    }

    /// Mark a blocking relationship created after declaration (CR 509.3).
    pub fn after_declaration(mut self, entered_blocking: bool, blocker_was_blocking: bool) -> Self {
        self.entered_blocking = entered_blocking;
        self.added_by_effect = !entered_blocking;
        self.blocker_was_blocking = blocker_was_blocking;
        self
    }

    /// CR 509.3a: whether this event is the blocker's transition into being
    /// a blocking creature (for "whenever ... blocks" triggers).
    pub fn is_blocker_block_transition(&self, game: &GameState) -> bool {
        if self.entered_blocking {
            return false;
        }
        if self.added_by_effect {
            return !self.blocker_was_blocking;
        }
        crate::triggers::combat::is_first_blocked_attacker_for_blocker(
            game,
            self.blocker,
            self.attacker,
        )
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
