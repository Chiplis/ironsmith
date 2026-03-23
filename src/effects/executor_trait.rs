//! Effect executor trait for the modular effect system.
//!
//! This module defines the `EffectExecutor` trait that all effect implementations
//! must implement. Each effect type (damage, life, mana, etc.) implements this trait
//! with its own execution logic.

use std::any::Any;

use crate::costs::PaymentReason;
use crate::effect::{EffectOutcome, Value};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::mana::ManaSymbol;
use crate::target::ChooseSpec;

/// Specification for a modal effect, used during spell casting per MTG rule 601.2b.
///
/// This contains the information needed to present mode choices to the player
/// during the casting process (before targets are chosen).
#[derive(Debug, Clone)]
pub struct ModalSpec {
    /// Descriptions of each available mode.
    pub mode_descriptions: Vec<String>,
    /// Maximum number of modes that can be chosen.
    pub max_modes: Value,
    /// Minimum number of modes that must be chosen.
    pub min_modes: Value,
    /// Whether the same mode can be chosen more than once.
    pub allow_repeated_modes: bool,
}

/// Trait for executing effects.
///
/// All modular effects implement this trait. Each effect is responsible for:
/// - Resolving any dynamic values (X, counts, etc.)
/// - Validating targets (if applicable)
/// - Mutating game state appropriately
/// - Returning an appropriate `EffectOutcome` (result + events)
///
/// # Example
///
/// ```ignore
/// use ironsmith::effects::EffectExecutor;
///
/// impl EffectExecutor for MyEffect {
///     fn execute(
///         &self,
///         game: &mut GameState,
///         ctx: &mut ExecutionContext,
///     ) -> Result<EffectOutcome, ExecutionError> {
///         // Implementation here
///         Ok(EffectOutcome::resolved())
///     }
/// }
/// ```
pub trait EffectExecutorClone {
    /// Clone this effect into a boxed trait object.
    fn clone_boxed(&self) -> Box<dyn EffectExecutor>;
}

impl<T> EffectExecutorClone for T
where
    T: EffectExecutor + Clone + 'static,
{
    fn clone_boxed(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}

pub trait EffectExecutor:
    std::fmt::Debug + Any + Send + Sync + EffectExecutorClone + 'static
{
    /// Execute this effect, mutating the game state and returning the outcome.
    ///
    /// # Arguments
    ///
    /// * `game` - The mutable game state to modify
    /// * `ctx` - The execution context containing source, controller, targets, etc.
    ///
    /// # Returns
    ///
    /// * `Ok(EffectOutcome)` - The outcome (result + events) of executing the effect
    /// * `Err(ExecutionError)` - If the effect could not be executed
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError>;

    /// Clone this effect into a boxed trait object.
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        EffectExecutorClone::clone_boxed(self)
    }

    /// Get the target specification for this effect, if it has one.
    ///
    /// Used for target selection during spell/ability resolution.
    /// Returns `None` for effects that don't require targeting.
    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        None
    }

    /// Get a human-readable description of what this effect targets.
    ///
    /// Used for UI/logging during target selection.
    fn target_description(&self) -> &'static str {
        "target"
    }

    /// Get the target count for this effect, if it has one.
    ///
    /// Used for determining min/max targets during target selection.
    /// Returns `None` to use default (exactly 1 target).
    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        None
    }

    /// Get the modal specification for this effect, if it's a modal effect.
    ///
    /// Per MTG rule 601.2b, modes must be chosen during spell casting (before targets).
    /// This method returns the information needed to present mode choices to the player.
    /// Returns `None` for non-modal effects.
    fn get_modal_spec(&self) -> Option<ModalSpec> {
        None
    }

    /// Get the modal specification with game context, allowing conditional evaluation.
    ///
    /// For compositional effects like ConditionalEffect, this method allows evaluating
    /// the condition at cast time to determine which branch's modal spec to use.
    /// For example, Akroma's Will wraps ChooseModeEffect in a ConditionalEffect that
    /// checks if you control a commander - this method evaluates that condition and
    /// returns the appropriate modal spec.
    ///
    /// Default implementation delegates to `get_modal_spec()`.
    fn get_modal_spec_with_context(
        &self,
        _game: &GameState,
        _controller: PlayerId,
        _source: ObjectId,
    ) -> Option<ModalSpec> {
        self.get_modal_spec()
    }

    /// Returns this effect as a cost-capable trait object when it can legally
    /// participate in cost payment.
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        None
    }

    /// If this is a "pay life" effect, returns the amount.
    ///
    /// Used for checking if alternative cost effects can be paid.
    fn pay_life_amount(&self) -> Option<u32> {
        None
    }

    /// If this is an "exile from hand as cost" effect, returns (count, color_filter).
    ///
    /// Used for checking if alternative cost effects can be paid.
    fn exile_from_hand_cost_info(&self) -> Option<(u32, Option<crate::color::ColorSet>)> {
        None
    }

    /// Check if this effect can be executed as a cost.
    ///
    /// This is used for non-mana cost components in mana abilities and alternative casting costs.
    /// Returns Ok(()) if the cost can be paid, or Err with a reason if not.
    ///
    /// Default implementation returns Ok(()) (effect can always be executed).
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
    ) -> Result<(), CostValidationError> {
        self.can_execute_as_cost_with_reason(game, source, controller, PaymentReason::Other)
    }

    /// Check if this effect can be executed as a cost for a specific payment reason.
    fn can_execute_as_cost_with_reason(
        &self,
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
        reason: PaymentReason,
    ) -> Result<(), CostValidationError> {
        if let Some(cost_effect) = self.as_cost_executable() {
            return CostExecutableEffect::can_execute_as_cost_with_reason(
                cost_effect,
                game,
                source,
                controller,
                reason,
            );
        }
        Ok(())
    }

    /// Returns true if this is a "tap source" cost effect.
    ///
    /// Used for checking summoning sickness restrictions.
    fn is_tap_source_cost(&self) -> bool {
        false
    }

    /// Returns true if this is an "untap source" cost effect.
    fn is_untap_source_cost(&self) -> bool {
        false
    }

    /// Returns true if this is a "sacrifice source" cost effect.
    fn is_sacrifice_source_cost(&self) -> bool {
        false
    }

    /// Returns a human-readable description of this effect when used as a cost.
    ///
    /// Used for displaying alternative casting costs like "Pay 1 life, exile a blue card".
    /// Returns None if no description is available, in which case a generic display is used.
    fn cost_description(&self) -> Option<String> {
        None
    }

    /// Returns mana symbols this effect can produce when used as a mana ability payload.
    ///
    /// This is a best-effort capability hook used by inference effects such as
    /// "add one mana of any type that a land could produce". Implementations
    /// should return all possible symbols for the given source/controller context.
    fn producible_mana_symbols(
        &self,
        _game: &GameState,
        _source: ObjectId,
        _controller: PlayerId,
    ) -> Option<Vec<ManaSymbol>> {
        None
    }

    /// Downcast support for effect introspection.
    fn as_any(&self) -> &dyn Any
    where
        Self: Sized,
    {
        self
    }
}

/// Error returned when a cost effect cannot be paid.
#[derive(Debug, Clone, PartialEq)]
pub enum CostValidationError {
    /// Source is already tapped
    AlreadyTapped,
    /// Source is already untapped
    AlreadyUntapped,
    /// Creature has summoning sickness (can't tap)
    SummoningSickness,
    /// Not enough life to pay
    NotEnoughLife,
    /// Not enough cards to exile
    NotEnoughCards,
    /// Cannot sacrifice required permanent
    CannotSacrifice,
    /// Generic error with message
    Other(String),
}

/// Additional behavior required for effects that can be used as costs.
pub trait CostExecutableEffect: EffectExecutor {
    /// Check whether this effect can be paid in a cost context.
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
    ) -> Result<(), CostValidationError>;

    /// Check whether this effect can be paid in a cost context for a specific reason.
    fn can_execute_as_cost_with_reason(
        &self,
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
        _reason: PaymentReason,
    ) -> Result<(), CostValidationError> {
        CostExecutableEffect::can_execute_as_cost(self, game, source, controller)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A simple test effect that always resolves.
    #[derive(Debug, Clone)]
    struct TestEffect;

    impl EffectExecutor for TestEffect {
        fn execute(
            &self,
            _game: &mut GameState,
            _ctx: &mut ExecutionContext,
        ) -> Result<EffectOutcome, ExecutionError> {
            Ok(EffectOutcome::resolved())
        }
    }

    #[test]
    fn test_effect_executor_trait_is_object_safe() {
        // This test verifies that EffectExecutor can be used as a trait object
        let effect: Box<dyn EffectExecutor> = Box::new(TestEffect);
        assert!(format!("{:?}", effect).contains("TestEffect"));
    }
}
