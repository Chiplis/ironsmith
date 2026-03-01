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

/// A life payment cost whose amount is `per_card * (cards in your hand)`.
///
/// Used for patterns like: "Pay 1 life for each card in your hand."
#[derive(Debug, Clone, PartialEq)]
pub struct LifePerCardInHandCost {
    pub per_card: u32,
}

impl LifePerCardInHandCost {
    pub fn new(per_card: u32) -> Self {
        Self { per_card }
    }
}

impl CostPayer for LifePerCardInHandCost {
    fn can_pay(&self, game: &GameState, ctx: &CostContext) -> Result<(), CostPaymentError> {
        let player = game
            .player(ctx.payer)
            .ok_or(CostPaymentError::PlayerNotFound)?;

        let needed = (player.hand.len() as u32).saturating_mul(self.per_card);
        if !game.can_pay_life(ctx.payer, needed) {
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

        let needed = game
            .player(ctx.payer)
            .map(|p| (p.hand.len() as u32).saturating_mul(self.per_card))
            .unwrap_or(0);

        if !game.pay_life(ctx.payer, needed) {
            return Err(CostPaymentError::InsufficientLife);
        }

        Ok(CostPaymentResult::Paid)
    }

    fn clone_box(&self) -> Box<dyn CostPayer> {
        Box::new(self.clone())
    }

    fn display(&self) -> String {
        // Mirror the oracle phrasing ("for each card...") even when per_card != 1.
        if self.per_card == 1 {
            "Pay 1 life for each card in your hand".to_string()
        } else {
            format!("Pay {} life for each card in your hand", self.per_card)
        }
    }

    fn is_life_cost(&self) -> bool {
        true
    }

    fn life_amount(&self) -> Option<u32> {
        // Dynamic based on hand size.
        None
    }
}

impl CostPayer for LifeCost {
    fn can_pay(&self, game: &GameState, ctx: &CostContext) -> Result<(), CostPaymentError> {
        game.player(ctx.payer)
            .ok_or(CostPaymentError::PlayerNotFound)?;

        // Check if player can pay this life amount (including life-total lock effects).
        if !game.can_pay_life(ctx.payer, self.amount) {
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
        if !game.pay_life(ctx.payer, self.amount) {
            return Err(CostPaymentError::InsufficientLife);
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
