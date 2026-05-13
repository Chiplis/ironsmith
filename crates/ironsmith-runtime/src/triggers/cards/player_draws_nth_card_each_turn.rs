//! "Whenever [player] draws their Nth card each turn" trigger.

use crate::events::EventKind;
use crate::events::other::CardsDrawnEvent;
use crate::target::PlayerFilter;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};
use crate::triggers::{TriggerEvent, describe_player_filter_subject};

/// Trigger for "Whenever [player] draws their Nth card each turn".
///
/// This fires once when the draw event includes the configured draw number.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerDrawsNthCardEachTurnTrigger {
    pub player: PlayerFilter,
    pub card_number: u32,
}

impl PlayerDrawsNthCardEachTurnTrigger {
    pub fn new(player: PlayerFilter, card_number: u32) -> Self {
        Self {
            player,
            card_number,
        }
    }
}

impl TriggerMatcher for PlayerDrawsNthCardEachTurnTrigger {
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
        if !player_matches {
            return false;
        }

        if self.card_number == 0 {
            return false;
        }

        let total_after = ctx
            .game
            .turn_store
            .turn_history
            .cards_drawn_by_player(e.player);
        let drawn_now = e.amount();
        let total_before = total_after.saturating_sub(drawn_now);

        total_before < self.card_number && self.card_number <= total_after
    }

    fn display(&self) -> String {
        let ordinal =
            ironsmith_core::ordinal_word(self.card_number).unwrap_or_else(|| "nth".to_string());
        match &self.player {
            PlayerFilter::You => format!("Whenever you draw your {ordinal} card each turn"),
            PlayerFilter::Any => format!("Whenever a player draws their {ordinal} card each turn"),
            PlayerFilter::Opponent => {
                format!("Whenever an opponent draws their {ordinal} card each turn")
            }
            PlayerFilter::Specific(_) | PlayerFilter::IteratedPlayer => {
                format!("Whenever that player draws their {ordinal} card each turn")
            }
            _ => format!(
                "Whenever {} draws their {ordinal} card each turn",
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

    #[test]
    fn test_display() {
        let trigger = PlayerDrawsNthCardEachTurnTrigger::new(PlayerFilter::You, 2);
        assert!(trigger.display().contains("second card each turn"));
    }

    #[test]
    fn test_matches_second_draw() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);

        let prior_event = TriggerEvent::new_with_provenance(
            CardsDrawnEvent::single(alice, ObjectId::from_raw(10), true),
            crate::provenance::ProvNodeId::default(),
        );
        game.stage_turn_history_event(&prior_event);
        let event = TriggerEvent::new_with_provenance(
            CardsDrawnEvent::single(alice, ObjectId::from_raw(2), false),
            crate::provenance::ProvNodeId::default(),
        );
        game.stage_turn_history_event(&event);
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let trigger = PlayerDrawsNthCardEachTurnTrigger::new(PlayerFilter::You, 2);
        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_matches_second_draw_in_two_card_draw_event() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);

        let event = TriggerEvent::new_with_provenance(
            CardsDrawnEvent::new(
                alice,
                vec![ObjectId::from_raw(2), ObjectId::from_raw(3)],
                true,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        game.stage_turn_history_event(&event);
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let trigger = PlayerDrawsNthCardEachTurnTrigger::new(PlayerFilter::You, 2);
        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn test_does_not_match_wrong_number() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);

        let prior_event = TriggerEvent::new_with_provenance(
            CardsDrawnEvent::new(
                alice,
                vec![ObjectId::from_raw(10), ObjectId::from_raw(11)],
                true,
            ),
            crate::provenance::ProvNodeId::default(),
        );
        game.stage_turn_history_event(&prior_event);
        let event = TriggerEvent::new_with_provenance(
            CardsDrawnEvent::single(alice, ObjectId::from_raw(2), false),
            crate::provenance::ProvNodeId::default(),
        );
        game.stage_turn_history_event(&event);
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let trigger = PlayerDrawsNthCardEachTurnTrigger::new(PlayerFilter::You, 2);
        assert!(!trigger.matches(&event, &ctx));
    }
}
