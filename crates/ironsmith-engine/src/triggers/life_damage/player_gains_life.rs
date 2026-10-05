//! A filtered life-gaining player; the affected player is not the ability's
//! controller and an opponent excludes teammates.
use crate::events::{EventKind, life::LifeGainEvent};
use crate::filter::player_filter_matches_game;
use crate::target::PlayerFilter;
use crate::triggers::matcher_trait::{
    TriggerContext, TriggerMatcher, current_turn_matches_player_filter,
};
use crate::triggers::{TriggerEvent, describe_player_filter_subject};

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerGainsLifeTrigger {
    pub player: PlayerFilter,
    pub during_turn: Option<PlayerFilter>,
}

impl TriggerMatcher for PlayerGainsLifeTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        let Some(event) = event.downcast::<LifeGainEvent>() else {
            return false;
        };
        event.amount > 0
            && player_filter_matches_game(&self.player, event.player, ctx.game, &ctx.filter_ctx)
            && self.during_turn.as_ref().is_none_or(|turn| {
                current_turn_matches_player_filter(turn, ctx, Some(event.player))
            })
    }
    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::LifeGain])
    }
    fn display(&self) -> String {
        let subject = describe_player_filter_subject(&self.player);
        let verb = if self.player == PlayerFilter::You {
            "gain"
        } else {
            "gains"
        };
        let suffix = match self.during_turn.as_ref() {
            Some(PlayerFilter::You) => " during your turn".to_owned(),
            Some(PlayerFilter::Opponent) => " during an opponent's turn".to_owned(),
            Some(PlayerFilter::IteratedPlayer) => " during their turn".to_owned(),
            Some(player) => format!(
                " during {} turn",
                crate::triggers::describe_player_filter_possessive(player)
            ),
            None => String::new(),
        };
        format!("Whenever {subject} {verb} life{suffix}")
    }
}
