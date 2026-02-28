//! Attach to effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::find_target_object;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;
use crate::zone::Zone;

/// Effect that attaches the source permanent to a target permanent.
///
/// Used primarily by Auras that grant control or Equipment that auto-attach.
/// The source becomes attached to the target, and the target gains
/// the source in its attachments list.
///
/// # Fields
///
/// * `target` - The target specification for what to attach to
///
/// # Example
///
/// ```ignore
/// // Create an attach effect for an aura
/// let effect = AttachToEffect::new(ChooseSpec::target_creature());
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct AttachToEffect {
    /// The target to attach to.
    pub target: ChooseSpec,
}

impl AttachToEffect {
    /// Create a new attach to effect.
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }
}

impl EffectExecutor for AttachToEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let target_id = find_target_object(&ctx.targets)?;

        // If this is a spell on the stack (Aura resolving), defer attachment
        if let Some(source) = game.object(ctx.source)
            && source.zone == Zone::Stack
        {
            return Ok(EffectOutcome::resolved());
        }

        // Detach from previous parent if needed.
        let previous_parent = game
            .object(ctx.source)
            .and_then(|source| source.attached_to);
        if let Some(previous_parent) = previous_parent
            && previous_parent != target_id
            && let Some(parent) = game.object_mut(previous_parent)
        {
            parent.attachments.retain(|id| *id != ctx.source);
        }

        // Attach the source to the target
        if let Some(source) = game.object_mut(ctx.source) {
            source.attached_to = Some(target_id);
        }

        if let Some(target) = game.object_mut(target_id)
            && !target.attachments.contains(&ctx.source)
        {
            target.attachments.push(ctx.source);
        }
        game.continuous_effects.record_attachment(ctx.source);

        Ok(EffectOutcome::resolved())
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "target to attach to"
    }
}
