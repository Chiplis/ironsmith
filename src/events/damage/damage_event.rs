//! Damage event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType, RedirectValidTypes, RedirectableTarget};
use crate::game_event::DamageTarget;
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};

/// A damage event that can be processed through the replacement effect system.
#[derive(Debug, Clone)]
pub struct DamageEvent {
    /// The source of the damage (creature, spell, etc.)
    pub source: ObjectId,
    /// The target of the damage
    pub target: DamageTarget,
    /// The amount of damage to deal
    pub amount: u32,
    /// Whether this is combat damage
    pub is_combat: bool,
    /// Whether this damage cannot be prevented
    pub is_unpreventable: bool,
}

impl DamageEvent {
    /// Create a new damage event.
    pub fn new(source: ObjectId, target: DamageTarget, amount: u32, is_combat: bool) -> Self {
        Self {
            source,
            target,
            amount,
            is_combat,
            is_unpreventable: false,
        }
    }

    /// Create a new damage event that cannot be prevented.
    pub fn unpreventable(
        source: ObjectId,
        target: DamageTarget,
        amount: u32,
        is_combat: bool,
    ) -> Self {
        Self {
            source,
            target,
            amount,
            is_combat,
            is_unpreventable: true,
        }
    }

    /// Return a new event with doubled damage.
    pub fn doubled(&self) -> Self {
        Self {
            amount: self.amount.saturating_mul(2),
            ..self.clone()
        }
    }

    /// Return a new event with damage reduced by the given amount.
    pub fn reduced(&self, by: u32) -> Self {
        Self {
            amount: self.amount.saturating_sub(by),
            ..self.clone()
        }
    }

    /// Return a new event with damage set to a specific value.
    pub fn with_amount(&self, amount: u32) -> Self {
        Self {
            amount,
            ..self.clone()
        }
    }

    /// Return a new event with damage prevented (set to 0).
    pub fn prevented(&self) -> Self {
        Self {
            amount: 0,
            ..self.clone()
        }
    }

    /// Return a new event with a different target.
    pub fn with_target(&self, target: DamageTarget) -> Self {
        Self {
            target,
            ..self.clone()
        }
    }
}

impl GameEventType for DamageEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::Damage
    }

    fn clone_box(&self) -> Box<dyn GameEventType> {
        Box::new(self.clone())
    }

    fn affected_player(&self, game: &GameState) -> PlayerId {
        match self.target {
            DamageTarget::Player(player) => player,
            DamageTarget::Object(obj_id) => game
                .object(obj_id)
                .map(|o| o.controller)
                .unwrap_or(game.turn.active_player),
        }
    }

    fn redirectable_targets(&self) -> Vec<RedirectableTarget> {
        vec![RedirectableTarget {
            target: match self.target {
                DamageTarget::Player(p) => Target::Player(p),
                DamageTarget::Object(o) => Target::Object(o),
            },
            description: "damage target",
            valid_redirect_types: RedirectValidTypes::PlayersOrObjects,
        }]
    }

    fn with_target_replaced(&self, old: &Target, new: &Target) -> Option<Box<dyn GameEventType>> {
        let current = match self.target {
            DamageTarget::Player(p) => Target::Player(p),
            DamageTarget::Object(o) => Target::Object(o),
        };

        if &current != old {
            return None;
        }

        let new_target = match new {
            Target::Player(p) => DamageTarget::Player(*p),
            Target::Object(o) => DamageTarget::Object(*o),
        };

        Some(Box::new(self.with_target(new_target)))
    }

    fn source_object(&self) -> Option<ObjectId> {
        Some(self.source)
    }

    fn display(&self) -> String {
        let target_str = match self.target {
            DamageTarget::Player(_) => "player",
            DamageTarget::Object(_) => "permanent",
        };
        let combat_str = if self.is_combat { "combat " } else { "" };
        format!(
            "Deal {} {}damage to {}",
            self.amount, combat_str, target_str
        )
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_damage_event_creation() {
        let event = DamageEvent::new(
            ObjectId::from_raw(1),
            DamageTarget::Player(PlayerId::from_index(0)),
            3,
            false,
        );

        assert_eq!(event.amount, 3);
        assert!(!event.is_combat);
        assert!(!event.is_unpreventable);
    }

    #[test]
    fn test_damage_event_doubled() {
        let event = DamageEvent::new(
            ObjectId::from_raw(1),
            DamageTarget::Player(PlayerId::from_index(0)),
            3,
            false,
        );

        let doubled = event.doubled();
        assert_eq!(doubled.amount, 6);
    }

    #[test]
    fn test_damage_event_reduced() {
        let event = DamageEvent::new(
            ObjectId::from_raw(1),
            DamageTarget::Player(PlayerId::from_index(0)),
            5,
            false,
        );

        let reduced = event.reduced(3);
        assert_eq!(reduced.amount, 2);

        // Test underflow protection
        let reduced_more = event.reduced(10);
        assert_eq!(reduced_more.amount, 0);
    }

    #[test]
    fn test_damage_event_prevented() {
        let event = DamageEvent::new(
            ObjectId::from_raw(1),
            DamageTarget::Player(PlayerId::from_index(0)),
            5,
            false,
        );

        let prevented = event.prevented();
        assert_eq!(prevented.amount, 0);
    }

    #[test]
    fn test_damage_event_kind() {
        let event = DamageEvent::new(
            ObjectId::from_raw(1),
            DamageTarget::Player(PlayerId::from_index(0)),
            3,
            false,
        );

        assert_eq!(event.event_kind(), EventKind::Damage);
    }

    #[test]
    fn test_damage_event_redirectable_targets() {
        let event = DamageEvent::new(
            ObjectId::from_raw(1),
            DamageTarget::Object(ObjectId::from_raw(2)),
            3,
            true,
        );

        let targets = event.redirectable_targets();
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].description, "damage target");
        assert_eq!(
            targets[0].valid_redirect_types,
            RedirectValidTypes::PlayersOrObjects
        );
    }

    #[test]
    fn test_damage_event_with_target_replaced() {
        let event = DamageEvent::new(
            ObjectId::from_raw(1),
            DamageTarget::Player(PlayerId::from_index(0)),
            3,
            false,
        );

        let old_target = Target::Player(PlayerId::from_index(0));
        let new_target = Target::Player(PlayerId::from_index(1));

        let replaced = event.with_target_replaced(&old_target, &new_target);
        assert!(replaced.is_some());

        let replaced = replaced.unwrap();
        let replaced_damage = replaced.as_any().downcast_ref::<DamageEvent>().unwrap();
        assert_eq!(
            replaced_damage.target,
            DamageTarget::Player(PlayerId::from_index(1))
        );
    }

    #[test]
    fn test_damage_event_display() {
        let event = DamageEvent::new(
            ObjectId::from_raw(1),
            DamageTarget::Player(PlayerId::from_index(0)),
            3,
            false,
        );
        assert_eq!(event.display(), "Deal 3 damage to player");

        let combat_event = DamageEvent::new(
            ObjectId::from_raw(1),
            DamageTarget::Object(ObjectId::from_raw(2)),
            5,
            true,
        );
        assert_eq!(combat_event.display(), "Deal 5 combat damage to permanent");
    }

    #[test]
    fn test_damage_event_source_object() {
        let source = ObjectId::from_raw(42);
        let event = DamageEvent::new(
            source,
            DamageTarget::Player(PlayerId::from_index(0)),
            3,
            false,
        );

        assert_eq!(event.source_object(), Some(source));
    }
}
