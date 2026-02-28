//! Attach arbitrary objects to a target permanent.

use crate::effect::{EffectOutcome, EffectResult};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_objects_from_spec;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;
use crate::zone::Zone;

/// Effect that attaches one or more objects to a destination object.
#[derive(Debug, Clone, PartialEq)]
pub struct AttachObjectsEffect {
    /// Objects to attach.
    pub objects: ChooseSpec,
    /// Destination to attach objects to.
    pub target: ChooseSpec,
}

impl AttachObjectsEffect {
    /// Create a new attach-objects effect.
    pub fn new(objects: ChooseSpec, target: ChooseSpec) -> Self {
        Self { objects, target }
    }
}

impl EffectExecutor for AttachObjectsEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let target_ids = resolve_objects_from_spec(game, &self.target, ctx)?;
        let Some(target_id) = target_ids.first().copied() else {
            return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid));
        };
        if game
            .object(target_id)
            .is_none_or(|target| target.zone != Zone::Battlefield)
        {
            return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid));
        }

        let object_ids = resolve_objects_from_spec(game, &self.objects, ctx)?;
        if object_ids.is_empty() {
            return Ok(EffectOutcome::count(0));
        }

        let mut attached_count = 0i32;
        for object_id in object_ids {
            if object_id == target_id {
                continue;
            }
            let Some(attached_obj) = game.object(object_id) else {
                continue;
            };
            if attached_obj.zone != Zone::Battlefield {
                continue;
            }

            let previous_parent = attached_obj.attached_to;
            if let Some(previous_parent) = previous_parent
                && let Some(parent) = game.object_mut(previous_parent)
            {
                parent.attachments.retain(|id| *id != object_id);
            }

            if let Some(object_mut) = game.object_mut(object_id) {
                object_mut.attached_to = Some(target_id);
            } else {
                continue;
            }

            if let Some(target_mut) = game.object_mut(target_id)
                && !target_mut.attachments.contains(&object_id)
            {
                target_mut.attachments.push(object_id);
            }
            game.continuous_effects.record_attachment(object_id);
            attached_count += 1;
        }

        Ok(EffectOutcome::count(attached_count))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "object to attach to"
    }
}
