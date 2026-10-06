//! Land-play event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;
use crate::zone::Zone;

/// A land-play event.
///
/// Triggered when a player plays a land as a special action or during resolution.
#[derive(Debug, Clone)]
pub struct LandPlayedEvent {
    /// The land permanent/object resulting from the play.
    pub land: ObjectId,
    /// The player who played the land.
    pub player: PlayerId,
    /// The zone the land was played from.
    pub from_zone: Zone,
    /// Characteristics of the committed land at the play observation boundary.
    pub snapshot: Option<ObjectSnapshot>,
}

impl LandPlayedEvent {
    /// Create a new land-play event.
    pub fn new(land: ObjectId, player: PlayerId, from_zone: Zone) -> Self {
        Self {
            land,
            player,
            from_zone,
            snapshot: None,
        }
    }

    pub fn with_snapshot(mut self, snapshot: Option<ObjectSnapshot>) -> Self {
        self.snapshot = snapshot;
        self
    }
}

impl GameEventType for LandPlayedEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::LandPlayed
    }

    fn affected_player(&self, _game: &GameState) -> PlayerId {
        self.player
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }

    fn display(&self) -> String {
        format!("Land played by player {}", self.player.0)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn object_id(&self) -> Option<ObjectId> {
        Some(self.land)
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
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_land_played_event_creation() {
        let event =
            LandPlayedEvent::new(ObjectId::from_raw(1), PlayerId::from_index(0), Zone::Hand);
        assert_eq!(event.land, ObjectId::from_raw(1));
        assert_eq!(event.player, PlayerId::from_index(0));
        assert_eq!(event.from_zone, Zone::Hand);
    }

    #[test]
    fn test_land_played_event_kind() {
        let event =
            LandPlayedEvent::new(ObjectId::from_raw(1), PlayerId::from_index(0), Zone::Hand);
        assert_eq!(event.event_kind(), EventKind::LandPlayed);
    }
}
