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
    /// Checked frame of the original completed play, before additions.
    pub snapshot: Option<ObjectSnapshot>,
    /// Actual original destination, including redirected entry outcomes.
    /// Legacy actor-only notices carry no completed characteristic frame.
    pub completed_destination: Option<Zone>,
}

impl LandPlayedEvent {
    /// Create a new land-play event.
    pub fn new(land: ObjectId, player: PlayerId, from_zone: Zone) -> Self {
        Self {
            land,
            player,
            from_zone,
            snapshot: None,
            completed_destination: None,
        }
    }

    /// Capture the original entry receipt before its deferred additions run.
    /// A redirected entry still completes the play with its exact successor.
    pub(crate) fn from_completed_entry(
        entry: &crate::game_state::EntersResult,
        player: PlayerId,
        from_zone: Zone,
        game: &GameState,
    ) -> Result<Self, crate::effects::ExecutionError> {
        let destination = game.object(entry.new_id).map(|object| object.zone)
            .ok_or_else(|| crate::effects::ExecutionError::IncompleteEvidence(
                "completed land entry lacks its exact original successor".into(),
            ))?;
        Self::with_current_snapshot(entry.new_id, player, from_zone, destination, game)
    }

    pub(crate) fn required_completed_snapshot(
        &self,
    ) -> Result<&ObjectSnapshot, crate::effects::ExecutionError> {
        self.snapshot.as_ref().filter(|snapshot|
            snapshot.object_id == self.land && self.completed_destination == Some(snapshot.zone)
        ).ok_or_else(|| crate::effects::ExecutionError::IncompleteEvidence(
            "land-play characteristic predicate lacks its exact completed play receipt".into(),
        ))
    }

    pub fn with_current_snapshot(
        land: ObjectId,
        player: PlayerId,
        from_zone: Zone,
        completed_destination: Zone,
        game: &GameState,
    ) -> Result<Self, crate::effects::ExecutionError> {
        let unavailable = || crate::effects::ExecutionError::IncompleteEvidence(
            format!("completed land play lacks exact characteristics for {land:?}"),
        );
        let object = game.object(land).ok_or_else(unavailable)?;
        if object.zone != completed_destination {
            return Err(crate::effects::ExecutionError::IncompleteEvidence(
                "completed land-play destination disagrees with its exact object".into(),
            ));
        }
        let chars = game.try_current_characteristics(land)
            .map_err(crate::effects::ExecutionError::ContinuousDiscovery)?.ok_or_else(unavailable)?;
        let snapshot = ObjectSnapshot::try_from_object_with_known_characteristics(object, game, Some(&chars))?;
        Ok(Self { land, player, from_zone, snapshot: Some(snapshot), completed_destination: Some(completed_destination) })
    }

    pub fn with_snapshot(mut self, snapshot: Option<ObjectSnapshot>) -> Self {
        self.completed_destination = snapshot.as_ref().map(|snapshot| snapshot.zone);
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
