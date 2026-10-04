//! A completed voluntary life payment, distinct from ordinary life loss.
use crate::events::{EventKind, GameEventType};
use crate::{GameState, PlayerId, Target};
use std::any::Any;
#[derive(Debug, Clone)]
pub struct LifePaidEvent {
    pub player: PlayerId,
    pub amount: u32,
}
impl GameEventType for LifePaidEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::LifePaid
    }
    fn is_replacement_proposal(&self) -> bool {
        false
    }
    fn affected_player(&self, _: &GameState) -> PlayerId {
        self.player
    }
    fn player(&self) -> Option<PlayerId> {
        Some(self.player)
    }
    fn with_target_replaced(&self, _: &Target, _: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }
    fn display(&self) -> String {
        format!("Paid {} life", self.amount)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
