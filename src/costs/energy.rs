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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{ObjectId, PlayerId};

    fn create_test_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    #[test]
    fn test_energy_cost_display() {
        assert_eq!(EnergyCost::new(1).display(), "Pay {E}");
        assert_eq!(EnergyCost::new(3).display(), "Pay {E}{E}{E}");
    }

    #[test]
    fn test_energy_cost_can_pay_with_energy() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(1);

        // Give Alice energy
        if let Some(player) = game.player_mut(alice) {
            player.energy_counters = 5;
        }

        let cost = EnergyCost::new(3);
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = CostContext::new(source, alice, &mut dm);

        assert!(cost.can_pay(&game, &ctx).is_ok());
    }

    #[test]
    fn test_energy_cost_cannot_pay_insufficient() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(1);

        // Give Alice only 2 energy
        if let Some(player) = game.player_mut(alice) {
            player.energy_counters = 2;
        }

        let cost = EnergyCost::new(3);
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = CostContext::new(source, alice, &mut dm);

        assert_eq!(
            cost.can_pay(&game, &ctx),
            Err(CostPaymentError::InsufficientEnergy)
        );
    }

    #[test]
    fn test_energy_cost_pay_success() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(1);

        // Give Alice energy
        if let Some(player) = game.player_mut(alice) {
            player.energy_counters = 5;
        }

        let cost = EnergyCost::new(3);
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx = CostContext::new(source, alice, &mut dm);

        let result = cost.pay(&mut game, &mut ctx);
        assert_eq!(result, Ok(CostPaymentResult::Paid));

        // Should have 2 energy left
        assert_eq!(game.player(alice).unwrap().energy_counters, 2);
    }

    #[test]
    fn test_energy_cost_clone_box() {
        let cost = EnergyCost::new(2);
        let cloned = cost.clone_box();
        assert!(format!("{:?}", cloned).contains("EnergyCost"));
    }
}
