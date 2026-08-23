use crate::effect::{Effect, EffectMode};
use crate::effects::EffectExecutor;

pub trait EffectModel {
    type Effect: Clone + 'static;
    type StaticAbility: Clone + 'static;
    type CardDefinition: Clone + 'static;
    type Ability: Clone + 'static;
    type EmblemDescription: Clone + 'static;
    type ContinuousTarget: Clone + Into<crate::continuous::EffectTarget> + 'static;
    type ContinuousModification: Clone + 'static;
    type RuntimeModification: Clone + 'static;
    type Grantable: Clone + 'static;
    type GrantDuration: Clone + Copy + 'static;
    type GrantSpec: Clone + 'static;

    fn downcast_ref<T: 'static>(effect: &Self::Effect) -> Option<&T>;
    fn payload_type_name(effect: &Self::Effect) -> &str;
}

pub trait EffectModelInterpreterHooks<M: EffectModel> {
    type Error;

    fn unsupported_effect(&mut self, detail: String) -> Self::Error;

    fn runtime_static_ability_hook(
        &mut self,
        ability: M::StaticAbility,
    ) -> Result<crate::static_abilities::StaticAbility, Self::Error>;

    fn runtime_card_definition_hook(
        &mut self,
        definition: M::CardDefinition,
    ) -> Result<crate::cards::CardDefinition, Self::Error>;

    fn runtime_ability_hook(
        &mut self,
        ability: M::Ability,
    ) -> Result<crate::ability::Ability, Self::Error>;

    fn runtime_emblem_hook(
        &mut self,
        emblem: M::EmblemDescription,
    ) -> Result<crate::effect::EmblemDescription, Self::Error>;

    fn runtime_continuous_modification_hook(
        &mut self,
        modification: M::ContinuousModification,
    ) -> Result<crate::continuous::Modification, Self::Error>;

    fn runtime_continuous_runtime_modification_hook(
        &mut self,
        modification: M::RuntimeModification,
    ) -> Result<crate::effects::continuous::RuntimeModification, Self::Error>;

    fn runtime_grantable_hook(
        &mut self,
        grantable: M::Grantable,
    ) -> Result<crate::grant::Grantable, Self::Error>;

    fn runtime_grant_duration_hook(
        &mut self,
        duration: M::GrantDuration,
    ) -> Result<crate::grant::GrantDuration, Self::Error>;

    fn runtime_grant_spec_hook(
        &mut self,
        spec: M::GrantSpec,
    ) -> Result<crate::grant::GrantSpec, Self::Error>;

    fn runtime_external_model_effect_hook(
        &mut self,
        _effect: &M::Effect,
    ) -> Result<Option<Effect>, Self::Error> {
        Ok(None)
    }
}

fn interpret_core_cost_model<M, H>(
    cost: ironsmith_core::Cost<M::Effect>,
    hooks: &mut H,
) -> Result<crate::costs::Cost, H::Error>
where
    M: EffectModel,
    H: EffectModelInterpreterHooks<M>,
{
    let runtime_model = match cost {
        ironsmith_core::Cost::Mana(mana) => ironsmith_core::Cost::Mana(mana),
        ironsmith_core::Cost::DynamicMana(dynamic_mana) => {
            ironsmith_core::Cost::DynamicMana(dynamic_mana)
        }
        ironsmith_core::Cost::Tap => ironsmith_core::Cost::Tap,
        ironsmith_core::Cost::Untap => ironsmith_core::Cost::Untap,
        ironsmith_core::Cost::DiscardSource => ironsmith_core::Cost::DiscardSource,
        ironsmith_core::Cost::SacrificeSelf => ironsmith_core::Cost::SacrificeSelf,
        ironsmith_core::Cost::Sacrifice(filter) => ironsmith_core::Cost::Sacrifice(filter),
        ironsmith_core::Cost::Discard { count, card_types } => {
            ironsmith_core::Cost::Discard { count, card_types }
        }
        ironsmith_core::Cost::DiscardHand => ironsmith_core::Cost::DiscardHand,
        ironsmith_core::Cost::RemoveCounters {
            counter_type,
            count,
        } => ironsmith_core::Cost::RemoveCounters {
            counter_type,
            count,
        },
        ironsmith_core::Cost::AddCounters {
            counter_type,
            count,
        } => ironsmith_core::Cost::AddCounters {
            counter_type,
            count,
        },
        ironsmith_core::Cost::RemoveAnyCountersFromSource {
            counter_type,
            display_x,
            remove_all,
        } => ironsmith_core::Cost::RemoveAnyCountersFromSource {
            counter_type,
            display_x,
            remove_all,
        },
        ironsmith_core::Cost::Energy(amount) => ironsmith_core::Cost::Energy(amount),
        ironsmith_core::Cost::Mill(count) => ironsmith_core::Cost::Mill(count),
        ironsmith_core::Cost::Life(amount) => ironsmith_core::Cost::Life(amount),
        ironsmith_core::Cost::ExileSelf => ironsmith_core::Cost::ExileSelf,
        ironsmith_core::Cost::ExileFromHand {
            count,
            color_filter,
        } => ironsmith_core::Cost::ExileFromHand {
            count,
            color_filter,
        },
        ironsmith_core::Cost::ExileFromGraveyard { count, card_types } => {
            ironsmith_core::Cost::ExileFromGraveyard { count, card_types }
        }
        ironsmith_core::Cost::ReturnSelfToHand => ironsmith_core::Cost::ReturnSelfToHand,
        ironsmith_core::Cost::Effect(effect) => {
            ironsmith_core::Cost::Effect(interpret_effect_model::<M, H>(effect, hooks)?)
        }
    };

    crate::costs::Cost::from_model(runtime_model).map_err(|detail| {
        hooks.unsupported_effect(format!("unsupported runtime cost model: {detail}"))
    })
}

fn interpret_core_total_cost_model<M, H>(
    cost: ironsmith_core::TotalCost<ironsmith_core::Cost<M::Effect>>,
    hooks: &mut H,
) -> Result<crate::cost::TotalCost, H::Error>
where
    M: EffectModel,
    H: EffectModelInterpreterHooks<M>,
{
    match cost.kind() {
        ironsmith_core::TotalCostKind::All(_) => {
            let mut runtime_costs = Vec::with_capacity(cost.costs().len());
            for component in cost.costs().iter().cloned() {
                if let ironsmith_core::Cost::Effect(effect) = component {
                    let runtime_effect = interpret_effect_model::<M, H>(effect, hooks)?;
                    if crate::costs::Cost::is_tagged_type_marker_effect(&runtime_effect) {
                        continue;
                    }
                    runtime_costs.push(
                        crate::costs::Cost::from_model(ironsmith_core::Cost::Effect(
                            runtime_effect,
                        ))
                        .map_err(|detail| {
                            hooks.unsupported_effect(format!(
                                "unsupported runtime cost model: {detail}"
                            ))
                        })?,
                    );
                    continue;
                }
                runtime_costs.push(interpret_core_cost_model::<M, H>(component, hooks)?);
            }
            Ok(crate::cost::TotalCost::from_costs(runtime_costs))
        }
        ironsmith_core::TotalCostKind::OneOf(branches) => {
            let mut runtime_branches = Vec::with_capacity(branches.len());
            for branch in branches.iter().cloned() {
                runtime_branches.push(interpret_core_total_cost_model::<M, H>(branch, hooks)?);
            }
            Ok(crate::cost::TotalCost::one_of(runtime_branches))
        }
    }
}

pub fn interpret_effect_model<M, H>(effect: M::Effect, hooks: &mut H) -> Result<Effect, H::Error>
where
    M: EffectModel,
    H: EffectModelInterpreterHooks<M>,
{
    if let Some(payload) = M::downcast_ref::<ironsmith_core::TaggedEffect<M::Effect>>(&effect) {
        return Ok(Effect::new(crate::effects::TaggedEffect::new(
            payload.tag.clone(),
            interpret_effect_model((*payload.effect).clone(), hooks)?,
        )));
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::TargetOnlyEffect>(&effect) {
        return Ok(converted);
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::SearchLibraryEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(converted) =
        clone_direct_effect::<M, crate::effects::SearchLibrarySlotsEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::DealDamageEffect>(&effect) {
        return Ok(converted);
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::PutCountersEffect>(&effect) {
        return Ok(converted);
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::ChooseModeEffect<M::Effect>>(&effect) {
        let mut converted = crate::effects::ChooseModeEffect::new(
            payload
                .modes
                .iter()
                .cloned()
                .map(|mode| convert_effect_mode(mode, hooks))
                .collect::<Result<Vec<_>, _>>()?,
            payload.min.clone(),
            payload.max.clone(),
            payload.allow_repeat,
        );
        converted.choose_count = payload.choose_count.clone();
        converted.common_prefix_effects = payload
            .common_prefix_effects
            .iter()
            .cloned()
            .map(|effect| interpret_effect_model(effect, hooks))
            .collect::<Result<Vec<_>, _>>()?;
        converted.chooser = payload.chooser.clone();
        converted.min_choose_count = payload.min_choose_count.clone();
        converted.allow_repeated_modes = payload.allow_repeated_modes;
        converted.random = payload.random;
        converted.mode_point_costs = payload.mode_point_costs.clone();
        converted.spree = payload.spree;
        converted.tiered = payload.tiered;
        converted.mode_additional_mana_costs = payload.mode_additional_mana_costs.clone();
        converted.common_suffix_effect_count = payload.common_suffix_effect_count;
        converted.disallow_previously_chosen_modes = payload.disallow_previously_chosen_modes;
        converted.disallow_previously_chosen_modes_this_turn =
            payload.disallow_previously_chosen_modes_this_turn;
        converted.distinct_player_targets_per_mode = payload.distinct_player_targets_per_mode;
        return Ok(Effect::new(converted));
    }
    if let Some(payload) =
        M::downcast_ref::<ironsmith_core::VillainousChoiceEffect<M::Effect>>(&effect)
    {
        let mut converted = crate::effects::VillainousChoiceEffect::new(
            payload.player.clone(),
            payload
                .modes
                .iter()
                .cloned()
                .map(|mode| convert_effect_mode(mode, hooks))
                .collect::<Result<Vec<_>, _>>()?,
        );
        converted.player_surface = payload.player_surface.clone();
        return Ok(Effect::new(converted));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::ConditionalEffect<M::Effect>>(&effect)
    {
        return Ok(Effect::new(
            crate::effects::ConditionalEffect::new(
                payload.condition.clone(),
                convert_effects(payload.if_true.iter().cloned(), hooks)?,
                convert_effects(payload.if_false.iter().cloned(), hooks)?,
            )
            .with_surface(payload.surface),
        ));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::IfEffect<M::Effect>>(&effect) {
        return Ok(Effect::new(
            crate::effects::IfEffect::new(
                payload.condition,
                payload.predicate.clone(),
                convert_effects(payload.then.iter().cloned(), hooks)?,
                convert_effects(payload.else_.iter().cloned(), hooks)?,
            )
            .with_per_player_result(payload.per_player_result)
            .with_prior_result_replacement_surface(payload.prior_result_replacement_surface),
        ));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::WithIdEffect<M::Effect>>(&effect) {
        return Ok(Effect::with_id(
            payload.id.0,
            interpret_effect_model((*payload.effect).clone(), hooks)?,
        ));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::HauntExileEffect<M::Effect>>(&effect) {
        return Ok(Effect::new(crate::effects::HauntExileEffect::new(
            convert_effects(payload.haunt_effects.iter().cloned(), hooks)?,
            payload.haunt_choices.clone(),
        )));
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::DrawCardsEffect>(&effect) {
        return Ok(converted);
    }
    if let Some(payload) =
        M::downcast_ref::<ironsmith_core::DrawForEachTaggedMatchingEffect>(&effect)
    {
        return Ok(Effect::new(
            crate::effects::DrawForEachTaggedMatchingEffect::new(
                payload.player.clone(),
                payload.tag.clone(),
                payload.filter.clone(),
            ),
        ));
    }
    if let Some(payload) =
        M::downcast_ref::<ironsmith_core::CreateTokenEffect<M::CardDefinition>>(&effect)
    {
        let mut converted = crate::effects::CreateTokenEffect::new(
            hooks.runtime_card_definition_hook(payload.token.clone())?,
            payload.count.clone(),
            payload.controller.clone(),
        );
        if payload.use_source_chosen_color {
            converted = converted.with_source_chosen_color();
        }
        if payload.use_source_chosen_creature_type {
            converted = converted.with_source_chosen_creature_type();
        }
        if payload.actor_surface_explicit {
            converted = converted.with_explicit_actor_surface();
        }
        if payload.enters_tapped {
            converted = converted.tapped();
        }
        if payload.enters_attacking {
            converted = converted.attacking();
        }
        if let Some(attack_target_mode) = &payload.attack_target_mode {
            converted = converted.attack_target_mode(attack_target_mode.clone());
        }
        if payload.suppress_aura_attachment_choice {
            converted = converted.suppress_aura_attachment_choice();
        }
        if let Some(presentation) = payload.ability_presentation {
            converted = converted.with_ability_presentation(presentation);
        }
        if payload.exile_at_end_of_combat {
            converted = converted.exile_at_end_of_combat();
        }
        if payload.sacrifice_at_end_of_combat {
            converted = converted.sacrifice_at_end_of_combat();
        }
        if payload.sacrifice_at_next_end_step {
            converted = converted.sacrifice_at_next_end_step();
        }
        if payload.exile_at_next_end_step {
            converted = converted.exile_at_next_end_step();
        }
        converted = converted.next_end_step_player(payload.next_end_step_player.clone());
        return Ok(Effect::new(converted));
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::TapEffect>(&effect) {
        return Ok(converted);
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::UntapEffect>(&effect) {
        return Ok(converted);
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::RemoveCountersEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::UnearthEffect>(&effect) {
        return Ok(converted);
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::NinjutsuCostEffect>(&effect) {
        return Ok(converted);
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::SneakCostEffect>(&effect) {
        return Ok(converted);
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::ConspireCostEffect>(&effect) {
        return Ok(converted);
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::NinjutsuEffect>(&effect) {
        return Ok(converted);
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::CastSourceEffect>(&effect) {
        return Ok(converted);
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::CipherEffect>(&effect) {
        return Ok(converted);
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::CounterEffect>(&effect) {
        return Ok(converted);
    }
    if let Some(payload) =
        M::downcast_ref::<ironsmith_core::ScheduleDelayedTriggerEffect<M::Effect>>(&effect)
    {
        if !payload.target_choices.is_empty() {
            return Err(hooks.unsupported_effect(
                "delayed trigger conversion with compiler target choices".to_string(),
            ));
        }
        let converted_effects = convert_effects(payload.effects.iter().cloned(), hooks)?;
        let trigger = crate::triggers::Trigger::from_delayed_trigger_spec(payload.trigger.clone());
        let mut converted = if let Some(tag) = &payload.target_tag {
            crate::effects::ScheduleDelayedTriggerEffect::from_tag(
                trigger,
                converted_effects,
                payload.one_shot,
                tag.clone(),
                payload.controller.clone(),
            )
        } else {
            crate::effects::ScheduleDelayedTriggerEffect::new(
                trigger,
                converted_effects,
                payload.one_shot,
                Vec::new(),
                payload.controller.clone(),
            )
        };
        if let Some(filter) = &payload.target_filter {
            converted = converted.with_target_filter(filter.clone());
        }
        if payload.watch_ability_source {
            converted = converted.watch_ability_source();
        }
        if payload.watch_all_object_targets {
            converted = converted.watch_all_object_targets();
        }
        if payload.either_of_watched_objects {
            converted = converted.with_either_of_watched_objects_surface();
        }
        if let Some((tag, zone)) = &payload.while_any_tagged_object_in_zone {
            converted = converted.while_any_tagged_object_in_zone(tag.clone(), *zone);
        }
        if payload.start_next_turn {
            converted = converted.starting_next_turn();
        }
        if payload.leading_duration_surface {
            converted = converted.with_leading_duration_surface();
        }
        if let Some(prepayment) = &payload.prepayment {
            converted = converted.unless_paid_before_trigger(
                prepayment.player.clone(),
                interpret_core_total_cost_model::<M, H>(prepayment.cost.clone(), hooks)?,
            );
        }
        if payload.event_value_from_prior_prevention {
            converted = converted.with_prior_prevention_event_value();
        }
        converted = match payload.duration {
            ironsmith_core::DelayedTriggerDuration::Forever => converted,
            ironsmith_core::DelayedTriggerDuration::EndOfTurn => converted.until_end_of_turn(),
            ironsmith_core::DelayedTriggerDuration::EndOfCombat => converted.until_end_of_combat(),
            ironsmith_core::DelayedTriggerDuration::UntilControllerNextTurn => {
                converted.until_controller_next_turn()
            }
        };
        // Backward compatibility for compiler effects authored before the
        // typed duration field was introduced.
        if payload.duration == ironsmith_core::DelayedTriggerDuration::Forever {
            if payload.until_end_of_turn {
                converted = converted.until_end_of_turn();
            }
            if payload.until_end_of_combat {
                converted = converted.until_end_of_combat();
            }
        }
        return Ok(Effect::new(converted));
    }
    if let Some(payload) =
        M::downcast_ref::<ironsmith_core::GrantAbilitiesTargetEffect<M::StaticAbility>>(&effect)
    {
        return Ok(Effect::new(
            crate::effects::GrantAbilitiesTargetEffect::new(
                payload.target.clone(),
                payload
                    .abilities
                    .iter()
                    .cloned()
                    .map(|ability| hooks.runtime_static_ability_hook(ability))
                    .collect::<Result<Vec<_>, _>>()?,
                payload.duration.clone(),
            ),
        ));
    }
    if let Some(payload) =
        M::downcast_ref::<ironsmith_core::CreateTokenCopyEffect<M::StaticAbility>>(&effect)
    {
        return Ok(Effect::new(crate::effects::CreateTokenCopyEffect {
            target: payload.target.clone(),
            count: payload.count.clone(),
            controller: payload.controller.clone(),
            enters_tapped: payload.enters_tapped,
            has_haste: payload.has_haste,
            haste_followup_reference_surface: payload.haste_followup_reference_surface,
            enters_attacking: payload.enters_attacking,
            attack_target_mode: payload.attack_target_mode.clone(),
            exile_at_end_of_combat: payload.exile_at_end_of_combat,
            exile_at_end_of_combat_reference_surface: payload
                .exile_at_end_of_combat_reference_surface,
            loses_soulbond: payload.loses_soulbond,
            sacrifice_at_next_end_step: payload.sacrifice_at_next_end_step,
            sacrifice_at_next_end_step_reference_surface: payload
                .sacrifice_at_next_end_step_reference_surface,
            sacrifice_at_next_end_step_ability_text: payload
                .sacrifice_at_next_end_step_ability_text
                .clone(),
            exile_at_next_end_step: payload.exile_at_next_end_step,
            exile_at_next_end_step_reference_surface: payload
                .exile_at_next_end_step_reference_surface,
            next_end_step_player: payload.next_end_step_player.clone(),
            pt_adjustment: payload.pt_adjustment.clone(),
            clear_mana_cost: payload.clear_mana_cost,
            added_card_types: payload.added_card_types.clone(),
            added_subtypes: payload.added_subtypes.clone(),
            removed_supertypes: payload.removed_supertypes.clone(),
            set_base_power_toughness: payload.set_base_power_toughness,
            set_base_power_toughness_value: payload.set_base_power_toughness_value.clone(),
            starting_loyalty: payload.starting_loyalty,
            set_colors: payload.set_colors,
            set_card_types: payload.set_card_types.clone(),
            set_subtypes: payload.set_subtypes.clone(),
            granted_static_abilities: payload
                .granted_static_abilities
                .iter()
                .cloned()
                .map(|ability| hooks.runtime_static_ability_hook(ability))
                .collect::<Result<Vec<_>, _>>()?,
        }));
    }
    if let Some(payload) =
        M::downcast_ref::<ironsmith_core::GrantNextSpellAbilityEffect<M::Ability>>(&effect)
    {
        return Ok(Effect::new(
            crate::effects::GrantNextSpellAbilityEffect::new(
                payload.player.clone(),
                payload.filter.clone(),
                hooks.runtime_ability_hook(payload.ability.clone())?,
            ),
        ));
    }
    if let Some(payload) = M::downcast_ref::<
        ironsmith_core::ApplyContinuousEffect<
            M::ContinuousTarget,
            M::ContinuousModification,
            M::RuntimeModification,
            crate::ConditionExpr,
        >,
    >(&effect)
    {
        let converted = crate::effects::ApplyContinuousEffect {
            target: payload.target.clone().into(),
            target_spec: payload.target_spec.clone(),
            modification: payload
                .modification
                .clone()
                .map(|modification| hooks.runtime_continuous_modification_hook(modification))
                .transpose()?,
            additional_modifications: payload
                .additional_modifications
                .iter()
                .cloned()
                .map(|modification| hooks.runtime_continuous_modification_hook(modification))
                .collect::<Result<Vec<_>, _>>()?,
            runtime_modifications: payload
                .runtime_modifications
                .iter()
                .cloned()
                .map(|modification| {
                    hooks.runtime_continuous_runtime_modification_hook(modification)
                })
                .collect::<Result<Vec<_>, _>>()?,
            until: payload.until.clone(),
            condition: payload.condition.clone(),
            source_type: None,
            source_reference_surface: payload.source_reference_surface.clone(),
            set_quantifier_surface: payload.set_quantifier_surface,
            type_retention_surface: payload.type_retention_surface,
            animation_pt_surface: payload.animation_pt_surface,
            animation_duration_surface: payload.animation_duration_surface,
            lock_filter_at_resolution: payload.lock_filter_at_resolution,
            resolve_set_pt_values_at_resolution: payload.resolve_set_pt_values_at_resolution,
            require_creature_target: payload.require_creature_target,
        };
        if converted.modification.is_none()
            && converted.additional_modifications.is_empty()
            && converted.runtime_modifications.is_empty()
        {
            return Err(hooks.unsupported_effect(
                "apply continuous effect without any modification".to_string(),
            ));
        }
        return Ok(Effect::new(converted));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::LoseLifeEffect>(&effect) {
        return Ok(Effect::new(crate::effects::LoseLifeEffect::with_filter(
            payload.amount.clone(),
            payload.player.clone(),
        )));
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::GainLifeEffect>(&effect) {
        return Ok(converted);
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::SacrificeEffect>(&effect) {
        let mut sacrifice = crate::effects::SacrificeEffect::you(
            payload.filter.clone(),
            crate::effect::Value::Fixed(payload.count),
        );
        for tag in &payload.event_object_tags {
            sacrifice = sacrifice.with_event_object_tag(tag.clone());
        }
        for tag in &payload.event_source_tags {
            sacrifice = sacrifice.with_event_source_tag(tag.clone());
        }
        return Ok(Effect::new(sacrifice));
    }
    if let Some(converted) =
        clone_direct_effect::<M, crate::effects::zones::SacrificePlayerEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::DestroyEffect>(&effect) {
        return Ok(Effect::new(crate::effects::DestroyEffect::with_spec(
            payload.spec.clone(),
        )));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::DestroyNoRegenerationEffect>(&effect) {
        let converted = if let Some(target) = &payload.target {
            crate::effects::DestroyNoRegenerationEffect::with_spec(target.clone())
        } else if let Some(filter) = &payload.filter {
            crate::effects::DestroyNoRegenerationEffect::all(filter.clone())
        } else {
            return Err(hooks.unsupported_effect(
                "destroy-no-regeneration effect without target or filter".to_string(),
            ));
        }
        .with_creature_destroyed_this_way_surface(payload.creature_destroyed_this_way_surface);
        return Ok(Effect::new(converted));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::RegenerateEffect<M::Effect>>(&effect) {
        let follow_up_effects = convert_effects(payload.follow_up_effects.clone(), hooks)?;
        return Ok(Effect::new(
            crate::effects::RegenerateEffect::new(payload.target.clone(), payload.duration.clone())
                .with_follow_up_effects(follow_up_effects),
        ));
    }
    if let Some(converted) =
        clone_direct_effect::<M, crate::effects::AddManaFromCommanderColorIdentityEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(converted) =
        clone_direct_effect::<M, crate::effects::AddManaOfAnyColorEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(converted) =
        clone_direct_effect::<M, crate::effects::AddManaOfAnyOneColorEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(converted) =
        clone_direct_effect::<M, crate::effects::AddManaOfColorsAmongEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(converted) =
        clone_direct_effect::<M, crate::effects::AddOneManaOfAnyColorAmongEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::AddManaEffect>(&effect) {
        return Ok(converted);
    }
    if let Some(converted) =
        clone_direct_effect::<M, crate::effects::PreventAllCombatDamageEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(converted) =
        clone_direct_effect::<M, crate::effects::AssignNoCombatDamageEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(converted) =
        clone_direct_effect::<M, crate::effects::EmitKeywordActionEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::AscendEffect>(&effect) {
        return Ok(converted);
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::RenownEffect>(&effect) {
        return Ok(converted);
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::SolveCaseEffect>(&effect) {
        return Ok(converted);
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::RestartGameEffect>(&effect) {
        return Ok(converted);
    }
    if let Some(converted) =
        clone_direct_effect::<M, crate::effects::ReverseTurnOrderEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::PlaySubgameEffect<M::Effect>>(&effect)
    {
        return Ok(Effect::new(crate::effects::PlaySubgameEffect::new(
            convert_effects(payload.nonwinner_effects.iter().cloned(), hooks)?,
        )));
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::BolsterEffect>(&effect) {
        return Ok(converted);
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::FlipEffect>(&effect) {
        return Ok(converted);
    }
    if let Some(converted) =
        clone_direct_effect::<M, crate::effects::PreventAllDamageEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(payload) =
        M::downcast_ref::<ironsmith_core::PreventAllDamageToTargetEffect<M::Effect>>(&effect)
    {
        return Ok(Effect::new(
            crate::effects::PreventAllDamageToTargetEffect::new(
                payload.target.clone(),
                payload.until.clone(),
            )
            .with_follow_up_effects(convert_effects(
                payload.follow_up_effects.iter().cloned(),
                hooks,
            )?),
        ));
    }
    if let Some(payload) =
        M::downcast_ref::<ironsmith_core::PreventDamageEffect<M::Effect>>(&effect)
    {
        let mut prevent = crate::effects::PreventDamageEffect::new(
            payload.amount.clone(),
            payload.target.clone(),
            payload.until.clone(),
        )
        .with_follow_up_effects(convert_effects(
            payload.follow_up_effects.iter().cloned(),
            hooks,
        )?);
        prevent.source_of_your_choice = payload.source_of_your_choice;
        prevent.protect_you_and_permanents_you_control =
            payload.protect_you_and_permanents_you_control;
        return Ok(Effect::new(prevent));
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::LoseTheGameEffect>(&effect) {
        return Ok(converted);
    }
    if let Some(payload) =
        M::downcast_ref::<ironsmith_core::CreateEmblemEffect<M::EmblemDescription>>(&effect)
    {
        return Ok(Effect::new(crate::effects::CreateEmblemEffect::new(
            hooks.runtime_emblem_hook(payload.emblem.clone())?,
        )));
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::DiscardHandEffect>(&effect) {
        return Ok(converted);
    }
    if let Some(converted) =
        clone_direct_effect::<M, crate::effects::RemoveAnyCountersAmongEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::ChooseCardTypeEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(converted) =
        clone_direct_effect::<M, crate::effects::ShuffleObjectsIntoLibraryEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(converted) =
        clone_direct_effect::<M, crate::effects::ExchangeTextBoxesEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::EnergyCountersEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::TicketCountersEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::DiscoverEffect>(&effect) {
        return Ok(converted);
    }
    if let Some(converted) =
        clone_direct_effect::<M, crate::effects::SetBasePowerToughnessEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::CantEffect>(&effect) {
        return Ok(converted);
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::ExtraTurnEffect>(&effect) {
        return Ok(converted);
    }
    if let Some(converted) =
        clone_direct_effect::<M, crate::effects::ExtraTurnAfterNextTurnEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(converted) =
        clone_direct_effect::<M, crate::effects::AdditionalPhasesEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(converted) =
        clone_direct_effect::<M, crate::effects::ModifyPowerToughnessForEachEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::CastTaggedEffect>(&effect) {
        return Ok(converted);
    }
    if let Some(converted) =
        clone_direct_effect::<M, crate::effects::PutTaggedRemainderOnLibraryBottomEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(converted) =
        clone_direct_effect::<M, crate::effects::RemoveUpToAnyCountersEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(converted) =
        clone_direct_effect::<M, crate::effects::TagTriggeringObjectEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(converted) =
        clone_direct_effect::<M, crate::effects::TagTriggeringBlockersEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(converted) =
        clone_direct_effect::<M, crate::effects::TagTriggeringAttackerEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(converted) =
        clone_direct_effect::<M, crate::effects::TagTriggeringSourceEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::AttachToEffect>(&effect) {
        return Ok(converted);
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::ReconfigureEffect>(&effect) {
        return Ok(converted);
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::AttachObjectsEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(converted) =
        clone_direct_effect::<M, crate::effects::UnattachObjectsEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::SequenceEffect<M::Effect>>(&effect) {
        let effects = convert_effects(payload.effects.iter().cloned(), hooks)?;
        let mut sequence = match payload.surface {
            ironsmith_core::SequenceSurface::Sequential => {
                crate::effects::SequenceEffect::new(effects)
            }
            ironsmith_core::SequenceSurface::SentenceLeadingThen => {
                crate::effects::SequenceEffect::sentence_leading_then(effects)
            }
            ironsmith_core::SequenceSurface::CommaThen => {
                crate::effects::SequenceEffect::comma_then(effects)
            }
            ironsmith_core::SequenceSurface::RepeatedCommaThen => {
                crate::effects::SequenceEffect::repeated_comma_then(effects)
            }
            ironsmith_core::SequenceSurface::Coordinated => {
                crate::effects::SequenceEffect::coordinated(effects)
            }
            ironsmith_core::SequenceSurface::CoordinatedLeadingDuration => {
                crate::effects::SequenceEffect::coordinated_with_leading_duration(effects)
            }
            ironsmith_core::SequenceSurface::ResultConjunction { leading_duration } => {
                crate::effects::SequenceEffect::result_conjunction(effects, leading_duration)
            }
        };
        sequence.result_label = payload.result_label.clone();
        return Ok(Effect::new(sequence));
    }
    if let Some(payload) =
        M::downcast_ref::<ironsmith_core::ManaRestrictedEffect<M::Effect>>(&effect)
    {
        let restrictions = payload
            .restrictions
            .iter()
            .cloned()
            .map(|restriction| {
                restriction.try_map_effects(&mut |effect| interpret_effect_model(effect, hooks))
            })
            .collect::<Result<Vec<_>, H::Error>>()?;
        return Ok(Effect::new(crate::effects::ManaRestrictedEffect::new(
            convert_effects(payload.effects.iter().cloned(), hooks)?,
            restrictions,
        )));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::ManaRetainedEffect<M::Effect>>(&effect)
    {
        return Ok(Effect::new(crate::effects::ManaRetainedEffect::new(
            convert_effects(payload.effects.iter().cloned(), hooks)?,
            payload.duration,
        )));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::MayEffect<M::Effect>>(&effect) {
        let effects = convert_effects(payload.effects.iter().cloned(), hooks)?;
        let converted = if let Some(decider) = &payload.decider {
            crate::effects::MayEffect::new_for_player(effects, decider.clone())
        } else {
            crate::effects::MayEffect::new(effects)
        };
        return Ok(Effect::new(converted));
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::RevealTopEffect>(&effect) {
        return Ok(converted);
    }
    if let Some(converted) =
        clone_direct_effect::<M, crate::effects::TagAttachedToSourceEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::ForPlayersEffect<M::Effect>>(&effect) {
        let effects = convert_effects(payload.effects.iter().cloned(), hooks)?;
        let mut converted = if payload.starting_with_controller {
            crate::effects::ForPlayersEffect::new_starting_with_controller(
                payload.filter.clone(),
                effects,
            )
        } else {
            crate::effects::ForPlayersEffect::new(payload.filter.clone(), effects)
        };
        if payload.stop_after_first_happened {
            converted = converted.stop_after_first_happened();
        }
        return Ok(Effect::new(converted));
    }
    if let Some(payload) =
        M::downcast_ref::<ironsmith_core::ForEachTaggedEffect<M::Effect>>(&effect)
    {
        let mut converted = crate::effects::ForEachTaggedEffect::new(
            payload.tag.clone(),
            convert_effects(payload.effects.iter().cloned(), hooks)?,
        );
        if let Some(blocker_tag) = &payload.controller_at_last_blocked_by {
            converted = converted.with_controller_at_last_blocked_by(blocker_tag.clone());
        }
        return Ok(Effect::new(converted));
    }
    if let Some(payload) =
        M::downcast_ref::<ironsmith_core::ForEachControllerOfTaggedEffect<M::Effect>>(&effect)
    {
        return Ok(Effect::new(
            crate::effects::ForEachControllerOfTaggedEffect::new(
                payload.tag.clone(),
                convert_effects(payload.effects.iter().cloned(), hooks)?,
            ),
        ));
    }
    if let Some(payload) =
        M::downcast_ref::<ironsmith_core::ForEachTaggedPlayerEffect<M::Effect>>(&effect)
    {
        return Ok(Effect::new(crate::effects::ForEachTaggedPlayerEffect::new(
            payload.tag.clone(),
            convert_effects(payload.effects.iter().cloned(), hooks)?,
        )));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::VoteEffect<M::Effect>>(&effect) {
        let converted = match &payload.choice {
            ironsmith_core::VoteChoice::NamedOptions(options) => {
                crate::effects::VoteEffect::with_optional_extra(
                    options
                        .iter()
                        .map(|option| {
                            Ok(crate::effects::VoteOption::new(
                                option.name.to_string(),
                                convert_effects(option.effects_per_vote.iter().cloned(), hooks)?,
                            ))
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                    payload.controller_extra_votes,
                    payload.controller_optional_extra_votes,
                )
                .with_secret(payload.secret)
            }
            ironsmith_core::VoteChoice::Objects { filter, count } => {
                crate::effects::VoteEffect::vote_objects_with_optional_extra(
                    filter.clone(),
                    *count,
                    payload.controller_extra_votes,
                    payload.controller_optional_extra_votes,
                )
                .with_secret(payload.secret)
            }
            ironsmith_core::VoteChoice::Players {
                filter,
                exclude_voter,
            } => crate::effects::VoteEffect::vote_players_with_optional_extra(
                filter.clone(),
                *exclude_voter,
                payload.controller_extra_votes,
                payload.controller_optional_extra_votes,
            )
            .with_secret(payload.secret),
        }
        .starting_with_controller(payload.starting_with_controller);
        return Ok(Effect::new(converted));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::BidLifeEffect<M::Effect>>(&effect) {
        return Ok(Effect::new(crate::effects::BidLifeEffect::new(
            payload.target.clone(),
            payload.starting_bid,
            convert_effects(payload.winner_effects.iter().cloned(), hooks)?,
        )));
    }
    if let Some(converted) = hooks.runtime_external_model_effect_hook(&effect)? {
        return Ok(converted);
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::DiscardEffect>(&effect) {
        let mut converted = crate::effects::DiscardEffect::new_with_filter(
            payload.count.clone(),
            payload.player.clone(),
            payload.random,
            payload.card_filter.clone(),
        )
        .with_any_number(payload.any_number);
        if let Some(tag) = &payload.tag {
            converted = converted.with_tag(tag.clone());
        }
        return Ok(Effect::new(converted));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::InvestigateEffect>(&effect) {
        return Ok(Effect::new(crate::effects::InvestigateEffect::new(
            payload.count.clone(),
            payload.player.clone(),
        )));
    }
    if let Some(converted) =
        clone_direct_effect::<M, crate::effects::ReturnFromGraveyardToBattlefieldEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(converted) = clone_direct_effect::<
        M,
        crate::effects::ReturnFromGraveyardOrExileToBattlefieldEffect,
    >(&effect)
    {
        return Ok(converted);
    }
    if let Some(converted) =
        clone_direct_effect::<M, crate::effects::ReturnFromGraveyardToHandEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(converted) =
        clone_direct_effect::<M, crate::effects::PutOntoBattlefieldEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::ShuffleLibraryEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::MayMoveToZoneEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::ExileTopOfLibraryEffect>(&effect) {
        let mut converted = crate::effects::ExileTopOfLibraryEffect::new(
            payload.count.clone(),
            payload.player.clone(),
        );
        if let Some(surface) = payload.surface {
            converted = converted.with_surface(surface);
        }
        for tag in &payload.moved_tags {
            converted = converted.tag_moved(tag.clone());
        }
        for tag in &payload.accumulated_tags {
            converted = converted.append_tagged(tag.clone());
        }
        if payload.face_down {
            converted = converted.face_down();
        }
        return Ok(Effect::new(converted));
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::LookAtTopCardsEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(converted) =
        clone_direct_effect::<M, crate::effects::ReorderTopPlanarDeckEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::LookAtObjectsEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::ChooseCardNameEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::ControlPlayerEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(payload) =
        M::downcast_ref::<ironsmith_core::GrantEffect<M::Grantable, M::GrantDuration>>(&effect)
    {
        return Ok(Effect::new(crate::effects::GrantEffect::new(
            hooks.runtime_grantable_hook(payload.grantable.clone())?,
            payload.target.clone(),
            hooks.runtime_grant_duration_hook(payload.duration)?,
        )));
    }
    if let Some(payload) = M::downcast_ref::<
        ironsmith_core::GrantBySpecEffect<M::GrantSpec, M::GrantDuration>,
    >(&effect)
    {
        return Ok(Effect::new(crate::effects::GrantBySpecEffect::new(
            hooks.runtime_grant_spec_hook(payload.spec.clone())?,
            payload.player.clone(),
            hooks.runtime_grant_duration_hook(payload.duration)?,
        )));
    }
    if let Some(converted) =
        clone_direct_effect::<M, crate::effects::MoveAllCountersEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::MoveCountersEffect>(&effect) {
        return Ok(converted);
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::MoveOneCounterEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::ProliferateEffect>(&effect) {
        return Ok(converted);
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::MonstrosityEffect>(&effect) {
        return Ok(converted);
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::TransformEffect>(&effect) {
        return Ok(converted);
    }
    if let Some(payload) =
        M::downcast_ref::<ironsmith_core::RepeatEffectsEffect<M::Effect>>(&effect)
    {
        return Ok(Effect::new(crate::effects::RepeatEffectsEffect::new(
            payload.count.clone(),
            convert_effects(payload.effects.iter().cloned(), hooks)?,
        )));
    }
    if let Some(payload) =
        M::downcast_ref::<ironsmith_core::RepeatProcessEffect<M::Effect>>(&effect)
    {
        return Ok(Effect::new(crate::effects::RepeatProcessEffect::new(
            convert_effects(payload.effects.iter().cloned(), hooks)?,
            payload.condition,
            payload.predicate.clone(),
        )));
    }
    if let Some(payload) = M::downcast_ref::<
        ironsmith_core::GrantRepeatableManaPaymentActionUntilEndOfTurnEffect<M::Effect>,
    >(&effect)
    {
        return Ok(Effect::new(
            crate::effects::GrantRepeatableManaPaymentActionUntilEndOfTurnEffect::new(
                payload.player.clone(),
                payload.cost.clone(),
                convert_effects(payload.effects.iter().cloned(), hooks)?,
            ),
        ));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::RepeatProcessPromptEffect>(&effect) {
        return Ok(Effect::new(crate::effects::RepeatProcessPromptEffect::new(
            payload.kind,
        )));
    }
    if let Some(converted) =
        clone_direct_effect::<M, crate::effects::RearrangeLookedCardsInLibraryEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::ChoosePlayerEffect>(&effect) {
        let mut converted = crate::effects::ChoosePlayerEffect::new(
            payload.chooser.clone(),
            payload.filter.clone(),
            payload.tag.clone(),
        );
        if payload.random {
            converted = converted.at_random();
        }
        if !payload.excluded_tags.is_empty() {
            converted = converted.excluding_tags(payload.excluded_tags.clone());
        }
        if payload.remember_as_chosen_player {
            converted = converted.remember_as_chosen_player();
        }
        return Ok(Effect::new(converted));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::DealDistributedDamageEffect>(&effect) {
        return Ok(Effect::new(
            crate::effects::DealDistributedDamageEffect::new(
                payload.amount.clone(),
                payload.target.clone(),
            )
            .with_source(payload.source.clone())
            .with_chooser(payload.chooser.clone())
            .with_distribution(payload.distribution),
        ));
    }
    if let Some(payload) =
        M::downcast_ref::<ironsmith_core::PreventNextTimeDamageEffect<M::Effect>>(&effect)
    {
        let source = match &payload.source {
            ironsmith_core::PreventNextTimeDamageSource::Choice => {
                crate::effects::PreventNextTimeDamageSource::Choice
            }
            ironsmith_core::PreventNextTimeDamageSource::ChoiceMatching(filter) => {
                crate::effects::PreventNextTimeDamageSource::ChoiceMatching(filter.clone())
            }
            ironsmith_core::PreventNextTimeDamageSource::Target(spec) => {
                crate::effects::PreventNextTimeDamageSource::Target(spec.clone())
            }
            ironsmith_core::PreventNextTimeDamageSource::Filter(filter) => {
                crate::effects::PreventNextTimeDamageSource::Filter(filter.clone())
            }
        };
        let target = match &payload.target {
            ironsmith_core::PreventNextTimeDamageTarget::AnyTarget => {
                crate::effects::PreventNextTimeDamageTarget::AnyTarget
            }
            ironsmith_core::PreventNextTimeDamageTarget::Omitted => {
                crate::effects::PreventNextTimeDamageTarget::Omitted
            }
            ironsmith_core::PreventNextTimeDamageTarget::You => {
                crate::effects::PreventNextTimeDamageTarget::You
            }
            ironsmith_core::PreventNextTimeDamageTarget::Target(spec) => {
                crate::effects::PreventNextTimeDamageTarget::Target(spec.clone())
            }
        };
        let mut effect = crate::effects::PreventNextTimeDamageEffect::new(source, target);
        effect = effect.with_follow_up_effects(convert_effects(
            payload.follow_up_effects.iter().cloned(),
            hooks,
        )?);
        if payload.reflect_damage_to_source_controller {
            effect = effect.reflecting_to_source_controller();
        }
        return Ok(Effect::new(effect));
    }
    if let Some(payload) =
        M::downcast_ref::<ironsmith_core::ReplaceNextDamageToTargetEffect<M::Effect>>(&effect)
    {
        return Ok(Effect::new(
            crate::effects::ReplaceNextDamageToTargetEffect::new(
                payload.target.clone(),
                convert_effects(payload.replacement_effects.iter().cloned(), hooks)?,
            ),
        ));
    }
    if let Some(payload) =
        M::downcast_ref::<ironsmith_core::RedirectNextDamageToTargetEffect>(&effect)
    {
        let Some(amount) = payload.amount.clone() else {
            return Err(hooks.unsupported_effect(
                "redirect next damage to target without an amount".to_string(),
            ));
        };
        let effect = match payload.destination {
            ironsmith_core::RedirectNextDamageDestination::Controller => {
                let Some(protected_target) = payload.protected_target.clone() else {
                    return Err(hooks.unsupported_effect(
                        "redirect next damage to controller without a protected target".to_string(),
                    ));
                };
                crate::effects::RedirectNextDamageToTargetEffect::to_controller(
                    amount,
                    protected_target,
                )
            }
            ironsmith_core::RedirectNextDamageDestination::TargetObject => {
                let Some(destination_target) = payload.destination_target.clone() else {
                    return Err(hooks.unsupported_effect(
                        "redirect next damage to target object without a target".to_string(),
                    ));
                };
                let mut effect = crate::effects::RedirectNextDamageToTargetEffect::new(
                    amount,
                    destination_target,
                );
                effect.protected_target = payload.protected_target.clone();
                effect
            }
        };
        return Ok(Effect::new(effect));
    }
    if let Some(payload) =
        M::downcast_ref::<ironsmith_core::RedirectNextTimeDamageToSourceEffect>(&effect)
    {
        let source = match &payload.source {
            ironsmith_core::RedirectNextTimeDamageSource::Choice => {
                crate::effects::RedirectNextTimeDamageSource::Choice
            }
            ironsmith_core::RedirectNextTimeDamageSource::Filter(filter) => {
                crate::effects::RedirectNextTimeDamageSource::Filter(filter.clone())
            }
            ironsmith_core::RedirectNextTimeDamageSource::Target(spec) => {
                crate::effects::RedirectNextTimeDamageSource::Target(spec.clone())
            }
        };
        let effect = if let Some(target) = payload.target.clone() {
            crate::effects::RedirectNextTimeDamageToSourceEffect::new(source, target)
        } else {
            crate::effects::RedirectNextTimeDamageToSourceEffect {
                source,
                target: None,
                destination: crate::effects::RedirectNextTimeDamageDestination::SourceObject,
                destination_target: None,
                all_this_turn: false,
            }
        };
        let effect = match payload.destination {
            ironsmith_core::RedirectNextTimeDamageDestination::SourceObject => effect,
            ironsmith_core::RedirectNextTimeDamageDestination::Controller => effect.to_controller(),
            ironsmith_core::RedirectNextTimeDamageDestination::SourceController => {
                effect.to_source_controller()
            }
            ironsmith_core::RedirectNextTimeDamageDestination::TargetObject => {
                let Some(target) = payload.destination_target.clone() else {
                    return Err(hooks.unsupported_effect(
                        "redirect next time damage to target object without a target".to_string(),
                    ));
                };
                effect.to_target(target)
            }
        };
        let effect = if payload.all_this_turn {
            effect.all_this_turn()
        } else {
            effect
        };
        return Ok(Effect::new(effect));
    }
    if let Some(payload) =
        M::downcast_ref::<ironsmith_core::RedirectAllDamageThisTurnToTargetEffect>(&effect)
    {
        return Ok(Effect::new(
            crate::effects::RedirectAllDamageThisTurnToTargetEffect::new(
                payload.player_filter.clone(),
                payload.object_filter.clone(),
                payload.target.clone(),
            ),
        ));
    }
    if let Some(payload) =
        M::downcast_ref::<ironsmith_core::ExecuteWithSourceEffect<M::Effect>>(&effect)
    {
        return Ok(Effect::new(crate::effects::ExecuteWithSourceEffect::new(
            payload.source.clone(),
            interpret_effect_model((*payload.effect).clone(), hooks)?,
        )));
    }
    if let Some(payload) =
        M::downcast_ref::<ironsmith_core::ExileTaggedWhenSourceLeavesEffect>(&effect)
    {
        return Ok(Effect::new(
            crate::effects::ExileTaggedWhenSourceLeavesEffect::new(
                payload.tag.clone(),
                payload.controller.clone(),
            ),
        ));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::ForEachObject<M::Effect>>(&effect) {
        return Ok(Effect::new(crate::effects::ForEachObject::new(
            payload.filter.clone(),
            convert_effects(payload.effects.iter().cloned(), hooks)?,
        )));
    }
    if let Some(payload) =
        M::downcast_ref::<ironsmith_core::ForEachObjectCorrelatedResultEffect<M::Effect>>(&effect)
    {
        return Ok(Effect::new(
            crate::effects::ForEachObjectCorrelatedResultEffect::new(
                payload.filter.clone(),
                convert_effects(payload.producer_effects.iter().cloned(), hooks)?,
                payload.result_tag.clone(),
                payload.source_binding_tag.clone(),
                payload.result_binding_tag.clone(),
                convert_effects(payload.consumer_effects.iter().cloned(), hooks)?,
            ),
        ));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::GrantPlayTaggedEffect>(&effect) {
        let mut grant = crate::effects::GrantPlayTaggedEffect::new(
            payload.tag.clone(),
            payload.player.clone(),
            payload.duration,
            payload.allow_land,
            payload.mana_spend_mode,
        )
        .while_on_top_of_library_if(payload.while_on_top_of_library)
        .cast_pool_is_plural(payload.cast_pool_is_plural)
        .with_max_plays(payload.max_plays);
        if let Some(surface) = payload.surface.clone() {
            grant = grant.with_surface(surface);
        }
        if let Some(filter) = payload.filter.clone() {
            grant = grant.with_filter(filter);
        }
        if let Some(counter_type) = payload.during_turns_counter_put_on_source {
            grant = grant.during_turns_counter_put_on_source(counter_type);
        }
        if let Some(cost) = payload.spell_cost_increase.clone() {
            grant = grant.with_spell_cost_increase(cost);
        }
        grant = grant.with_lands_enter_tapped(payload.lands_enter_tapped);
        return Ok(Effect::new(grant));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::LocalRewriteEffect<M::Effect>>(&effect)
    {
        return Ok(Effect::new(crate::effects::LocalRewriteEffect::new(
            interpret_effect_model((*payload.effect).clone(), hooks)?,
            payload.zone_replacements.clone(),
        )));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::PhaseOutEffect>(&effect) {
        let mut phase_out = crate::effects::PhaseOutEffect::with_spec(payload.target.clone());
        phase_out.duration = payload.duration;
        phase_out.source_surface = payload.source_surface.clone();
        return Ok(Effect::new(phase_out));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::PhaseInEffect>(&effect) {
        return Ok(Effect::new(crate::effects::PhaseInEffect::with_spec(
            payload.target.clone(),
        )));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::RemoveFromCombatEffect>(&effect) {
        return Ok(Effect::new(
            crate::effects::RemoveFromCombatEffect::with_spec(payload.target.clone()),
        ));
    }
    if let Some(payload) =
        M::downcast_ref::<ironsmith_core::ForEachCounterKindPutOrRemoveEffect>(&effect)
    {
        let effect = if let Some(counter_type) = payload.fixed_counter_type {
            crate::effects::ForEachCounterKindPutOrRemoveEffect::fixed_counter_type(
                payload.target.clone(),
                counter_type,
                payload.optional_action,
            )
        } else if payload.all_kinds {
            crate::effects::ForEachCounterKindPutOrRemoveEffect::new(payload.target.clone())
        } else {
            crate::effects::ForEachCounterKindPutOrRemoveEffect::one_kind(payload.target.clone())
        };
        return Ok(Effect::new(effect));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::PutCounterOfChosenKindEffect>(&effect)
    {
        return Ok(Effect::new(
            crate::effects::PutCounterOfChosenKindEffect::new(payload.target.clone()),
        ));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::DoubleCountersEffect>(&effect) {
        return Ok(Effect::new(crate::effects::DoubleCountersEffect::new(
            payload.counter_type,
            payload.target.clone(),
        )));
    }
    if let Some(payload) =
        M::downcast_ref::<ironsmith_core::ReflexiveTriggerEffect<M::Effect>>(&effect)
    {
        return Ok(Effect::new(crate::effects::ReflexiveTriggerEffect::new(
            payload.condition,
            payload.predicate.clone(),
            convert_effects(payload.effects.iter().cloned(), hooks)?,
            payload.choices.clone(),
        )));
    }
    if let Some(payload) =
        M::downcast_ref::<ironsmith_core::ScheduleEffectsWhenTaggedLeavesEffect<M::Effect>>(&effect)
    {
        let mut converted = crate::effects::ScheduleEffectsWhenTaggedLeavesEffect::new(
            payload.tag.clone(),
            convert_effects(payload.effects.iter().cloned(), hooks)?,
            payload.controller.clone(),
        );
        if matches!(
            payload.ability_source,
            ironsmith_core::TaggedLeavesAbilitySource::CurrentSource
        ) {
            converted = converted.with_current_source_as_ability_source();
        }
        return Ok(Effect::new(converted));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::UnlessActionEffect<M::Effect>>(&effect)
    {
        return Ok(Effect::new(crate::effects::UnlessActionEffect::new(
            convert_effects(payload.effects.iter().cloned(), hooks)?,
            convert_effects(payload.alternative.iter().cloned(), hooks)?,
            payload.player.clone(),
        )));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::UnlessPaysEffect<M::Effect>>(&effect) {
        let effects = convert_effects(payload.effects.iter().cloned(), hooks)?;
        let cost = interpret_core_total_cost_model::<M, H>(payload.cost.clone(), hooks)?;
        return Ok(Effect::new(
            crate::effects::UnlessPaysEffect::new_total_cost(effects, payload.player.clone(), cost)
                .with_leading_surface(payload.leading_surface)
                .before_delayed_step(payload.before_delayed_step),
        ));
    }
    if let Some(payload) =
        M::downcast_ref::<ironsmith_core::CumulativeUpkeepEffect<M::Effect>>(&effect)
    {
        return Ok(Effect::new(crate::effects::CumulativeUpkeepEffect::new(
            payload.player.clone(),
            convert_effects(payload.payment.iter().cloned(), hooks)?,
            convert_effects(payload.failure.iter().cloned(), hooks)?,
        )));
    }
    if let Some(converted) =
        clone_direct_effect::<M, crate::effects::ChooseNewTargetsEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::RegisterZoneReplacementEffect>(&effect)
    {
        let mut converted = crate::effects::RegisterZoneReplacementEffect::new(
            payload.target.clone(),
            payload.from_zone,
            payload.to_zone,
            payload.replacement_zone,
            payload.mode,
        )
        .with_counters(payload.counters.clone());
        if let Some(placement) = payload.library_placement {
            converted = converted.with_library_placement(placement);
        }
        if let Some(follow_up) = payload.linked_exile_follow_up {
            converted = converted.with_linked_exile_follow_up(follow_up);
        }
        if payload.optional {
            converted.optional = true;
            converted.choice_description = payload.choice_description.clone();
        }
        return Ok(Effect::new(converted));
    }
    if let Some(converted) =
        clone_direct_effect::<M, crate::effects::RegisterFutureZoneReplacementEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(payload) =
        M::downcast_ref::<ironsmith_core::RegisterDrawReplacementEffect<M::Effect>>(&effect)
    {
        return Ok(Effect::new(
            crate::effects::RegisterDrawReplacementEffect::new(
                payload.player.clone(),
                convert_effects(payload.replacement_effects.iter().cloned(), hooks)?,
                payload.mode,
            ),
        ));
    }
    if let Some(converted) = clone_direct_effect::<
        M,
        crate::effects::RegisterDamagedBySourceZoneReplacementEffect,
    >(&effect)
    {
        return Ok(converted);
    }
    if let Some(converted) = clone_direct_effect::<
        M,
        crate::effects::RegisterEnterUnderControlReplacementEffect,
    >(&effect)
    {
        return Ok(converted);
    }
    if let Some(converted) =
        clone_direct_effect::<M, crate::effects::RegisterEnterTappedReplacementEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(converted) =
        clone_direct_effect::<M, crate::effects::RegisterNextBatchEnterWithCountersEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(converted) =
        clone_direct_effect::<M, crate::effects::RegisterManaReplacementEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(converted) =
        clone_direct_effect::<M, crate::effects::ExileInsteadOfGraveyardEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::WinTheGameEffect>(&effect) {
        return Ok(converted);
    }
    if let Some(converted) =
        clone_direct_effect::<M, crate::effects::ConsultTopOfLibraryEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::ExploreEffect>(&effect) {
        return Ok(Effect::new(crate::effects::ExploreEffect::new(
            payload.target.clone(),
        )));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::RemoveUpToCountersEffect>(&effect) {
        return Ok(Effect::new(crate::effects::RemoveUpToCountersEffect::new(
            payload.counter_type,
            payload.max_count.clone(),
            payload.target.clone(),
        )));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::OpenAttractionEffect>(&effect) {
        return Ok(Effect::new(
            crate::effects::OpenAttractionEffect::new().with_reminder(payload.reminder),
        ));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::AdaptEffect>(&effect) {
        return Ok(Effect::new(crate::effects::AdaptEffect::new(
            payload.amount,
        )));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::BackupEffect<M::Ability>>(&effect) {
        return Ok(Effect::new(crate::effects::BackupEffect::new(
            payload.amount,
            payload
                .granted_abilities
                .iter()
                .cloned()
                .map(|ability| hooks.runtime_ability_hook(ability))
                .collect::<Result<Vec<_>, _>>()?,
        )));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::BeholdEffect>(&effect) {
        return Ok(Effect::new(crate::effects::BeholdEffect::new(
            payload.subtype,
            payload.count,
            payload.chooser.clone(),
        )));
    }
    if M::downcast_ref::<ironsmith_core::ManifestDreadEffect>(&effect).is_some() {
        return Ok(Effect::new(crate::effects::ManifestDreadEffect::new()));
    }
    if let Some(payload) =
        M::downcast_ref::<ironsmith_core::ManifestTopCardOfLibraryEffect>(&effect)
    {
        let runtime = if payload.cloak {
            crate::effects::ManifestTopCardOfLibraryEffect::cloak(payload.player.clone())
        } else {
            crate::effects::ManifestTopCardOfLibraryEffect::new(payload.player.clone())
        };
        return Ok(Effect::new(runtime));
    }
    if M::downcast_ref::<ironsmith_core::ManifestCardFromHandEffect>(&effect).is_some() {
        return Ok(Effect::new(
            crate::effects::ManifestCardFromHandEffect::new(),
        ));
    }
    if let Some(converted) =
        clone_direct_effect::<M, crate::effects::ManifestObjectsEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::SupportEffect>(&effect) {
        return Ok(Effect::new(crate::effects::SupportEffect::new(
            payload.amount,
        )));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::ConniveEffect>(&effect) {
        return Ok(Effect::new(crate::effects::ConniveEffect::new_with_count(
            payload.target.clone(),
            payload.count.clone(),
        )));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::DetainEffect>(&effect) {
        return Ok(Effect::new(crate::effects::DetainEffect::new(
            payload.target.clone(),
        )));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::GoadEffect>(&effect) {
        return Ok(Effect::new(crate::effects::GoadEffect::with_duration(
            payload.target.clone(),
            payload.duration.clone(),
        )));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::SuspectEffect>(&effect) {
        return Ok(Effect::new(crate::effects::SuspectEffect::new(
            payload.target.clone(),
        )));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::ClearSuspectedEffect>(&effect) {
        return Ok(Effect::new(crate::effects::ClearSuspectedEffect {
            target: payload.target.clone(),
        }));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::ChooseColorEffect>(&effect) {
        return Ok(Effect::new(crate::effects::ChooseColorEffect::new(
            payload.chooser.clone(),
        )));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::ChooseLandTypeEffect>(&effect) {
        return Ok(Effect::new(crate::effects::ChooseLandTypeEffect::new(
            payload.chooser.clone(),
            payload.exclude_basic,
        )));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::ChooseCreatureTypeEffect>(&effect) {
        let mut runtime = crate::effects::ChooseCreatureTypeEffect::for_family(
            payload.chooser.clone(),
            payload.family,
        );
        runtime.excluded_subtypes = payload.excluded_subtypes.clone();
        return Ok(Effect::new(runtime));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::FlipCoinEffect>(&effect) {
        let mut runtime = match payload.kind {
            ironsmith_core::CoinFlipKind::Called => {
                crate::effects::FlipCoinEffect::new(payload.player.clone())
            }
            ironsmith_core::CoinFlipKind::FaceOnly => {
                crate::effects::FlipCoinEffect::face_only(payload.player.clone())
            }
        };
        runtime.forced_face = payload.forced_face;
        runtime.forced_winner = payload.forced_winner.clone();
        runtime.forced_loser = payload.forced_loser.clone();
        return Ok(Effect::new(runtime));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::RollDieEffect>(&effect) {
        return Ok(Effect::new(
            crate::effects::RollDieEffect::new_with_die_text(
                payload.player.clone(),
                payload.sides,
                payload.die_text.clone(),
            ),
        ));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::RollDiceChooseResultEffect>(&effect) {
        return Ok(Effect::new(
            crate::effects::RollDiceChooseResultEffect::new_with_die_text(
                payload.player.clone(),
                payload.count,
                payload.sides,
                payload.die_text.clone(),
            ),
        ));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::EmitGiftGivenEffect>(&effect) {
        return Ok(Effect::new(crate::effects::EmitGiftGivenEffect::new(
            payload.recipient.clone(),
        )));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::ChooseNamedOptionEffect>(&effect) {
        return Ok(Effect::new(crate::effects::ChooseNamedOptionEffect::new(
            payload.chooser.clone(),
            payload.options.clone(),
        )));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::SecretChoiceEffect>(&effect) {
        let secret_choice = if let Some(object_choice) = &payload.object_choice {
            crate::effects::SecretChoiceEffect::new_objects(
                payload.participants.clone(),
                object_choice.clone(),
            )
        } else {
            crate::effects::SecretChoiceEffect::new(
                payload.options.clone(),
                payload.participants.clone(),
            )
        };
        return Ok(Effect::new(secret_choice));
    }
    if let Some(converted) =
        clone_direct_effect::<M, crate::effects::DirectionalAdjacentPlayerControlEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::SetLifeTotalEffect>(&effect) {
        return Ok(Effect::new(crate::effects::SetLifeTotalEffect::new(
            payload.amount.clone(),
            payload.player.clone(),
        )));
    }
    if M::downcast_ref::<ironsmith_core::NoteLifeTotalEffect>(&effect).is_some() {
        return Ok(Effect::note_life_total());
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::IncreaseSpeedEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::ReduceSpeedEffect>(&effect) {
        return Ok(converted);
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::ExchangeLifeTotalsEffect>(&effect) {
        return Ok(Effect::new(crate::effects::ExchangeLifeTotalsEffect::new(
            payload.player1.clone(),
            payload.player2.clone(),
        )));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::DoubleManaPoolEffect>(&effect) {
        return Ok(Effect::new(crate::effects::DoubleManaPoolEffect::new(
            payload.player.clone(),
        )));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::EmptyManaPoolEffect>(&effect) {
        return Ok(Effect::new(crate::effects::EmptyManaPoolEffect::new(
            payload.player.clone(),
        )));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::EndTurnEffect>(&effect) {
        return Ok(Effect::new(crate::effects::EndTurnEffect::new(
            payload.player.clone(),
        )));
    }
    if M::downcast_ref::<ironsmith_core::EndCombatPhaseEffect>(&effect).is_some() {
        return Ok(Effect::end_combat_phase());
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::SkipTurnEffect>(&effect) {
        return Ok(Effect::new(crate::effects::SkipTurnEffect::new(
            payload.player.clone(),
        )));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::SkipDrawStepEffect>(&effect) {
        return Ok(Effect::new(crate::effects::SkipDrawStepEffect::new(
            payload.player.clone(),
        )));
    }
    if let Some(payload) =
        M::downcast_ref::<ironsmith_core::SkipNextCombatPhaseThisTurnEffect>(&effect)
    {
        return Ok(Effect::new(
            crate::effects::SkipNextCombatPhaseThisTurnEffect::new(payload.player.clone()),
        ));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::SkipMainPhasesThisTurnEffect>(&effect)
    {
        return Ok(Effect::new(
            crate::effects::SkipMainPhasesThisTurnEffect::new(payload.player.clone()),
        ));
    }
    if let Some(payload) =
        M::downcast_ref::<ironsmith_core::SkipCombatPhasesThisTurnEffect>(&effect)
    {
        return Ok(Effect::new(
            crate::effects::SkipCombatPhasesThisTurnEffect::new(payload.player.clone()),
        ));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::SkipCombatPhasesEffect>(&effect) {
        return Ok(Effect::new(crate::effects::SkipCombatPhasesEffect::new(
            payload.player.clone(),
        )));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::AdditionalLandPlaysEffect>(&effect) {
        return Ok(Effect::new(crate::effects::AdditionalLandPlaysEffect::new(
            payload.count.clone(),
            payload.player.clone(),
            payload.duration.clone(),
        )));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::BecomeMonarchEffect>(&effect) {
        return Ok(Effect::new(crate::effects::BecomeMonarchEffect::new(
            payload.player.clone(),
        )));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::RingTemptsYouEffect>(&effect) {
        return Ok(Effect::new(crate::effects::RingTemptsYouEffect::new(
            payload.player.clone(),
        )));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::VentureIntoDungeonEffect>(&effect) {
        let converted = if payload.undercity_if_no_active {
            crate::effects::VentureIntoDungeonEffect::via_initiative(payload.player.clone())
        } else {
            crate::effects::VentureIntoDungeonEffect::new(payload.player.clone())
        };
        return Ok(Effect::new(converted));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::TakeInitiativeEffect>(&effect) {
        return Ok(Effect::new(crate::effects::TakeInitiativeEffect::new(
            payload.player.clone(),
        )));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::PoisonCountersEffect>(&effect) {
        return Ok(Effect::new(crate::effects::PoisonCountersEffect::new(
            payload.count.clone(),
            payload.player.clone(),
        )));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::ExperienceCountersEffect>(&effect) {
        return Ok(Effect::new(crate::effects::ExperienceCountersEffect::new(
            payload.count.clone(),
            payload.player.clone(),
        )));
    }
    if let Some(payload) =
        M::downcast_ref::<ironsmith_core::ControlCombatChoicesThisTurnEffect>(&effect)
    {
        return Ok(Effect::new(
            crate::effects::ControlCombatChoicesThisTurnEffect::new_with_surface(
                payload.attackers,
                payload.blockers,
                payload.this_combat,
            ),
        ));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::ExchangeZonesEffect>(&effect) {
        return Ok(Effect::new(crate::effects::ExchangeZonesEffect::new(
            payload.player.clone(),
            payload.zone1,
            payload.zone2,
        )));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::ExchangeValuesEffect>(&effect) {
        return Ok(Effect::new(crate::effects::ExchangeValuesEffect::new(
            convert_exchange_value_operand(&payload.left),
            convert_exchange_value_operand(&payload.right),
            payload.duration.clone(),
        )));
    }
    if let Some(payload) =
        M::downcast_ref::<ironsmith_core::AddManaOfLandProducedTypesEffect>(&effect)
    {
        let runtime = match payload.mana_type_source {
            ironsmith_core::ManaTypeSource::MatchingLandsCouldProduce => {
                crate::effects::AddManaOfLandProducedTypesEffect::new(
                    payload.amount.clone(),
                    payload.player.clone(),
                    payload.land_filter.clone(),
                    payload.allow_colorless,
                    payload.same_type,
                )
            }
            ironsmith_core::ManaTypeSource::TriggeringEventProduced => {
                crate::effects::AddManaOfLandProducedTypesEffect::from_triggering_event(
                    payload.amount.clone(),
                    payload.player.clone(),
                    payload.land_filter.clone(),
                    payload.allow_colorless,
                    payload.same_type,
                )
            }
        };
        return Ok(Effect::new(runtime));
    }
    if let Some(payload) =
        M::downcast_ref::<ironsmith_core::TagTriggeringDamageTargetEffect>(&effect)
    {
        return Ok(Effect::new(
            crate::effects::TagTriggeringDamageTargetEffect::new(payload.tag.clone()),
        ));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::ConvertEffect>(&effect) {
        return Ok(Effect::new(crate::effects::ConvertEffect::new(
            payload.target.clone(),
        )));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::PutStickerEffect>(&effect) {
        return Ok(Effect::new(crate::effects::PutStickerEffect::new(
            payload.target.clone(),
            payload.action,
        )));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::UnlockRoomDoorEffect>(&effect) {
        return Ok(Effect::new(crate::effects::UnlockRoomDoorEffect::new(
            payload.player.clone(),
            payload.room_filter.clone(),
        )));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::FatesealEffect>(&effect) {
        return Ok(Effect::new(crate::effects::FatesealEffect::new(
            payload.count.clone(),
            payload.player.clone(),
        )));
    }
    if let Some(payload) =
        M::downcast_ref::<ironsmith_core::ShuffleHandAndGraveyardIntoLibraryEffect>(&effect)
    {
        let runtime = if payload.include_owned_permanents {
            crate::effects::ShuffleHandAndGraveyardIntoLibraryEffect::including_owned_permanents(
                payload.player.clone(),
            )
        } else {
            crate::effects::ShuffleHandAndGraveyardIntoLibraryEffect::new(payload.player.clone())
        };
        return Ok(Effect::new(runtime));
    }
    if let Some(payload) =
        M::downcast_ref::<ironsmith_core::ShuffleGraveyardIntoLibraryEffect>(&effect)
    {
        let runtime = if payload.explicit_all_cards_from {
            crate::effects::ShuffleGraveyardIntoLibraryEffect::with_all_cards_from_surface(
                payload.player.clone(),
            )
        } else {
            crate::effects::ShuffleGraveyardIntoLibraryEffect::new(payload.player.clone())
        };
        return Ok(Effect::new(runtime));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::ReorderGraveyardEffect>(&effect) {
        return Ok(Effect::new(crate::effects::ReorderGraveyardEffect::new(
            payload.player.clone(),
        )));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::SurveilEffect>(&effect) {
        return Ok(Effect::new(crate::effects::SurveilEffect::new(
            payload.count.clone(),
            payload.player.clone(),
        )));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::FightEffect>(&effect) {
        return Ok(Effect::new(crate::effects::FightEffect::new(
            payload.creature1.clone(),
            payload.creature2.clone(),
        )));
    }
    if M::downcast_ref::<ironsmith_core::BecomeSaddledUntilEotEffect>(&effect).is_some() {
        return Ok(Effect::new(
            crate::effects::BecomeSaddledUntilEotEffect::new(),
        ));
    }

    macro_rules! clone_direct {
        ($($ty:path),* $(,)?) => {
            $(
                if let Some(converted) = clone_direct_effect::<M, $ty>(&effect) {
                    return Ok(converted);
                }
            )*
        };
    }

    clone_direct!(
        crate::effects::AmassEffect,
        crate::effects::AmplifyEffect,
        crate::effects::DevourEffect,
        crate::effects::AuraSwapEffect,
        crate::effects::IncubateEffect,
        crate::effects::LearnEffect,
        crate::effects::BecomeBasicLandTypeChoiceEffect,
        crate::effects::BecomeColorChoiceEffect,
        crate::effects::BecomeCreatureTypeChoiceEffect,
        crate::effects::ChooseObjectsEffect,
        crate::effects::ChooseSpellCastHistoryEffect,
        crate::effects::ClashEffect,
        crate::effects::CopySpellEffect,
        crate::effects::CopySpellForEachTargetEffect,
        crate::effects::CounterEffect,
        crate::effects::CrewCostEffect,
        crate::effects::DealDamageEffect,
        crate::effects::HealDamageEffect,
        crate::effects::DrawCardsEffect,
        crate::effects::EachPlayerScryEffect,
        crate::effects::EarthbendEffect,
        crate::effects::EvolveEffect,
        crate::effects::ExchangeControlEffect,
        crate::effects::ExertCostEffect,
        crate::effects::ExileEffect,
        crate::effects::ExileUntilEffect,
        crate::effects::GrantNextSpellCostReductionEffect,
        crate::effects::GrantTaggedSpellFreeCastUntilEndOfTurnEffect,
        crate::effects::GrantTaggedSpellLifeCostByManaValueEffect,
        crate::effects::MayCastMatchingSpellWithoutPayingManaCostEffect,
        crate::effects::LookAtHandEffect,
        crate::effects::MeldEffect,
        crate::effects::MillEffect,
        crate::effects::ModifyPowerToughnessEffect,
        crate::effects::MoveCountersEffect,
        crate::effects::MoveToLibraryNthFromTopEffect,
        crate::effects::MoveToLibraryTopOrBottomChoiceEffect,
        crate::effects::MoveToZoneEffect,
        crate::effects::PayAnyEnergyEffect,
        crate::effects::PayAnyLifeEffect,
        crate::effects::PayEnergyEffect,
        crate::effects::PayLifeEffect,
        crate::effects::PayManaEffect,
        crate::effects::PopulateEffect,
        crate::effects::PutCountersEffect,
        crate::effects::RemoveCountersEffect,
        crate::effects::ReorderLibraryTopEffect,
        crate::effects::RetainManaUntilEndOfTurnEffect,
        crate::effects::TurnFaceUpEffect,
        crate::effects::RetargetStackObjectEffect,
        crate::effects::ReturnAllToBattlefieldEffect,
        crate::effects::ReturnToHandEffect,
        crate::effects::RevealFromHandEffect,
        crate::effects::RevealSourceFromHandEffect,
        crate::effects::RevealTaggedEffect,
        crate::effects::SacrificeTargetEffect,
        crate::effects::ScryEffect,
        crate::effects::SearchLibraryEffect,
        crate::effects::SearchLibrarySlotsEffect,
        crate::effects::SoulbondPairEffect,
        crate::effects::TagMatchingObjectsEffect,
        crate::effects::TagOtherBlockParticipantEffect,
        crate::effects::TargetOnlyEffect,
        crate::effects::TapEffect,
        crate::effects::UntapEffect,
        crate::effects::VariableCasualtyPlaneswalkerCopyEffect,
        crate::effects::AddManaOfChosenColorEffect,
        crate::effects::AddManaOfColorsAmongEffect,
        crate::effects::AddOneManaOfAnyColorAmongEffect,
        crate::effects::mana::AddManaOfImprintedColorsEffect,
        crate::effects::AddScaledManaEffect,
    );
    Err(hooks.unsupported_effect(format!(
        "no runtime conversion registered for compiler effect payload `{}`",
        M::payload_type_name(&effect)
    )))
}

pub fn prune_redundant_target_only_effects_in_program(
    program: &mut crate::resolution::ResolutionProgram,
) {
    let mut segments = std::mem::take(&mut program.segments);
    for segment in &mut segments {
        segment.default_effects =
            remove_redundant_target_only_effects(std::mem::take(&mut segment.default_effects));
        for branch in &mut segment.self_replacements {
            branch.replacement_effects = remove_redundant_target_only_effects(std::mem::take(
                &mut branch.replacement_effects,
            ));
        }
    }
    *program = crate::resolution::ResolutionProgram::new(segments);
}

fn convert_effect_mode<M, H>(
    mode: ironsmith_core::EffectMode<M::Effect>,
    hooks: &mut H,
) -> Result<EffectMode, H::Error>
where
    M: EffectModel,
    H: EffectModelInterpreterHooks<M>,
{
    let effects = convert_effects(mode.effects.into_iter(), hooks)?;
    Ok(EffectMode::new(
        mode.source_text,
        remove_redundant_target_only_effects(effects),
    ))
}

fn convert_effects<M, H>(
    effects: impl IntoIterator<Item = M::Effect>,
    hooks: &mut H,
) -> Result<Vec<Effect>, H::Error>
where
    M: EffectModel,
    H: EffectModelInterpreterHooks<M>,
{
    effects
        .into_iter()
        .map(|effect| interpret_effect_model(effect, hooks))
        .collect()
}

fn clone_direct_effect<M, T>(effect: &M::Effect) -> Option<Effect>
where
    M: EffectModel,
    T: Clone + EffectExecutor + 'static,
{
    M::downcast_ref::<T>(effect).map(|payload| Effect::new(payload.clone()))
}

fn convert_exchange_value_operand(
    operand: &ironsmith_core::ExchangeValueOperand,
) -> crate::effects::ExchangeValueOperand {
    match operand {
        ironsmith_core::ExchangeValueOperand::LifeTotal(player) => {
            crate::effects::ExchangeValueOperand::LifeTotal(player.clone())
        }
        ironsmith_core::ExchangeValueOperand::Power(target) => {
            crate::effects::ExchangeValueOperand::Power(target.clone())
        }
        ironsmith_core::ExchangeValueOperand::Toughness(target) => {
            crate::effects::ExchangeValueOperand::Toughness(target.clone())
        }
    }
}

fn remove_redundant_target_only_effects(effects: Vec<Effect>) -> Vec<Effect> {
    let mut out = Vec::with_capacity(effects.len());

    for (idx, effect) in effects.iter().enumerate() {
        let Some(target_only) = effect.downcast_ref::<crate::effects::TargetOnlyEffect>() else {
            out.push(effect.clone());
            continue;
        };

        let duplicated_by_later_effect = effects[idx + 1..].iter().any(|later| {
            later
                .0
                .get_target_spec()
                .is_some_and(|spec| spec == &target_only.target)
        });
        if !duplicated_by_later_effect {
            out.push(effect.clone());
        }
    }

    out
}
