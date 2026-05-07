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
    fn payload_type_name(effect: &Self::Effect) -> &'static str;
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
        } => ironsmith_core::Cost::RemoveAnyCountersFromSource {
            counter_type,
            display_x,
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
        converted.min_choose_count = payload.min_choose_count.clone();
        converted.allow_repeated_modes = payload.allow_repeated_modes;
        converted.disallow_previously_chosen_modes = payload.disallow_previously_chosen_modes;
        converted.disallow_previously_chosen_modes_this_turn =
            payload.disallow_previously_chosen_modes_this_turn;
        return Ok(Effect::new(converted));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::ConditionalEffect<M::Effect>>(&effect)
    {
        return Ok(Effect::conditional(
            payload.condition.clone(),
            convert_effects(payload.if_true.iter().cloned(), hooks)?,
            convert_effects(payload.if_false.iter().cloned(), hooks)?,
        ));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::IfEffect<M::Effect>>(&effect) {
        return Ok(Effect::new(crate::effects::IfEffect::new(
            payload.condition,
            payload.predicate.clone(),
            convert_effects(payload.then.iter().cloned(), hooks)?,
            convert_effects(payload.else_.iter().cloned(), hooks)?,
        )));
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
        if payload.enters_tapped {
            converted = converted.tapped();
        }
        if payload.enters_attacking {
            converted = converted.attacking();
        }
        if payload.suppress_aura_attachment_choice {
            converted = converted.suppress_aura_attachment_choice();
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
        if payload.start_next_turn {
            converted = converted.starting_next_turn();
        }
        if payload.until_end_of_turn {
            converted = converted.until_end_of_turn();
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
            enters_attacking: payload.enters_attacking,
            attack_target_mode: payload.attack_target_mode.clone(),
            exile_at_end_of_combat: payload.exile_at_end_of_combat,
            sacrifice_at_next_end_step: payload.sacrifice_at_next_end_step,
            exile_at_next_end_step: payload.exile_at_next_end_step,
            pt_adjustment: payload.pt_adjustment.clone(),
            added_card_types: payload.added_card_types.clone(),
            added_subtypes: payload.added_subtypes.clone(),
            removed_supertypes: payload.removed_supertypes.clone(),
            set_base_power_toughness: payload.set_base_power_toughness,
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
        M::downcast_ref::<ironsmith_core::GrantNextSpellAbilityEffect<M::StaticAbility>>(&effect)
    {
        return Ok(Effect::new(
            crate::effects::GrantNextSpellAbilityEffect::new(
                payload.player.clone(),
                payload.filter.clone(),
                hooks.runtime_static_ability_hook(payload.ability.clone())?,
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
        };
        return Ok(Effect::new(converted));
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::RegenerateEffect>(&effect) {
        return Ok(converted);
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
    if let Some(converted) = clone_direct_effect::<M, crate::effects::AddManaEffect>(&effect) {
        return Ok(converted);
    }
    if let Some(converted) =
        clone_direct_effect::<M, crate::effects::PreventAllCombatDamageEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(converted) =
        clone_direct_effect::<M, crate::effects::EmitKeywordActionEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::RenownEffect>(&effect) {
        return Ok(converted);
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
        return Ok(Effect::new(
            crate::effects::PreventDamageEffect::new(
                payload.amount.clone(),
                payload.target.clone(),
                payload.until.clone(),
            )
            .with_follow_up_effects(convert_effects(
                payload.follow_up_effects.iter().cloned(),
                hooks,
            )?),
        ));
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
    if let Some(converted) = clone_direct_effect::<M, crate::effects::AttachToEffect>(&effect) {
        return Ok(converted);
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::AttachObjectsEffect>(&effect)
    {
        return Ok(converted);
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::SequenceEffect<M::Effect>>(&effect) {
        return Ok(Effect::new(crate::effects::SequenceEffect::new(
            convert_effects(payload.effects.iter().cloned(), hooks)?,
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
        return Ok(Effect::new(crate::effects::ForPlayersEffect::new(
            payload.filter.clone(),
            convert_effects(payload.effects.iter().cloned(), hooks)?,
        )));
    }
    if let Some(payload) =
        M::downcast_ref::<ironsmith_core::ForEachTaggedEffect<M::Effect>>(&effect)
    {
        return Ok(Effect::new(crate::effects::ForEachTaggedEffect::new(
            payload.tag.clone(),
            convert_effects(payload.effects.iter().cloned(), hooks)?,
        )));
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
                                option.name.clone(),
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
        };
        return Ok(Effect::new(converted));
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
        for tag in &payload.moved_tags {
            converted = converted.tag_moved(tag.clone());
        }
        for tag in &payload.accumulated_tags {
            converted = converted.append_tagged(tag.clone());
        }
        return Ok(Effect::new(converted));
    }
    if let Some(converted) = clone_direct_effect::<M, crate::effects::LookAtTopCardsEffect>(&effect)
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
    if let Some(payload) = M::downcast_ref::<ironsmith_core::RepeatProcessPromptEffect>(&effect) {
        return Ok(Effect::new(crate::effects::RepeatProcessPromptEffect::new(
            payload.text.clone(),
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
            ),
        ));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::PreventNextTimeDamageEffect>(&effect) {
        let source = match &payload.source {
            ironsmith_core::PreventNextTimeDamageSource::Choice => {
                crate::effects::PreventNextTimeDamageSource::Choice
            }
            ironsmith_core::PreventNextTimeDamageSource::Filter(filter) => {
                crate::effects::PreventNextTimeDamageSource::Filter(filter.clone())
            }
        };
        let target = match payload.target {
            ironsmith_core::PreventNextTimeDamageTarget::AnyTarget => {
                crate::effects::PreventNextTimeDamageTarget::AnyTarget
            }
            ironsmith_core::PreventNextTimeDamageTarget::You => {
                crate::effects::PreventNextTimeDamageTarget::You
            }
        };
        return Ok(Effect::new(
            crate::effects::PreventNextTimeDamageEffect::new(source, target),
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
        return Ok(Effect::new(
            crate::effects::RedirectNextDamageToTargetEffect::new(amount, payload.target.clone()),
        ));
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
        };
        let Some(target) = payload.target.clone() else {
            return Err(hooks.unsupported_effect(
                "redirect next time damage to source without a protected target".to_string(),
            ));
        };
        return Ok(Effect::new(
            crate::effects::RedirectNextTimeDamageToSourceEffect::new(source, target),
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
    if let Some(payload) = M::downcast_ref::<ironsmith_core::GrantPlayTaggedEffect>(&effect) {
        return Ok(Effect::new(crate::effects::GrantPlayTaggedEffect::new(
            payload.tag.clone(),
            payload.player.clone(),
            payload.duration,
            payload.allow_land,
            payload.allow_any_color_for_cast,
        )));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::LocalRewriteEffect<M::Effect>>(&effect)
    {
        return Ok(Effect::new(crate::effects::LocalRewriteEffect::new(
            interpret_effect_model((*payload.effect).clone(), hooks)?,
            payload.zone_replacements.clone(),
        )));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::PhaseOutEffect>(&effect) {
        return Ok(Effect::new(crate::effects::PhaseOutEffect::with_spec(
            payload.target.clone(),
        )));
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
        return Ok(Effect::new(
            crate::effects::ForEachCounterKindPutOrRemoveEffect::new(payload.target.clone()),
        ));
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
            crate::effects::UnlessPaysEffect::new_total_cost(effects, payload.player.clone(), cost),
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
        );
        if payload.optional {
            converted.optional = true;
            converted.choice_description = payload.choice_description.clone();
        }
        return Ok(Effect::new(converted));
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
    if M::downcast_ref::<ironsmith_core::OpenAttractionEffect>(&effect).is_some() {
        return Ok(Effect::new(crate::effects::OpenAttractionEffect::new()));
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
        return Ok(Effect::new(
            crate::effects::ManifestTopCardOfLibraryEffect::new(payload.player.clone()),
        ));
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
        return Ok(Effect::new(crate::effects::GoadEffect::new(
            payload.target.clone(),
        )));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::ChooseColorEffect>(&effect) {
        return Ok(Effect::new(crate::effects::ChooseColorEffect::new(
            payload.chooser.clone(),
        )));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::ChooseCreatureTypeEffect>(&effect) {
        return Ok(Effect::new(crate::effects::ChooseCreatureTypeEffect::new(
            payload.chooser.clone(),
            payload.excluded_subtypes.clone(),
        )));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::FlipCoinEffect>(&effect) {
        return Ok(Effect::new(crate::effects::FlipCoinEffect::new(
            payload.player.clone(),
        )));
    }
    if let Some(payload) = M::downcast_ref::<ironsmith_core::RollDieEffect>(&effect) {
        return Ok(Effect::new(crate::effects::RollDieEffect::new(
            payload.player.clone(),
            payload.sides,
        )));
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
    if let Some(payload) = M::downcast_ref::<ironsmith_core::SetLifeTotalEffect>(&effect) {
        return Ok(Effect::new(crate::effects::SetLifeTotalEffect::new(
            payload.amount.clone(),
            payload.player.clone(),
        )));
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
    if let Some(payload) =
        M::downcast_ref::<ironsmith_core::ControlCombatChoicesThisTurnEffect>(&effect)
    {
        return Ok(Effect::new(
            crate::effects::ControlCombatChoicesThisTurnEffect::new(
                payload.attackers,
                payload.blockers,
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
        return Ok(Effect::new(
            crate::effects::AddManaOfLandProducedTypesEffect::new(
                payload.amount.clone(),
                payload.player.clone(),
                payload.land_filter.clone(),
                payload.allow_colorless,
                payload.same_type,
            ),
        ));
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
    if let Some(payload) = M::downcast_ref::<ironsmith_core::FatesealEffect>(&effect) {
        return Ok(Effect::new(crate::effects::FatesealEffect::new(
            payload.count.clone(),
            payload.player.clone(),
        )));
    }
    if let Some(payload) =
        M::downcast_ref::<ironsmith_core::ShuffleHandAndGraveyardIntoLibraryEffect>(&effect)
    {
        return Ok(Effect::new(
            crate::effects::ShuffleHandAndGraveyardIntoLibraryEffect::new(payload.player.clone()),
        ));
    }
    if let Some(payload) =
        M::downcast_ref::<ironsmith_core::ShuffleGraveyardIntoLibraryEffect>(&effect)
    {
        return Ok(Effect::new(
            crate::effects::ShuffleGraveyardIntoLibraryEffect::new(payload.player.clone()),
        ));
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
        crate::effects::IncubateEffect,
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
        crate::effects::LookAtHandEffect,
        crate::effects::MeldEffect,
        crate::effects::MillEffect,
        crate::effects::ModifyPowerToughnessEffect,
        crate::effects::MoveCountersEffect,
        crate::effects::MoveToLibraryNthFromTopEffect,
        crate::effects::MoveToLibraryTopOrBottomChoiceEffect,
        crate::effects::MoveToZoneEffect,
        crate::effects::PayAnyEnergyEffect,
        crate::effects::PayEnergyEffect,
        crate::effects::PayManaEffect,
        crate::effects::PopulateEffect,
        crate::effects::PutCountersEffect,
        crate::effects::RemoveCountersEffect,
        crate::effects::ReorderLibraryTopEffect,
        crate::effects::RetainManaUntilEndOfTurnEffect,
        crate::effects::RetargetStackObjectEffect,
        crate::effects::ReturnAllToBattlefieldEffect,
        crate::effects::ReturnToHandEffect,
        crate::effects::RevealTaggedEffect,
        crate::effects::SacrificeTargetEffect,
        crate::effects::ScryEffect,
        crate::effects::SearchLibraryEffect,
        crate::effects::SearchLibrarySlotsEffect,
        crate::effects::SoulbondPairEffect,
        crate::effects::TagMatchingObjectsEffect,
        crate::effects::TargetOnlyEffect,
        crate::effects::TapEffect,
        crate::effects::UntapEffect,
        crate::effects::AddManaOfChosenColorEffect,
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
        mode.description,
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
