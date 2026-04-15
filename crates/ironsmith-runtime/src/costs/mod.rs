//! Modular cost system for MTG.
//!
//! This module provides the `Cost` wrapper and shared infrastructure for cost payment.
//! Most non-mana costs now execute through [`CostEffect`], which routes them through
//! the normal effect pipeline while preserving the `CostPayer` interface.
//!
//! # Module Structure
//!
//! ```text
//! costs/
//!   mod.rs              - This file, module organization and Cost wrapper
//!   cost_effect.rs      - Effect-backed CostPayer implementation
//!   payer_trait.rs      - CostPayer trait definition and CostContext
//!   mana.rs             - ManaPaymentCost implementation
//!   non-mana costs      - Effect-backed via CostEffect
//! ```
//!
//! # Usage
//!
//! Costs can be checked and paid through the `CostPayer` trait:
//!
//! ```ignore
//! use ironsmith::costs::{Cost, CostContext};
//!
//! let cost = Cost::tap();
//! let ctx = CostContext::new(permanent_id, player_id);
//!
//! // Check if cost can be paid
//! if cost.can_pay(&game, &ctx).is_ok() {
//!     cost.pay(&mut game, &mut ctx)?;
//! }
//! ```

mod cost_effect;
mod mana;
mod payer_trait;
mod processing_mode;

// Re-export the trait and context
pub use payer_trait::{
    CostCheckContext, can_pay_with_check_context, can_potentially_pay_with_check_context,
};
pub use payer_trait::{CostContext, CostPayer, CostPaymentResult, PaymentReason};
pub use processing_mode::CostProcessingMode;

// Re-export all cost implementations
pub use cost_effect::CostEffect;
pub use mana::ManaPaymentCost;

use crate::color::ColorSet;
use crate::filter::ObjectFilter;
use crate::mana::ManaCost;
use crate::object::CounterType;
use crate::types::CardType;
use std::sync::Arc;

/// A wrapper around a boxed CostPayer trait object.
///
/// This provides a convenient way to work with costs as values while
/// maintaining the flexibility of trait objects.
#[derive(Debug)]
pub struct Cost(pub Arc<dyn CostPayer>);

impl Clone for Cost {
    fn clone(&self) -> Self {
        Cost(Arc::clone(&self.0))
    }
}

impl PartialEq for Cost {
    fn eq(&self, other: &Self) -> bool {
        // Compare costs by their display string representation.
        // This is an approximation but sufficient for most use cases.
        self.0.display() == other.0.display()
    }
}

impl Cost {
    /// Create a new Cost from any CostPayer implementation.
    pub fn new<C: CostPayer + 'static>(payer: C) -> Self {
        Cost(Arc::new(payer))
    }

    // ========================================================================
    // Convenience constructors
    // ========================================================================

    /// Create a tap cost ({T}).
    pub fn tap() -> Self {
        Self::effect(crate::effects::TapEffect::source())
    }

    /// Create an untap cost ({Q}).
    pub fn untap() -> Self {
        Self::effect(crate::effects::UntapEffect::with_spec(
            crate::target::ChooseSpec::Source,
        ))
    }

    /// Create a life payment cost.
    pub fn life(amount: u32) -> Self {
        Self::effect(crate::effects::LoseLifeEffect::you(amount))
    }

    /// Create a mana cost.
    pub fn mana(cost: ManaCost) -> Self {
        Self::new(ManaPaymentCost::new(cost))
    }

    /// Create a cost backed by an effect executor.
    pub fn effect<E: crate::effects::CostExecutableEffect + 'static>(effect: E) -> Self {
        Self::new(CostEffect::new(effect))
    }

    /// Create a cost from an effect value after validating that the runtime effect
    /// explicitly opted into cost execution.
    pub(crate) fn validated_effect(effect: crate::effect::Effect) -> Self {
        Self::new(
            CostEffect::from_validated_effect(effect)
                .expect("attempted to use a non-cost effect as a cost"),
        )
    }

    /// Convert a runtime effect into a canonical cost component.
    pub(crate) fn try_from_runtime_effect(effect: crate::effect::Effect) -> Result<Self, String> {
        if let Some(amount) = effect.0.pay_life_amount() {
            return Ok(Self::life(amount));
        }
        if let Some((count, color_filter)) = effect.0.exile_from_hand_cost_info() {
            return Ok(Self::exile_from_hand(count, color_filter));
        }
        if effect.0.is_sacrifice_source_cost() {
            return Ok(Self::sacrifice_self());
        }
        if effect.0.is_tap_source_cost() {
            return Ok(Self::tap());
        }
        if effect.0.is_untap_source_cost() {
            return Ok(Self::untap());
        }
        CostEffect::from_validated_effect(effect).map(Self::new)
    }

    /// Create a sacrifice self cost.
    pub fn sacrifice_self() -> Self {
        Self::validated_effect(crate::effect::Effect::sacrifice_source())
    }

    /// Create a sacrifice another permanent cost.
    pub fn sacrifice(filter: ObjectFilter) -> Self {
        Self::validated_effect(crate::effect::Effect::sacrifice(filter, 1))
    }

    /// Create a discard cards cost.
    pub fn discard(count: u32, card_type: Option<CardType>) -> Self {
        Self::discard_types(count, card_type.into_iter().collect())
    }

    /// Create a discard cards cost with one-or-more allowed card types.
    pub fn discard_types(count: u32, card_types: Vec<CardType>) -> Self {
        let card_filter = if card_types.is_empty() {
            None
        } else {
            Some(crate::filter::ObjectFilter {
                zone: Some(crate::zone::Zone::Hand),
                card_types,
                ..Default::default()
            })
        };
        Self::validated_effect(crate::effect::Effect::discard_player_filtered(
            count as i32,
            crate::target::PlayerFilter::You,
            false,
            card_filter,
        ))
    }

    /// Create a discard hand cost.
    pub fn discard_hand() -> Self {
        Self::validated_effect(crate::effect::Effect::discard_hand())
    }

    /// Create a discard-this-card cost.
    pub fn discard_source() -> Self {
        Self::validated_effect(crate::effect::Effect::discard_source_as_cost())
    }

    /// Create an exile self cost.
    pub fn exile_self() -> Self {
        Self::validated_effect(crate::effect::Effect::exile_source_as_cost())
    }

    /// Create an exile from graveyard cost.
    pub fn exile_from_graveyard(count: u32, card_type: Option<CardType>) -> Self {
        Self::validated_effect(crate::effect::Effect::exile_from_graveyard_as_cost(
            count, card_type,
        ))
    }

    /// Create an exile from hand cost.
    pub fn exile_from_hand(count: u32, color_filter: Option<ColorSet>) -> Self {
        Self::validated_effect(crate::effect::Effect::exile_from_hand_as_cost(
            count,
            color_filter,
        ))
    }

    /// Create a remove counters cost.
    pub fn remove_counters(counter_type: CounterType, count: u32) -> Self {
        Self::validated_effect(crate::effect::Effect::remove_counters(
            counter_type,
            count as i32,
            crate::target::ChooseSpec::Source,
        ))
    }

    /// Create an add counters cost.
    pub fn add_counters(counter_type: CounterType, count: u32) -> Self {
        Self::validated_effect(crate::effect::Effect::put_counters_on_source(
            counter_type,
            count as i32,
        ))
    }

    /// Create an energy payment cost.
    pub fn energy(amount: u32) -> Self {
        Self::validated_effect(crate::effect::Effect::new(
            crate::effects::PayEnergyEffect::new(
                amount as i32,
                crate::target::ChooseSpec::Player(crate::target::PlayerFilter::You),
            ),
        ))
    }

    /// Create a reveal from hand cost.
    pub fn reveal_from_hand(count: u32, card_type: Option<CardType>) -> Self {
        Self::validated_effect(crate::effect::Effect::reveal_from_hand(count, card_type))
    }

    /// Create a remove-any-counters-from-source cost.
    pub fn remove_any_counters_from_source(
        counter_type: Option<CounterType>,
        display_x: bool,
    ) -> Self {
        Self::validated_effect(crate::effect::Effect::remove_any_counters_from_source(
            counter_type,
            display_x,
        ))
    }

    /// Create a return self to hand cost.
    pub fn return_self_to_hand() -> Self {
        Self::validated_effect(crate::effect::Effect::new(
            crate::effects::ReturnToHandEffect::with_spec(crate::target::ChooseSpec::Source),
        ))
    }

    /// Create a return another permanent to hand cost.
    pub fn return_to_hand(filter: ObjectFilter) -> Self {
        Self::validated_effect(crate::effect::Effect::return_to_hand(filter))
    }

    /// Create a mill cost.
    pub fn mill(count: u32) -> Self {
        Self::validated_effect(crate::effect::Effect::mill(count as i32))
    }

    // ========================================================================
    // Delegate methods to inner CostPayer
    // ========================================================================

    /// Check if this cost can be paid right now.
    pub fn can_pay(
        &self,
        game: &crate::game_state::GameState,
        ctx: &CostContext,
    ) -> Result<(), crate::cost::CostPaymentError> {
        self.0.can_pay(game, ctx)
    }

    /// Check if this cost could potentially be paid.
    pub fn can_potentially_pay(
        &self,
        game: &crate::game_state::GameState,
        ctx: &CostContext,
    ) -> Result<(), crate::cost::CostPaymentError> {
        self.0.can_potentially_pay(game, ctx)
    }

    /// Pay this cost.
    pub fn pay(
        &self,
        game: &mut crate::game_state::GameState,
        ctx: &mut CostContext,
    ) -> Result<CostPaymentResult, crate::cost::CostPaymentError> {
        self.0.pay(game, ctx)
    }

    /// Get the display text for this cost.
    pub fn display(&self) -> String {
        self.0.display()
    }

    /// Check if this is a mana cost.
    pub fn is_mana_cost(&self) -> bool {
        self.0.is_mana_cost()
    }

    /// Check if this cost requires tapping the source.
    pub fn requires_tap(&self) -> bool {
        self.0.requires_tap()
    }

    /// Check if this cost requires untapping the source.
    pub fn requires_untap(&self) -> bool {
        self.0.requires_untap()
    }

    /// Check if this is a life payment cost.
    pub fn is_life_cost(&self) -> bool {
        self.0.is_life_cost()
    }

    /// Get the life amount if this is a life cost.
    pub fn life_amount(&self) -> Option<u32> {
        self.0.life_amount()
    }

    /// Check if this is a sacrifice self cost.
    pub fn is_sacrifice_self(&self) -> bool {
        self.0.is_sacrifice_self()
    }

    /// Check if this is a sacrifice (other permanent) cost.
    pub fn is_sacrifice(&self) -> bool {
        self.0.is_sacrifice()
    }

    /// Get the sacrifice filter if this is a sacrifice cost.
    pub fn sacrifice_filter(&self) -> Option<&ObjectFilter> {
        self.0.sacrifice_filter()
    }

    /// Check if this is a discard cost.
    pub fn is_discard(&self) -> bool {
        self.0.is_discard()
    }

    /// Get the discard details if this is a discard cost.
    pub fn discard_details(&self) -> Option<(u32, Option<crate::types::CardType>)> {
        self.0.discard_details()
    }

    /// Check if this is an exile from hand cost.
    pub fn is_exile_from_hand(&self) -> bool {
        self.0.is_exile_from_hand()
    }

    /// Get the exile from hand details if applicable.
    pub fn exile_from_hand_details(&self) -> Option<(u32, Option<crate::color::ColorSet>)> {
        self.0.exile_from_hand_details()
    }

    /// Check if this is a remove counters cost.
    pub fn is_remove_counters(&self) -> bool {
        self.0.is_remove_counters()
    }

    /// Get the mana cost if this is a mana payment cost.
    pub fn mana_cost_ref(&self) -> Option<&crate::mana::ManaCost> {
        self.0.mana_cost()
    }

    /// Get the backing effect for effect-backed costs.
    pub fn effect_ref(&self) -> Option<&crate::effect::Effect> {
        self.0.effect_ref()
    }

    /// Check if this cost needs player interaction/choice.
    pub fn needs_player_choice(&self) -> bool {
        self.0.needs_player_choice()
    }

    /// Get the processing mode for this cost.
    /// This determines how the game loop handles cost payment.
    pub fn processing_mode(&self) -> CostProcessingMode {
        self.0.processing_mode()
    }

    pub fn downcast_ref<C: 'static>(&self) -> Option<&C> {
        (&*self.0 as &dyn std::any::Any).downcast_ref::<C>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cost_wrapper_tap() {
        let cost = Cost::tap();
        assert!(cost.requires_tap());
        assert!(!cost.is_mana_cost());
        assert_eq!(cost.display(), "{T}");
    }

    #[test]
    fn test_cost_wrapper_life() {
        let cost = Cost::life(2);
        assert!(!cost.requires_tap());
        assert!(!cost.is_mana_cost());
        assert_eq!(cost.display(), "Pay 2 life");
    }

    #[test]
    fn test_cost_wrapper_untap_is_effect_backed() {
        let cost = Cost::untap();
        assert!(cost.0.effect_ref().is_some());
        assert!(cost.requires_untap());
        assert_eq!(cost.display(), "{Q}");
    }

    #[test]
    fn test_cost_wrapper_clone() {
        let cost = Cost::tap();
        let cloned = cost.clone();
        assert_eq!(cloned.display(), "{T}");
    }

    #[test]
    fn test_total_cost_iteration() {
        use crate::cost::TotalCost;
        let total = TotalCost::from_costs(vec![Cost::tap(), Cost::life(2)]);
        let costs = total.costs();
        assert_eq!(costs.len(), 2);
        assert!(costs[0].requires_tap());
        assert_eq!(costs[1].display(), "Pay 2 life");
    }

    #[test]
    fn test_discard_cost_constructor_is_effect_backed() {
        let cost = Cost::discard(2, Some(crate::types::CardType::Creature));
        assert!(cost.0.effect_ref().is_some());
        match cost.processing_mode() {
            CostProcessingMode::DiscardCards { count, card_types } => {
                assert_eq!(count, 2);
                assert_eq!(card_types, vec![crate::types::CardType::Creature]);
            }
            other => panic!("expected discard processing mode, got {other:?}"),
        }
    }

    #[test]
    fn test_sacrifice_cost_constructor_is_effect_backed() {
        let cost = Cost::sacrifice(crate::filter::ObjectFilter::creature().you_control());
        assert!(cost.0.effect_ref().is_some());
        match cost.processing_mode() {
            CostProcessingMode::SacrificeTarget { .. } => {}
            other => panic!("expected sacrifice processing mode, got {other:?}"),
        }
    }

    #[test]
    fn test_discard_source_cost_constructor_uses_generic_discard_effect() {
        let cost = Cost::discard_source();
        let effect = cost
            .0
            .effect_ref()
            .expect("effect-backed discard source cost");
        let discard = effect
            .downcast_ref::<crate::effects::DiscardEffect>()
            .expect("generic discard effect");
        assert!(discard
            .card_filter
            .as_ref()
            .is_some_and(|filter| filter.source && filter.zone == Some(crate::zone::Zone::Hand)));
        assert!(matches!(
            cost.processing_mode(),
            CostProcessingMode::Immediate
        ));
    }

    #[test]
    fn test_exile_cost_constructors_use_generic_exile_effect() {
        let hand_cost = Cost::exile_from_hand(
            1,
            Some(crate::color::ColorSet::from(crate::color::Color::Blue)),
        );
        let hand_effect = hand_cost
            .0
            .effect_ref()
            .expect("effect-backed exile-from-hand cost");
        let hand_exile = hand_effect
            .downcast_ref::<crate::effects::ExileEffect>()
            .expect("generic exile effect");
        assert!(matches!(
            hand_cost.processing_mode(),
            CostProcessingMode::ExileFromHand { count: 1, .. }
        ));
        assert!(matches!(
            hand_exile.spec.base(),
            crate::target::ChooseSpec::Object(filter)
                if filter.zone == Some(crate::zone::Zone::Hand)
        ));

        let graveyard_cost = Cost::exile_from_graveyard(2, Some(crate::types::CardType::Instant));
        let graveyard_effect = graveyard_cost
            .0
            .effect_ref()
            .expect("effect-backed exile-from-graveyard cost");
        let graveyard_exile = graveyard_effect
            .downcast_ref::<crate::effects::ExileEffect>()
            .expect("generic exile effect");
        assert!(matches!(
            graveyard_cost.processing_mode(),
            CostProcessingMode::ExileFromGraveyard { count: 2, .. }
        ));
        assert!(matches!(
            graveyard_exile.spec.base(),
            crate::target::ChooseSpec::Object(filter)
                if filter.zone == Some(crate::zone::Zone::Graveyard)
        ));
    }

    #[test]
    fn test_remove_any_counters_among_effect_ref_survives_trait_object() {
        let cost = Cost::effect(crate::effects::RemoveAnyCountersAmongEffect::new(
            3,
            crate::filter::ObjectFilter::creature().you_control(),
        ));
        assert!(
            cost.effect_ref().is_some_and(|effect| effect
                .downcast_ref::<crate::effects::RemoveAnyCountersAmongEffect>()
                .is_some()),
            "effect-backed remove-counters-among ref should survive Cost trait-object wrapping"
        );
    }
}
