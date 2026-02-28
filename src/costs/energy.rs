//! Energy cost implementation.

use crate::cost::CostPaymentError;
use crate::costs::{CostContext, CostPayer, CostPaymentResult};
use crate::game_state::GameState;

/// An energy payment cost (e.g., pay {E}{E}{E}).
///
/// The player must have enough energy counters.
#[derive(Debug, Clone, PartialEq)]
pub struct EnergyCost {
    /// The amount of energy to pay.
    pub amount: u32,
}

impl EnergyCost {
    /// Create a new energy payment cost.
    pub fn new(amount: u32) -> Self {
        Self { amount }
    }
}

impl CostPayer for EnergyCost {
    fn can_pay(&self, game: &GameState, ctx: &CostContext) -> Result<(), CostPaymentError> {
        let player = game
            .player(ctx.payer)
            .ok_or(CostPaymentError::PlayerNotFound)?;

        if player.energy_counters < self.amount {
            return Err(CostPaymentError::InsufficientEnergy);
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

        // Pay energy
        if let Some(player) = game.player_mut(ctx.payer) {
            player.energy_counters = player.energy_counters.saturating_sub(self.amount);
        }

        Ok(CostPaymentResult::Paid)
    }

    fn clone_box(&self) -> Box<dyn CostPayer> {
        Box::new(self.clone())
    }

    fn display(&self) -> String {
        // Energy uses {E} symbol
        let symbols: String = (0..self.amount).map(|_| "{E}").collect();
        if symbols.is_empty() {
            "Pay no energy".to_string()
        } else {
            format!("Pay {}", symbols)
        }
    }
}
