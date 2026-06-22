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
}

impl SpellCounteredEvent {
    pub fn new(spell: ObjectId, controller: PlayerId, snapshot: Option<ObjectSnapshot>) -> Self {
        Self {
            spell,
            controller,
            snapshot,
        }
    }
}

impl GameEventType for SpellCounteredEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::SpellCountered
    }

    fn affected_player(&self, _game: &GameState) -> PlayerId {
        self.controller
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
