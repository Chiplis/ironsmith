//! Permanent-phased-out event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;

#[derive(Debug, Clone)]
pub struct PermanentPhasedOutEvent {
    pub permanent: ObjectId,
    pub controller: PlayerId,
    pub snapshot: Option<ObjectSnapshot>,
}

impl PermanentPhasedOutEvent {
    pub fn new(
        permanent: ObjectId,
        controller: PlayerId,
        snapshot: Option<ObjectSnapshot>,
    ) -> Self {
        Self {
            permanent,
            controller,
            snapshot,
        }
    }
}

impl GameEventType for PermanentPhasedOutEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::PermanentPhasedOut
    }

    fn affected_player(&self, _game: &GameState) -> PlayerId {
        self.controller
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }

    fn display(&self) -> String {
        "Permanent phased out".to_string()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn object_id(&self) -> Option<ObjectId> {
        Some(self.permanent)
    }

    fn controller(&self) -> Option<PlayerId> {
        Some(self.controller)
    }

    fn snapshot(&self) -> Option<&ObjectSnapshot> {
        self.snapshot.as_ref()
    }
}
