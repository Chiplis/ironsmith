//! Sacrifice event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType, RedirectValidTypes, RedirectableTarget};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};

/// A sacrifice event that can be processed through the replacement effect system.
#[derive(Debug, Clone)]
pub struct SacrificeEvent {
    /// The permanent being sacrificed
    pub permanent: ObjectId,
    /// The source requiring the sacrifice
    pub source: Option<ObjectId>,
}

impl SacrificeEvent {
    /// Create a new sacrifice event.
    pub fn new(permanent: ObjectId, source: Option<ObjectId>) -> Self {
        Self { permanent, source }
    }

    /// Create a sacrifice event from a specific source.
    pub fn from_source(permanent: ObjectId, source: ObjectId) -> Self {
        Self {
            permanent,
            source: Some(source),
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

impl GameEventType for SacrificeEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::Sacrifice
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
        // Sacrifice typically can't be redirected (you sacrifice your own stuff)
        // but we include it for completeness - validation will reject invalid redirects
        vec![RedirectableTarget {
            target: Target::Object(self.permanent),
            description: "sacrifice target",
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
        "Sacrifice permanent".to_string()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sacrifice_event_creation() {
        let event = SacrificeEvent::new(ObjectId::from_raw(1), Some(ObjectId::from_raw(2)));

        assert_eq!(event.permanent, ObjectId::from_raw(1));
        assert_eq!(event.source, Some(ObjectId::from_raw(2)));
    }

    #[test]
    fn test_sacrifice_event_kind() {
        let event = SacrificeEvent::new(ObjectId::from_raw(1), None);
        assert_eq!(event.event_kind(), EventKind::Sacrifice);
    }

    #[test]
    fn test_sacrifice_event_display() {
        let event = SacrificeEvent::new(ObjectId::from_raw(1), None);
        assert_eq!(event.display(), "Sacrifice permanent");
    }
}
