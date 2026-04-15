//! Flip effect implementation (for Kamigawa flip cards).

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_single_object_for_effect;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;

/// Effect that flips a flip-card permanent to its other face.
///
/// This uses `Object.other_face` to find the destination definition.
#[derive(Debug, Clone, PartialEq)]
pub struct FlipEffect {
    pub target: ChooseSpec,
}

impl FlipEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }

    pub fn source() -> Self {
        Self::new(ChooseSpec::Source)
    }
}

impl EffectExecutor for FlipEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let target_id = resolve_single_object_for_effect(game, ctx, &self.target)?;

        if game.is_flipped(target_id) {
            return Ok(EffectOutcome::resolved());
        }

        let Some(other_face) = game.object(target_id).and_then(|obj| obj.other_face) else {
            return Ok(EffectOutcome::resolved());
        };

        let Some(other_def) = crate::cards::builtin_registry().get_by_id(other_face) else {
            return Ok(EffectOutcome::resolved());
        };

        if let Some(obj) = game.object_mut(target_id) {
            obj.apply_definition_face(other_def);
        }

        game.flip(target_id);

        Ok(EffectOutcome::resolved())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "permanent to flip"
    }
}
