//! "Whenever [player] draws a card" trigger.

use crate::events::EventKind;
use crate::events::other::CardsDrawnEvent;
use crate::target::PlayerFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

/// Trigger for "Whenever [player] draws a card" or "Whenever [player] draws one or more cards".
///
/// By default, this matches when the specified player draws cards.
/// The `per_card` field controls whether the trigger fires once per card or once per draw action.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerDrawsCardTrigger {
    pub player: PlayerFilter,
    /// If true, fires once per card drawn. If false, fires once per draw action.
    pub per_card: bool,
}

impl PlayerDrawsCardTrigger {
    /// Create a trigger that fires once per draw action ("whenever you draw one or more cards").
    pub fn new(player: PlayerFilter) -> Self {
        Self {
            player,
            per_card: false,
        }
    }

    /// Create a trigger that fires once per card drawn ("whenever you draw a card").
    pub fn per_card(player: PlayerFilter) -> Self {
        Self {
            player,
            per_card: true,
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
        match &self.player {
            PlayerFilter::You => e.player == ctx.controller,
            PlayerFilter::Opponent => e.player != ctx.controller,
            PlayerFilter::Any => true,
            PlayerFilter::Specific(id) => e.player == *id,
            _ => true,
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
        let action = if self.per_card {
            "draws a card"
        } else {
            "draws one or more cards"
        };
        match &self.player {
            PlayerFilter::You => format!("Whenever you {}", action),
            PlayerFilter::Any => format!("Whenever a player {}", action),
            PlayerFilter::Opponent => format!("Whenever an opponent {}", action),
            PlayerFilter::Specific(_) | PlayerFilter::IteratedPlayer => {
                format!("Whenever that player {}", action)
            }
            _ => format!("Whenever {:?} {}", self.player, action),
        }
    }

    fn clone_box(&self) -> Box<dyn TriggerMatcher> {
        Box::new(self.clone())
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
        assert!(per_card.display().contains("draws a card"));
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
        let event = TriggerEvent::new(CardsDrawnEvent::single(alice, card_id, true));
        assert!(trigger.matches(&event, &ctx));

        // Bob draws - should not match (controller is Alice)
        let event2 = TriggerEvent::new(CardsDrawnEvent::single(bob, card_id, true));
        assert!(!trigger.matches(&event2, &ctx));
    }

    #[test]
    fn test_trigger_count() {
        let cards = vec![
            ObjectId::from_raw(1),
            ObjectId::from_raw(2),
            ObjectId::from_raw(3),
        ];
        let event = TriggerEvent::new(CardsDrawnEvent::new(PlayerId::from_index(0), cards, true));

        // Batch trigger fires once
        let batch_trigger = PlayerDrawsCardTrigger::new(PlayerFilter::Any);
        assert_eq!(batch_trigger.trigger_count(&event), 1);

        // Per-card trigger fires 3 times
        let per_card_trigger = PlayerDrawsCardTrigger::per_card(PlayerFilter::Any);
        assert_eq!(per_card_trigger.trigger_count(&event), 3);
    }
}
