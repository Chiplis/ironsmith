//! Transform effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::find_target_object;
use crate::events::other::TransformedEvent;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;
use crate::triggers::TriggerEvent;

/// Effect that transforms a double-faced permanent.
///
/// Toggles the face state of a DFC (double-faced card).
/// When face_down is false, the card shows its front face.
/// When face_down is true, the card shows its back face.
///
/// # Fields
///
/// * `target` - The permanent to transform
///
/// # Example
///
/// ```ignore
/// // Transform target permanent
/// let effect = TransformEffect::new(ChooseSpec::permanent());
///
/// // Transform this permanent (the source)
/// let effect = TransformEffect::source();
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct TransformEffect {
    /// The targeting specification.
    pub target: ChooseSpec,
}

impl TransformEffect {
    /// Create a new transform effect.
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }

    /// Create an effect that transforms the source permanent.
    pub fn source() -> Self {
        Self::new(ChooseSpec::Source)
    }

    /// Create an effect that transforms target permanent.
    pub fn target_permanent() -> Self {
        Self::new(ChooseSpec::permanent())
    }
}

impl EffectExecutor for TransformEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let target_id = match &self.target {
            ChooseSpec::Source => ctx.source,
            ChooseSpec::SpecificObject(id) => *id,
            _ => find_target_object(&ctx.targets)?,
        };

        if !game.can_transform(target_id) {
            return Ok(EffectOutcome::resolved());
        }

        // Toggle the face state (for DFCs, face_down = back face)
        if game.is_face_down(target_id) {
            game.set_face_up(target_id);
        } else {
            game.set_face_down(target_id);
        }

        Ok(EffectOutcome::resolved()
            .with_event(TriggerEvent::new(TransformedEvent::new(target_id))))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "permanent to transform"
    }
}
