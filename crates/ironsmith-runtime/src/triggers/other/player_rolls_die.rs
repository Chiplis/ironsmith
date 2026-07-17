//! "Whenever [player] rolls a die" trigger.

use crate::events::EventKind;
use crate::events::other::DieRolledEvent;
use crate::target::PlayerFilter;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};
use crate::triggers::{TriggerEvent, player_filter_matches_with_context};

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerRollsDieTrigger {
    pub player: PlayerFilter,
    /// Oracle groups a single roll event as "one or more dice" even when the
    /// event contains multiple physical dice.
    pub one_or_more: bool,
}

impl PlayerRollsDieTrigger {
    pub fn new(player: PlayerFilter) -> Self {
        Self::with_surface(player, false)
    }

    pub fn with_surface(player: PlayerFilter, one_or_more: bool) -> Self {
        Self {
            player,
            one_or_more,
        }
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
        if self.one_or_more {
            return match &self.player {
                PlayerFilter::You => "Whenever you roll one or more dice".to_string(),
                PlayerFilter::Opponent => "Whenever an opponent rolls one or more dice".to_string(),
                PlayerFilter::Any => "Whenever a player rolls one or more dice".to_string(),
                PlayerFilter::Active => {
                    "Whenever the active player rolls one or more dice".to_string()
                }
                PlayerFilter::Specific(_) => {
                    "Whenever that player rolls one or more dice".to_string()
                }
                _ => "Whenever a player rolls one or more dice".to_string(),
            };
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grouped_roll_surface_is_preserved() {
        assert_eq!(
            PlayerRollsDieTrigger::with_surface(PlayerFilter::You, true).display(),
            "Whenever you roll one or more dice"
        );
        assert_eq!(
            PlayerRollsDieTrigger::new(PlayerFilter::You).display(),
            "Whenever you roll a die"
        );
    }
}
