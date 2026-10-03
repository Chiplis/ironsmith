//! Permanent untapped event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;

/// A permanent became untapped event.
///
/// Triggered when a permanent becomes untapped.
#[derive(Debug, Clone)]
pub struct PermanentUntappedEvent {
    /// The permanent that became untapped
    pub permanent: ObjectId,
    /// Characteristics when the state transition completed.
    pub snapshot: Option<ObjectSnapshot>,
    /// Player performing the action, if the producer explicitly knows it.
    pub actor: Option<PlayerId>,
    /// Recipient before the action; active "you tap an untapped creature"
    /// clauses qualify this state, while passive clauses use `snapshot`.
    pub before_snapshot: Option<ObjectSnapshot>,
    /// Empty outside an untap step; shared team turns retain all step players.
    pub untap_step_players: Vec<PlayerId>,
}

impl PermanentUntappedEvent {
    /// Create a new permanent untapped event.
    pub fn new(permanent: ObjectId) -> Self {
        Self {
            permanent,
            snapshot: None,
            actor: None,
            before_snapshot: None,
            untap_step_players: Vec::new(),
        }
    }

    /// Capture a completed transition at its producer, before later effects
    /// can change its characteristics or controller.
    pub fn capture(game: &GameState, permanent: ObjectId, actor: Option<PlayerId>) -> Self {
        Self {
            permanent,
            snapshot: game.object(permanent).map(|object| {
                ObjectSnapshot::from_object_with_calculated_characteristics(object, game)
            }),
            actor,
            before_snapshot: None,
            untap_step_players: if game.turn.phase == crate::game_state::Phase::Beginning
                && game.turn.step == Some(crate::game_state::Step::Untap)
            {
                game.turn_players()
            } else {
                Vec::new()
            },
        }
    }

    pub fn with_before_snapshot(mut self, snapshot: ObjectSnapshot) -> Self {
        self.before_snapshot = Some(snapshot);
        self
    }

    pub fn with_snapshot(mut self, snapshot: ObjectSnapshot) -> Self {
        self.snapshot = Some(snapshot);
        self
    }

    pub fn with_actor(mut self, actor: PlayerId) -> Self {
        self.actor = Some(actor);
        self
    }
}

impl GameEventType for PermanentUntappedEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::PermanentUntapped
    }

    fn affected_player(&self, game: &GameState) -> PlayerId {
        game.object(self.permanent)
            .map(|o| game.controller_of(o))
            .unwrap_or(game.turn.active_player)
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }

    fn display(&self) -> String {
        "Permanent became untapped".to_string()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn object_id(&self) -> Option<ObjectId> {
        Some(self.permanent)
    }

    fn player(&self) -> Option<PlayerId> {
        self.actor
    }

    fn controller(&self) -> Option<PlayerId> {
        None // Determined from game state
    }

    fn snapshot(&self) -> Option<&ObjectSnapshot> {
        self.snapshot.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permanent_untapped_event_creation() {
        let event = PermanentUntappedEvent::new(ObjectId::from_raw(1));
        assert_eq!(event.permanent, ObjectId::from_raw(1));
    }

    #[test]
    fn test_permanent_untapped_event_kind() {
        let event = PermanentUntappedEvent::new(ObjectId::from_raw(1));
        assert_eq!(event.event_kind(), EventKind::PermanentUntapped);
    }
}
