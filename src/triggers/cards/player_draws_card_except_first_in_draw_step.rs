//! "Whenever [player] draws a card except the first one they draw in each of their draw steps" trigger.

use crate::events::EventKind;
use crate::events::other::CardsDrawnEvent;
use crate::target::PlayerFilter;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};
use crate::triggers::{TriggerEvent, describe_player_filter_subject};

/// Trigger for draw patterns like Orcish Bowmasters.
///
/// This fires once for each card drawn except the first card that player draws in their own draw step.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerDrawsCardExceptFirstInDrawStepTrigger {
    pub player: PlayerFilter,
}

impl PlayerDrawsCardExceptFirstInDrawStepTrigger {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }

    fn matching_cards_drawn(&self, event: &CardsDrawnEvent) -> u32 {
        if !event.is_during_players_draw_step {
            return event.amount();
        }

        let skipped = if event.cards_previously_drawn_this_draw_step == 0 {
            1
        } else {
            0
        };
        event.amount().saturating_sub(skipped)
    }
}

impl TriggerMatcher for PlayerDrawsCardExceptFirstInDrawStepTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::CardsDrawn {
            return false;
        }
        let Some(e) = event.downcast::<CardsDrawnEvent>() else {
            return false;
        };

        let player_matches = match &self.player {
            PlayerFilter::You => e.player == ctx.controller,
            PlayerFilter::Opponent => e.player != ctx.controller,
            PlayerFilter::Any => true,
            PlayerFilter::Specific(id) => e.player == *id,
            _ => true,
        };
        player_matches && self.matching_cards_drawn(e) > 0
    }

    fn trigger_count(&self, event: &TriggerEvent) -> u32 {
        event
            .downcast::<CardsDrawnEvent>()
            .map(|drawn| self.matching_cards_drawn(drawn))
            .unwrap_or(0)
    }

    fn display(&self) -> String {
        match &self.player {
            PlayerFilter::You => {
                "Whenever you draw a card except the first one you draw in each of your draw steps"
                    .to_string()
            }
            PlayerFilter::Opponent => "Whenever an opponent draws a card except the first one they draw in each of their draw steps".to_string(),
            PlayerFilter::Any => "Whenever a player draws a card except the first one they draw in each of their draw steps".to_string(),
            PlayerFilter::Specific(_) | PlayerFilter::IteratedPlayer => "Whenever that player draws a card except the first one they draw in each of their draw steps".to_string(),
            _ => format!(
                "Whenever {} draws a card except the first one they draw in each of their draw steps",
                describe_player_filter_subject(&self.player)
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_state::GameState;
    use crate::ids::{ObjectId, PlayerId};

    fn draw_event(
        player: PlayerId,
        cards: &[u32],
        is_during_players_draw_step: bool,
        cards_previously_drawn_this_draw_step: u32,
    ) -> TriggerEvent {
        TriggerEvent::new_with_provenance(
            CardsDrawnEvent::new_with_step_context(
                player,
                cards
                    .iter()
                    .map(|id| ObjectId::from_raw((*id).into()))
                    .collect(),
                false,
                is_during_players_draw_step,
                cards_previously_drawn_this_draw_step,
            ),
            crate::provenance::ProvNodeId::default(),
        )
    }

    #[test]
    fn ignores_first_card_drawn_in_draw_step() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);
        let ctx = TriggerContext::for_source(source_id, alice, &game);
        let trigger = PlayerDrawsCardExceptFirstInDrawStepTrigger::new(PlayerFilter::You);
        let event = draw_event(alice, &[10], true, 0);

        assert!(!trigger.matches(&event, &ctx));
        assert_eq!(trigger.trigger_count(&event), 0);
    }

    #[test]
    fn matches_second_card_drawn_in_draw_step() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);
        let ctx = TriggerContext::for_source(source_id, alice, &game);
        let trigger = PlayerDrawsCardExceptFirstInDrawStepTrigger::new(PlayerFilter::You);
        let event = draw_event(alice, &[11], true, 1);

        assert!(trigger.matches(&event, &ctx));
        assert_eq!(trigger.trigger_count(&event), 1);
    }

    #[test]
    fn multi_card_draw_in_draw_step_skips_only_first_card() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);
        let ctx = TriggerContext::for_source(source_id, alice, &game);
        let trigger = PlayerDrawsCardExceptFirstInDrawStepTrigger::new(PlayerFilter::You);
        let event = draw_event(alice, &[12, 13], true, 0);

        assert!(trigger.matches(&event, &ctx));
        assert_eq!(trigger.trigger_count(&event), 1);
    }

    #[test]
    fn off_draw_step_draws_all_match() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);
        let ctx = TriggerContext::for_source(source_id, alice, &game);
        let trigger = PlayerDrawsCardExceptFirstInDrawStepTrigger::new(PlayerFilter::You);
        let event = draw_event(alice, &[14, 15], false, 0);

        assert!(trigger.matches(&event, &ctx));
        assert_eq!(trigger.trigger_count(&event), 2);
    }
}
