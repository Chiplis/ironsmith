//! Conditional effect implementation.

use crate::effect::{Condition, Effect, EffectOutcome};
use crate::effects::{EffectExecutor, ModalSpec};
use crate::executor::{ExecutionContext, ExecutionError, execute_effect};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};

/// Effect that branches based on game state conditions.
///
/// Unlike `If` which checks the result of a prior effect, `Conditional`
/// evaluates game state conditions like "if you control a creature" or
/// "if your life total is 10 or less".
///
/// # Fields
///
/// * `condition` - The game state condition to check
/// * `if_true` - Effects to execute if condition is true
/// * `if_false` - Effects to execute if condition is false
///
/// # Example
///
/// ```ignore
/// // If you control a creature, draw a card. Otherwise, gain 2 life.
/// let effect = ConditionalEffect::new(
///     Condition::YouControl(ObjectFilter::creature()),
///     vec![Effect::draw(1)],
///     vec![Effect::gain_life(2)],
/// );
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ConditionalEffect {
    /// The game state condition to check.
    pub condition: Condition,
    /// Effects to execute if condition is true.
    pub if_true: Vec<Effect>,
    /// Effects to execute if condition is false.
    pub if_false: Vec<Effect>,
}

impl ConditionalEffect {
    /// Create a new Conditional effect.
    pub fn new(condition: Condition, if_true: Vec<Effect>, if_false: Vec<Effect>) -> Self {
        Self {
            condition,
            if_true,
            if_false,
        }
    }

    /// Create a conditional with no else clause.
    pub fn if_only(condition: Condition, if_true: Vec<Effect>) -> Self {
        Self::new(condition, if_true, vec![])
    }
}

impl EffectExecutor for ConditionalEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let result = evaluate_condition(game, &self.condition, ctx)?;

        let effects_to_execute = if result {
            &self.if_true
        } else {
            &self.if_false
        };

        let mut outcomes = Vec::new();
        for effect in effects_to_execute {
            outcomes.push(execute_effect(game, effect, ctx)?);
        }

        Ok(EffectOutcome::aggregate(outcomes))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn get_modal_spec_with_context(
        &self,
        game: &GameState,
        controller: PlayerId,
        source: ObjectId,
    ) -> Option<ModalSpec> {
        // Evaluate the condition at cast time to determine which branch to use
        let condition_result = evaluate_condition_simple(game, &self.condition, controller, source);

        // Search the appropriate branch for modal specs
        let effects_to_search = if condition_result {
            &self.if_true
        } else {
            &self.if_false
        };

        // Recursively search through the effects in this branch
        for effect in effects_to_search {
            // First try the context-aware version
            if let Some(spec) = effect
                .0
                .get_modal_spec_with_context(game, controller, source)
            {
                return Some(spec);
            }
            // Fall back to the simple version
            if let Some(spec) = effect.0.get_modal_spec() {
                return Some(spec);
            }
        }

        None
    }
}

fn evaluate_condition_simple(
    game: &GameState,
    condition: &Condition,
    controller: PlayerId,
    source: ObjectId,
) -> bool {
    crate::condition_eval::evaluate_condition_cast_time(game, condition, controller, source)
}

fn evaluate_condition(
    game: &GameState,
    condition: &Condition,
    ctx: &ExecutionContext,
) -> Result<bool, ExecutionError> {
    crate::condition_eval::evaluate_condition_resolution(game, condition, ctx)
}
