//! Life loss event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType, RedirectValidTypes, RedirectableTarget};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};

/// A life loss event that can be processed through the replacement effect system.
///
/// Note: Life loss from damage is handled separately through the damage event system.
/// This event is for life loss from effects like "lose 2 life" or payments.
#[derive(Debug, Clone)]
pub struct LifeLossEvent {
    /// The player losing life
    pub player: PlayerId,
    /// Amount of life to lose
    pub amount: u32,
    /// Whether this loss is from damage (false = payment or effect)
    pub from_damage: bool,
}

impl LifeLossEvent {
    /// Create a new life loss event.
    pub fn new(player: PlayerId, amount: u32, from_damage: bool) -> Self {
        Self {
            player,
            amount,
            from_damage,
        }
    }

    /// Create a life loss event from a non-damage effect.
    pub fn from_effect(player: PlayerId, amount: u32) -> Self {
        Self::new(player, amount, false)
    }

    /// Return a new event with reduced life loss.
    pub fn reduced(&self, by: u32) -> Self {
        Self {
            amount: self.amount.saturating_sub(by),
            ..self.clone()
        }
    }

    /// Return a new event with life loss set to a specific value.
    pub fn with_amount(&self, amount: u32) -> Self {
        Self {
            amount,
            ..self.clone()
        }
    }

    /// Return a new event with a different player.
    pub fn with_player(&self, player: PlayerId) -> Self {
        Self {
            player,
            ..self.clone()
        }
    }
}

impl GameEventType for LifeLossEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::LifeLoss
    }

    fn clone_box(&self) -> Box<dyn GameEventType> {
        Box::new(self.clone())
    }

    fn affected_player(&self, _game: &GameState) -> PlayerId {
        self.player
    }

    fn redirectable_targets(&self) -> Vec<RedirectableTarget> {
        vec![RedirectableTarget {
            target: Target::Player(self.player),
            description: "life loss target",
            valid_redirect_types: RedirectValidTypes::PlayersOnly,
        }]
    }

    fn with_target_replaced(&self, old: &Target, new: &Target) -> Option<Box<dyn GameEventType>> {
        if &Target::Player(self.player) != old {
            return None;
        }

        if let Target::Player(new_player) = new {
            Some(Box::new(self.with_player(*new_player)))
        } else {
            None
        }
    }

    fn source_object(&self) -> Option<ObjectId> {
        None
    }

    fn display(&self) -> String {
        format!("Lose {} life", self.amount)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_life_loss_event_creation() {
        let event = LifeLossEvent::from_effect(PlayerId::from_index(0), 5);
        assert_eq!(event.amount, 5);
        assert!(!event.from_damage);
    }

    #[test]
    fn test_life_loss_event_reduced() {
        let event = LifeLossEvent::from_effect(PlayerId::from_index(0), 5);
        let reduced = event.reduced(3);
        assert_eq!(reduced.amount, 2);
    }

    #[test]
    fn test_life_loss_event_kind() {
        let event = LifeLossEvent::from_effect(PlayerId::from_index(0), 5);
        assert_eq!(event.event_kind(), EventKind::LifeLoss);
    }

    #[test]
    fn test_life_loss_display() {
        let event = LifeLossEvent::from_effect(PlayerId::from_index(0), 3);
        assert_eq!(event.display(), "Lose 3 life");
    }
}
