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
