//! Preserve an unrepresentable authored life amount without narrowing it.
//!
//! The amount exceeds every representable (saturated `i32`) life total, so
//! the cost is simply unpayable (CR 119.4: a player can pay life only if
//! their life total is greater than or equal to the payment).
use super::{CostContext, CostPayer, CostPaymentResult};
use crate::{cost::CostPaymentError, game_state::GameState};
#[derive(Debug, Clone, PartialEq)]
pub(super) struct UnrepresentableLifePayment {
    pub amount: u32,
}
impl UnrepresentableLifePayment {
    fn error(&self) -> CostPaymentError {
        CostPaymentError::InsufficientLife
    }
}
impl CostPayer for UnrepresentableLifePayment {
    fn can_pay(&self, _: &GameState, _: &CostContext) -> Result<(), CostPaymentError> {
        Err(self.error())
    }
    fn pay(
        &self,
        _: &mut GameState,
        _: &mut CostContext,
    ) -> Result<CostPaymentResult, CostPaymentError> {
        Err(self.error())
    }
    fn display(&self) -> String {
        format!("Pay {} life", self.amount)
    }
    fn is_life_cost(&self) -> bool {
        true
    }
    fn life_amount(&self) -> Option<u32> {
        Some(self.amount)
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
