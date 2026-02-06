//! Became monstrous event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;

/// A creature became monstrous event.
///
/// Triggered when a creature's monstrosity ability resolves.
#[derive(Debug, Clone)]
pub struct BecameMonstrousEvent {
    /// The creature that became monstrous
    pub creature: ObjectId,
    /// The controller of the creature
    pub controller: PlayerId,
    /// The N value from the monstrosity ability (number of +1/+1 counters)
    pub n: u32,
}

impl BecameMonstrousEvent {
    /// Create a new became monstrous event.
    pub fn new(creature: ObjectId, controller: PlayerId, n: u32) -> Self {
        Self {
            creature,
            controller,
            n,
        }
    }
}

impl GameEventType for BecameMonstrousEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::BecameMonstrous
    }

    fn clone_box(&self) -> Box<dyn GameEventType> {
        Box::new(self.clone())
    }

    fn affected_player(&self, _game: &GameState) -> PlayerId {
        self.controller
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }

    fn display(&self) -> String {
        format!("Creature became monstrous ({})", self.n)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn object_id(&self) -> Option<ObjectId> {
        Some(self.creature)
    }

    fn player(&self) -> Option<PlayerId> {
        Some(self.controller)
    }

    fn controller(&self) -> Option<PlayerId> {
        Some(self.controller)
    }

    fn snapshot(&self) -> Option<&ObjectSnapshot> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_became_monstrous_event_creation() {
        let event = BecameMonstrousEvent::new(ObjectId::from_raw(1), PlayerId::from_index(0), 3);
        assert_eq!(event.creature, ObjectId::from_raw(1));
        assert_eq!(event.controller, PlayerId::from_index(0));
        assert_eq!(event.n, 3);
    }

    #[test]
    fn test_became_monstrous_event_kind() {
        let event = BecameMonstrousEvent::new(ObjectId::from_raw(1), PlayerId::from_index(0), 3);
        assert_eq!(event.event_kind(), EventKind::BecameMonstrous);
    }
}
