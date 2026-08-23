//! "Whenever [player] draws a card" trigger.

use crate::events::EventKind;
use crate::events::other::CardsDrawnEvent;
use crate::target::PlayerFilter;
use crate::triggers::matcher_trait::{
    TriggerContext, TriggerMatcher, current_turn_matches_player_filter,
};
use crate::triggers::{TriggerEvent, describe_player_filter_subject};

/// Trigger for "Whenever [player] draws a card" or "Whenever [player] draws one or more cards".
///
/// By default, this matches when the specified player draws cards.
/// The `per_card` field controls whether the trigger fires once per card or once per draw action.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerDrawsCardTrigger {
    pub player: PlayerFilter,
    /// If true, fires once per card drawn. If false, fires once per draw action.
    pub per_card: bool,
    pub not_during_turn: Option<PlayerFilter>,
}

impl PlayerDrawsCardTrigger {
    /// Create a trigger that fires once per draw action ("whenever you draw one or more cards").
    pub fn new(player: PlayerFilter) -> Self {
        Self {
            player,
            per_card: false,
            not_during_turn: None,
        }
    }

    /// Create a trigger that fires once per card drawn ("whenever you draw a card").
    pub fn per_card(player: PlayerFilter) -> Self {
        Self {
            player,
            per_card: true,
            not_during_turn: None,
        }
    }

    pub fn not_during_turn(player: PlayerFilter, during_turn: PlayerFilter) -> Self {
        Self {
            player,
            per_card: true,
            not_during_turn: Some(during_turn),
        }
    }
}

impl TriggerMatcher for PlayerDrawsCardTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::CardsDrawn {
            return false;
        }
        let Some(e) = event.downcast::<CardsDrawnEvent>() else {
            return false;
        };
        (match &self.player {
            PlayerFilter::You => e.player == ctx.controller,
            PlayerFilter::Opponent => e.player != ctx.controller,
            PlayerFilter::Any => true,
            PlayerFilter::Active => ctx.game.is_active_player(e.player),
            PlayerFilter::Specific(id) => e.player == *id,
            _ => true,
        }) && if let Some(not_during_turn) = &self.not_during_turn {
            !current_turn_matches_player_filter(not_during_turn, ctx, Some(e.player))
        } else {
            true
        }
    }

    /// Return how many times this trigger should fire for the event.
    ///
    /// For per_card triggers, returns the number of cards drawn.
    /// For batch triggers, returns 1.
    fn trigger_count(&self, event: &TriggerEvent) -> u32 {
        if !self.per_card {
            return 1;
        }
        if let Some(e) = event.downcast::<CardsDrawnEvent>() {
            e.amount()
        } else {
            1
        }
    }

    fn display(&self) -> String {
        let you = self.player == PlayerFilter::You;
        let action = if self.per_card && you {
            "draw a card"
        } else if self.per_card {
            "draws a card"
        } else if you {
            "draw one or more cards"
        } else {
            "draws one or more cards"
        };
        match &self.player {
            PlayerFilter::You => {
                let base = format!("Whenever you {}", action);
                if let Some(not_during_turn) = &self.not_during_turn {
                    let suffix = match not_during_turn {
                        PlayerFilter::You => ", if it isn't your turn",
                        PlayerFilter::Opponent => ", if it isn't an opponent's turn",
                        PlayerFilter::Specific(_) | PlayerFilter::IteratedPlayer => {
                            ", if it isn't that player's turn"
                        }
                        _ => "",
                    };
                    format!("{base}{suffix}")
                } else {
                    base
                }
            }
            _ => {
                let base = format!(
                    "Whenever {} {}",
                    describe_player_filter_subject(&self.player),
                    action
                );
                if let Some(not_during_turn) = &self.not_during_turn {
                    let suffix = match not_during_turn {
                        PlayerFilter::You => ", if it isn't your turn",
                        PlayerFilter::Opponent => ", if it isn't an opponent's turn",
                        PlayerFilter::Specific(_) | PlayerFilter::IteratedPlayer => {
                            ", if it isn't that player's turn"
                        }
                        _ => "",
                    };
                    format!("{base}{suffix}")
                } else {
                    base
                }
            }
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
        let trigger = PlayerDrawsCardTrigger::new(PlayerFilter::Any);
        assert!(trigger.display().contains("draws one or more cards"));

        let per_card = PlayerDrawsCardTrigger::per_card(PlayerFilter::You);
        assert_eq!(per_card.display(), "Whenever you draw a card");

        let batch = PlayerDrawsCardTrigger::new(PlayerFilter::You);
        assert_eq!(batch.display(), "Whenever you draw one or more cards");
    }

    #[test]
    fn test_matches() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(1);
        let card_id = ObjectId::from_raw(2);

        let trigger = PlayerDrawsCardTrigger::new(PlayerFilter::You);
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        // Alice draws - should match
        let event = TriggerEvent::new_with_provenance(
            CardsDrawnEvent::single(alice, card_id, true),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(trigger.matches(&event, &ctx));

        // Bob draws - should not match (controller is Alice)
        let event2 = TriggerEvent::new_with_provenance(
            CardsDrawnEvent::single(bob, card_id, true),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&event2, &ctx));
    }

    #[test]
    fn test_trigger_count() {
        let cards = vec![
            ObjectId::from_raw(1),
            ObjectId::from_raw(2),
            ObjectId::from_raw(3),
        ];
        let event = TriggerEvent::new_with_provenance(
            CardsDrawnEvent::new(PlayerId::from_index(0), cards, true),
            crate::provenance::ProvNodeId::default(),
        );

        // Batch trigger fires once
        let batch_trigger = PlayerDrawsCardTrigger::new(PlayerFilter::Any);
        assert_eq!(batch_trigger.trigger_count(&event), 1);

        // Per-card trigger fires 3 times
        let per_card_trigger = PlayerDrawsCardTrigger::per_card(PlayerFilter::Any);
        assert_eq!(per_card_trigger.trigger_count(&event), 3);
    }

    #[test]
    fn shared_turn_restriction_recognizes_a_nonprimary_active_teammate() {
        let mut game = GameState::new(
            vec![
                "Alice".into(),
                "Bob".into(),
                "Charlie".into(),
                "Diana".into(),
            ],
            20,
        );
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let charlie = PlayerId::from_index(2);
        let diana = PlayerId::from_index(3);
        game.set_teams(vec![vec![alice, bob], vec![charlie, diana]])
            .expect("valid teams");
        game.enable_shared_team_turns().expect("shared turns");
        assert_eq!(game.active_player_id(), Some(bob));

        let source_id = ObjectId::from_raw(1);
        let card_id = ObjectId::from_raw(2);
        let event = TriggerEvent::new_with_provenance(
            CardsDrawnEvent::single(alice, card_id, true),
            crate::provenance::ProvNodeId::default(),
        );
        let trigger = PlayerDrawsCardTrigger::not_during_turn(PlayerFilter::You, PlayerFilter::You);
        let ctx = TriggerContext::for_source(source_id, alice, &game);
        assert!(
            !trigger.matches(&event, &ctx),
            "Alice is taking the shared turn even though Bob is its primary player"
        );

        let opponent_ctx = TriggerContext::for_source(source_id, charlie, &game);
        let opponent_turn =
            PlayerDrawsCardTrigger::not_during_turn(PlayerFilter::Any, PlayerFilter::Opponent);
        assert!(!opponent_turn.matches(&event, &opponent_ctx));
    }
}
