//! Destroy event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType, RedirectValidTypes, RedirectableTarget};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};

/// A destroy event that can be processed through the replacement effect system.
#[derive(Debug, Clone)]
pub struct DestroyEvent {
    /// The permanent being destroyed
    pub permanent: ObjectId,
    /// The source causing the destruction (may be None for SBA destruction)
    pub source: Option<ObjectId>,
}

impl DestroyEvent {
    /// Create a new destroy event.
    pub fn new(permanent: ObjectId, source: Option<ObjectId>) -> Self {
        Self { permanent, source }
    }

    /// Create a destroy event from a specific source.
    pub fn from_source(permanent: ObjectId, source: ObjectId) -> Self {
        Self {
            permanent,
            source: Some(source),
        }
    }

    /// Create a destroy event from state-based actions.
    pub fn from_sba(permanent: ObjectId) -> Self {
        Self {
            permanent,
            source: None,
        }
    }

    /// Return a new event with a different permanent.
    pub fn with_permanent(&self, permanent: ObjectId) -> Self {
        Self {
            permanent,
            source: self.source,
        }
    }
}

impl GameEventType for DestroyEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::Destroy
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
            description: "destruction target",
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
        self.source
    }

    fn display(&self) -> String {
        "Destroy permanent".to_string()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_destroy_event_creation() {
        let event = DestroyEvent::new(ObjectId::from_raw(1), Some(ObjectId::from_raw(2)));

        assert_eq!(event.permanent, ObjectId::from_raw(1));
        assert_eq!(event.source, Some(ObjectId::from_raw(2)));
    }

    #[test]
    fn test_destroy_event_from_sba() {
        let event = DestroyEvent::from_sba(ObjectId::from_raw(1));

        assert_eq!(event.permanent, ObjectId::from_raw(1));
        assert!(event.source.is_none());
    }

    #[test]
    fn test_destroy_event_kind() {
        let event = DestroyEvent::new(ObjectId::from_raw(1), None);
        assert_eq!(event.event_kind(), EventKind::Destroy);
    }

    #[test]
    fn test_destroy_event_source_object() {
        let event_with_source =
            DestroyEvent::from_source(ObjectId::from_raw(1), ObjectId::from_raw(2));
        assert_eq!(
            event_with_source.source_object(),
            Some(ObjectId::from_raw(2))
        );

        let event_without_source = DestroyEvent::from_sba(ObjectId::from_raw(1));
        assert!(event_without_source.source_object().is_none());
    }

    #[test]
    fn test_destroy_event_redirect() {
        let event = DestroyEvent::from_source(ObjectId::from_raw(1), ObjectId::from_raw(10));

        let old_target = Target::Object(ObjectId::from_raw(1));
        let new_target = Target::Object(ObjectId::from_raw(2));

        let replaced = event.with_target_replaced(&old_target, &new_target);
        assert!(replaced.is_some());

        let replaced = replaced.unwrap();
        let replaced_destroy = replaced.as_any().downcast_ref::<DestroyEvent>().unwrap();
        assert_eq!(replaced_destroy.permanent, ObjectId::from_raw(2));
        // Source should be preserved
        assert_eq!(replaced_destroy.source, Some(ObjectId::from_raw(10)));
    }

    #[test]
    fn test_destroy_event_display() {
        let event = DestroyEvent::new(ObjectId::from_raw(1), None);
        assert_eq!(event.display(), "Destroy permanent");
    }
}
