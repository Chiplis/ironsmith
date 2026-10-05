//! Preserve an unrepresentable authored life amount without narrowing it.
use super::{CostContext, CostPayer, CostPaymentResult};
use crate::{cost::CostPaymentError, effects::ExecutionError, game_state::GameState};
#[derive(Debug, Clone, PartialEq)]
pub(super) struct UnrepresentableLifePayment {
    pub amount: u32,
}
impl UnrepresentableLifePayment {
    fn error(&self) -> CostPaymentError {
        CostPaymentError::ExecutionFailed(ExecutionError::ResourceLimitExceeded {
            resource: "life payment cost amount",
            requested: u128::from(self.amount),
            maximum: i32::MAX as u128,
        })
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
