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
        // CR 400.7: an old ability must not follow the stable card identity
        // when its source leaves and returns as a new object.
        let attachment_id = ctx.source;
        if game.is_phased_out(attachment_id)
            || !game
                .object(attachment_id)
                .is_some_and(|object| object.zone == crate::zone::Zone::Battlefield)
        {
            return Ok(EffectOutcome::resolved());
        }

        // The two reconfigure activations are distinct (CR 702.151a).
        // Source denotes the targetless unattach branch; attach always
        // requires another target creature.
        if matches!(self.target.base(), ChooseSpec::Source) {
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
        (!matches!(self.target.base(), ChooseSpec::Source)).then_some(&self.target)
    }

    fn target_selection_profile(&self) -> Option<TargetSelectionProfile<'_>> {
        self.get_target_spec()?;
        Some(TargetSelectionProfile {
            spec: &self.target,
            chooser: None,
            description: "target creature to attach to",
            min_targets: 1,
            max_targets: Some(1),
            count_value: None,
            distribution_value: None,
            distribution_min_per_target: 1,
            reuse_policy: TargetReusePolicy::ReuseCompatiblePrevious,
        })
    }

    fn get_target_count(&self) -> Option<ChoiceCount> {
        self.get_target_spec()?;
        Some(ChoiceCount {
            min: 1,
            max: Some(1),
            dynamic_x: false,
            up_to_x: false,
            random: false,
            explicit_exactly: false,
        })
    }
}
