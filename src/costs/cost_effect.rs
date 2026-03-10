//! Effect-backed cost component.
//!
//! This lets costs flow through the normal effect executor/event pipeline
//! while still being represented as a first-class `Cost` inside `TotalCost`.

use crate::cost::CostPaymentError;
use crate::costs::{CostContext, CostPayer, CostPaymentResult};
use crate::effect::Effect;
use crate::effects::{CostExecutableEffect, CostValidationError};
use crate::events::cause::EventCause;
use crate::executor::{ExecutionContext, execute_effect};
use crate::game_state::GameState;

/// Convert a CostValidationError to CostPaymentError.
fn convert_validation_error(err: CostValidationError) -> CostPaymentError {
    match err {
        CostValidationError::AlreadyTapped => CostPaymentError::AlreadyTapped,
        CostValidationError::AlreadyUntapped => CostPaymentError::AlreadyUntapped,
        CostValidationError::SummoningSickness => CostPaymentError::SummoningSickness,
        CostValidationError::NotEnoughLife => CostPaymentError::InsufficientLife,
        CostValidationError::NotEnoughCards => CostPaymentError::InsufficientCardsInHand,
        CostValidationError::CannotSacrifice => CostPaymentError::NoValidSacrificeTarget,
        CostValidationError::Other(msg) => CostPaymentError::Other(msg),
    }
}

/// A cost paid by executing a single effect.
#[derive(Debug, Clone)]
pub struct CostEffect {
    /// Effect executed as part of paying this cost.
    pub effect: Effect,
}

impl CostEffect {
    pub fn new<E: CostExecutableEffect + 'static>(effect: E) -> Self {
        Self {
            effect: Effect::new(effect),
        }
    }

    pub fn from_validated_effect(effect: Effect) -> Result<Self, String> {
        if effect.0.as_cost_executable().is_some() {
            Ok(Self { effect })
        } else {
            Err("effect is not marked as cost-executable".to_string())
        }
    }
}

impl PartialEq for CostEffect {
    fn eq(&self, _other: &Self) -> bool {
        // Effect partial-eq is intentionally behavioral/not structural.
        false
    }
}

impl CostPayer for CostEffect {
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
        let chosen_targets = ctx
            .pre_chosen_cards
            .iter()
            .copied()
            .map(crate::executor::ResolvedTarget::Object)
            .collect();

        let mut exec_ctx = ExecutionContext::new(ctx.source, ctx.payer, &mut *ctx.decision_maker)
            .with_cause(EventCause::from_cost(ctx.source, ctx.payer))
            .with_tagged_objects(existing_tags)
            .with_targets(chosen_targets)
            .with_provenance(ctx.provenance);
        if let Some(x) = ctx.x_value {
            exec_ctx = exec_ctx.with_x(x);
        }

        let outcome = execute_effect(game, &self.effect, &mut exec_ctx)
            .map_err(|e| CostPaymentError::Other(format!("{e:?}")))?;
        for event in outcome.events {
            game.queue_trigger_event(ctx.provenance, event);
        }

        if ctx.x_value.is_none()
            && let crate::effect::EffectResult::Count(count) = outcome.result
            && count > 0
            && self
                .effect
                .downcast_ref::<crate::effects::RemoveCountersEffect>()
                .is_some_and(|effect| matches!(effect.target, crate::target::ChooseSpec::Source))
        {
            ctx.x_value = Some(count as u32);
        }

        // Copy any new tags back to CostContext for subsequent costs
        ctx.tagged_objects = exec_ctx.tagged_objects;
        ctx.pre_chosen_cards.clear();

        Ok(CostPaymentResult::Paid)
    }

    fn display(&self) -> String {
        self.effect
            .0
            .cost_description()
            .or_else(|| {
                let rendered =
                    crate::compiled_text::compile_effect_list(std::slice::from_ref(&self.effect));
                if rendered.trim().is_empty() {
                    None
                } else {
                    Some(rendered)
                }
            })
            .unwrap_or_else(|| "Perform the stated effect".to_string())
    }

    fn requires_tap(&self) -> bool {
        self.effect.0.is_tap_source_cost()
    }

    fn requires_untap(&self) -> bool {
        self.effect.0.is_untap_source_cost()
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

    fn is_sacrifice(&self) -> bool {
        self.effect
            .downcast_ref::<crate::effects::SacrificeEffect>()
            .is_some()
    }

    fn sacrifice_filter(&self) -> Option<&crate::filter::ObjectFilter> {
        self.effect
            .downcast_ref::<crate::effects::SacrificeEffect>()
            .map(|effect| &effect.filter)
    }

    fn is_discard(&self) -> bool {
        self.effect
            .downcast_ref::<crate::effects::DiscardEffect>()
            .is_some()
            || self
                .effect
                .downcast_ref::<crate::effects::DiscardHandEffect>()
                .is_some()
    }

    fn discard_details(&self) -> Option<(u32, Option<crate::types::CardType>)> {
        let effect = self
            .effect
            .downcast_ref::<crate::effects::DiscardEffect>()?;
        let crate::effect::Value::Fixed(count) = effect.count else {
            return None;
        };
        Some((
            count.max(0) as u32,
            effect
                .card_filter
                .as_ref()
                .and_then(|filter| filter.card_types.first().copied()),
        ))
    }

    fn is_exile_from_hand(&self) -> bool {
        self.exile_from_hand_details().is_some()
    }

    fn exile_from_hand_details(&self) -> Option<(u32, Option<crate::color::ColorSet>)> {
        self.effect.0.exile_from_hand_cost_info()
    }

    fn is_remove_counters(&self) -> bool {
        self.effect
            .downcast_ref::<crate::effects::RemoveCountersEffect>()
            .is_some()
    }

    fn processing_mode(&self) -> crate::costs::CostProcessingMode {
        use crate::costs::CostProcessingMode;
        use crate::effects::{
            DiscardEffect, DiscardHandEffect, ExileEffect, MillEffect, PayEnergyEffect,
            PutCountersEffect, RemoveCountersEffect, ReturnToHandEffect, SacrificeEffect,
            SacrificeTargetEffect, TapEffect, UntapEffect,
        };
        use crate::target::{ChooseSpec, PlayerFilter};

        if let Some(effect) = self.effect.downcast_ref::<TapEffect>()
            && matches!(effect.spec, ChooseSpec::Source)
        {
            return CostProcessingMode::Immediate;
        }

        if let Some(effect) = self.effect.downcast_ref::<UntapEffect>()
            && matches!(effect.spec, ChooseSpec::Source)
        {
            return CostProcessingMode::Immediate;
        }

        if self
            .effect
            .downcast_ref::<crate::effects::LoseLifeEffect>()
            .is_some()
            || self.effect.downcast_ref::<PayEnergyEffect>().is_some()
            || self.effect.downcast_ref::<MillEffect>().is_some()
        {
            return CostProcessingMode::Immediate;
        }

        if let Some(effect) = self.effect.downcast_ref::<PutCountersEffect>()
            && matches!(effect.target, ChooseSpec::Source)
        {
            return CostProcessingMode::Immediate;
        }

        if let Some(effect) = self.effect.downcast_ref::<RemoveCountersEffect>()
            && matches!(effect.target, ChooseSpec::Source)
        {
            return CostProcessingMode::Immediate;
        }

        if let Some(effect) = self.effect.downcast_ref::<DiscardHandEffect>()
            && effect.player == PlayerFilter::You
        {
            return CostProcessingMode::Immediate;
        }

        if let Some(effect) = self.effect.downcast_ref::<SacrificeTargetEffect>()
            && matches!(effect.target, ChooseSpec::Source)
        {
            return CostProcessingMode::InlineWithTriggers;
        }

        if let Some(effect) = self.effect.downcast_ref::<SacrificeEffect>()
            && effect.player == PlayerFilter::You
            && matches!(effect.count, crate::effect::Value::Fixed(1))
        {
            return CostProcessingMode::SacrificeTarget {
                filter: effect.filter.clone(),
            };
        }

        if let Some(effect) = self.effect.downcast_ref::<DiscardEffect>()
            && effect.player == PlayerFilter::You
            && !effect.random
            && let crate::effect::Value::Fixed(count) = effect.count
        {
            if effect
                .card_filter
                .as_ref()
                .is_some_and(|filter| filter.source && filter.zone == Some(crate::zone::Zone::Hand))
            {
                return CostProcessingMode::Immediate;
            }
            return CostProcessingMode::DiscardCards {
                count: count.max(0) as u32,
                card_types: effect
                    .card_filter
                    .as_ref()
                    .map(|filter| filter.card_types.clone())
                    .unwrap_or_default(),
            };
        }

        if let Some(effect) = self.effect.downcast_ref::<ExileEffect>() {
            if matches!(effect.spec.base(), ChooseSpec::Source) {
                return CostProcessingMode::Immediate;
            }

            if let ChooseSpec::Object(filter) = effect.spec.base()
                && let crate::effect::ChoiceCount {
                    min,
                    max: Some(max),
                    dynamic_x: false,
                    ..
                } = effect.spec.count()
                && min == max
            {
                if filter.zone == Some(crate::zone::Zone::Hand) {
                    return CostProcessingMode::ExileFromHand {
                        count: min as u32,
                        color_filter: filter.colors,
                    };
                }
                if filter.zone == Some(crate::zone::Zone::Graveyard) {
                    return CostProcessingMode::ExileFromGraveyard {
                        count: min as u32,
                        card_type: filter.card_types.first().copied(),
                    };
                }
            }
        }

        if let Some(effect) = self.effect.downcast_ref::<ReturnToHandEffect>() {
            return match effect.spec.base() {
                ChooseSpec::Source => CostProcessingMode::Immediate,
                ChooseSpec::Object(filter) => CostProcessingMode::ReturnToHandTarget {
                    filter: filter.clone(),
                },
                _ => CostProcessingMode::Immediate,
            };
        }

        CostProcessingMode::Immediate
    }

    fn effect_ref(&self) -> Option<&crate::effect::Effect> {
        Some(&self.effect)
    }
}
