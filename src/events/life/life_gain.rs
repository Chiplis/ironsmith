//! Life gain event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType, RedirectValidTypes, RedirectableTarget};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};

/// A life gain event that can be processed through the replacement effect system.
#[derive(Debug, Clone)]
pub struct LifeGainEvent {
    /// The player gaining life
    pub player: PlayerId,
    /// Amount of life to gain
    pub amount: u32,
}

impl LifeGainEvent {
    /// Create a new life gain event.
    pub fn new(player: PlayerId, amount: u32) -> Self {
        Self { player, amount }
    }

    /// Return a new event with doubled life gain.
    pub fn doubled(&self) -> Self {
        Self {
            amount: self.amount.saturating_mul(2),
            ..self.clone()
        }
    }

    /// Return a new event with additional life gain.
    pub fn with_additional(&self, extra: u32) -> Self {
        Self {
            amount: self.amount.saturating_add(extra),
            ..self.clone()
        }
    }

    /// Return a new event with life gain set to a specific value.
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

impl GameEventType for LifeGainEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::LifeGain
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
            description: "life gain recipient",
            valid_redirect_types: RedirectValidTypes::PlayersOnly,
        }]
    }

    fn with_target_replaced(&self, old: &Target, new: &Target) -> Option<Box<dyn GameEventType>> {
        if &Target::Player(self.player) != old {
            return None;
        }

        // Life gain can only be redirected to players
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
        format!("Gain {} life", self.amount)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_life_gain_event_creation() {
        let event = LifeGainEvent::new(PlayerId::from_index(0), 5);
        assert_eq!(event.amount, 5);
    }

    #[test]
    fn test_life_gain_event_doubled() {
        let event = LifeGainEvent::new(PlayerId::from_index(0), 3);
        let doubled = event.doubled();
        assert_eq!(doubled.amount, 6);
    }

    #[test]
    fn test_life_gain_event_with_additional() {
        let event = LifeGainEvent::new(PlayerId::from_index(0), 3);
        let with_extra = event.with_additional(2);
        assert_eq!(with_extra.amount, 5);
    }

    #[test]
    fn test_life_gain_event_kind() {
        let event = LifeGainEvent::new(PlayerId::from_index(0), 5);
        assert_eq!(event.event_kind(), EventKind::LifeGain);
    }

    #[test]
    fn test_life_gain_redirect_to_player() {
        let event = LifeGainEvent::new(PlayerId::from_index(0), 5);

        let old_target = Target::Player(PlayerId::from_index(0));
        let new_target = Target::Player(PlayerId::from_index(1));

        let replaced = event.with_target_replaced(&old_target, &new_target);
        assert!(replaced.is_some());

        let replaced = replaced.unwrap();
        let replaced_gain = replaced.as_any().downcast_ref::<LifeGainEvent>().unwrap();
        assert_eq!(replaced_gain.player, PlayerId::from_index(1));
    }

    #[test]
    fn test_life_gain_redirect_to_object_fails() {
        let event = LifeGainEvent::new(PlayerId::from_index(0), 5);

        let old_target = Target::Player(PlayerId::from_index(0));
        let new_target = Target::Object(ObjectId::from_raw(1));

        let replaced = event.with_target_replaced(&old_target, &new_target);
        assert!(replaced.is_none());
    }

    #[test]
    fn test_life_gain_display() {
        let event = LifeGainEvent::new(PlayerId::from_index(0), 5);
        assert_eq!(event.display(), "Gain 5 life");
    }
}
