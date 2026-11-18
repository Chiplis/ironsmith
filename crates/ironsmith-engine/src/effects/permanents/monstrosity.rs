//! Monstrosity effect implementation.

use crate::effects::{CompletedEffectOutputs, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
pub use ironsmith_core::MonstrosityEffect;

/// Effect that makes a creature monstrous.
///
/// Monstrosity N is an activated ability that, when resolved:
/// 1. Checks if the creature is already monstrous (if so, does nothing)
/// 2. Puts N +1/+1 counters on the creature
/// 3. Marks the creature as monstrous
///
/// This enables "When this creature becomes monstrous" triggered abilities.
///
/// # Fields
///
/// * `n` - The number of +1/+1 counters to put on the creature
///
/// # Example
///
/// ```ignore
/// // Monstrosity 3
/// let effect = MonstrosityEffect::new(3);
///
/// // Monstrosity X (where X was chosen when activating)
/// let effect = MonstrosityEffect::new(Value::X);
/// ```
impl EffectExecutor for MonstrosityEffect {
    fn canonical_program(&self) -> Option<crate::effect::Effect> {
        Some(crate::effect::Effect::new(
            super::designation::DesignationProgram {
                designation: super::designation::PermanentDesignation::Monstrous,
                amount: self.n.clone(),
                missing_source_is_invalid: true,
            },
        ))
    }

    #[crate::effects::project_effect_outcome]
    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        self.canonical_program()
            .expect("designation program")
            .0
            .execute_child_with_outputs(game, ctx)
    }
}
