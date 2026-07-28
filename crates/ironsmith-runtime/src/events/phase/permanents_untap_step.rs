//! Turn-based permanent-untap event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;

/// The point in a player's untap step when they untap their permanents.
///
/// This is distinct from individual `PermanentUntappedEvent`s: effects such
/// as Undiscovered Paradise happen as the turn-based untap action begins and
/// can move a permanent before it ever becomes untapped.
#[derive(Debug, Clone)]
pub struct PermanentsUntapStepEvent {
    pub player: PlayerId,
}

impl PermanentsUntapStepEvent {
    pub fn new(player: PlayerId) -> Self {
        Self { player }
    }
}

impl GameEventType for PermanentsUntapStepEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::PermanentsUntapStep
    }

    fn affected_player(&self, _game: &GameState) -> PlayerId {
        self.player
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }

    fn display(&self) -> String {
        "Untap permanents during untap step".to_string()
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
    fn permanents_untap_step_event_exposes_player_and_kind() {
        let player = PlayerId::from_index(0);
        let event = PermanentsUntapStepEvent::new(player);

        assert_eq!(event.player(), Some(player));
        assert_eq!(event.event_kind(), EventKind::PermanentsUntapStep);
    }
}
