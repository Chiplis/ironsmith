//! Completed monarch designation, distinct from a request or unchanged holder.
use crate::events::{EventKind, GameEventType};
use crate::{GameState, PlayerId};
use std::any::Any;
#[derive(Debug, Clone)]
pub struct MonarchChangedEvent {
    pub previous: Option<PlayerId>,
    pub monarch: PlayerId,
}
impl GameEventType for MonarchChangedEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::MonarchChanged
    }
    fn is_replacement_proposal(&self) -> bool {
        false
    }
    fn affected_player(&self, _: &GameState) -> PlayerId {
        self.monarch
    }
    fn player(&self) -> Option<PlayerId> {
        Some(self.monarch)
    }
    fn display(&self) -> String {
        format!("Player {} became the monarch", self.monarch.0)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
