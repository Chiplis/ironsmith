//! Mill cost implementation.

use crate::cost::CostPaymentError;
use crate::costs::{CostContext, CostPayer, CostPaymentResult};
use crate::game_state::GameState;
use crate::zone::Zone;

/// A mill cost (put cards from library into graveyard).
///
/// The player mills cards from the top of their library.
/// Milling from an empty library is legal (just does nothing).
#[derive(Debug, Clone, PartialEq)]
pub struct MillCost {
    /// The number of cards to mill.
    pub count: u32,
}

impl MillCost {
    /// Create a new mill cost.
    pub fn new(count: u32) -> Self {
        Self { count }
    }
}

impl CostPayer for MillCost {
    fn can_pay(&self, _game: &GameState, _ctx: &CostContext) -> Result<(), CostPaymentError> {
        // Milling from empty library is legal (just does nothing)
        Ok(())
    }

    fn pay(
        &self,
        game: &mut GameState,
        ctx: &mut CostContext,
    ) -> Result<CostPaymentResult, CostPaymentError> {
        // Mill cards from library
        if let Some(player) = game.player(ctx.payer) {
            let cards_to_mill: Vec<_> = player
                .library
                .iter()
                .take(self.count as usize)
                .copied()
                .collect();

            for card_id in cards_to_mill {
                game.move_object(card_id, Zone::Graveyard);
            }
        }

        Ok(CostPaymentResult::Paid)
    }

    fn clone_box(&self) -> Box<dyn CostPayer> {
        Box::new(self.clone())
    }

    fn display(&self) -> String {
        if self.count == 1 {
            "Mill a card".to_string()
        } else {
            format!("Mill {} cards", self.count)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::types::CardType;

    fn create_test_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn simple_card(name: &str, id: u32) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(id), name)
            .card_types(vec![CardType::Creature])
            .build()
    }

    #[test]
    fn test_mill_cost_display() {
        assert_eq!(MillCost::new(1).display(), "Mill a card");
        assert_eq!(MillCost::new(3).display(), "Mill 3 cards");
    }

    #[test]
    fn test_mill_cost_can_always_pay() {
        let game = create_test_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(999);

        let cost = MillCost::new(5);
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = CostContext::new(source, alice, &mut dm);

        // Can pay even with empty library
        assert!(cost.can_pay(&game, &ctx).is_ok());
    }

    #[test]
    fn test_mill_cost_pay_moves_cards() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(999);

        // Add cards to library
        for i in 0..5 {
            let card = simple_card(&format!("Card {}", i), i + 1);
            let _id = game.create_object_from_card(&card, alice, Zone::Library);
        }

        let initial_library_size = game.player(alice).unwrap().library.len();
        let initial_graveyard_size = game.player(alice).unwrap().graveyard.len();

        let cost = MillCost::new(3);
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx = CostContext::new(source, alice, &mut dm);

        let result = cost.pay(&mut game, &mut ctx);
        assert_eq!(result, Ok(CostPaymentResult::Paid));

        // Library should have 2 fewer cards (3 milled)
        assert_eq!(
            game.player(alice).unwrap().library.len(),
            initial_library_size - 3
        );
        // Graveyard should have 3 more cards
        assert_eq!(
            game.player(alice).unwrap().graveyard.len(),
            initial_graveyard_size + 3
        );
    }

    #[test]
    fn test_mill_cost_partial_mill() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(999);

        // Add only 2 cards to library
        for i in 0..2 {
            let card = simple_card(&format!("Card {}", i), i + 1);
            let _id = game.create_object_from_card(&card, alice, Zone::Library);
        }

        let cost = MillCost::new(5); // Try to mill more than available
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx = CostContext::new(source, alice, &mut dm);

        let result = cost.pay(&mut game, &mut ctx);
        assert_eq!(result, Ok(CostPaymentResult::Paid));

        // Library should be empty (milled all 2 cards)
        assert!(game.player(alice).unwrap().library.is_empty());
        // Graveyard should have 2 cards
        assert_eq!(game.player(alice).unwrap().graveyard.len(), 2);
    }

    #[test]
    fn test_mill_cost_clone_box() {
        let cost = MillCost::new(2);
        let cloned = cost.clone_box();
        assert!(format!("{:?}", cloned).contains("MillCost"));
    }
}
