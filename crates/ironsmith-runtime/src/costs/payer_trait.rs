//! CostPayer trait for the modular cost system.
//!
//! This module defines the `CostPayer` trait used by `Cost` and `CostEffect`.

use std::any::Any;
use std::collections::HashMap;

use crate::cost::CostPaymentError;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::provenance::ProvNodeId;
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;

/// Why a cost is being paid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PaymentReason {
    /// Casting a spell.
    CastSpell,
    /// Activating a non-mana ability.
    ActivateAbility,
    /// Activating a mana ability.
    ActivateManaAbility,
    /// Turning a face-down permanent face up.
    TurnFaceUp,
    /// Paying a cost during effect or triggered-ability resolution.
    Effect,
    /// Paying another special-action or generic engine cost.
    #[default]
    Other,
}

impl PaymentReason {
    pub fn is_cast_or_ability_payment(self) -> bool {
        matches!(
            self,
            Self::CastSpell | Self::ActivateAbility | Self::ActivateManaAbility
        )
    }
}

/// Result of paying a cost.
#[derive(Debug, Clone, PartialEq)]
pub enum CostPaymentResult {
    /// Cost was paid successfully.
    Paid,
    /// Cost requires a choice from the player (e.g., which creature to sacrifice).
    /// Contains a description of the choice needed.
    NeedsChoice(String),
}

/// Context for cost payment operations.
///
/// Similar to ExecutionContext for effects, this provides the necessary
/// context for checking and paying costs.
pub struct CostContext<'dm> {
    /// The source object (permanent or spell whose cost is being paid).
    pub source: ObjectId,
    /// The player paying the cost.
    pub payer: PlayerId,
    /// X value for variable costs.
    pub x_value: Option<u32>,
    /// Why this cost is being paid.
    pub reason: PaymentReason,
    /// Decision maker for player choices during cost payment.
    pub decision_maker: &'dm mut dyn crate::decision::DecisionMaker,
    /// Pre-chosen cards for costs that require card selection (e.g., ExileFromHand).
    /// When present, costs should use these instead of prompting for choice.
    pub pre_chosen_cards: Vec<ObjectId>,
    /// Tagged objects that persist across cost effects.
    ///
    /// This allows effects like "choose a creature, then sacrifice it" to work
    /// when both are cost effects. The first effect tags the chosen creature,
    /// and the second effect can reference it via the tag.
    pub tagged_objects: HashMap<TagKey, Vec<ObjectSnapshot>>,
    /// Provenance parent node for events emitted while paying this cost.
    pub provenance: ProvNodeId,
}

impl std::fmt::Debug for CostContext<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CostContext")
            .field("source", &self.source)
            .field("payer", &self.payer)
            .field("x_value", &self.x_value)
            .field("reason", &self.reason)
            .field("pre_chosen_cards", &self.pre_chosen_cards)
            .field(
                "tagged_objects",
                &self.tagged_objects.keys().collect::<Vec<_>>(),
            )
            .field("provenance", &self.provenance)
            .finish()
    }
}

impl<'dm> CostContext<'dm> {
    /// Create a new cost context with a decision maker.
    pub fn new(
        source: ObjectId,
        payer: PlayerId,
        decision_maker: &'dm mut dyn crate::decision::DecisionMaker,
    ) -> Self {
        Self {
            source,
            payer,
            x_value: None,
            reason: PaymentReason::Other,
            decision_maker,
            pre_chosen_cards: Vec::new(),
            tagged_objects: HashMap::new(),
            provenance: ProvNodeId::default(),
        }
    }

    /// Set the X value.
    pub fn with_x(mut self, x: u32) -> Self {
        self.x_value = Some(x);
        self
    }

    /// Set the payment reason.
    pub fn with_reason(mut self, reason: PaymentReason) -> Self {
        self.reason = reason;
        self
    }

    /// Set pre-chosen cards for costs that require card selection.
    pub fn with_pre_chosen_cards(mut self, cards: Vec<ObjectId>) -> Self {
        self.pre_chosen_cards = cards;
        self
    }

    /// Set provenance parent for emitted events.
    pub fn with_provenance(mut self, provenance: ProvNodeId) -> Self {
        self.provenance = provenance;
        self
    }
}

/// A context for checking costs without a decision maker.
///
/// This is used by query functions (like `can_pay_cost`, `compute_legal_actions`)
/// that need to check if costs CAN be paid but don't actually pay them.
/// Since `can_pay()` implementations never use the decision_maker field,
/// this is safe for all read-only cost checking operations.
pub struct CostCheckContext {
    /// The source object (permanent or spell whose cost is being checked).
    pub source: ObjectId,
    /// The player whose cost payment is being checked.
    pub payer: PlayerId,
    /// X value for variable costs.
    pub x_value: Option<u32>,
    /// Why this cost would be paid.
    pub reason: PaymentReason,
    /// Pre-chosen cards (usually empty for checking).
    pub pre_chosen_cards: Vec<ObjectId>,
}

impl CostCheckContext {
    /// Create a new cost check context.
    pub fn new(source: ObjectId, payer: PlayerId) -> Self {
        Self {
            source,
            payer,
            x_value: None,
            reason: PaymentReason::Other,
            pre_chosen_cards: Vec::new(),
        }
    }

    /// Set the X value.
    pub fn with_x(mut self, x: u32) -> Self {
        self.x_value = Some(x);
        self
    }

    /// Set the payment reason.
    pub fn with_reason(mut self, reason: PaymentReason) -> Self {
        self.reason = reason;
        self
    }

    /// Create a temporary CostContext for use with can_pay checking.
    ///
    /// This is safe because can_pay implementations never use decision_maker.
    /// The returned context uses a dummy decision maker that would panic if
    /// any actual decisions were attempted (which should never happen in can_pay).
    pub fn as_cost_context<'a>(
        &self,
        dm: &'a mut dyn crate::decision::DecisionMaker,
    ) -> CostContext<'a> {
        CostContext {
            source: self.source,
            payer: self.payer,
            x_value: self.x_value,
            reason: self.reason,
            decision_maker: dm,
            pre_chosen_cards: self.pre_chosen_cards.clone(),
            tagged_objects: HashMap::new(),
            provenance: ProvNodeId::default(),
        }
    }
}

/// Check if a cost can be paid using a check-only context.
///
/// This is a convenience function for query operations that don't have
/// access to a decision maker. Since `can_pay` never uses the decision_maker,
/// this is safe.
pub fn can_pay_with_check_context(
    cost: &dyn CostPayer,
    game: &crate::game_state::GameState,
    ctx: &CostCheckContext,
) -> Result<(), crate::cost::CostPaymentError> {
    // Create a temporary AutoPass decision maker just for the check
    let mut auto_dm = crate::decision::CliDecisionMaker;
    let cost_ctx = ctx.as_cost_context(&mut auto_dm);
    cost.can_pay(game, &cost_ctx)
}

/// Check if a cost can potentially be paid using a check-only context.
pub fn can_potentially_pay_with_check_context(
    cost: &dyn CostPayer,
    game: &crate::game_state::GameState,
    ctx: &CostCheckContext,
) -> Result<(), crate::cost::CostPaymentError> {
    let mut auto_dm = crate::decision::CliDecisionMaker;
    let cost_ctx = ctx.as_cost_context(&mut auto_dm);
    cost.can_potentially_pay(game, &cost_ctx)
}

/// Trait for paying costs.
///
/// All modular costs implement this trait. Each cost is responsible for:
/// - Checking if it can be paid (right now)
/// - Checking if it could potentially be paid (with untapped mana sources)
/// - Actually paying the cost
/// - Providing display text
///
/// # Example
///
/// ```ignore
/// use ironsmith::costs::CostPayer;
///
/// #[derive(Debug, Clone)]
/// struct MyCost;
///
/// impl CostPayer for MyCost {
///     fn can_pay(&self, game: &GameState, ctx: &CostContext) -> Result<(), CostPaymentError> {
///         // Validate whether the cost can be paid
///         Ok(())
///     }
///
///     fn pay(&self, game: &mut GameState, ctx: &mut CostContext) -> Result<CostPaymentResult, CostPaymentError> {
///         // Mutate the game state to pay the cost
///         Ok(CostPaymentResult::Paid)
///     }
///
///     fn display(&self) -> String {
///         "My cost".to_string()
///     }
/// }
/// ```
pub trait CostPayerClone {
    /// Clone this cost into a boxed trait object.
    fn clone_boxed(&self) -> Box<dyn CostPayer>;
}

impl<T> CostPayerClone for T
where
    T: CostPayer + Clone + 'static,
{
    fn clone_boxed(&self) -> Box<dyn CostPayer> {
        Box::new(self.clone())
    }
}

pub trait CostPayer: std::fmt::Debug + Send + Sync + CostPayerClone + Any {
    /// Check if this cost can be paid RIGHT NOW.
    ///
    /// For mana costs, this checks if the mana is in the pool.
    /// For tap costs, this checks if the permanent is untapped.
    /// For sacrifice costs, this checks if valid targets exist.
    fn can_pay(&self, game: &GameState, ctx: &CostContext) -> Result<(), CostPaymentError>;

    /// Check if this cost COULD potentially be paid.
    ///
    /// For mana costs, this includes untapped mana sources.
    /// For non-mana costs, this typically equals `can_pay()`.
    ///
    /// This is used for UI to show actions that could be afforded after
    /// tapping mana sources.
    fn can_potentially_pay(
        &self,
        game: &GameState,
        ctx: &CostContext,
    ) -> Result<(), CostPaymentError> {
        // Default implementation: same as can_pay
        self.can_pay(game, ctx)
    }

    /// Actually pay the cost, mutating game state.
    ///
    /// # Errors
    ///
    /// Returns an error if the cost cannot be paid.
    fn pay(
        &self,
        game: &mut GameState,
        ctx: &mut CostContext,
    ) -> Result<CostPaymentResult, CostPaymentError>;

    /// Clone this cost into a boxed trait object.
    fn clone_box(&self) -> Box<dyn CostPayer> {
        CostPayerClone::clone_boxed(self)
    }

    /// Human-readable display text for this cost.
    ///
    /// Examples: "{T}", "Pay 2 life", "{2}{W}", "Sacrifice a creature"
    fn display(&self) -> String;

    /// Returns true if this is a mana cost.
    ///
    /// Used for separating mana costs from other costs in display.
    fn is_mana_cost(&self) -> bool {
        false
    }

    /// Returns true if this cost requires tapping the source.
    ///
    /// Used for display formatting ("Tap X" vs "X (cost description)").
    fn requires_tap(&self) -> bool {
        false
    }

    /// Returns true if this cost requires untapping the source.
    fn requires_untap(&self) -> bool {
        false
    }

    /// Returns true if this is a life payment cost.
    fn is_life_cost(&self) -> bool {
        false
    }

    /// Returns the life amount if this is a life payment cost.
    fn life_amount(&self) -> Option<u32> {
        None
    }

    /// Returns true if this is a sacrifice self cost.
    fn is_sacrifice_self(&self) -> bool {
        false
    }

    /// Returns true if this is a sacrifice (other permanent) cost.
    fn is_sacrifice(&self) -> bool {
        false
    }

    /// Returns the sacrifice filter if this is a sacrifice cost.
    fn sacrifice_filter(&self) -> Option<&crate::filter::ObjectFilter> {
        None
    }

    /// Returns true if this is a discard cost.
    fn is_discard(&self) -> bool {
        false
    }

    /// Returns the discard details (count, optional card type) if this is a discard cost.
    fn discard_details(&self) -> Option<(u32, Option<crate::types::CardType>)> {
        None
    }

    /// Returns true if this is an exile from hand cost.
    fn is_exile_from_hand(&self) -> bool {
        false
    }

    /// Returns the exile from hand details (count, color filter) if applicable.
    fn exile_from_hand_details(&self) -> Option<(u32, Option<crate::color::ColorSet>)> {
        None
    }

    /// Returns true if this is a remove counters cost.
    fn is_remove_counters(&self) -> bool {
        false
    }

    /// Returns the mana cost if this is a mana payment cost.
    fn mana_cost(&self) -> Option<&crate::mana::ManaCost> {
        None
    }

    /// Returns true if this cost requires player interaction/choice.
    ///
    /// Immediate costs (tap, untap, life, remove counters, sacrifice self) return false.
    /// Costs needing selection (mana payment, sacrifice target) return true.
    fn needs_player_choice(&self) -> bool {
        false
    }

    /// Returns how this cost should be processed during cost payment.
    ///
    /// This determines the game loop's handling:
    /// - `Immediate`: Pay directly via `pay()`
    /// - `ManaPayment`: Use mana payment UI
    /// - `SacrificeTarget`: Use target selection UI
    /// - `DiscardCards`: Use card selection UI
    /// - `ExileFromHand`: Use card selection UI
    /// - `InlineWithTriggers`: Handle inline for trigger detection
    fn processing_mode(&self) -> crate::costs::CostProcessingMode {
        // Default: immediate payment
        crate::costs::CostProcessingMode::Immediate
    }

    /// Returns the backing effect when this cost is effect-backed.
    ///
    /// Default is `None` for non-effect costs.
    fn effect_ref(&self) -> Option<&crate::effect::Effect> {
        None
    }

    /// Downcast support for staged cost handling.
    fn as_any(&self) -> &dyn Any;
}

// Implement Clone for Box<dyn CostPayer>
impl Clone for Box<dyn CostPayer> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A simple test cost that always succeeds.
    #[derive(Debug, Clone)]
    struct TestCost;

    impl CostPayer for TestCost {
        fn can_pay(&self, _game: &GameState, _ctx: &CostContext) -> Result<(), CostPaymentError> {
            Ok(())
        }

        fn pay(
            &self,
            _game: &mut GameState,
            _ctx: &mut CostContext,
        ) -> Result<CostPaymentResult, CostPaymentError> {
            Ok(CostPaymentResult::Paid)
        }

        fn display(&self) -> String {
            "Test".to_string()
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[test]
    fn test_cost_payer_trait_is_object_safe() {
        // This test verifies that CostPayer can be used as a trait object
        let cost: Box<dyn CostPayer> = Box::new(TestCost);
        assert!(format!("{:?}", cost).contains("TestCost"));
        assert_eq!(cost.display(), "Test");
    }

    #[test]
    fn test_box_dyn_cost_payer_clone() {
        let cost: Box<dyn CostPayer> = Box::new(TestCost);
        let cloned = cost.clone();
        assert!(format!("{:?}", cloned).contains("TestCost"));
    }

    #[test]
    fn test_cost_context_creation() {
        let source = ObjectId::from_raw(1);
        let payer = PlayerId::from_index(0);
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = CostContext::new(source, payer, &mut dm);

        assert_eq!(ctx.source, source);
        assert_eq!(ctx.payer, payer);
        assert!(ctx.x_value.is_none());
    }

    #[test]
    fn test_cost_context_with_x() {
        let source = ObjectId::from_raw(1);
        let payer = PlayerId::from_index(0);
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let ctx = CostContext::new(source, payer, &mut dm).with_x(5);

        assert_eq!(ctx.x_value, Some(5));
    }
}
