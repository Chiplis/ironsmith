//! Player-loses-game event implementation.

use std::any::Any;

use crate::events::context::EventContext;
use crate::events::traits::{EventKind, GameEventType, ReplacementMatcher};
use crate::game_state::{GameState, Target};
use crate::ids::PlayerId;

#[derive(Debug, Clone)]
pub struct PlayerLosesGameEvent {
    pub player: PlayerId,
}

impl PlayerLosesGameEvent {
    pub fn new(player: PlayerId) -> Self {
        Self { player }
    }
}

impl GameEventType for PlayerLosesGameEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::PlayerLosesGame
    }

    fn affected_player(&self, _game: &GameState) -> PlayerId {
        self.player
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }

    fn display(&self) -> String {
        format!("Player {} loses the game", self.player.0)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn player(&self) -> Option<PlayerId> {
        Some(self.player)
    }
}

/// Matches an impending game loss for the replacement effect's controller.
///
/// This is deliberately a replacement matcher rather than a prohibition:
/// effects such as Lich's Mirror replace one loss event with a sequence of
/// ordinary effects, while "you can't lose the game" remains a separate rule
/// restriction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WouldLoseGameMatcher;

impl ReplacementMatcher for WouldLoseGameMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        event
            .as_any()
            .downcast_ref::<PlayerLosesGameEvent>()
            .is_some_and(|loss| loss.player == ctx.controller)
    }

    fn display(&self) -> String {
        "You would lose the game".to_string()
    }
}
