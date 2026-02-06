//! Tap and untap event implementations.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType, RedirectValidTypes, RedirectableTarget};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};

/// A tap event that can be processed through the replacement effect system.
#[derive(Debug, Clone)]
pub struct TapEvent {
    /// The permanent becoming tapped
    pub permanent: ObjectId,
}

impl TapEvent {
    /// Create a new tap event.
    pub fn new(permanent: ObjectId) -> Self {
        Self { permanent }
    }

    /// Return a new event with a different permanent.
    pub fn with_permanent(&self, permanent: ObjectId) -> Self {
        Self { permanent }
    }
}

impl GameEventType for TapEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::BecomeTapped
    }

    fn clone_box(&self) -> Box<dyn GameEventType> {
        Box::new(self.clone())
    }

    fn affected_player(&self, game: &GameState) -> PlayerId {
        game.object(self.permanent)
            .map(|o| o.controller)
            .unwrap_or(game.turn.active_player)
    }

    fn redirectable_targets(&self) -> Vec<RedirectableTarget> {
        vec![RedirectableTarget {
            target: Target::Object(self.permanent),
            description: "tap target",
            valid_redirect_types: RedirectValidTypes::ObjectsOnly,
        }]
    }

    fn with_target_replaced(&self, old: &Target, new: &Target) -> Option<Box<dyn GameEventType>> {
        if &Target::Object(self.permanent) != old {
            return None;
        }

        if let Target::Object(new_obj) = new {
            Some(Box::new(self.with_permanent(*new_obj)))
        } else {
            None
        }
    }

    fn source_object(&self) -> Option<ObjectId> {
        None
    }

    fn display(&self) -> String {
        "Become tapped".to_string()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// An untap event that can be processed through the replacement effect system.
#[derive(Debug, Clone)]
pub struct UntapEvent {
    /// The permanent becoming untapped
    pub permanent: ObjectId,
}

impl UntapEvent {
    /// Create a new untap event.
    pub fn new(permanent: ObjectId) -> Self {
        Self { permanent }
    }

    /// Return a new event with a different permanent.
    pub fn with_permanent(&self, permanent: ObjectId) -> Self {
        Self { permanent }
    }
}

impl GameEventType for UntapEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::BecomeUntapped
    }

    fn clone_box(&self) -> Box<dyn GameEventType> {
        Box::new(self.clone())
    }

    fn affected_player(&self, game: &GameState) -> PlayerId {
        game.object(self.permanent)
            .map(|o| o.controller)
            .unwrap_or(game.turn.active_player)
    }

    fn redirectable_targets(&self) -> Vec<RedirectableTarget> {
        vec![RedirectableTarget {
            target: Target::Object(self.permanent),
            description: "untap target",
            valid_redirect_types: RedirectValidTypes::ObjectsOnly,
        }]
    }

    fn with_target_replaced(&self, old: &Target, new: &Target) -> Option<Box<dyn GameEventType>> {
        if &Target::Object(self.permanent) != old {
            return None;
        }

        if let Target::Object(new_obj) = new {
            Some(Box::new(self.with_permanent(*new_obj)))
        } else {
            None
        }
    }

    fn source_object(&self) -> Option<ObjectId> {
        None
    }

    fn display(&self) -> String {
        "Become untapped".to_string()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tap_event_creation() {
        let event = TapEvent::new(ObjectId::from_raw(1));
        assert_eq!(event.permanent, ObjectId::from_raw(1));
    }

    #[test]
    fn test_tap_event_kind() {
        let event = TapEvent::new(ObjectId::from_raw(1));
        assert_eq!(event.event_kind(), EventKind::BecomeTapped);
    }

    #[test]
    fn test_untap_event_kind() {
        let event = UntapEvent::new(ObjectId::from_raw(1));
        assert_eq!(event.event_kind(), EventKind::BecomeUntapped);
    }

    #[test]
    fn test_tap_event_display() {
        let event = TapEvent::new(ObjectId::from_raw(1));
        assert_eq!(event.display(), "Become tapped");
    }

    #[test]
    fn test_untap_event_display() {
        let event = UntapEvent::new(ObjectId::from_raw(1));
        assert_eq!(event.display(), "Become untapped");
    }

    #[test]
    fn test_tap_event_redirect() {
        let event = TapEvent::new(ObjectId::from_raw(1));

        let old_target = Target::Object(ObjectId::from_raw(1));
        let new_target = Target::Object(ObjectId::from_raw(2));

        let replaced = event.with_target_replaced(&old_target, &new_target);
        assert!(replaced.is_some());

        let replaced = replaced.unwrap();
        let replaced_tap = replaced.as_any().downcast_ref::<TapEvent>().unwrap();
        assert_eq!(replaced_tap.permanent, ObjectId::from_raw(2));
    }
}
