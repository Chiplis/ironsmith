//! Successful spell completion, independent of casting and zone movement.
use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;
use std::any::Any;
#[derive(Debug, Clone)]
pub struct SpellResolvedEvent {
    pub spell: ObjectId,
    pub controller: PlayerId,
    /// Exact resolving incarnation, retained before its instructions can move it.
    pub snapshot: ObjectSnapshot,
}
impl SpellResolvedEvent {
    pub fn new(spell: ObjectId, controller: PlayerId, snapshot: ObjectSnapshot) -> Self {
        Self {
            spell,
            controller,
            snapshot,
        }
    }
}
impl GameEventType for SpellResolvedEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::SpellResolved
    }
    fn affected_player(&self, _: &GameState) -> PlayerId {
        self.controller
    }
    fn with_target_replaced(&self, _: &Target, _: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }
    fn display(&self) -> String {
        format!(
            "Spell resolved under player {}'s control",
            self.controller.0
        )
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn object_id(&self) -> Option<ObjectId> {
        Some(self.spell)
    }
    fn player(&self) -> Option<PlayerId> {
        Some(self.controller)
    }
    fn controller(&self) -> Option<PlayerId> {
        Some(self.controller)
    }
    fn snapshot(&self) -> Option<&ObjectSnapshot> {
        Some(&self.snapshot)
    }
}
