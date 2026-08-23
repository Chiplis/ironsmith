//! Dynamic mana payment cost model.
//!
//! Dynamic mana costs carry `Value` expressions and must be resolved by a
//! payment helper that has the active execution context. The `CostPayer`
//! implementation is intentionally non-paying so accidentally routing an
//! unresolved dynamic cost through a flat payment path fails loudly.

use crate::cost::CostPaymentError;
use crate::costs::{CostContext, CostPayer, CostPaymentResult};
use crate::game_state::GameState;

#[derive(Debug, Clone, PartialEq)]
pub struct DynamicManaPaymentCost {
    pub cost: ironsmith_core::DynamicManaCost,
}

impl DynamicManaPaymentCost {
    pub fn new(cost: ironsmith_core::DynamicManaCost) -> Self {
        Self { cost }
    }
}

impl CostPayer for DynamicManaPaymentCost {
    fn can_pay(&self, _game: &GameState, _ctx: &CostContext) -> Result<(), CostPaymentError> {
        if let Some(static_base) = self.cost.resolved_static_base()
            && static_base.is_empty()
        {
            return Ok(());
        }
        Err(CostPaymentError::Other(
            "dynamic mana cost must be resolved before payment".to_string(),
        ))
    }

    fn pay(
        &self,
        _game: &mut GameState,
        _ctx: &mut CostContext,
    ) -> Result<CostPaymentResult, CostPaymentError> {
        Err(CostPaymentError::Other(
            "dynamic mana cost must be resolved before payment".to_string(),
        ))
    }

    fn display(&self) -> String {
        self.cost.display()
    }

    fn is_mana_cost(&self) -> bool {
        true
    }

    fn needs_player_choice(&self) -> bool {
        true
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
