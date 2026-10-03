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
    /// What the creature is attacking (player, planeswalker, or battle)
    pub target: AttackEventTarget,
    /// Total number of attackers declared in this combat.
    ///
    /// This enables "attacks alone" semantics without depending on combat-state
    /// mutation timing at trigger-check time.
    pub total_attackers: usize,
    /// Every creature declared as an attacker in the same declaration
    /// (CR 508.1), in declaration order. "One or more … attack" and
    /// aggregate ("attack with creatures with total power …") matchers read
    /// the declaration from here rather than from the live combat, which also
    /// holds creatures put onto the battlefield attacking (CR 508.4) and, when
    /// this event is replayed from turn history, a later combat's attackers.
    pub declared_attackers: Option<std::sync::Arc<[crate::combat_state::AttackerInfo]>>,
    /// The turn-local combat phase in which this attack was declared. The
    /// history must not mistake an earlier combat's attack for this combat.
    pub combat_phase: Option<u32>,
}

impl CreatureAttackedEvent {
    /// Create a new creature attacked event.
    pub fn new(attacker: ObjectId, target: AttackEventTarget) -> Self {
        Self {
            attacker,
            target,
            total_attackers: 1,
            declared_attackers: None,
            combat_phase: None,
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
            declared_attackers: None,
            combat_phase: None,
        }
    }

    /// Identify the combat phase that owns this historical declaration.
    pub fn with_combat_phase(mut self, combat_phase: u32) -> Self {
        self.combat_phase = Some(combat_phase);
        self
    }

    /// Attach the full attack declaration this event belongs to.
    pub fn with_declared_attackers(
        mut self,
        declared_attackers: std::sync::Arc<[crate::combat_state::AttackerInfo]>,
    ) -> Self {
        self.declared_attackers = Some(declared_attackers);
        self
    }
}

impl GameEventType for CreatureAttackedEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::CreatureAttacked
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
        match self.target {
            AttackEventTarget::Player(_) => "Creature attacks player".to_string(),
            AttackEventTarget::Planeswalker(_) => "Creature attacks planeswalker".to_string(),
            AttackEventTarget::Battle(_) => "Creature attacks battle".to_string(),
            AttackEventTarget::Nothing => "Creature attacks".to_string(),
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
            AttackEventTarget::Planeswalker(_)
            | AttackEventTarget::Battle(_)
            | AttackEventTarget::Nothing => None,
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
