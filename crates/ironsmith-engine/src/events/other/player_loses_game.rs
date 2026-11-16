//! Player-loses-game event implementation.

use std::any::Any;


use crate::events::traits::{EventKind, GameEventType, ReplacementMatcher};
use crate::game_state::{GameState, Target};
use crate::ids::PlayerId;

#[derive(Debug, Clone)]
pub struct PlayerLosesGameEvent {
    pub player: PlayerId,
    /// Last known information of the permanents the player controlled as
    /// they lost (CR 800.4a): those objects leave the game with the player,
    /// before any "when that player loses the game" ability resolves.
    pub controlled_permanents: Vec<crate::snapshot::ObjectSnapshot>,
}

impl PlayerLosesGameEvent {
    pub fn new(player: PlayerId) -> Self {
        Self { player, controlled_permanents: Vec::new() }
    }

    pub fn with_controlled_permanents(
        mut self,
        permanents: Vec<crate::snapshot::ObjectSnapshot>,
    ) -> Self {
        self.controlled_permanents = permanents;
        self
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
    fn may_match_event_kind(&self, kind: EventKind) -> bool {
        kind == EventKind::PlayerLosesGame
    }

    fn matches_prepared_event(&self, event: &dyn GameEventType, ctx: &crate::events::context::PreparedEventContext) -> bool {
        event
            .as_any()
            .downcast_ref::<PlayerLosesGameEvent>()
            .is_some_and(|loss| loss.player == ctx.controller)
    }

    fn display(&self) -> String {
        "You would lose the game".to_string()
    }
}
