//! Counter spell effect implementation.

use crate::ability::AbilityKind;
use crate::effect::{EffectOutcome, EffectResult};
use crate::effects::EffectExecutor;
use crate::effects::helpers::find_target_object;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;
use crate::zone::Zone;

/// Effect that counters a target spell on the stack.
///
/// This removes the spell from the stack and puts it into its owner's graveyard.
/// Abilities that are countered simply disappear.
///
/// # Fields
///
/// * `target` - Which spell to counter (resolved from ctx.targets)
///
/// # Example
///
/// ```ignore
/// // Counter target spell
/// let effect = CounterEffect::new(ChooseSpec::spell());
///
/// // Counter target creature spell
/// let effect = CounterEffect::new(ChooseSpec::creature_spell());
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct CounterEffect {
    /// The targeting specification (for UI/validation purposes).
    pub target: ChooseSpec,
}

impl CounterEffect {
    /// Create a new counter effect.
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }

    /// Create an effect that counters any spell.
    pub fn any_spell() -> Self {
        Self::new(ChooseSpec::spell())
    }
}

impl EffectExecutor for CounterEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // Counter removes the spell/ability from the stack
        // Find the target in our resolved targets
        let target_id = find_target_object(&ctx.targets)?;

        // Check if the spell can't be countered
        if let Some(obj) = game.object(target_id) {
            let cant_be_countered = obj.abilities.iter().any(|ability| {
                if let AbilityKind::Static(s) = &ability.kind {
                    s.cant_be_countered()
                } else {
                    false
                }
            });
            if cant_be_countered {
                // Spell can't be countered - effect does nothing
                return Ok(EffectOutcome::from_result(EffectResult::Protected));
            }
        }

        // Find the stack entry for this object
        if let Some(idx) = game.stack.iter().position(|e| e.object_id == target_id) {
            let entry = game.stack.remove(idx);
            // Move countered spell to graveyard (abilities just disappear)
            if !entry.is_ability {
                game.move_object(entry.object_id, Zone::Graveyard);
            }
            Ok(EffectOutcome::resolved())
        } else {
            // Target is no longer on the stack
            Ok(EffectOutcome::from_result(EffectResult::TargetInvalid))
        }
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "spell to counter"
    }
}
