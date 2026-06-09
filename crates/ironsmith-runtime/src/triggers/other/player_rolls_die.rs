//! "Whenever [player] rolls a die" trigger.

use crate::events::EventKind;
use crate::events::other::DieRolledEvent;
use crate::target::PlayerFilter;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};
use crate::triggers::{TriggerEvent, player_filter_matches_with_context};

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerRollsDieTrigger {
    pub player: PlayerFilter,
}

impl PlayerRollsDieTrigger {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }
}

impl TriggerMatcher for PlayerRollsDieTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::DieRolled {
            return false;
        }
        let Some(e) = event.downcast::<DieRolledEvent>() else {
            return false;
        };

        player_filter_matches_with_context(&self.player, e.player, ctx.controller, ctx.game, None)
    }

    fn display(&self) -> String {
        match &self.player {
            PlayerFilter::You => "Whenever you roll a die".to_string(),
            PlayerFilter::Opponent => "Whenever an opponent rolls a die".to_string(),
            PlayerFilter::Any => "Whenever a player rolls a die".to_string(),
            PlayerFilter::Active => "Whenever the active player rolls a die".to_string(),
            PlayerFilter::Specific(_) => "Whenever that player rolls a die".to_string(),
            _ => "Whenever a player rolls a die".to_string(),
        }
    }
}
