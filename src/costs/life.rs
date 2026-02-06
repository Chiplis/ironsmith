//! Life payment cost implementation.

use crate::cost::CostPaymentError;
use crate::costs::{CostContext, CostPayer, CostPaymentResult};
use crate::game_state::GameState;

/// A life payment cost (e.g., "Pay 2 life").
///
/// The player must have enough life to pay.
/// Note: You can pay life down to (but not below) 0.
#[derive(Debug, Clone, PartialEq)]
pub struct LifeCost {
    /// The amount of life to pay.
    pub amount: u32,
}

impl LifeCost {
    /// Create a new life payment cost.
    pub fn new(amount: u32) -> Self {
        Self { amount }
    }
}

impl CostPayer for LifeCost {
    fn can_pay(&self, game: &GameState, ctx: &CostContext) -> Result<(), CostPaymentError> {
        let player = game
            .player(ctx.payer)
            .ok_or(CostPaymentError::PlayerNotFound)?;

        // Check if player has enough life (can go to 0)
        if player.life < (self.amount as i32) {
            return Err(CostPaymentError::InsufficientLife);
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

        // Pay life
        if let Some(player) = game.player_mut(ctx.payer) {
            player.life -= self.amount as i32;
        }

        Ok(CostPaymentResult::Paid)
    }

    fn clone_box(&self) -> Box<dyn CostPayer> {
        Box::new(self.clone())
    }

    fn display(&self) -> String {
        if self.amount == 1 {
            "Pay 1 life".to_string()
        } else {
            format!("Pay {} life", self.amount)
        }
    }

    fn is_life_cost(&self) -> bool {
        true
    }

    fn life_amount(&self) -> Option<u32> {
        Some(self.amount)
    }

    // LifeCost is an immediate cost - no player choice needed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{ObjectId, PlayerId};

    fn create_test_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    #[test]
    fn test_life_cost_display() {
        assert_eq!(LifeCost::new(1).display(), "Pay 1 life");
        assert_eq!(LifeCost::new(2).display(), "Pay 2 life");
        assert_eq!(LifeCost::new(5).display(), "Pay 5 life");
    }

    #[test]
    fn test_life_cost_not_mana_or_tap() {
        let cost = LifeCost::new(2);
        assert!(!cost.requires_tap());
        assert!(!cost.is_mana_cost());
    }

    #[test]
    fn test_life_cost_can_pay_sufficient_life() {
        let game = create_test_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(1);

        let cost = LifeCost::new(5);
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = CostContext::new(source, alice, &mut dm);

        // Alice has 20 life, can pay 5
        assert!(cost.can_pay(&game, &ctx).is_ok());
    }

    #[test]
    fn test_life_cost_cannot_pay_insufficient_life() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(1);

        // Set Alice's life to 3
        if let Some(player) = game.player_mut(alice) {
            player.life = 3;
        }

        let cost = LifeCost::new(5);
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = CostContext::new(source, alice, &mut dm);

        assert_eq!(
            cost.can_pay(&game, &ctx),
            Err(CostPaymentError::InsufficientLife)
        );
    }

    #[test]
    fn test_life_cost_can_pay_exact_life() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(1);

        // Set Alice's life to exactly 5
        if let Some(player) = game.player_mut(alice) {
            player.life = 5;
        }

        let cost = LifeCost::new(5);
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = CostContext::new(source, alice, &mut dm);

        // Can pay exactly down to 0
        assert!(cost.can_pay(&game, &ctx).is_ok());
    }

    #[test]
    fn test_life_cost_pay_success() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(1);

        let cost = LifeCost::new(5);
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx = CostContext::new(source, alice, &mut dm);

        assert_eq!(game.player(alice).unwrap().life, 20);
        let result = cost.pay(&mut game, &mut ctx);
        assert_eq!(result, Ok(CostPaymentResult::Paid));
        assert_eq!(game.player(alice).unwrap().life, 15);
    }

    #[test]
    fn test_life_cost_pay_to_zero() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(1);

        // Set Alice's life to exactly 5
        if let Some(player) = game.player_mut(alice) {
            player.life = 5;
        }

        let cost = LifeCost::new(5);
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx = CostContext::new(source, alice, &mut dm);

        let result = cost.pay(&mut game, &mut ctx);
        assert_eq!(result, Ok(CostPaymentResult::Paid));
        assert_eq!(game.player(alice).unwrap().life, 0);
    }

    #[test]
    fn test_life_cost_clone_box() {
        let cost = LifeCost::new(3);
        let cloned = cost.clone_box();
        assert!(format!("{:?}", cloned).contains("LifeCost"));
        assert!(format!("{:?}", cloned).contains("3"));
    }
}
