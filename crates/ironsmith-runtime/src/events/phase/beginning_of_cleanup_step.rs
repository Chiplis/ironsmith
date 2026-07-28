//! Beginning of cleanup step event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;

/// Event emitted as a player's cleanup step begins.
#[derive(Debug, Clone)]
pub struct BeginningOfCleanupStepEvent {
    pub player: PlayerId,
}

impl BeginningOfCleanupStepEvent {
    pub fn new(player: PlayerId) -> Self {
        Self { player }
    }
}

impl GameEventType for BeginningOfCleanupStepEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::BeginningOfCleanupStep
    }

    fn affected_player(&self, _game: &GameState) -> PlayerId {
        self.player
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }

    fn display(&self) -> String {
        "Beginning of cleanup step".to_string()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn object_id(&self) -> Option<ObjectId> {
        None
    }

    fn player(&self) -> Option<PlayerId> {
        Some(self.player)
    }

    fn controller(&self) -> Option<PlayerId> {
        None
    }

    fn snapshot(&self) -> Option<&ObjectSnapshot> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleanup_event_carries_player_and_kind() {
        let player = PlayerId::from_index(0);
        let event = BeginningOfCleanupStepEvent::new(player);
        assert_eq!(event.player, player);
        assert_eq!(event.event_kind(), EventKind::BeginningOfCleanupStep);
    }
}
