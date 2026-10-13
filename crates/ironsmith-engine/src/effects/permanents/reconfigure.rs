use crate::effect::{ChoiceCount, EffectOutcome};
use crate::effects::helpers::resolve_single_target_from_spec;
use crate::effects::{
    EffectExecutor, ExecutionContext, ExecutionError, TargetReusePolicy, TargetSelectionProfile,
};
use crate::target::ChooseSpec;

pub use ironsmith_core::ReconfigureEffect;

impl EffectExecutor for ReconfigureEffect {
    fn execute(
        &self,
        game: &mut crate::game_state::GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut crate::game_state::GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        // CR 400.7: an old ability must not follow the stable card identity
        // when its source leaves and returns as a new object.
        let attachment_id = ctx.source;
        if game.is_phased_out(attachment_id)
            || !game
                .object(attachment_id)
                .is_some_and(|object| object.zone == crate::zone::Zone::Battlefield)
        {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::resolved(),
            ));
        }

        // The two reconfigure activations are distinct (CR 702.151a).
        // Source denotes the targetless unattach branch; attach always
        // requires another target creature.
        if matches!(self.target.base(), ChooseSpec::Source) {
            let child =
                crate::effect::Effect::unattach_objects(ChooseSpec::SpecificObject(attachment_id))
                    .0
                    .execute_child_with_outputs(game, ctx)?;
            return Ok(child.project_aggregate(EffectOutcome::resolved()));
        }

        let target = resolve_single_target_from_spec(game, &self.target, ctx)?;
        super::attach_to::complete_resolved_attachment(
            game,
            ctx,
            attachment_id,
            target,
            super::attach_to::AttachmentChoicePolicy::NoChoice,
        );

        Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
            EffectOutcome::resolved(),
        ))
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

#[cfg(test)]
mod composition_tests {
    use super::*;
    use crate::{
        card::CardBuilder,
        ids::{CardId, PlayerId},
        types::{CardType, Subtype},
        zone::Zone,
    };

    #[test]
    fn attach_detach_and_departed_source_keep_exact_identity() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let player = PlayerId::from_index(0);
        let equipment = CardBuilder::new(CardId::new(), "Equipment")
            .card_types(vec![CardType::Artifact])
            .subtypes(vec![Subtype::Equipment])
            .build();
        let source = game.create_object_from_card(&equipment, player, Zone::Battlefield);
        let creature = CardBuilder::new(CardId::new(), "Creature")
            .card_types(vec![CardType::Creature])
            .build();
        let target = game.create_object_from_card(&creature, player, Zone::Battlefield);
        let mut ctx = ExecutionContext::new_default(source, player);
        ReconfigureEffect::new(ChooseSpec::SpecificObject(target))
            .execute(&mut game, &mut ctx)
            .unwrap();
        assert_eq!(
            game.object(source).unwrap().attached_to,
            Some(crate::object::AttachmentTarget::Object(target))
        );
        let detach = ReconfigureEffect::new(ChooseSpec::Source);
        assert!(detach.get_target_spec().is_none());
        detach.execute(&mut game, &mut ctx).unwrap();
        assert!(game.object(source).unwrap().attached_to.is_none());
        game.phase_out(source);
        ReconfigureEffect::new(ChooseSpec::SpecificObject(target))
            .execute(&mut game, &mut ctx)
            .unwrap();
        assert!(game.object(source).unwrap().attached_to.is_none());
        game.phase_in(source);
        let departed = game.move_object_by_effect(source, Zone::Exile).unwrap();
        let returned = game
            .move_object_by_effect(departed, Zone::Battlefield)
            .unwrap();
        ReconfigureEffect::new(ChooseSpec::SpecificObject(target))
            .execute(&mut game, &mut ctx)
            .unwrap();
        assert!(game.object(returned).unwrap().attached_to.is_none());
    }
}
