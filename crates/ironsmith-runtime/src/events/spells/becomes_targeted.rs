//! Becomes-targeted event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};

/// A permanent became the target of a spell or ability.
#[derive(Debug, Clone)]
pub struct BecomesTargetedEvent {
    /// The object that became targeted.
    pub target: ObjectId,
    /// The spell or ability source that targeted it.
    pub source: ObjectId,
    /// The controller of the source.
    pub source_controller: PlayerId,
    /// Whether the source was an ability (`true`) or spell (`false`).
    pub by_ability: bool,
}

impl BecomesTargetedEvent {
    /// Create a new becomes-targeted event.
    pub fn new(
        target: ObjectId,
        source: ObjectId,
        source_controller: PlayerId,
        by_ability: bool,
    ) -> Self {
        Self {
            target,
            source,
            source_controller,
            by_ability,
        }
    }
}

impl GameEventType for BecomesTargetedEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::BecomesTargeted
    }

    fn affected_player(&self, game: &GameState) -> PlayerId {
        game.object(self.target)
            .map(|o| o.controller)
            .unwrap_or(self.source_controller)
    }

    fn with_target_replaced(&self, old: &Target, new: &Target) -> Option<Box<dyn GameEventType>> {
        if &Target::Object(self.target) != old {
            return None;
        }
        let Target::Object(new_target) = new else {
            return None;
        };
        Some(Box::new(Self {
            target: *new_target,
            source: self.source,
            source_controller: self.source_controller,
            by_ability: self.by_ability,
        }))
    }

    fn source_object(&self) -> Option<ObjectId> {
        Some(self.source)
    }

    fn display(&self) -> String {
        "Object became targeted".to_string()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn object_id(&self) -> Option<ObjectId> {
        Some(self.target)
    }

    fn player(&self) -> Option<PlayerId> {
        Some(self.source_controller)
    }

    fn controller(&self) -> Option<PlayerId> {
        Some(self.source_controller)
    }
}
