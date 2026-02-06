//! Card drawn event implementation.

use std::any::Any;

use crate::events::traits::{EventKind, GameEventType};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;

/// A player drew one or more cards event.
///
/// This unified event is used for all draw triggers:
/// - "Whenever you draw a card" triggers iterate over `cards`
/// - "Whenever you draw one or more cards" triggers fire once
/// - Miracle checks `first_card()` + `is_first_this_turn`
#[derive(Debug, Clone)]
pub struct CardsDrawnEvent {
    /// The player who drew the cards
    pub player: PlayerId,
    /// The cards that were drawn (in hand after drawing)
    pub cards: Vec<ObjectId>,
    /// Whether this draw action started as the first draw this turn
    pub is_first_this_turn: bool,
}

impl CardsDrawnEvent {
    /// Create a new cards drawn event.
    pub fn new(player: PlayerId, cards: Vec<ObjectId>, is_first_this_turn: bool) -> Self {
        Self {
            player,
            cards,
            is_first_this_turn,
        }
    }

    /// Create a cards drawn event for a single card.
    pub fn single(player: PlayerId, card: ObjectId, is_first_this_turn: bool) -> Self {
        Self::new(player, vec![card], is_first_this_turn)
    }

    /// Get the first card drawn, if any.
    ///
    /// Used by miracle triggers to check if a specific card was the first drawn.
    pub fn first_card(&self) -> Option<ObjectId> {
        self.cards.first().copied()
    }

    /// Get the number of cards drawn.
    pub fn amount(&self) -> u32 {
        self.cards.len() as u32
    }

    /// Check if a specific card was drawn in this event.
    pub fn contains(&self, card: ObjectId) -> bool {
        self.cards.contains(&card)
    }

    /// Check if a specific card was the first card drawn (for miracle).
    pub fn is_miracle_eligible(&self, card: ObjectId) -> bool {
        self.is_first_this_turn && self.first_card() == Some(card)
    }
}

impl GameEventType for CardsDrawnEvent {
    fn event_kind(&self) -> EventKind {
        EventKind::CardsDrawn
    }

    fn clone_box(&self) -> Box<dyn GameEventType> {
        Box::new(self.clone())
    }

    fn affected_player(&self, _game: &GameState) -> PlayerId {
        self.player
    }

    fn with_target_replaced(&self, _old: &Target, _new: &Target) -> Option<Box<dyn GameEventType>> {
        None
    }

    fn display(&self) -> String {
        let amount = self.cards.len();
        if amount == 1 {
            "Player drew a card".to_string()
        } else {
            format!("Player drew {} cards", amount)
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn object_id(&self) -> Option<ObjectId> {
        self.first_card()
    }

    fn player(&self) -> Option<PlayerId> {
        Some(self.player)
    }

    fn controller(&self) -> Option<PlayerId> {
        None
    }

    fn snapshot(&self) -> Option<&ObjectSnapshot> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cards_drawn_event_creation() {
        let cards = vec![ObjectId::from_raw(1), ObjectId::from_raw(2)];
        let event = CardsDrawnEvent::new(PlayerId::from_index(0), cards.clone(), false);
        assert_eq!(event.player, PlayerId::from_index(0));
        assert_eq!(event.cards, cards);
        assert!(!event.is_first_this_turn);
        assert_eq!(event.amount(), 2);
    }

    #[test]
    fn test_cards_drawn_event_single() {
        let card = ObjectId::from_raw(1);
        let event = CardsDrawnEvent::single(PlayerId::from_index(0), card, true);
        assert_eq!(event.cards.len(), 1);
        assert_eq!(event.first_card(), Some(card));
        assert!(event.is_first_this_turn);
    }

    #[test]
    fn test_cards_drawn_event_first_card() {
        let cards = vec![ObjectId::from_raw(1), ObjectId::from_raw(2)];
        let event = CardsDrawnEvent::new(PlayerId::from_index(0), cards, true);
        assert_eq!(event.first_card(), Some(ObjectId::from_raw(1)));
    }

    #[test]
    fn test_cards_drawn_event_empty() {
        let event = CardsDrawnEvent::new(PlayerId::from_index(0), vec![], false);
        assert_eq!(event.first_card(), None);
        assert_eq!(event.amount(), 0);
    }

    #[test]
    fn test_cards_drawn_event_contains() {
        let card1 = ObjectId::from_raw(1);
        let card2 = ObjectId::from_raw(2);
        let card3 = ObjectId::from_raw(3);
        let event = CardsDrawnEvent::new(PlayerId::from_index(0), vec![card1, card2], false);
        assert!(event.contains(card1));
        assert!(event.contains(card2));
        assert!(!event.contains(card3));
    }

    #[test]
    fn test_cards_drawn_event_miracle_eligible() {
        let card1 = ObjectId::from_raw(1);
        let card2 = ObjectId::from_raw(2);

        // First card on first draw - eligible
        let event = CardsDrawnEvent::new(PlayerId::from_index(0), vec![card1, card2], true);
        assert!(event.is_miracle_eligible(card1));
        assert!(!event.is_miracle_eligible(card2)); // Not first card

        // Not first draw - not eligible
        let event2 = CardsDrawnEvent::new(PlayerId::from_index(0), vec![card1], false);
        assert!(!event2.is_miracle_eligible(card1));
    }

    #[test]
    fn test_cards_drawn_event_kind() {
        let event =
            CardsDrawnEvent::new(PlayerId::from_index(0), vec![ObjectId::from_raw(1)], true);
        assert_eq!(event.event_kind(), EventKind::CardsDrawn);
    }

    #[test]
    fn test_cards_drawn_event_display() {
        let event1 = CardsDrawnEvent::single(PlayerId::from_index(0), ObjectId::from_raw(1), false);
        assert_eq!(event1.display(), "Player drew a card");

        let event2 = CardsDrawnEvent::new(
            PlayerId::from_index(0),
            vec![
                ObjectId::from_raw(1),
                ObjectId::from_raw(2),
                ObjectId::from_raw(3),
            ],
            false,
        );
        assert_eq!(event2.display(), "Player drew 3 cards");
    }
}
