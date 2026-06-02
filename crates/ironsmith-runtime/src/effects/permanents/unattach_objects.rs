//! Unattach arbitrary objects from their current attachments.

use crate::costs::PaymentReason;
use crate::effect::EffectOutcome;
use crate::effects::helpers::resolve_objects_for_effect;
use crate::effects::{CostExecutableEffect, CostValidationError, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::target::ChooseSpec;

pub use ironsmith_core::UnattachObjectsEffect;

impl EffectExecutor for UnattachObjectsEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let object_ids = resolve_objects_for_effect(game, ctx, &self.objects)?;
        let mut count = 0;
        for object_id in object_ids {
            if game.detach_object_from_current_target(object_id) {
                count += 1;
            }
        }
        Ok(EffectOutcome::count(count))
    }

    fn decision_related_object_specs(&self) -> Vec<ChooseSpec> {
        vec![self.objects.clone()]
    }

    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn cost_description(&self) -> Option<String> {
        Some("Unattach the chosen object".to_string())
    }
}

impl CostExecutableEffect for UnattachObjectsEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
    ) -> Result<(), CostValidationError> {
        CostExecutableEffect::can_execute_as_cost_with_reason(
            self,
            game,
            source,
            controller,
            PaymentReason::Other,
        )
    }

    fn can_execute_as_cost_with_reason(
        &self,
        game: &GameState,
        source: ObjectId,
        _controller: PlayerId,
        _reason: PaymentReason,
    ) -> Result<(), CostValidationError> {
        match self.objects.base() {
            ChooseSpec::Tagged(_) => Ok(()),
            ChooseSpec::SpecificObject(id) => game
                .object(*id)
                .filter(|object| object.attached_to.and_then(|target| target.object_id()) == Some(source))
                .map(|_| ())
                .ok_or_else(|| {
                    CostValidationError::Other(
                        "chosen object is not attached to this source".to_string(),
                    )
                }),
            _ => Ok(()),
        }
    }
}
