//! Object-became-attached event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::object::AttachmentTarget;
use crate::snapshot::ObjectSnapshot;

#[derive(Debug, Clone)]
pub struct ObjectBecameAttachedEvent {
    pub object: ObjectId,
    pub target: AttachmentTarget,
    pub controller: PlayerId,
    pub snapshot: Option<ObjectSnapshot>,
    pub target_snapshot: Option<ObjectSnapshot>,
}

impl ObjectBecameAttachedEvent {
    pub fn new(
        object: ObjectId,
        target: AttachmentTarget,
        controller: PlayerId,
        snapshot: Option<ObjectSnapshot>,
    ) -> Self {
        Self {
            object,
            target,
            controller,
            snapshot,
            target_snapshot: None,
        }
    }
}

impl ObjectBecameAttachedEvent {
    pub fn with_target_snapshot(mut self, snapshot: Option<ObjectSnapshot>) -> Self {
        self.target_snapshot = snapshot;
        self
    }
}

impl GameEventType for ObjectBecameAttachedEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::ObjectBecameAttached
    }

    fn affected_player(&self, _game: &GameState) -> PlayerId {
        self.controller
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }

    fn display(&self) -> String {
        "Object became attached".to_string()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn object_id(&self) -> Option<ObjectId> {
        Some(self.object)
    }

    fn controller(&self) -> Option<PlayerId> {
        Some(self.controller)
    }

    fn snapshot(&self) -> Option<&ObjectSnapshot> {
        self.snapshot.as_ref()
    }
}
