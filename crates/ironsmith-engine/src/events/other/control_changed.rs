//! Control changed event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;

/// A permanent came under a different player's control.
#[derive(Debug, Clone)]
pub struct ControlChangedEvent {
    /// The permanent whose controller changed.
    pub permanent: ObjectId,
    /// The controller before the change.
    pub previous_controller: PlayerId,
    /// The controller after the change.
    pub new_controller: PlayerId,
    /// Characteristics immediately before and after the completed transition.
    pub previous_snapshot: Option<ObjectSnapshot>,
    pub snapshot: Option<ObjectSnapshot>,
    pub complete_source_lookback: bool,
}

impl ControlChangedEvent {
    pub fn new(
        permanent: ObjectId,
        previous_controller: PlayerId,
        new_controller: PlayerId,
    ) -> Self {
        Self {
            permanent,
            previous_controller,
            new_controller,
            previous_snapshot: None,
            snapshot: None,
            complete_source_lookback: false,
        }
    }
}

impl ControlChangedEvent {
    pub fn with_complete_source_lookback(mut self) -> Self {
        self.complete_source_lookback = true;
        self
    }
    pub fn with_snapshots(mut self, previous: ObjectSnapshot, current: ObjectSnapshot) -> Self {
        self.previous_snapshot = Some(previous);
        self.snapshot = Some(current);
        self
    }
}

impl GameEventType for ControlChangedEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::ControlChanged
    }

    fn is_replacement_proposal(&self) -> bool { false }

    fn affected_player(&self, _game: &GameState) -> PlayerId {
        self.new_controller
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }

    fn display(&self) -> String {
        "Permanent changed control".to_string()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn object_id(&self) -> Option<ObjectId> {
        Some(self.permanent)
    }

    fn player(&self) -> Option<PlayerId> {
        Some(self.new_controller)
    }

    fn controller(&self) -> Option<PlayerId> {
        Some(self.new_controller)
    }

    fn snapshot(&self) -> Option<&ObjectSnapshot> {
        self.snapshot.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_changed_event_reports_kind_and_players() {
        let previous_controller = PlayerId::from_index(0);
        let new_controller = PlayerId::from_index(1);
        let event =
            ControlChangedEvent::new(ObjectId::from_raw(7), previous_controller, new_controller);

        assert_eq!(event.event_kind(), EventKind::ControlChanged);
        assert_eq!(event.previous_controller, previous_controller);
        assert_eq!(event.player(), Some(new_controller));
        assert_eq!(event.controller(), Some(new_controller));
    }
}
