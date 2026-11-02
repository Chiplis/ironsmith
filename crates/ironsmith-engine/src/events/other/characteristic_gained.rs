//! A completed actual characteristic gain, separate from performed actions.
use std::any::Any;
use crate::events::{EventKind, GameEventType};
use crate::{GameState, ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;
use ironsmith_core::trigger_model::GainedCharacteristic;
#[derive(Debug, Clone)]
pub struct CharacteristicGainedEvent {
    pub permanent: ObjectId,
    pub characteristic: GainedCharacteristic,
    pub previous_snapshot: ObjectSnapshot,
    pub snapshot: ObjectSnapshot,
}
impl GameEventType for CharacteristicGainedEvent {
    fn event_kind(&self) -> EventKind { EventKind::CharacteristicGained }
    fn is_replacement_proposal(&self) -> bool { false }
    fn affected_player(&self, _: &GameState) -> PlayerId { self.snapshot.controller }
    fn object_id(&self) -> Option<ObjectId> { Some(self.permanent) }
    fn controller(&self) -> Option<PlayerId> { Some(self.snapshot.controller) }
    fn player(&self) -> Option<PlayerId> { Some(self.snapshot.controller) }
    fn snapshot(&self) -> Option<&ObjectSnapshot> { Some(&self.snapshot) }
    fn as_any(&self) -> &dyn Any { self }
    fn display(&self) -> String { format!("Permanent gained {:?}", self.characteristic) }
}
