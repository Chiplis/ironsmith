//! Effect-backed cost component.
//!
//! This lets costs flow through the normal effect executor/event pipeline
//! while still being represented as a first-class `Cost` inside `TotalCost`.

use crate::cost::CostPaymentError;
use crate::costs::{CostContext, CostPayer, CostPaymentResult};
use crate::effect::Effect;
use crate::effects::CostValidationError;
use crate::events::cause::EventCause;
use crate::executor::{ExecutionContext, execute_effect};
use crate::game_state::GameState;

/// Convert a CostValidationError to CostPaymentError.
fn convert_validation_error(err: CostValidationError) -> CostPaymentError {
    match err {
        CostValidationError::AlreadyTapped => CostPaymentError::AlreadyTapped,
        CostValidationError::SummoningSickness => CostPaymentError::SummoningSickness,
        CostValidationError::NotEnoughLife => CostPaymentError::InsufficientLife,
        CostValidationError::NotEnoughCards => CostPaymentError::InsufficientCardsInHand,
        CostValidationError::CannotSacrifice => CostPaymentError::NoValidSacrificeTarget,
        CostValidationError::Other(msg) => CostPaymentError::Other(msg),
    }
}

/// A cost paid by executing a single effect.
#[derive(Debug, Clone)]
pub struct EffectCost {
    /// Effect executed as part of paying this cost.
    pub effect: Effect,
}

impl EffectCost {
    pub fn new(effect: Effect) -> Self {
        Self { effect }
    }
}

impl PartialEq for EffectCost {
    fn eq(&self, _other: &Self) -> bool {
        // Effect partial-eq is intentionally behavioral/not structural.
        false
    }
}

impl CostPayer for EffectCost {
    fn can_pay(&self, game: &GameState, ctx: &CostContext) -> Result<(), CostPaymentError> {
        self.effect
            .0
            .can_execute_as_cost(game, ctx.source, ctx.payer)
            .map_err(convert_validation_error)
    }

    fn pay(
        &self,
        game: &mut GameState,
        ctx: &mut CostContext,
    ) -> Result<CostPaymentResult, CostPaymentError> {
        self.can_pay(game, ctx)?;

        // Clone the existing tags to pass to ExecutionContext
        let existing_tags = ctx.tagged_objects.clone();

        let mut exec_ctx = ExecutionContext::new(ctx.source, ctx.payer, &mut *ctx.decision_maker)
            .with_cause(EventCause::from_cost(ctx.source, ctx.payer))
            .with_tagged_objects(existing_tags);

        execute_effect(game, &self.effect, &mut exec_ctx)
            .map_err(|e| CostPaymentError::Other(format!("{e:?}")))?;

        // Copy any new tags back to CostContext for subsequent costs
        ctx.tagged_objects = exec_ctx.tagged_objects;

        Ok(CostPaymentResult::Paid)
    }

    fn clone_box(&self) -> Box<dyn CostPayer> {
        Box::new(self.clone())
    }

    fn display(&self) -> String {
        self.effect
            .0
            .cost_description()
            .unwrap_or_else(|| format!("{:?}", self.effect))
    }

    fn requires_tap(&self) -> bool {
        self.effect.0.is_tap_source_cost()
    }

    fn is_life_cost(&self) -> bool {
        self.effect.0.pay_life_amount().is_some()
    }

    fn life_amount(&self) -> Option<u32> {
        self.effect.0.pay_life_amount()
    }

    fn is_sacrifice_self(&self) -> bool {
        self.effect.0.is_sacrifice_source_cost()
    }

    fn processing_mode(&self) -> crate::costs::CostProcessingMode {
        crate::costs::CostProcessingMode::Immediate
    }
}
