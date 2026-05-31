//! "Whenever [player] rolls [result]" trigger.

use crate::events::EventKind;
use crate::events::other::DieRolledEvent;
use crate::target::PlayerFilter;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};
use crate::triggers::{player_filter_matches_with_context, TriggerEvent};

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerRollsResultTrigger {
    pub player: PlayerFilter,
    pub result: u32,
}

impl PlayerRollsResultTrigger {
    pub fn new(player: PlayerFilter, result: u32) -> Self {
        Self { player, result }
    }
}

impl TriggerMatcher for PlayerRollsResultTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::DieRolled {
            return false;
        }
        let Some(e) = event.downcast::<DieRolledEvent>() else {
            return false;
        };
        if e.result != self.result {
            return false;
        }

        player_filter_matches_with_context(&self.player, e.player, ctx.controller, ctx.game, None)
    }

    fn display(&self) -> String {
        let result = self.result;
        match &self.player {
            PlayerFilter::You => format!("Whenever you roll a {result}"),
            PlayerFilter::Opponent => format!("Whenever an opponent rolls a {result}"),
            PlayerFilter::Any => format!("Whenever a player rolls a {result}"),
            PlayerFilter::Active => format!("Whenever the active player rolls a {result}"),
            PlayerFilter::Specific(_) => format!("Whenever that player rolls a {result}"),
            _ => format!("Whenever a player rolls a {result}"),
        }
    }
}
