//! Beginning of declare attackers step event.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::GameState;
use crate::ids::PlayerId;

/// "At the beginning of the declare attackers step" (CR 508.1, 508.2): the
/// turn-based attack declaration has happened, and abilities that trigger at
/// the beginning of this step trigger alongside the attack triggers.
#[derive(Debug, Clone)]
pub struct BeginningOfDeclareAttackersStepEvent {
    /// The active player whose combat this step belongs to.
    pub player: PlayerId,
}

impl BeginningOfDeclareAttackersStepEvent {
    pub fn new(player: PlayerId) -> Self {
        Self { player }
    }
}

impl GameEventType for BeginningOfDeclareAttackersStepEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::BeginningOfDeclareAttackersStep
    }

    fn is_replacement_proposal(&self) -> bool {
        false
    }

    fn affected_player(&self, _game: &GameState) -> PlayerId {
        self.player
    }

    fn player(&self) -> Option<PlayerId> {
        Some(self.player)
    }

    fn display(&self) -> String {
        "Beginning of declare attackers step".to_string()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
