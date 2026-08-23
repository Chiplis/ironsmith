//! "Whenever [player] loses life" trigger.

use crate::events::EventKind;
use crate::events::life::LifeLossEvent;
use crate::target::PlayerFilter;
use crate::triggers::matcher_trait::{
    TriggerContext, TriggerMatcher, current_turn_matches_player_filter,
};
use crate::triggers::{TriggerEvent, describe_player_filter_subject};

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerLosesLifeTrigger {
    pub player: PlayerFilter,
    pub during_turn: Option<PlayerFilter>,
    pub one_or_more: bool,
    /// Trigger only when the life-loss event is for exactly this much life
    /// ("Whenever one or more opponents each lose exactly 1 life").
    pub exact_amount: Option<u32>,
}

impl PlayerLosesLifeTrigger {
    pub fn new(player: PlayerFilter) -> Self {
        Self {
            player,
            during_turn: None,
            one_or_more: false,
            exact_amount: None,
        }
    }

    pub fn one_or_more(player: PlayerFilter) -> Self {
        Self {
            player,
            during_turn: None,
            one_or_more: true,
            exact_amount: None,
        }
    }

    pub fn during_turn(player: PlayerFilter, during_turn: PlayerFilter) -> Self {
        Self {
            player,
            during_turn: Some(during_turn),
            one_or_more: false,
            exact_amount: None,
        }
    }

    pub fn exact_amount(player: PlayerFilter, amount: u32) -> Self {
        Self {
            player,
            during_turn: None,
            one_or_more: true,
            exact_amount: Some(amount),
        }
    }
}

impl TriggerMatcher for PlayerLosesLifeTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::LifeLoss {
            return false;
        }
        let Some(e) = event.downcast::<LifeLossEvent>() else {
            return false;
        };
        let player_matches = match &self.player {
            PlayerFilter::You => e.player == ctx.controller,
            PlayerFilter::Opponent => e.player != ctx.controller,
            PlayerFilter::Any => true,
            PlayerFilter::Active => ctx.game.is_active_player(e.player),
            PlayerFilter::Specific(id) => e.player == *id,
            _ => true,
        };
        if !player_matches {
            return false;
        }
        if let Some(exact) = self.exact_amount
            && e.amount != exact
        {
            return false;
        }
        if let Some(during_turn) = &self.during_turn {
            return current_turn_matches_player_filter(during_turn, ctx, None);
        }
        true
    }

    fn display(&self) -> String {
        if let Some(exact) = self.exact_amount {
            let subject = match &self.player {
                PlayerFilter::Opponent => "one or more opponents each".to_string(),
                other => describe_player_filter_subject(other),
            };
            return format!("Whenever {subject} lose exactly {exact} life");
        }
        if self.one_or_more {
            let subject = match &self.player {
                PlayerFilter::Opponent => "one or more opponents".to_string(),
                other => describe_player_filter_subject(other),
            };
            return format!("Whenever {subject} lose life");
        }
        let base = match &self.player {
            PlayerFilter::You => "Whenever you lose life".to_string(),
            _ => format!(
                "Whenever {} loses life",
                describe_player_filter_subject(&self.player)
            ),
        };
        if let Some(during_turn) = &self.during_turn {
            let suffix = match during_turn {
                PlayerFilter::You => " during your turn",
                PlayerFilter::Opponent => " during an opponent's turn",
                PlayerFilter::Specific(_) => " during that player's turn",
                _ => "",
            };
            format!("{base}{suffix}")
        } else {
            base
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        let trigger = PlayerLosesLifeTrigger::new(PlayerFilter::Any);
        assert!(trigger.display().contains("loses life"));
    }
}
