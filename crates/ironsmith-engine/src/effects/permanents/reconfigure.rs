use crate::effect::{ChoiceCount, EffectOutcome};
use crate::effects::helpers::resolve_single_target_from_spec;
use crate::effects::{
    EffectExecutor, ExecutionContext, ExecutionError, TargetReusePolicy, TargetSelectionProfile,
};
use crate::object::AttachmentTarget;
use crate::target::ChooseSpec;

use super::attach_battlefield_object_to_target;
pub use ironsmith_core::ReconfigureEffect;

impl EffectExecutor for ReconfigureEffect {
    fn execute(
        &self,
        game: &mut crate::game_state::GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let snapshot_source = ctx
            .source_snapshot
            .as_ref()
            .and_then(|snapshot| game.find_object_by_stable_id(snapshot.stable_id))
            .filter(|id| {
                game.object(*id)
                    .is_some_and(|object| object.zone == crate::zone::Zone::Battlefield)
            });
        let attachment_id = snapshot_source.unwrap_or(ctx.source);

        if ctx.targets.is_empty() {
            game.detach_object_from_current_target(attachment_id);
            return Ok(EffectOutcome::resolved());
        }

        match resolve_single_target_from_spec(game, &self.target, ctx)? {
            crate::effects::ResolvedTarget::Object(id) => {
                attach_battlefield_object_to_target(
                    game,
                    attachment_id,
                    AttachmentTarget::Object(id),
                );
            }
            crate::effects::ResolvedTarget::Player(id) => {
                attach_battlefield_object_to_target(
                    game,
                    attachment_id,
                    AttachmentTarget::Player(id),
                );
            }
        }

        Ok(EffectOutcome::resolved())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_selection_profile(&self) -> Option<TargetSelectionProfile<'_>> {
        Some(TargetSelectionProfile {
            spec: &self.target,
            chooser: None,
            description: "target creature to attach to",
            min_targets: 0,
            max_targets: Some(1),
            count_value: None,
            distribution_value: None,
            distribution_min_per_target: 1,
            reuse_policy: TargetReusePolicy::ReuseCompatiblePrevious,
        })
    }

    fn get_target_count(&self) -> Option<ChoiceCount> {
        Some(ChoiceCount {
            min: 0,
            max: Some(1),
            dynamic_x: false,
            up_to_x: false,
            random: false,
            explicit_exactly: false,
        })
    }
}
