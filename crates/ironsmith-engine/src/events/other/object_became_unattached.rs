//! Object-became-unattached event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::object::AttachmentTarget;
use crate::snapshot::ObjectSnapshot;

#[derive(Debug, Clone)]
pub struct ObjectBecameUnattachedEvent {
    pub object: ObjectId,
    pub previous_target: AttachmentTarget,
    pub controller: PlayerId,
    pub snapshot: Option<ObjectSnapshot>,
}

impl ObjectBecameUnattachedEvent {
    pub fn new(
        object: ObjectId,
        previous_target: AttachmentTarget,
        controller: PlayerId,
        snapshot: Option<ObjectSnapshot>,
    ) -> Self {
        Self {
            object,
            previous_target,
            controller,
            snapshot,
        }
    }
}

impl GameEventType for ObjectBecameUnattachedEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::ObjectBecameUnattached
    }

    fn affected_player(&self, _game: &GameState) -> PlayerId {
        self.controller
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }

    fn display(&self) -> String {
        "Object became unattached".to_string()
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
