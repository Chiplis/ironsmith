//! A completed insertion of a spell or ability with its announced targets.
use std::any::Any;
use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, StackEntry, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;
#[derive(Debug, Clone)]
pub struct StackObjectAddedEvent {
    pub stack_id: ObjectId,
    pub source: ObjectId,
    pub controller: PlayerId,
    pub snapshot: Option<ObjectSnapshot>,
}
impl StackObjectAddedEvent {
    pub fn from_entry(entry: &StackEntry) -> Self {
        let stack_id = entry.target_id();
        let mut snapshot = entry.source_snapshot.clone();
        if let Some(snapshot) = &mut snapshot {
            snapshot.object_id = stack_id;
            snapshot.zone = crate::zone::Zone::Stack;
            snapshot.controller = entry.controller;
        }
        Self { stack_id, source: entry.object_id, controller: entry.controller, snapshot }
    }
}
impl GameEventType for StackObjectAddedEvent {
    fn event_kind(&self) -> EventKind { EventKind::StackObjectAdded }
    fn affected_player(&self, _: &GameState) -> PlayerId { self.controller }
    fn with_target_replaced(&self, _: &Target, _: &Target) -> Option<Box<dyn GameEventType>> { None }
    fn display(&self) -> String { "A spell or ability was put onto the stack".into() }
    fn as_any(&self) -> &dyn Any { self }
    fn object_id(&self) -> Option<ObjectId> { Some(self.stack_id) }
    fn player(&self) -> Option<PlayerId> { Some(self.controller) }
    fn controller(&self) -> Option<PlayerId> { Some(self.controller) }
    fn source_object(&self) -> Option<ObjectId> { Some(self.source) }
    fn snapshot(&self) -> Option<&ObjectSnapshot> { self.snapshot.as_ref() }
}
