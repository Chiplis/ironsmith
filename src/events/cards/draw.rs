//! Draw event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType, RedirectValidTypes, RedirectableTarget};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};

/// A draw event that can be processed through the replacement effect system.
#[derive(Debug, Clone)]
pub struct DrawEvent {
    /// The player drawing
    pub player: PlayerId,
    /// Number of cards to draw
    pub count: u32,
    /// Whether this is the first card drawn this turn
    pub is_first_this_turn: bool,
}

impl DrawEvent {
    /// Create a new draw event.
    pub fn new(player: PlayerId, count: u32, is_first_this_turn: bool) -> Self {
        Self {
            player,
            count,
            is_first_this_turn,
        }
    }

    /// Return a new event with doubled draw count.
    pub fn doubled(&self) -> Self {
        Self {
            count: self.count.saturating_mul(2),
            ..self.clone()
        }
    }

    /// Return a new event with additional draws.
    pub fn with_additional(&self, extra: u32) -> Self {
        Self {
            count: self.count.saturating_add(extra),
            ..self.clone()
        }
    }

    /// Return a new event with a different count.
    pub fn with_count(&self, count: u32) -> Self {
        Self {
            count,
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

impl GameEventType for DrawEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::Draw
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
            description: "draw target",
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
        if self.count == 1 {
            "Draw a card".to_string()
        } else {
            format!("Draw {} cards", self.count)
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_draw_event_creation() {
        let event = DrawEvent::new(PlayerId::from_index(0), 3, false);

        assert_eq!(event.count, 3);
        assert!(!event.is_first_this_turn);
    }

    #[test]
    fn test_draw_event_doubled() {
        let event = DrawEvent::new(PlayerId::from_index(0), 3, false);
        let doubled = event.doubled();
        assert_eq!(doubled.count, 6);
    }

    #[test]
    fn test_draw_event_with_additional() {
        let event = DrawEvent::new(PlayerId::from_index(0), 3, false);
        let with_extra = event.with_additional(2);
        assert_eq!(with_extra.count, 5);
    }

    #[test]
    fn test_draw_event_kind() {
        let event = DrawEvent::new(PlayerId::from_index(0), 1, true);
        assert_eq!(event.event_kind(), EventKind::Draw);
    }

    #[test]
    fn test_draw_event_display() {
        let event1 = DrawEvent::new(PlayerId::from_index(0), 1, false);
        assert_eq!(event1.display(), "Draw a card");

        let event2 = DrawEvent::new(PlayerId::from_index(0), 3, false);
        assert_eq!(event2.display(), "Draw 3 cards");
    }

    #[test]
    fn test_draw_event_redirect() {
        let event = DrawEvent::new(PlayerId::from_index(0), 2, true);

        let old_target = Target::Player(PlayerId::from_index(0));
        let new_target = Target::Player(PlayerId::from_index(1));

        let replaced = event.with_target_replaced(&old_target, &new_target);
        assert!(replaced.is_some());

        let replaced = replaced.unwrap();
        let replaced_draw = replaced.as_any().downcast_ref::<DrawEvent>().unwrap();
        assert_eq!(replaced_draw.player, PlayerId::from_index(1));
        assert!(replaced_draw.is_first_this_turn);
    }
}
