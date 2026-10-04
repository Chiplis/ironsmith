//! Spell-countered event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;

#[derive(Debug, Clone)]
pub struct SpellCounteredEvent {
    pub spell: ObjectId,
    pub controller: PlayerId,
    pub snapshot: Option<ObjectSnapshot>,
    pub cause: Option<crate::events::cause::EventCause>,
    pub complete_source_lookback: bool,
}

impl SpellCounteredEvent {
    pub fn new(spell: ObjectId, controller: PlayerId, snapshot: Option<ObjectSnapshot>) -> Self {
        Self {
            spell,
            controller,
            snapshot,
            cause: None,
            complete_source_lookback: false,
        }
    }
    pub fn with_complete_source_lookback(mut self) -> Self {
        self.complete_source_lookback = true; self
    }
    pub fn with_cause(mut self, cause: crate::events::cause::EventCause) -> Self {
        self.cause = Some(cause);
        self
    }
}

impl GameEventType for SpellCounteredEvent {
    fn cause(&self) -> Option<&crate::events::cause::EventCause> { self.cause.as_ref() }

    fn event_kind(&self) -> EventKind {
        EventKind::SpellCountered
    }

    fn affected_player(&self, _game: &GameState) -> PlayerId {
        self.controller
    }

    fn player(&self) -> Option<PlayerId> {
        Some(self.controller)
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }

    fn display(&self) -> String {
        "Spell was countered".to_string()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn source_object(&self) -> Option<ObjectId> {
        self.cause.as_ref().and_then(|cause| cause.source)
    }

    fn object_id(&self) -> Option<ObjectId> {
        Some(self.spell)
    }

    fn controller(&self) -> Option<PlayerId> {
        Some(self.controller)
    }

    fn snapshot(&self) -> Option<&ObjectSnapshot> {
        self.snapshot.as_ref()
    }
}
