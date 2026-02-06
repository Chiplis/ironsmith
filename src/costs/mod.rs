//! Modular cost system for MTG.
//!
//! This module provides a trait-based architecture for cost payment.
//! Each cost type implements the `CostPayer` trait, allowing for:
//! - Co-located tests with each cost implementation
//! - Self-contained cost payment logic
//! - Easy addition of new costs without modifying central dispatcher
//! - Support for "potential payability" checks for better UI
//!
//! # Module Structure
//!
//! ```text
//! costs/
//!   mod.rs              - This file, module organization and Cost wrapper
//!   payer_trait.rs      - CostPayer trait definition and CostContext
//!   tap.rs              - TapCost implementation
//!   untap.rs            - UntapCost implementation
//!   life.rs             - LifeCost implementation
//!   mana.rs             - ManaPaymentCost implementation
//!   sacrifice.rs        - SacrificeSelfCost, SacrificeCost implementations
//!   discard.rs          - DiscardCost, DiscardHandCost implementations
//!   exile.rs            - ExileSelfCost, ExileFromGraveyardCost, ExileFromHandCost
//!   counters.rs         - RemoveCountersCost, AddCountersCost
//!   energy.rs           - EnergyCost
//!   reveal.rs           - RevealFromHandCost
//!   return_to_hand.rs   - ReturnSelfToHandCost, ReturnToHandCost
//!   mill.rs             - MillCost
//! ```
//!
//! # Usage
//!
//! Costs can be checked and paid through the `CostPayer` trait:
//!
//! ```ignore
//! use ironsmith::costs::{CostPayer, TapCost, CostContext};
//!
//! let cost = TapCost::new();
//! let ctx = CostContext::new(permanent_id, player_id);
//!
//! // Check if cost can be paid
//! if cost.can_pay(&game, &ctx).is_ok() {
//!     cost.pay(&mut game, &mut ctx)?;
//! }
//! ```

mod counters;
mod discard;
mod effect;
mod energy;
mod exile;
mod life;
mod mana;
mod mill;
mod payer_trait;
mod processing_mode;
mod return_to_hand;
mod reveal;
mod sacrifice;
mod tap;
mod untap;

// Re-export the trait and context
pub use payer_trait::{
    CostCheckContext, can_pay_with_check_context, can_potentially_pay_with_check_context,
};
pub use payer_trait::{CostContext, CostPayer, CostPaymentResult};
pub use processing_mode::CostProcessingMode;

// Re-export all cost implementations
pub use counters::{AddCountersCost, RemoveAnyCountersAmongCost, RemoveCountersCost};
pub use discard::{DiscardCost, DiscardHandCost};
pub use effect::EffectCost;
pub use energy::EnergyCost;
pub use exile::{ExileFromGraveyardCost, ExileFromHandCost, ExileSelfCost};
pub use life::LifeCost;
pub use mana::ManaPaymentCost;
pub use mill::MillCost;
pub use return_to_hand::{ReturnSelfToHandCost, ReturnToHandCost};
pub use reveal::RevealFromHandCost;
pub use sacrifice::{SacrificeCost, SacrificeSelfCost};
pub use tap::TapCost;
pub use untap::UntapCost;

use crate::color::ColorSet;
use crate::cost::PermanentFilter;
use crate::mana::ManaCost;
use crate::object::CounterType;
use crate::types::CardType;

/// A wrapper around a boxed CostPayer trait object.
///
/// This provides a convenient way to work with costs as values while
/// maintaining the flexibility of trait objects.
#[derive(Debug)]
pub struct Cost(pub Box<dyn CostPayer>);

impl Clone for Cost {
    fn clone(&self) -> Self {
        Cost(self.0.clone_box())
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
        Cost(Box::new(payer))
    }

    // ========================================================================
    // Convenience constructors
    // ========================================================================

    /// Create a tap cost ({T}).
    pub fn tap() -> Self {
        Self::new(TapCost::new())
    }

    /// Create an untap cost ({Q}).
    pub fn untap() -> Self {
        Self::new(UntapCost::new())
    }

    /// Create a life payment cost.
    pub fn life(amount: u32) -> Self {
        Self::new(LifeCost::new(amount))
    }

    /// Create a mana cost.
    pub fn mana(cost: ManaCost) -> Self {
        Self::new(ManaPaymentCost::new(cost))
    }

    /// Create a cost backed by an effect executor.
    pub fn effect(effect: crate::effect::Effect) -> Self {
        Self::new(EffectCost::new(effect))
    }

    /// Create a sacrifice self cost.
    pub fn sacrifice_self() -> Self {
        Self::new(SacrificeSelfCost::new())
    }

    /// Create a sacrifice another permanent cost.
    pub fn sacrifice(filter: PermanentFilter) -> Self {
        Self::new(SacrificeCost::new(filter))
    }

    /// Create a discard cards cost.
    pub fn discard(count: u32, card_type: Option<CardType>) -> Self {
        Self::new(DiscardCost::new(count, card_type))
    }

    /// Create a discard hand cost.
    pub fn discard_hand() -> Self {
        Self::new(DiscardHandCost::new())
    }

    /// Create an exile self cost.
    pub fn exile_self() -> Self {
        Self::new(ExileSelfCost::new())
    }

    /// Create an exile from graveyard cost.
    pub fn exile_from_graveyard(count: u32, card_type: Option<CardType>) -> Self {
        Self::new(ExileFromGraveyardCost::new(count, card_type))
    }

    /// Create an exile from hand cost.
    pub fn exile_from_hand(count: u32, color_filter: Option<ColorSet>) -> Self {
        Self::new(ExileFromHandCost::new(count, color_filter))
    }

    /// Create a remove counters cost.
    pub fn remove_counters(counter_type: CounterType, count: u32) -> Self {
        Self::new(RemoveCountersCost::new(counter_type, count))
    }

    /// Create an add counters cost.
    pub fn add_counters(counter_type: CounterType, count: u32) -> Self {
        Self::new(AddCountersCost::new(counter_type, count))
    }

    /// Create an energy payment cost.
    pub fn energy(amount: u32) -> Self {
        Self::new(EnergyCost::new(amount))
    }

    /// Create a reveal from hand cost.
    pub fn reveal_from_hand(count: u32, card_type: Option<CardType>) -> Self {
        Self::new(RevealFromHandCost::new(count, card_type))
    }

    /// Create a return self to hand cost.
    pub fn return_self_to_hand() -> Self {
        Self::new(ReturnSelfToHandCost::new())
    }

    /// Create a return another permanent to hand cost.
    pub fn return_to_hand(filter: PermanentFilter) -> Self {
        Self::new(ReturnToHandCost::new(filter))
    }

    /// Create a mill cost.
    pub fn mill(count: u32) -> Self {
        Self::new(MillCost::new(count))
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
    pub fn sacrifice_filter(&self) -> Option<&crate::cost::PermanentFilter> {
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

    /// Check if this cost needs player interaction/choice.
    pub fn needs_player_choice(&self) -> bool {
        self.0.needs_player_choice()
    }

    /// Get the processing mode for this cost.
    /// This determines how the game loop handles cost payment.
    pub fn processing_mode(&self) -> CostProcessingMode {
        self.0.processing_mode()
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
}
