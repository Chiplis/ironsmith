//! "Whenever [player] rolls [result]" trigger.

use crate::events::EventKind;
use crate::events::other::DieRolledEvent;
use crate::target::PlayerFilter;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};
use crate::triggers::{TriggerEvent, player_filter_matches_with_context};

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerRollsResultTrigger {
    pub player: PlayerFilter,
    pub result: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerRollsHighestNaturalResultTrigger {
    pub player: PlayerFilter,
}

impl PlayerRollsResultTrigger {
    pub fn new(player: PlayerFilter, result: u32) -> Self {
        Self { player, result }
    }
}

impl PlayerRollsHighestNaturalResultTrigger {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
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

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::DieRolled])
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

impl TriggerMatcher for PlayerRollsHighestNaturalResultTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::DieRolled {
            return false;
        }
        let Some(e) = event.downcast::<DieRolledEvent>() else {
            return false;
        };
        if e.natural_result != e.sides {
            return false;
        }

        player_filter_matches_with_context(&self.player, e.player, ctx.controller, ctx.game, None)
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::DieRolled])
    }

    fn display(&self) -> String {
        match &self.player {
            PlayerFilter::You => "Whenever you roll a die's highest natural result".to_string(),
            PlayerFilter::Opponent => {
                "Whenever an opponent rolls a die's highest natural result".to_string()
            }
            PlayerFilter::Any => {
                "Whenever a player rolls a die's highest natural result".to_string()
            }
            PlayerFilter::Active => {
                "Whenever the active player rolls a die's highest natural result".to_string()
            }
            PlayerFilter::Specific(_) => {
                "Whenever that player rolls a die's highest natural result".to_string()
            }
            _ => "Whenever a player rolls a die's highest natural result".to_string(),
        }
    }
}
