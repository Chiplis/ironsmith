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
        if e.amount == 0
            || !crate::filter::player_filter_matches_game(
                &self.player, e.player, ctx.game, &ctx.filter_ctx,
            )
        {
            return false;
        }
        if let Some(exact) = self.exact_amount
            && e.amount != exact
        {
            return false;
        }
        if let Some(during_turn) = &self.during_turn {
            // `IteratedPlayer` names the life-losing player ("during each of
            // their turns").
            return current_turn_matches_player_filter(during_turn, ctx, Some(e.player));
        }
        true
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::LifeLoss])
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
                PlayerFilter::IteratedPlayer => " during their turn",
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
    #[test]
    fn life_loss_filters_preserve_teammates_and_fail_closed_for_unknown_bindings() {
        let a = crate::ids::PlayerId(0);
        let b = crate::ids::PlayerId(1);
        let c = crate::ids::PlayerId(2);
        let mut game = crate::game_state::GameState::new(vec!["A".into(), "B".into(), "C".into()], 20);
        game.set_teams(vec![vec![a, b], vec![c]]).unwrap();
        let ctx = TriggerContext::for_source(crate::ids::ObjectId(99), a, &game);
        let event = |player, amount| TriggerEvent::new_with_provenance(
            LifeLossEvent::from_effect(player, amount), crate::provenance::ProvNodeId::default(),
        );
        let opponent = PlayerLosesLifeTrigger::new(PlayerFilter::Opponent);
        assert!(!opponent.matches(&event(b, 2), &ctx));
        assert!(opponent.matches(&event(c, 2), &ctx));
        assert!(!opponent.matches(&event(c, 0), &ctx));
        let missing = PlayerLosesLifeTrigger::new(PlayerFilter::TaggedPlayer("unbound-life-player".into()));
        assert!(!missing.matches(&event(c, 2), &ctx));
    }

}
