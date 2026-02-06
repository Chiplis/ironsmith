//! Game event types for triggers.
//!
//! This module contains helper types used by the trigger event system.

use crate::ids::{ObjectId, PlayerId};
/// Target of a damage event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamageEventTarget {
    Player(PlayerId),
    Object(ObjectId),
}

/// Target of an attack event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttackEventTarget {
    Player(PlayerId),
    Planeswalker(ObjectId),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_damage_event_target() {
        let player_target = DamageEventTarget::Player(PlayerId::from_index(0));
        let object_target = DamageEventTarget::Object(ObjectId::from_raw(1));
        assert_ne!(player_target, object_target);
    }

    #[test]
    fn test_attack_event_target() {
        let player_target = AttackEventTarget::Player(PlayerId::from_index(0));
        let planeswalker_target = AttackEventTarget::Planeswalker(ObjectId::from_raw(1));
        assert_ne!(player_target, planeswalker_target);
    }
}
