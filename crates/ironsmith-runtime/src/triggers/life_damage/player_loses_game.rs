//! "Whenever [player] loses the game" trigger.

use crate::events::EventKind;
use crate::events::other::PlayerLosesGameEvent;
use crate::target::PlayerFilter;
use crate::triggers::check::player_filter_matches_with_context;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};
use crate::triggers::{TriggerEvent, describe_player_filter_subject};

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerLosesGameTrigger {
    pub player: PlayerFilter,
}

impl PlayerLosesGameTrigger {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }
}

impl TriggerMatcher for PlayerLosesGameTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::PlayerLosesGame {
            return false;
        }
        let Some(e) = event.downcast::<PlayerLosesGameEvent>() else {
            return false;
        };
        player_filter_matches_with_context(&self.player, e.player, ctx.controller, ctx.game, None)
    }

    fn display(&self) -> String {
        match &self.player {
            PlayerFilter::You => "Whenever you lose the game".to_string(),
            PlayerFilter::NotYou => "Whenever another player loses the game".to_string(),
            _ => format!(
                "Whenever {} loses the game",
                describe_player_filter_subject(&self.player)
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        let trigger = PlayerLosesGameTrigger::new(PlayerFilter::NotYou);
        assert_eq!(trigger.display(), "Whenever another player loses the game");
    }
}
