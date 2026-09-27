//! Beginning of main phase event implementations.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;

/// Beginning of precombat main phase event.
///
/// Triggered at the beginning of a player's first main phase.
#[derive(Debug, Clone)]
pub struct BeginningOfPrecombatMainPhaseEvent {
    /// The player whose main phase it is
    pub player: PlayerId,
}

impl BeginningOfPrecombatMainPhaseEvent {
    /// Create a new beginning of precombat main phase event.
    pub fn new(player: PlayerId) -> Self {
        Self { player }
    }
}

impl GameEventType for BeginningOfPrecombatMainPhaseEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::BeginningOfPrecombatMainPhase
    }

    fn affected_player(&self, _game: &GameState) -> PlayerId {
        self.player
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }

    fn display(&self) -> String {
        "Beginning of precombat main phase".to_string()
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

/// Beginning of postcombat main phase event.
///
/// Triggered at the beginning of a player's second main phase.
#[derive(Debug, Clone)]
pub struct BeginningOfPostcombatMainPhaseEvent {
    /// The player whose main phase it is
    pub player: PlayerId,
    /// Which main phase of the turn this is (CR 505.1b), when known.
    pub main_phase_ordinal: Option<u32>,
}

impl BeginningOfPostcombatMainPhaseEvent {
    /// Create a new beginning of postcombat main phase event.
    pub fn new(player: PlayerId) -> Self {
        Self {
            player,
            main_phase_ordinal: None,
        }
    }

    /// Record which main phase of the turn is beginning (CR 505.1b).
    pub fn with_main_phase_ordinal(mut self, ordinal: u32) -> Self {
        self.main_phase_ordinal = Some(ordinal);
        self
    }
}

impl GameEventType for BeginningOfPostcombatMainPhaseEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::BeginningOfPostcombatMainPhase
    }

    fn affected_player(&self, _game: &GameState) -> PlayerId {
        self.player
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }

    fn display(&self) -> String {
        "Beginning of postcombat main phase".to_string()
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
    fn test_beginning_of_precombat_main_phase_event_creation() {
        let event = BeginningOfPrecombatMainPhaseEvent::new(PlayerId::from_index(0));
        assert_eq!(event.player, PlayerId::from_index(0));
        assert_eq!(event.event_kind(), EventKind::BeginningOfPrecombatMainPhase);
    }

    #[test]
    fn test_beginning_of_postcombat_main_phase_event_creation() {
        let event = BeginningOfPostcombatMainPhaseEvent::new(PlayerId::from_index(0));
        assert_eq!(event.player, PlayerId::from_index(0));
        assert_eq!(
            event.event_kind(),
            EventKind::BeginningOfPostcombatMainPhase
        );
    }
}
