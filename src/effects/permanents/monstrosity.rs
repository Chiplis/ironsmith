//! Monstrosity effect implementation.

use crate::effect::{Effect, EffectOutcome, EffectResult, Value};
use crate::effects::helpers::resolve_value;
use crate::effects::{EffectExecutor, PutCountersEffect};
use crate::executor::{ExecutionContext, ExecutionError, ResolvedTarget, execute_effect};
use crate::game_state::GameState;
use crate::object::CounterType;
use crate::target::ChooseSpec;

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
#[derive(Debug, Clone, PartialEq)]
pub struct MonstrosityEffect {
    /// The number of +1/+1 counters to add.
    pub n: Value,
}

impl MonstrosityEffect {
    /// Create a new monstrosity effect.
    pub fn new(n: impl Into<Value>) -> Self {
        Self { n: n.into() }
    }
}

impl EffectExecutor for MonstrosityEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let n_value = resolve_value(game, &self.n, ctx)?.max(0) as u32;

        // Monstrosity targets the source (the creature with the ability)
        let source_id = ctx.source;

        // Check if already monstrous
        if game.object(source_id).is_none() {
            return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid));
        }
        if game.is_monstrous(source_id) {
            // Already monstrous - do nothing
            return Ok(EffectOutcome::count(0));
        }

        // Put N +1/+1 counters on it and mark as monstrous
        if n_value > 0 {
            let counters_outcome =
                ctx.with_temp_targets(vec![ResolvedTarget::Object(source_id)], |ctx| {
                    let counters_effect = PutCountersEffect::new(
                        CounterType::PlusOnePlusOne,
                        n_value,
                        ChooseSpec::AnyTarget,
                    );
                    execute_effect(game, &Effect::new(counters_effect), ctx)
                })?;

            if let EffectResult::Count(n) = counters_outcome.result
                && n > 0
            {
                game.continuous_effects.record_counter_change(source_id);
            }
        }
        game.set_monstrous(source_id);

        // Return a special result that indicates monstrosity happened
        // The game loop will need to generate the BecameMonstrous event
        Ok(EffectOutcome::from_result(
            EffectResult::MonstrosityApplied {
                creature: source_id,
                n: n_value,
            },
        ))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}
