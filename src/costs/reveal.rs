//! Reveal from hand cost implementation.

use crate::cost::CostPaymentError;
use crate::costs::{CostContext, CostPayer, CostPaymentResult};
use crate::game_state::GameState;
use crate::types::CardType;

/// A reveal cards from hand cost.
///
/// The player must reveal cards from their hand matching the filter.
/// Revealing doesn't move the cards, just shows them to opponents.
#[derive(Debug, Clone, PartialEq)]
pub struct RevealFromHandCost {
    /// The number of cards to reveal.
    pub count: u32,
    /// Optional card type restriction.
    pub card_type: Option<CardType>,
}

impl RevealFromHandCost {
    /// Create a new reveal from hand cost.
    pub fn new(count: u32, card_type: Option<CardType>) -> Self {
        Self { count, card_type }
    }

    /// Create a cost to reveal any cards.
    pub fn any(count: u32) -> Self {
        Self::new(count, None)
    }

    /// Create a cost to reveal a card of a specific type.
    pub fn of_type(count: u32, card_type: CardType) -> Self {
        Self::new(count, Some(card_type))
    }

    /// Get the number of valid cards in hand for this cost.
    pub fn count_valid_cards(
        &self,
        game: &GameState,
        player: crate::ids::PlayerId,
        source: crate::ids::ObjectId,
    ) -> usize {
        let Some(player_obj) = game.player(player) else {
            return 0;
        };

        player_obj
            .hand
            .iter()
            .filter(|&&card_id| {
                // Don't count the spell being cast
                if card_id == source {
                    return false;
                }
                // Check card type filter
                if let Some(ct) = self.card_type {
                    if let Some(obj) = game.object(card_id) {
                        obj.has_card_type(ct)
                    } else {
                        false
                    }
                } else {
                    true
                }
            })
            .count()
    }
}

impl CostPayer for RevealFromHandCost {
    fn can_pay(&self, game: &GameState, ctx: &CostContext) -> Result<(), CostPaymentError> {
        let valid_count = self.count_valid_cards(game, ctx.payer, ctx.source);

        if valid_count < self.count as usize {
            return Err(CostPaymentError::InsufficientCardsToReveal);
        }

        Ok(())
    }

    fn pay(
        &self,
        game: &mut GameState,
        ctx: &mut CostContext,
    ) -> Result<CostPaymentResult, CostPaymentError> {
        // Verify we can still pay
        self.can_pay(game, ctx)?;

        // The actual reveal choice happens in the game loop
        Ok(CostPaymentResult::NeedsChoice(self.display()))
    }

    fn clone_box(&self) -> Box<dyn CostPayer> {
        Box::new(self.clone())
    }

    fn display(&self) -> String {
        let type_str = self.card_type.map_or("card".to_string(), |ct| {
            match ct {
                CardType::Creature => "creature card",
                CardType::Artifact => "artifact card",
                CardType::Enchantment => "enchantment card",
                CardType::Land => "land card",
                CardType::Planeswalker => "planeswalker card",
                CardType::Instant => "instant card",
                CardType::Sorcery => "sorcery card",
                CardType::Battle => "battle card",
                CardType::Kindred => "kindred card",
            }
            .to_string()
        });

        if self.count == 1 {
            format!("Reveal a {} from your hand", type_str)
        } else {
            format!("Reveal {} {}s from your hand", self.count, type_str)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::zone::Zone;

    fn create_test_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn simple_card(name: &str, id: u32) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(id), name)
            .card_types(vec![CardType::Creature])
            .build()
    }

    #[test]
    fn test_reveal_cost_display() {
        assert_eq!(
            RevealFromHandCost::any(1).display(),
            "Reveal a card from your hand"
        );
        assert_eq!(
            RevealFromHandCost::of_type(1, CardType::Land).display(),
            "Reveal a land card from your hand"
        );
        assert_eq!(
            RevealFromHandCost::any(2).display(),
            "Reveal 2 cards from your hand"
        );
    }

    #[test]
    fn test_reveal_cost_can_pay() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(999);

        let card1 = simple_card("Card 1", 1);
        let _id1 = game.create_object_from_card(&card1, alice, Zone::Hand);

        let cost = RevealFromHandCost::any(1);
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = CostContext::new(source, alice, &mut dm);

        assert!(cost.can_pay(&game, &ctx).is_ok());
    }

    #[test]
    fn test_reveal_cost_cannot_pay_insufficient() {
        let game = create_test_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(999);

        // Empty hand
        let cost = RevealFromHandCost::any(1);
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = CostContext::new(source, alice, &mut dm);

        assert_eq!(
            cost.can_pay(&game, &ctx),
            Err(CostPaymentError::InsufficientCardsToReveal)
        );
    }

    #[test]
    fn test_reveal_cost_excludes_source() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        // Create the source card in hand
        let source_card = simple_card("Source", 1);
        let source_id = game.create_object_from_card(&source_card, alice, Zone::Hand);

        let cost = RevealFromHandCost::any(1);
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = CostContext::new(source_id, alice, &mut dm);

        // Should fail because the only card is the source
        assert_eq!(
            cost.can_pay(&game, &ctx),
            Err(CostPaymentError::InsufficientCardsToReveal)
        );
    }

    #[test]
    fn test_reveal_cost_pay_returns_needs_choice() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(999);

        let card1 = simple_card("Card 1", 1);
        let _id1 = game.create_object_from_card(&card1, alice, Zone::Hand);

        let cost = RevealFromHandCost::any(1);
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx = CostContext::new(source, alice, &mut dm);

        let result = cost.pay(&mut game, &mut ctx);
        assert!(matches!(result, Ok(CostPaymentResult::NeedsChoice(_))));
    }

    #[test]
    fn test_reveal_cost_clone_box() {
        let cost = RevealFromHandCost::any(1);
        let cloned = cost.clone_box();
        assert!(format!("{:?}", cloned).contains("RevealFromHandCost"));
    }
}
