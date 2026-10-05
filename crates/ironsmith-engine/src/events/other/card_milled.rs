//! Actual mill notification after replacement-aware movement (CR 701.17).
use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;
use std::any::Any;

#[derive(Debug, Clone)]
pub struct CardMilledEvent {
    pub player: PlayerId,
    /// Exact library incarnation selected by the mill instruction.
    pub original_card: ObjectId,
    /// Exact resulting incarnation; never follow a later blink or zone move.
    pub card: ObjectId,
    /// Completed-state characteristics only when the destination is public
    /// and the card is face up. Hidden destinations do not reveal a filter.
    pub snapshot: Option<ObjectSnapshot>,
}
impl GameEventType for CardMilledEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::CardMilled
    }
    fn affected_player(&self, _game: &GameState) -> PlayerId {
        self.player
    }
    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }
    fn display(&self) -> String {
        "A player mills a card".into()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn object_id(&self) -> Option<ObjectId> {
        self.snapshot.as_ref().map(|_| self.card)
    }
    fn player(&self) -> Option<PlayerId> {
        Some(self.player)
    }
    fn controller(&self) -> Option<PlayerId> {
        Some(self.player)
    }
    fn snapshot(&self) -> Option<&ObjectSnapshot> {
        self.snapshot.as_ref()
    }
}
