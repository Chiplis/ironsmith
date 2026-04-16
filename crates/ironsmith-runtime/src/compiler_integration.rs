use ironsmith_compiler as compiler;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompilerIntegrationError {
    Parse(compiler::CardTextError),
    UnsupportedEffect { detail: String },
    UnsupportedStaticAbility { detail: String },
    UnsupportedTrigger { detail: String },
}

impl std::fmt::Display for CompilerIntegrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(err) => err.fmt(f),
            Self::UnsupportedEffect { detail } => {
                write!(
                    f,
                    "runtime compiler integration does not support effect conversion: {detail}"
                )
            }
            Self::UnsupportedStaticAbility { detail } => {
                write!(
                    f,
                    "runtime compiler integration does not support static ability conversion: {detail}"
                )
            }
            Self::UnsupportedTrigger { detail } => {
                write!(
                    f,
                    "runtime compiler integration does not support trigger conversion: {detail}"
                )
            }
        }
    }
}

impl std::error::Error for CompilerIntegrationError {}

impl From<compiler::CardTextError> for CompilerIntegrationError {
    fn from(value: compiler::CardTextError) -> Self {
        Self::Parse(value)
    }
}

fn convert_effect_mode(
    mode: compiler::effect::EffectMode,
) -> Result<crate::effect::EffectMode, CompilerIntegrationError> {
    let effects = mode
        .effects
        .into_iter()
        .map(convert_effect)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(crate::effect::EffectMode::new(
        mode.description,
        remove_redundant_target_only_effects(effects),
    ))
}

fn convert_delayed_trigger_spec(spec: compiler::DelayedTriggerSpec) -> crate::triggers::Trigger {
    crate::triggers::Trigger::from_delayed_trigger_spec(spec)
}

fn convert_continuous_target(
    target: compiler::continuous::EffectTarget,
) -> crate::continuous::EffectTarget {
    target.into()
}

fn convert_continuous_modification(
    modification: compiler::continuous::Modification,
) -> Result<crate::continuous::Modification, CompilerIntegrationError> {
    crate::continuous::Modification::try_from_compiled(
        modification,
        runtime_static_ability,
        convert_ability,
        convert_removed_ability,
    )
}

fn convert_removed_ability(
    ability: compiler::ability::Ability,
) -> Result<crate::static_abilities::StaticAbility, CompilerIntegrationError> {
    match ability.kind {
        compiler::ability::AbilityKind::Static(static_ability) => {
            runtime_static_ability(static_ability)
        }
        other => Err(CompilerIntegrationError::UnsupportedEffect {
            detail: format!("continuous RemoveAbility for non-static ability `{other:?}`"),
        }),
    }
}

fn convert_runtime_modification(
    modification: compiler::effects::continuous::RuntimeModification,
) -> Result<crate::effects::continuous::RuntimeModification, CompilerIntegrationError> {
    Ok(match modification {
        compiler::effects::continuous::RuntimeModification::Placeholder(label) => {
            return Err(CompilerIntegrationError::UnsupportedEffect {
                detail: format!("continuous runtime placeholder `{label}`"),
            });
        }
        compiler::effects::continuous::RuntimeModification::ModifyPowerToughness {
            power,
            toughness,
        } => crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
            power,
            toughness,
        },
        compiler::effects::continuous::RuntimeModification::ChangeControllerToEffectController => {
            crate::effects::continuous::RuntimeModification::ChangeControllerToEffectController
        }
        compiler::effects::continuous::RuntimeModification::ChangeControllerToPlayer(player) => {
            crate::effects::continuous::RuntimeModification::ChangeControllerToPlayer(player)
        }
        compiler::effects::continuous::RuntimeModification::CopyOf {
            source,
            preserve_source_abilities,
        } => crate::effects::continuous::RuntimeModification::CopyOf {
            source,
            preserve_source_abilities,
        },
    })
}

fn convert_grant_duration(
    duration: compiler::grant::GrantDuration,
) -> Result<crate::grant::GrantDuration, CompilerIntegrationError> {
    match duration {
        compiler::grant::GrantDuration::Forever => Ok(crate::grant::GrantDuration::Forever),
        compiler::grant::GrantDuration::UntilEndOfTurn => {
            Ok(crate::grant::GrantDuration::UntilEndOfTurn)
        }
        compiler::grant::GrantDuration::UntilYourNextTurnEnd => {
            Err(CompilerIntegrationError::UnsupportedEffect {
                detail: "grant duration UntilYourNextTurnEnd has no runtime one-shot grant model"
                    .to_string(),
            })
        }
    }
}

fn convert_grantable(
    grantable: compiler::grant::Grantable,
) -> Result<crate::grant::Grantable, CompilerIntegrationError> {
    Ok(match grantable {
        compiler::grant::Grantable::Ability(ability) => {
            crate::grant::Grantable::Ability(runtime_static_ability(ability)?)
        }
        compiler::grant::Grantable::AlternativeCast(method) => {
            crate::grant::Grantable::AlternativeCast(convert_alternative_cast(method)?)
        }
        compiler::grant::Grantable::PlayFrom => crate::grant::Grantable::PlayFrom,
        compiler::grant::Grantable::DerivedAlternativeCast(spec) => {
            crate::grant::Grantable::DerivedAlternativeCast(convert_derived_alternative_cast(spec)?)
        }
    })
}

fn convert_derived_alternative_cast(
    spec: compiler::grant::DerivedAlternativeCast,
) -> Result<crate::grant::DerivedAlternativeCast, CompilerIntegrationError> {
    Ok(match spec {
        compiler::grant::DerivedAlternativeCast::FlashbackFromCardManaCost { additional_costs } => {
            crate::grant::DerivedAlternativeCast::FlashbackFromCardManaCost {
                additional_costs: additional_costs
                    .into_iter()
                    .map(convert_cost)
                    .collect::<Result<Vec<_>, _>>()?,
            }
        }
        compiler::grant::DerivedAlternativeCast::EscapeFromCardManaCost { exile_count } => {
            crate::grant::DerivedAlternativeCast::EscapeFromCardManaCost { exile_count }
        }
        compiler::grant::DerivedAlternativeCast::ManaValueAsGenericFromHand => {
            crate::grant::DerivedAlternativeCast::ManaValueAsGenericFromHand
        }
    })
}

fn convert_grant_spec(
    spec: compiler::grant::GrantSpec,
) -> Result<crate::grant::GrantSpec, CompilerIntegrationError> {
    Ok(crate::grant::GrantSpec {
        grantable: convert_grantable(spec.grantable)?,
        filter: spec.filter,
        zone: spec.zone,
        beneficiary: spec.beneficiary,
    })
}

fn clone_direct_effect<T>(effect: &compiler::effect::Effect) -> Option<crate::effect::Effect>
where
    T: Clone + crate::effects::EffectExecutor + 'static,
{
    effect
        .downcast_ref::<T>()
        .map(|payload| crate::effect::Effect::new(payload.clone()))
}

fn convert_effect(
    effect: compiler::effect::Effect,
) -> Result<crate::effect::Effect, CompilerIntegrationError> {
    if let Some(payload) = effect.as_tagged() {
        return Ok(crate::effect::Effect::new(
            crate::effects::TaggedEffect::new(
                payload.tag.clone(),
                convert_effect(*payload.effect.clone())?,
            ),
        ));
    }
    if let Some(payload) = effect.as_target_only() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.as_search() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.as_search_slots() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.as_deal_damage() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.as_put_counters() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.as_choose_mode() {
        let mut converted = crate::effects::ChooseModeEffect::new(
            payload
                .modes
                .iter()
                .cloned()
                .map(convert_effect_mode)
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
        return Ok(crate::effect::Effect::new(converted));
    }
    if let Some(payload) = effect.as_conditional() {
        return Ok(crate::effect::Effect::conditional(
            payload.condition.clone(),
            payload
                .if_true
                .iter()
                .cloned()
                .map(convert_effect)
                .collect::<Result<Vec<_>, _>>()?,
            payload
                .if_false
                .iter()
                .cloned()
                .map(convert_effect)
                .collect::<Result<Vec<_>, _>>()?,
        ));
    }
    if let Some(payload) = effect.as_if_effect() {
        return Ok(crate::effect::Effect::new(crate::effects::IfEffect::new(
            payload.condition,
            payload.predicate.clone(),
            payload
                .then
                .iter()
                .cloned()
                .map(convert_effect)
                .collect::<Result<Vec<_>, _>>()?,
            payload
                .else_
                .iter()
                .cloned()
                .map(convert_effect)
                .collect::<Result<Vec<_>, _>>()?,
        )));
    }
    if let Some(payload) = effect.as_with_id() {
        return Ok(crate::effect::Effect::with_id(
            payload.id.0,
            convert_effect((*payload.effect).clone())?,
        ));
    }
    if let Some(payload) = effect.as_haunt_exile() {
        return Ok(crate::effect::Effect::new(
            crate::effects::HauntExileEffect::new(
                payload
                    .haunt_effects
                    .iter()
                    .cloned()
                    .map(convert_effect)
                    .collect::<Result<Vec<_>, _>>()?,
                payload.haunt_choices.clone(),
            ),
        ));
    }
    if let Some(payload) = effect.as_draw_cards() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.as_create_token() {
        let mut converted = crate::effects::CreateTokenEffect::new(
            convert_card_definition(payload.token.clone())?,
            payload.count.clone(),
            payload.controller.clone(),
        );
        if payload.enters_tapped {
            converted = converted.tapped();
        }
        if payload.enters_attacking {
            converted = converted.attacking();
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
        return Ok(crate::effect::Effect::new(converted));
    }
    if let Some(payload) = effect.as_tap() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.as_untap() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.as_remove_counters() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::UnearthEffect>() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::NinjutsuCostEffect>() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::NinjutsuEffect>() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::CastSourceEffect>() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::CipherEffect>() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.as_counter() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.as_schedule_delayed_trigger() {
        if !payload.target_choices.is_empty() {
            return Err(CompilerIntegrationError::UnsupportedEffect {
                detail: "delayed trigger conversion with compiler target choices".to_string(),
            });
        }
        let converted_effects = payload
            .effects
            .iter()
            .cloned()
            .map(convert_effect)
            .collect::<Result<Vec<_>, _>>()?;
        let mut converted = if let Some(tag) = &payload.target_tag {
            crate::effects::ScheduleDelayedTriggerEffect::from_tag(
                convert_delayed_trigger_spec(payload.trigger.clone()),
                converted_effects,
                payload.one_shot,
                tag.clone(),
                payload.controller.clone(),
            )
        } else {
            crate::effects::ScheduleDelayedTriggerEffect::new(
                convert_delayed_trigger_spec(payload.trigger.clone()),
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
        return Ok(crate::effect::Effect::new(converted));
    }
    if let Some(payload) = effect.as_grant_abilities_target() {
        return Ok(crate::effect::Effect::new(
            crate::effects::GrantAbilitiesTargetEffect::new(
                payload.target.clone(),
                payload
                    .abilities
                    .iter()
                    .cloned()
                    .map(runtime_static_ability)
                    .collect::<Result<Vec<_>, _>>()?,
                payload.duration.clone(),
            ),
        ));
    }
    if let Some(payload) = effect.as_create_token_copy() {
        return Ok(crate::effect::Effect::new(
            crate::effects::CreateTokenCopyEffect {
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
                    .map(runtime_static_ability)
                    .collect::<Result<Vec<_>, _>>()?,
            },
        ));
    }
    if let Some(payload) = effect.as_grant_next_spell_ability() {
        return Ok(crate::effect::Effect::new(
            crate::effects::GrantNextSpellAbilityEffect::new(
                payload.player.clone(),
                payload.filter.clone(),
                runtime_static_ability(payload.ability.clone())?,
            ),
        ));
    }
    if let Some(payload) = effect.as_apply_continuous() {
        let converted = crate::effects::ApplyContinuousEffect {
            target: convert_continuous_target(payload.target.clone()),
            target_spec: payload.target_spec.clone(),
            modification: payload
                .modification
                .clone()
                .map(convert_continuous_modification)
                .transpose()?,
            additional_modifications: payload
                .additional_modifications
                .iter()
                .cloned()
                .map(convert_continuous_modification)
                .collect::<Result<Vec<_>, _>>()?,
            runtime_modifications: payload
                .runtime_modifications
                .iter()
                .cloned()
                .map(convert_runtime_modification)
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
            return Err(CompilerIntegrationError::UnsupportedEffect {
                detail: "apply continuous effect without any modification".to_string(),
            });
        }
        return Ok(crate::effect::Effect::new(converted));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::LoseLifeEffect>() {
        return Ok(crate::effect::Effect::new(
            crate::effects::LoseLifeEffect::with_filter(
                payload.amount.clone(),
                payload.player.clone(),
            ),
        ));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::GainLifeEffect>() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::SacrificeEffect>() {
        return Ok(crate::effect::Effect::new(
            crate::effects::SacrificeEffect::you(
                payload.filter.clone(),
                crate::effect::Value::Fixed(payload.count),
            ),
        ));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::SacrificePlayerEffect>() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::DestroyEffect>() {
        return Ok(crate::effect::Effect::new(
            crate::effects::DestroyEffect::with_spec(payload.spec.clone()),
        ));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::DestroyNoRegenerationEffect>() {
        let converted = if let Some(target) = &payload.target {
            crate::effects::DestroyNoRegenerationEffect::with_spec(target.clone())
        } else if let Some(filter) = &payload.filter {
            crate::effects::DestroyNoRegenerationEffect::all(filter.clone())
        } else {
            return Err(CompilerIntegrationError::UnsupportedEffect {
                detail: "destroy-no-regeneration effect without target or filter".to_string(),
            });
        };
        return Ok(crate::effect::Effect::new(converted));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::RegenerateEffect>() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) =
        effect.downcast_ref::<compiler::effects::AddManaFromCommanderColorIdentityEffect>()
    {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::AddManaOfAnyColorEffect>() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::AddManaOfAnyOneColorEffect>() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::AddManaEffect>() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::PreventAllCombatDamageEffect>()
    {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::EmitKeywordActionEffect>() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::RenownEffect>() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::BolsterEffect>() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::FlipEffect>() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::PreventAllDamageEffect>() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) =
        effect.downcast_ref::<compiler::effects::PreventAllDamageToTargetEffect>()
    {
        return Ok(crate::effect::Effect::new(
            crate::effects::PreventAllDamageToTargetEffect::new(
                payload.target.clone(),
                payload.until.clone(),
            )
            .with_follow_up_effects(
                payload
                    .follow_up_effects
                    .iter()
                    .cloned()
                    .map(convert_effect)
                    .collect::<Result<Vec<_>, _>>()?,
            ),
        ));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::LoseTheGameEffect>() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::CreateEmblemEffect>() {
        let mut emblem =
            crate::effect::EmblemDescription::new(&payload.emblem.name, &payload.emblem.text);
        for ability in &payload.emblem.abilities {
            emblem = emblem.with_ability(convert_ability(ability.clone())?);
        }
        return Ok(crate::effect::Effect::new(
            crate::effects::CreateEmblemEffect::new(emblem),
        ));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::DiscardHandEffect>() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::RemoveAnyCountersAmongEffect>()
    {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::ChooseCardTypeEffect>() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) =
        effect.downcast_ref::<compiler::effects::ShuffleObjectsIntoLibraryEffect>()
    {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::ExchangeTextBoxesEffect>() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::EnergyCountersEffect>() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::DiscoverEffect>() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::SetBasePowerToughnessEffect>() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::CantEffect>() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::ExtraTurnEffect>() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::ExtraTurnAfterNextTurnEffect>()
    {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) =
        effect.downcast_ref::<compiler::effects::ModifyPowerToughnessForEachEffect>()
    {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::CastTaggedEffect>() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) =
        effect.downcast_ref::<compiler::effects::PutTaggedRemainderOnLibraryBottomEffect>()
    {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::RemoveUpToAnyCountersEffect>() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::TagTriggeringObjectEffect>() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::AttachToEffect>() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::AttachObjectsEffect>() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::SequenceEffect>() {
        return Ok(crate::effect::Effect::new(
            crate::effects::SequenceEffect::new(
                payload
                    .effects
                    .iter()
                    .cloned()
                    .map(convert_effect)
                    .collect::<Result<Vec<_>, _>>()?,
            ),
        ));
    }
    if let Some(payload) =
        effect.downcast_ref::<compiler::effects::MayEffect<compiler::effect::Effect>>()
    {
        let effects = payload
            .effects
            .iter()
            .cloned()
            .map(convert_effect)
            .collect::<Result<Vec<_>, _>>()?;
        let converted = if let Some(decider) = &payload.decider {
            crate::effects::MayEffect::new_for_player(effects, decider.clone())
        } else {
            crate::effects::MayEffect::new(effects)
        };
        return Ok(crate::effect::Effect::new(converted));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::RevealTopEffect>() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::TagAttachedToSourceEffect>() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) =
        effect.downcast_ref::<compiler::effects::ForPlayersEffect<compiler::effect::Effect>>()
    {
        return Ok(crate::effect::Effect::new(
            crate::effects::ForPlayersEffect::new(
                payload.filter.clone(),
                payload
                    .effects
                    .iter()
                    .cloned()
                    .map(convert_effect)
                    .collect::<Result<Vec<_>, _>>()?,
            ),
        ));
    }
    if let Some(payload) =
        effect.downcast_ref::<compiler::effects::ForEachTaggedEffect<compiler::effect::Effect>>()
    {
        return Ok(crate::effect::Effect::new(
            crate::effects::ForEachTaggedEffect::new(
                payload.tag.clone(),
                payload
                    .effects
                    .iter()
                    .cloned()
                    .map(convert_effect)
                    .collect::<Result<Vec<_>, _>>()?,
            ),
        ));
    }
    if let Some(payload) = effect
        .downcast_ref::<compiler::effects::ForEachTaggedPlayerEffect<compiler::effect::Effect>>()
    {
        return Ok(crate::effect::Effect::new(
            crate::effects::ForEachTaggedPlayerEffect::new(
                payload.tag.clone(),
                payload
                    .effects
                    .iter()
                    .cloned()
                    .map(convert_effect)
                    .collect::<Result<Vec<_>, _>>()?,
            ),
        ));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::VoteEffect>() {
        let convert_options = |options: &[compiler::effects::composition::VoteOption]| {
            options
                .iter()
                .map(|option| {
                    Ok(crate::effects::VoteOption::new(
                        option.name.clone(),
                        option
                            .effects_per_vote
                            .iter()
                            .cloned()
                            .map(convert_effect)
                            .collect::<Result<Vec<_>, _>>()?,
                    ))
                })
                .collect::<Result<Vec<_>, CompilerIntegrationError>>()
        };
        let converted = match &payload.choice {
            compiler::effects::VoteChoice::NamedOptions(options) => {
                crate::effects::VoteEffect::with_optional_extra(
                    convert_options(options)?,
                    payload.controller_extra_votes,
                    payload.controller_optional_extra_votes,
                )
            }
            compiler::effects::VoteChoice::Objects { filter, count } => {
                crate::effects::VoteEffect::vote_objects_with_optional_extra(
                    filter.clone(),
                    *count,
                    payload.controller_extra_votes,
                    payload.controller_optional_extra_votes,
                )
            }
        };
        return Ok(crate::effect::Effect::new(converted));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::cards::ImprintFromHandEffect>()
    {
        return Ok(crate::effect::Effect::new(
            crate::effects::cards::ImprintFromHandEffect::new(payload.filter.clone()),
        ));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::DiscardEffect>() {
        let mut converted = crate::effects::DiscardEffect::new_with_filter(
            payload.count.clone(),
            payload.player.clone(),
            payload.random,
            payload.card_filter.clone(),
        );
        if let Some(tag) = &payload.tag {
            converted = converted.with_tag(tag.clone());
        }
        return Ok(crate::effect::Effect::new(converted));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::InvestigateEffect>() {
        return Ok(crate::effect::Effect::new(
            crate::effects::InvestigateEffect::new(payload.count.clone(), payload.player.clone()),
        ));
    }
    if let Some(payload) =
        effect.downcast_ref::<compiler::effects::ReturnFromGraveyardToBattlefieldEffect>()
    {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) =
        effect.downcast_ref::<compiler::effects::ReturnFromGraveyardToHandEffect>()
    {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::PutOntoBattlefieldEffect>() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::ShuffleLibraryEffect>() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::MayMoveToZoneEffect>() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::ExileTopOfLibraryEffect>() {
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
        return Ok(crate::effect::Effect::new(converted));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::LookAtTopCardsEffect>() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::ChooseCardNameEffect>() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::ControlPlayerEffect>() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::GrantEffect>() {
        return Ok(crate::effect::Effect::new(
            crate::effects::GrantEffect::new(
                convert_grantable(payload.grantable.clone())?,
                payload.target.clone(),
                convert_grant_duration(payload.duration)?,
            ),
        ));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::GrantBySpecEffect>() {
        return Ok(crate::effect::Effect::new(
            crate::effects::GrantBySpecEffect::new(
                convert_grant_spec(payload.spec.clone())?,
                payload.player.clone(),
                convert_grant_duration(payload.duration)?,
            ),
        ));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::MoveAllCountersEffect>() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::ProliferateEffect>() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::MonstrosityEffect>() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::TransformEffect>() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::RepeatEffectsEffect>() {
        return Ok(crate::effect::Effect::new(
            crate::effects::RepeatEffectsEffect::new(
                payload.count.clone(),
                payload
                    .effects
                    .iter()
                    .cloned()
                    .map(convert_effect)
                    .collect::<Result<Vec<_>, _>>()?,
            ),
        ));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::RepeatProcessEffect>() {
        return Ok(crate::effect::Effect::new(
            crate::effects::RepeatProcessEffect::new(
                payload
                    .effects
                    .iter()
                    .cloned()
                    .map(convert_effect)
                    .collect::<Result<Vec<_>, _>>()?,
                payload.condition,
                payload.predicate.clone(),
            ),
        ));
    }
    if let Some(payload) =
        effect.downcast_ref::<compiler::effects::RearrangeLookedCardsInLibraryEffect>()
    {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::ChoosePlayerEffect>() {
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
        return Ok(crate::effect::Effect::new(converted));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::DealDistributedDamageEffect>() {
        return Ok(crate::effect::Effect::new(
            crate::effects::DealDistributedDamageEffect::new(
                payload.amount.clone(),
                payload.target.clone(),
            ),
        ));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::ExecuteWithSourceEffect>() {
        return Ok(crate::effect::Effect::new(
            crate::effects::ExecuteWithSourceEffect::new(
                payload.source.clone(),
                convert_effect((*payload.effect).clone())?,
            ),
        ));
    }
    if let Some(payload) =
        effect.downcast_ref::<compiler::effects::ExileTaggedWhenSourceLeavesEffect>()
    {
        return Ok(crate::effect::Effect::new(
            crate::effects::ExileTaggedWhenSourceLeavesEffect::new(
                payload.tag.clone(),
                payload.controller.clone(),
            ),
        ));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::ForEachObject>() {
        return Ok(crate::effect::Effect::new(
            crate::effects::ForEachObject::new(
                payload.filter.clone(),
                payload
                    .effects
                    .iter()
                    .cloned()
                    .map(convert_effect)
                    .collect::<Result<Vec<_>, _>>()?,
            ),
        ));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::GrantPlayTaggedEffect>() {
        return Ok(crate::effect::Effect::new(
            crate::effects::GrantPlayTaggedEffect::new(
                payload.tag.clone(),
                payload.player.clone(),
                payload.duration,
                payload.allow_land,
                payload.allow_any_color_for_cast,
            ),
        ));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::LocalRewriteEffect>() {
        return Ok(crate::effect::Effect::new(
            crate::effects::LocalRewriteEffect::new(
                convert_effect((*payload.effect).clone())?,
                payload.zone_replacements.clone(),
            ),
        ));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::PhaseOutEffect>() {
        return Ok(crate::effect::Effect::new(
            crate::effects::PhaseOutEffect::with_spec(payload.target.clone()),
        ));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::ReflexiveTriggerEffect>() {
        return Ok(crate::effect::Effect::new(
            crate::effects::ReflexiveTriggerEffect::new(
                payload.condition,
                payload.predicate.clone(),
                payload
                    .effects
                    .iter()
                    .cloned()
                    .map(convert_effect)
                    .collect::<Result<Vec<_>, _>>()?,
                payload.choices.clone(),
            ),
        ));
    }
    if let Some(payload) =
        effect.downcast_ref::<compiler::effects::ScheduleEffectsWhenTaggedLeavesEffect>()
    {
        let mut converted = crate::effects::ScheduleEffectsWhenTaggedLeavesEffect::new(
            payload.tag.clone(),
            payload
                .effects
                .iter()
                .cloned()
                .map(convert_effect)
                .collect::<Result<Vec<_>, _>>()?,
            payload.controller.clone(),
        );
        if matches!(
            payload.ability_source,
            compiler::effects::TaggedLeavesAbilitySource::CurrentSource
        ) {
            converted = converted.with_current_source_as_ability_source();
        }
        return Ok(crate::effect::Effect::new(converted));
    }
    if let Some(payload) =
        effect.downcast_ref::<compiler::effects::UnlessActionEffect<compiler::effect::Effect>>()
    {
        return Ok(crate::effect::Effect::new(
            crate::effects::UnlessActionEffect::new(
                payload
                    .effects
                    .iter()
                    .cloned()
                    .map(convert_effect)
                    .collect::<Result<Vec<_>, _>>()?,
                payload
                    .alternative
                    .iter()
                    .cloned()
                    .map(convert_effect)
                    .collect::<Result<Vec<_>, _>>()?,
                payload.player.clone(),
            ),
        ));
    }
    if let Some(payload) =
        effect.downcast_ref::<compiler::effects::UnlessPaysEffect<compiler::effect::Effect>>()
    {
        return Ok(crate::effect::Effect::new(
            crate::effects::UnlessPaysEffect::new_with_life_and_additional_and_multiplier_and_x(
                payload
                    .effects
                    .iter()
                    .cloned()
                    .map(convert_effect)
                    .collect::<Result<Vec<_>, _>>()?,
                payload.player.clone(),
                payload.mana.clone(),
                payload.life.clone(),
                payload.additional_generic.clone(),
                payload.mana_multiplier.clone(),
                payload.x_value.clone(),
            ),
        ));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::ChooseNewTargetsEffect>() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::RegisterZoneReplacementEffect>()
    {
        return Ok(crate::effect::Effect::new(
            crate::effects::RegisterZoneReplacementEffect::new(
                payload.target.clone(),
                payload.from_zone,
                payload.to_zone,
                payload.replacement_zone,
                payload.mode,
            ),
        ));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::ExileInsteadOfGraveyardEffect>()
    {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::WinTheGameEffect>() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }
    if let Some(payload) = effect.downcast_ref::<compiler::effects::ConsultTopOfLibraryEffect>() {
        return Ok(crate::effect::Effect::new(payload.clone()));
    }

    macro_rules! clone_direct {
        ($($ty:path),* $(,)?) => {
            $(
                if let Some(converted) = clone_direct_effect::<$ty>(&effect) {
                    return Ok(converted);
                }
            )*
        };
    }

    clone_direct!(
        compiler::effects::AmassEffect,
        compiler::effects::BecomeBasicLandTypeChoiceEffect,
        compiler::effects::BecomeColorChoiceEffect,
        compiler::effects::BecomeCreatureTypeChoiceEffect,
        compiler::effects::ChooseObjectsEffect,
        compiler::effects::ChooseSpellCastHistoryEffect,
        compiler::effects::ClashEffect,
        compiler::effects::CopySpellEffect,
        compiler::effects::CounterEffect,
        compiler::effects::CrewCostEffect,
        compiler::effects::DealDamageEffect,
        compiler::effects::DrawCardsEffect,
        compiler::effects::EachPlayerScryEffect,
        compiler::effects::EarthbendEffect,
        compiler::effects::ExchangeControlEffect,
        compiler::effects::ExertCostEffect,
        compiler::effects::ExileEffect,
        compiler::effects::ExileUntilEffect,
        compiler::effects::GrantNextSpellCostReductionEffect,
        compiler::effects::GrantTaggedSpellFreeCastUntilEndOfTurnEffect,
        compiler::effects::GrantTaggedSpellLifeCostByManaValueEffect,
        compiler::effects::LookAtHandEffect,
        compiler::effects::MeldEffect,
        compiler::effects::MillEffect,
        compiler::effects::ModifyPowerToughnessEffect,
        compiler::effects::MoveToLibraryNthFromTopEffect,
        compiler::effects::MoveToZoneEffect,
        compiler::effects::PayEnergyEffect,
        compiler::effects::PayManaEffect,
        compiler::effects::PopulateEffect,
        compiler::effects::PutCountersEffect,
        compiler::effects::RemoveCountersEffect,
        compiler::effects::ReorderLibraryTopEffect,
        compiler::effects::RetainManaUntilEndOfTurnEffect,
        compiler::effects::RetargetStackObjectEffect,
        compiler::effects::ReturnAllToBattlefieldEffect,
        compiler::effects::ReturnToHandEffect,
        compiler::effects::RevealTaggedEffect,
        compiler::effects::SacrificeTargetEffect,
        compiler::effects::ScryEffect,
        compiler::effects::SearchLibraryEffect,
        compiler::effects::SearchLibrarySlotsEffect,
        compiler::effects::TagMatchingObjectsEffect,
        compiler::effects::TargetOnlyEffect,
        compiler::effects::TapEffect,
        compiler::effects::UntapEffect,
        compiler::effects::mana::AddManaOfChosenColorEffect,
        compiler::effects::mana::AddManaOfImprintedColorsEffect,
        compiler::effects::mana::AddScaledManaEffect,
    );
    if let Some(payload) = effect.as_placeholder() {
        return Err(CompilerIntegrationError::UnsupportedEffect {
            detail: format!("placeholder effect `{}`", payload.label),
        });
    }

    Err(CompilerIntegrationError::UnsupportedEffect {
        detail: format!(
            "no runtime conversion registered for compiler effect payload `{}`",
            effect.payload_type_name()
        ),
    })
}

fn convert_resolution_program(
    program: compiler::resolution::ResolutionProgram,
) -> Result<crate::resolution::ResolutionProgram, CompilerIntegrationError> {
    let mut mapped = program.try_map_effects(convert_effect)?;
    for segment in &mut mapped.segments {
        segment.default_effects =
            remove_redundant_target_only_effects(std::mem::take(&mut segment.default_effects));
        for branch in &mut segment.self_replacements {
            branch.replacement_effects = remove_redundant_target_only_effects(std::mem::take(
                &mut branch.replacement_effects,
            ));
        }
    }
    Ok(crate::resolution::ResolutionProgram::new(mapped.segments))
}

fn remove_redundant_target_only_effects(
    effects: Vec<crate::effect::Effect>,
) -> Vec<crate::effect::Effect> {
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

fn fixed_u32(
    value: compiler::effect::Value,
    context: &str,
) -> Result<u32, CompilerIntegrationError> {
    match value {
        compiler::effect::Value::Fixed(amount) if amount >= 0 => Ok(amount as u32),
        other => Err(CompilerIntegrationError::UnsupportedEffect {
            detail: format!("{context} requires a fixed non-negative value, got {other:?}"),
        }),
    }
}

fn convert_cost(
    cost: compiler::costs::Cost,
) -> Result<crate::costs::Cost, CompilerIntegrationError> {
    Ok(match cost {
        compiler::costs::Cost::Mana(mana) => crate::costs::Cost::mana(mana),
        compiler::costs::Cost::Tap => crate::costs::Cost::tap(),
        compiler::costs::Cost::Untap => crate::costs::Cost::untap(),
        compiler::costs::Cost::DiscardSource => crate::costs::Cost::discard_source(),
        compiler::costs::Cost::SacrificeSelf => crate::costs::Cost::sacrifice_self(),
        compiler::costs::Cost::Sacrifice(filter) => crate::costs::Cost::sacrifice(filter),
        compiler::costs::Cost::Discard { count, card_types } => {
            crate::costs::Cost::discard_types(count, card_types)
        }
        compiler::costs::Cost::DiscardHand => crate::costs::Cost::discard_hand(),
        compiler::costs::Cost::RemoveCounters {
            counter_type,
            count,
        } => crate::costs::Cost::remove_counters(counter_type, count),
        compiler::costs::Cost::AddCounters {
            counter_type,
            count,
        } => crate::costs::Cost::add_counters(counter_type, count),
        compiler::costs::Cost::RemoveAnyCountersFromSource {
            counter_type,
            display_x,
        } => crate::costs::Cost::remove_any_counters_from_source(counter_type, display_x),
        compiler::costs::Cost::Energy(amount) => {
            crate::costs::Cost::energy(fixed_u32(amount, "energy cost")?)
        }
        compiler::costs::Cost::Mill(count) => {
            crate::costs::Cost::mill(fixed_u32(count, "mill cost")?)
        }
        compiler::costs::Cost::Life(amount) => {
            crate::costs::Cost::life(fixed_u32(amount, "life cost")?)
        }
        compiler::costs::Cost::ExileSelf => crate::costs::Cost::exile_self(),
        compiler::costs::Cost::ExileFromHand {
            count,
            color_filter,
        } => crate::costs::Cost::exile_from_hand(count, color_filter),
        compiler::costs::Cost::ReturnSelfToHand => crate::costs::Cost::return_self_to_hand(),
        compiler::costs::Cost::Effect(effect) => {
            let converted = convert_effect(effect)?;
            crate::costs::Cost::try_from_runtime_effect(converted).map_err(|detail| {
                CompilerIntegrationError::UnsupportedEffect {
                    detail: format!("effect-backed cost is not cost-executable: {detail}"),
                }
            })?
        }
        compiler::costs::Cost::Placeholder(label) => {
            return Err(CompilerIntegrationError::UnsupportedEffect {
                detail: format!("untyped compiler cost placeholder: {label}"),
            });
        }
    })
}

fn convert_total_cost(
    cost: compiler::cost::TotalCost,
) -> Result<crate::cost::TotalCost, CompilerIntegrationError> {
    Ok(crate::cost::TotalCost::from_costs(
        cost.costs()
            .iter()
            .cloned()
            .map(convert_cost)
            .collect::<Result<Vec<_>, _>>()?,
    ))
}

fn convert_optional_cost(
    cost: compiler::cost::OptionalCost,
) -> Result<crate::cost::OptionalCost, CompilerIntegrationError> {
    Ok(crate::cost::OptionalCost {
        label: cost.label,
        cost: convert_total_cost(cost.cost)?,
        repeatable: cost.repeatable,
        returns_to_hand: cost.returns_to_hand,
    })
}

fn convert_alternative_cast(
    method: compiler::alternative_cast::AlternativeCastingMethod,
) -> Result<crate::alternative_cast::AlternativeCastingMethod, CompilerIntegrationError> {
    method.try_map(convert_effect, convert_cost)
}

fn runtime_static_ability_model(
    ability: compiler::static_abilities::StaticAbility,
) -> Result<crate::static_abilities::CompiledStaticAbility, CompilerIntegrationError> {
    ability.try_map(convert_trigger, convert_effect, convert_cost)
}

fn runtime_static_ability(
    ability: compiler::static_abilities::StaticAbility,
) -> Result<crate::static_abilities::StaticAbility, CompilerIntegrationError> {
    Ok(crate::static_abilities::StaticAbility::from_model(
        runtime_static_ability_model(ability)?,
    ))
}

fn convert_trigger(
    trigger: compiler::triggers::Trigger,
) -> Result<crate::triggers::Trigger, CompilerIntegrationError> {
    crate::triggers::Trigger::from_compiler_model(trigger)
        .map_err(|err| CompilerIntegrationError::UnsupportedTrigger { detail: err.detail })
}

fn convert_ability(
    ability: compiler::ability::Ability,
) -> Result<crate::ability::Ability, CompilerIntegrationError> {
    let source_text = ability.text.clone();
    let mut converted = match ability.kind {
        compiler::ability::AbilityKind::Static(static_ability) => {
            crate::ability::Ability::static_ability(runtime_static_ability(static_ability)?)
        }
        compiler::ability::AbilityKind::Triggered(triggered) => {
            let mut out = crate::ability::Ability::triggered(
                convert_trigger(triggered.trigger)?,
                convert_resolution_program(triggered.effects)?,
            );
            if let crate::ability::AbilityKind::Triggered(inner) = &mut out.kind {
                inner.choices = triggered.choices;
                inner.intervening_if = triggered.intervening_if;
            }
            out
        }
        compiler::ability::AbilityKind::Activated(activated) => {
            let mut out = crate::ability::Ability::activated_with_timing(
                convert_total_cost(activated.mana_cost)?,
                convert_resolution_program(activated.effects)?,
                activated.timing,
            );
            if let crate::ability::AbilityKind::Activated(inner) = &mut out.kind {
                inner.choices = activated.choices;
                inner.additional_restrictions = activated.additional_restrictions;
                inner.activation_restrictions = activated.activation_restrictions;
                inner.mana_output = activated.mana_output;
                inner.activation_condition = activated.activation_condition;
                inner.mana_usage_restrictions = activated.mana_usage_restrictions;
            }
            out
        }
    };
    converted.functional_zones = ability.functional_zones;
    converted.text = source_text.or_else(|| match &converted.kind {
        crate::ability::AbilityKind::Static(static_ability) => Some(static_ability.display()),
        _ => None,
    });
    Ok(converted)
}

fn combine_level_ability_statics(
    abilities: Vec<crate::ability::Ability>,
) -> Vec<crate::ability::Ability> {
    let mut out = Vec::with_capacity(abilities.len());
    let mut levels = Vec::new();

    for ability in abilities {
        let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
            out.push(ability);
            continue;
        };
        let Some(level_abilities) = static_ability.level_abilities() else {
            out.push(ability);
            continue;
        };
        levels.extend(level_abilities.iter().cloned());
    }

    if !levels.is_empty() {
        out.push(crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::with_level_abilities(levels),
        ));
    }

    out
}

fn convert_card_definition(
    definition: compiler::CardDefinition,
) -> Result<crate::cards::CardDefinition, CompilerIntegrationError> {
    let abilities = definition
        .abilities
        .into_iter()
        .map(convert_ability)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(crate::cards::CardDefinition {
        card: definition.card,
        abilities: combine_level_ability_statics(abilities),
        spell_effect: definition
            .spell_effect
            .map(convert_resolution_program)
            .transpose()?,
        aura_attach_filter: definition.aura_attach_filter,
        alternative_casts: definition
            .alternative_casts
            .into_iter()
            .map(convert_alternative_cast)
            .collect::<Result<Vec<_>, _>>()?,
        has_fuse: definition.has_fuse,
        optional_costs: definition
            .optional_costs
            .into_iter()
            .map(convert_optional_cost)
            .collect::<Result<Vec<_>, _>>()?,
        max_saga_chapter: definition.max_saga_chapter,
        additional_cost: convert_total_cost(definition.additional_cost)?,
    })
}

pub fn into_runtime_definition(
    definition: compiler::CardDefinition,
) -> Result<crate::cards::CardDefinition, CompilerIntegrationError> {
    Ok(convert_card_definition(definition)?)
}

pub fn into_runtime_compiled_card_text(
    compiled: compiler::CompiledCardText<compiler::CardDefinition>,
) -> Result<compiler::CompiledCardText<crate::cards::CardDefinition>, CompilerIntegrationError> {
    Ok(compiler::CompiledCardText {
        definition: into_runtime_definition(compiled.definition)?,
        annotations: compiled.annotations,
    })
}

pub fn compile_to_runtime_definition(
    name: &str,
    text: impl Into<String>,
    allow_unsupported: bool,
) -> Result<crate::cards::CardDefinition, CompilerIntegrationError> {
    let builder = compiler::CardDefinitionBuilder::new(crate::ids::CardId::new(), name);
    compile_builder_to_runtime_definition(builder, text, allow_unsupported)
}

pub fn compile_builder_to_runtime_definition(
    builder: compiler::CardDefinitionBuilder,
    text: impl Into<String>,
    allow_unsupported: bool,
) -> Result<crate::cards::CardDefinition, CompilerIntegrationError> {
    let text = text.into();
    let compiled =
        compile_builder_to_runtime_compiled_card_text(builder, text.clone(), allow_unsupported)?;
    let mut runtime = compiled.definition;
    if runtime.card.oracle_text.is_empty() {
        runtime.card.oracle_text = text;
    }
    Ok(runtime)
}

pub fn compile_builder_to_runtime_compiled_card_text(
    builder: compiler::CardDefinitionBuilder,
    text: impl Into<String>,
    allow_unsupported: bool,
) -> Result<compiler::CompiledCardText<crate::cards::CardDefinition>, CompilerIntegrationError> {
    let compiled = compiler::CompilerFacade::new().compile_definition(
        builder,
        text,
        compiler::CompilePolicy { allow_unsupported },
    )?;
    into_runtime_compiled_card_text(compiled)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::PlayerId;
    use crate::types::CardType;
    use crate::zone::Zone;

    #[test]
    fn compile_to_runtime_definition_handles_representative_spell_text() {
        let definition = compile_to_runtime_definition(
            "Lightning Bolt",
            "Mana cost: {R}\nType: Instant\nLightning Bolt deals 3 damage to any target.",
            false,
        )
        .expect("lightning bolt should compile through runtime compiler integration");

        assert_eq!(definition.name(), "Lightning Bolt");
        assert!(definition.spell_effect.is_some());
        assert_eq!(definition.card.name, "Lightning Bolt");
    }

    #[test]
    fn compile_builder_to_runtime_definition_preserves_manual_metadata() {
        let definition = compile_builder_to_runtime_definition(
            compiler::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Command Tower")
                .card_types(vec![CardType::Land]),
            "{T}: Add one mana of any color in your commander's color identity.",
            false,
        )
        .expect("command tower should compile through runtime compiler integration");

        assert!(definition.card.is_land());
        assert_eq!(definition.abilities.len(), 1);
    }

    #[test]
    fn compiler_integrated_definitions_execute_normally_in_runtime() {
        let definition = compile_to_runtime_definition(
            "Llanowar Elves",
            "Mana cost: {G}\nType: Creature — Elf Druid\nPower/Toughness: 1/1\n{T}: Add {G}.",
            false,
        )
        .expect("llanowar elves should compile");

        let mut game =
            crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let object_id = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let object = game.object(object_id).expect("object should exist");

        assert_eq!(object.name, "Llanowar Elves");
        assert_eq!(object.abilities.len(), 1);
        assert!(object.abilities[0].is_mana_ability());
    }
}
