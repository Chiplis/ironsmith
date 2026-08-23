//! Saga chapter ability resolution event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};

/// A Saga chapter ability resolved.
#[derive(Debug, Clone)]
pub struct ChapterAbilityResolvedEvent {
    pub saga: ObjectId,
    pub controller: PlayerId,
    pub final_chapter: bool,
}

impl ChapterAbilityResolvedEvent {
    pub fn new(saga: ObjectId, controller: PlayerId, final_chapter: bool) -> Self {
        Self {
            saga,
            controller,
            final_chapter,
        }
    }
}

impl GameEventType for ChapterAbilityResolvedEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::ChapterAbilityResolved
    }

    fn affected_player(&self, _game: &GameState) -> PlayerId {
        self.controller
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }

    fn display(&self) -> String {
        if self.final_chapter {
            "Final chapter ability resolved".to_string()
        } else {
            "Chapter ability resolved".to_string()
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn object_id(&self) -> Option<ObjectId> {
        Some(self.saga)
    }

    fn controller(&self) -> Option<PlayerId> {
        Some(self.controller)
    }
}
