//! Mutated event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;

/// A permanent mutated event.
#[derive(Debug, Clone)]
pub struct MutatedEvent {
    /// The mutated permanent.
    pub permanent: ObjectId,
    /// The controller of the mutated permanent.
    pub controller: PlayerId,
}

impl MutatedEvent {
    /// Create a new mutated event.
    pub fn new(permanent: ObjectId, controller: PlayerId) -> Self {
        Self {
            permanent,
            controller,
        }
    }
}

impl GameEventType for MutatedEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::Mutated
    }

    fn affected_player(&self, _game: &GameState) -> PlayerId {
        self.controller
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }

    fn display(&self) -> String {
        "Permanent mutated".to_string()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn object_id(&self) -> Option<ObjectId> {
        Some(self.permanent)
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
