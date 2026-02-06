//! Creature attacked event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;
use crate::triggers::event::AttackEventTarget;

/// A creature attacked event.
///
/// Triggered when a creature is declared as an attacker during the declare attackers step.
#[derive(Debug, Clone)]
pub struct CreatureAttackedEvent {
    /// The attacking creature
    pub attacker: ObjectId,
    /// What the creature is attacking (player or planeswalker)
    pub target: AttackEventTarget,
    /// Total number of attackers declared in this combat.
    ///
    /// This enables "attacks alone" semantics without depending on combat-state
    /// mutation timing at trigger-check time.
    pub total_attackers: usize,
}

impl CreatureAttackedEvent {
    /// Create a new creature attacked event.
    pub fn new(attacker: ObjectId, target: AttackEventTarget) -> Self {
        Self {
            attacker,
            target,
            total_attackers: 1,
        }
    }

    /// Create a new creature attacked event with an explicit attacker count.
    pub fn with_total_attackers(
        attacker: ObjectId,
        target: AttackEventTarget,
        total_attackers: usize,
    ) -> Self {
        Self {
            attacker,
            target,
            total_attackers,
        }
    }
}

impl GameEventType for CreatureAttackedEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::CreatureAttacked
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
        match self.target {
            AttackEventTarget::Player(_) => "Creature attacks player".to_string(),
            AttackEventTarget::Planeswalker(_) => "Creature attacks planeswalker".to_string(),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn object_id(&self) -> Option<ObjectId> {
        Some(self.attacker)
    }

    fn player(&self) -> Option<PlayerId> {
        match self.target {
            AttackEventTarget::Player(p) => Some(p),
            AttackEventTarget::Planeswalker(_) => None,
        }
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
    fn test_creature_attacked_event_creation() {
        let event = CreatureAttackedEvent::new(
            ObjectId::from_raw(1),
            AttackEventTarget::Player(PlayerId::from_index(0)),
        );
        assert_eq!(event.attacker, ObjectId::from_raw(1));
        assert_eq!(event.total_attackers, 1);
    }

    #[test]
    fn test_creature_attacked_event_kind() {
        let event = CreatureAttackedEvent::new(
            ObjectId::from_raw(1),
            AttackEventTarget::Player(PlayerId::from_index(0)),
        );
        assert_eq!(event.event_kind(), EventKind::CreatureAttacked);
    }
}
