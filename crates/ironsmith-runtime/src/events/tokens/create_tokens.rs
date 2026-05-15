//! Token creation event implementation.

use std::any::Any;

use crate::events::cause::EventCause;
use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::GameState;
use crate::ids::PlayerId;

/// An event representing one effect creating one or more tokens for a player.
#[derive(Debug, Clone)]
pub struct CreateTokensEvent {
    /// Player under whose control the tokens would be created.
    pub controller: PlayerId,
    /// Number of tokens that would be created.
    pub count: u32,
    /// What caused the token creation.
    pub cause: EventCause,
}

impl CreateTokensEvent {
    pub fn with_cause(controller: PlayerId, count: u32, cause: EventCause) -> Self {
        Self {
            controller,
            count,
            cause,
        }
    }

    pub fn doubled(&self) -> Self {
        Self {
            count: self.count.saturating_mul(2),
            ..self.clone()
        }
    }

    pub fn with_count(&self, count: u32) -> Self {
        Self {
            count,
            ..self.clone()
        }
    }
}

impl GameEventType for CreateTokensEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::CreateTokens
    }

    fn affected_player(&self, _game: &GameState) -> PlayerId {
        self.controller
    }

    fn player(&self) -> Option<PlayerId> {
        Some(self.controller)
    }

    fn controller(&self) -> Option<PlayerId> {
        Some(self.controller)
    }

    fn display(&self) -> String {
        format!("Create {} token(s)", self.count)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
