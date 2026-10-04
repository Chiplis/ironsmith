use crate::events::{CardsDrawnEvent, EventKind};
use crate::filter::player_filter_matches_game;
use crate::target::PlayerFilter;
use crate::triggers::matcher_trait::{
    TriggerContext, TriggerMatcher, current_turn_matches_player_filter,
};
use crate::triggers::{TriggerEvent, describe_player_filter_subject};

/// Positive turn qualification or the captured first draw of each own draw
/// step. The latter must not consult this turn's accumulated draw count.
#[derive(Debug, Clone, PartialEq)]
pub struct QualifiedPlayerDrawTrigger {
    pub player: PlayerFilter,
    pub during_turn: Option<PlayerFilter>,
    pub first_in_own_draw_step: bool,
}
impl QualifiedPlayerDrawTrigger {
    fn count(&self, draw: &CardsDrawnEvent) -> u32 {
        if self.first_in_own_draw_step {
            u32::from(
                draw.amount() > 0
                    && draw.is_during_players_draw_step
                    && draw.cards_previously_drawn_this_draw_step == 0,
            )
        } else {
            draw.amount()
        }
    }
}
impl TriggerMatcher for QualifiedPlayerDrawTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        let Some(draw) = event.downcast::<CardsDrawnEvent>() else {
            return false;
        };
        self.count(draw) > 0
            && player_filter_matches_game(&self.player, draw.player, ctx.game, &ctx.filter_ctx)
            && self
                .during_turn
                .as_ref()
                .is_none_or(|turn| current_turn_matches_player_filter(turn, ctx, Some(draw.player)))
    }
    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::CardsDrawn])
    }
    fn trigger_count(&self, event: &TriggerEvent) -> u32 {
        event
            .downcast::<CardsDrawnEvent>()
            .map_or(0, |draw| self.count(draw))
    }
    fn display(&self) -> String {
        let subject = describe_player_filter_subject(&self.player);
        let you = self.player == PlayerFilter::You;
        let verb = if you { "draw" } else { "draws" };
        if self.first_in_own_draw_step {
            let owner = if you { "your" } else { "their" };
            return format!(
                "Whenever {subject} {verb} {owner} first card during each of {owner} draw steps"
            );
        }
        let turn = match self.during_turn.as_ref() {
            Some(PlayerFilter::You) => "your".to_owned(),
            Some(PlayerFilter::Opponent) => "an opponent's".to_owned(),
            Some(PlayerFilter::IteratedPlayer) => "their".to_owned(),
            Some(player) => crate::triggers::describe_player_filter_possessive(player),
            None => return format!("Whenever {subject} {verb} a card"),
        };
        format!("Whenever {subject} {verb} a card during {turn} turn")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{GameState, ObjectId, PlayerId};
    #[test]
    fn first_draw_uses_captured_step_ordinal_and_unknown_players_fail_closed() {
        let game = GameState::new(vec!["A".into(), "B".into()], 20);
        let ctx = TriggerContext::for_source(ObjectId::from_raw(1), PlayerId(0), &game);
        let first = QualifiedPlayerDrawTrigger {
            player: PlayerFilter::You,
            during_turn: None,
            first_in_own_draw_step: true,
        };
        for (in_step, prior, count, expected) in [
            (true, 0, 3, 1),
            (true, 1, 3, 0),
            (false, 0, 3, 0),
            (true, 0, 0, 0),
        ] {
            let event = TriggerEvent::new(CardsDrawnEvent::new_with_step_context(
                PlayerId(0),
                (0..count).map(|id| ObjectId::from_raw(id + 10)).collect(),
                false,
                in_step,
                prior,
            ), Default::default());
            assert_eq!(first.trigger_count(&event), expected);
            assert_eq!(first.matches(&event, &ctx), expected > 0);
            let unresolved = QualifiedPlayerDrawTrigger {
                player: PlayerFilter::TaggedPlayer("unbound".into()),
                ..first.clone()
            };
            assert!(!unresolved.matches(&event, &ctx));
        }
    }
}
