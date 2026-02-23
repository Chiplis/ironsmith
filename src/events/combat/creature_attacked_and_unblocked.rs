//! Creature attacked and unblocked event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;
use crate::triggers::event::AttackEventTarget;

/// A creature attacked and was unblocked event.
///
/// Triggered after blockers are declared for each attacking creature that ended up unblocked.
#[derive(Debug, Clone)]
pub struct CreatureAttackedAndUnblockedEvent {
    /// The unblocked attacking creature.
    pub attacker: ObjectId,
    /// What the creature is attacking (player or planeswalker).
    pub target: AttackEventTarget,
}

impl CreatureAttackedAndUnblockedEvent {
    pub fn new(attacker: ObjectId, target: AttackEventTarget) -> Self {
        Self { attacker, target }
    }
}

impl GameEventType for CreatureAttackedAndUnblockedEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::CreatureAttackedAndUnblocked
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
        "Creature attacks and isn't blocked".to_string()
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
    fn event_kind_is_correct() {
        let event = CreatureAttackedAndUnblockedEvent::new(
            ObjectId::from_raw(1),
            AttackEventTarget::Player(PlayerId::from_index(0)),
        );
        assert_eq!(event.event_kind(), EventKind::CreatureAttackedAndUnblocked);
    }
}

