use ironsmith_compiled_artifact::{
    ArtifactCardId, ArtifactCardIdentity, CompiledCardArtifact, CompiledCardPayload,
    wire_definition_from_serializable,
};
use ironsmith_compiler as compiler;
#[cfg(test)]
mod remove_any_source_counter_payload_tests;
#[cfg(test)]
use ironsmith_runtime_catalog::CardRegistryArtifactExt as _;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompilerIntegrationError {
    Parse(compiler::CardTextError),
    UnsupportedEffect { detail: String },
    UnsupportedStaticAbility { detail: String },
    UnsupportedTrigger { detail: String },
    ArtifactEncoding { detail: String },
    ArtifactMaterialization { detail: String },
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
            Self::ArtifactEncoding { detail } => {
                write!(f, "failed to encode compiled-card artifact: {detail}")
            }
            Self::ArtifactMaterialization { detail } => {
                write!(f, "failed to materialize compiled-card artifact: {detail}")
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

struct CompilerEffectModel;

impl ironsmith::effect_model_interpreter::EffectModel for CompilerEffectModel {
    type Effect = compiler::effect::Effect;
    type StaticAbility = compiler::static_abilities::StaticAbility;
    type CardDefinition = compiler::cards::CardDefinition;
    type Ability = compiler::ability::Ability;
    type EmblemDescription = compiler::effect::EmblemDescription;
    type ContinuousTarget = compiler::continuous::EffectTarget;
    type ContinuousModification = compiler::continuous::Modification;
    type RuntimeModification = compiler::effects::continuous::RuntimeModification;
    type Grantable = compiler::grant::Grantable;
    type GrantDuration = compiler::grant::GrantDuration;
    type GrantSpec = compiler::grant::GrantSpec;

    fn downcast_ref<T: 'static>(effect: &Self::Effect) -> Option<&T> {
        effect.downcast_ref::<T>()
    }

    fn payload_type_name(effect: &Self::Effect) -> &str {
        effect.payload_type_name()
    }
}

struct CompilerEffectModelHooks;

impl ironsmith::effect_model_interpreter::EffectModelInterpreterHooks<CompilerEffectModel>
    for CompilerEffectModelHooks
{
    type Error = CompilerIntegrationError;

    fn unsupported_effect(&mut self, detail: String) -> Self::Error {
        CompilerIntegrationError::UnsupportedEffect { detail }
    }

    fn runtime_static_ability_hook(
        &mut self,
        ability: compiler::static_abilities::StaticAbility,
    ) -> Result<ironsmith::static_abilities::StaticAbility, Self::Error> {
        runtime_static_ability(ability)
    }

    fn runtime_card_definition_hook(
        &mut self,
        definition: compiler::cards::CardDefinition,
    ) -> Result<ironsmith::cards::CardDefinition, Self::Error> {
        runtime_definition_from_core_model(definition)
    }

    fn runtime_ability_hook(
        &mut self,
        ability: compiler::ability::Ability,
    ) -> Result<ironsmith::ability::Ability, Self::Error> {
        runtime_ability_from_core_model(ability)
    }

    fn runtime_emblem_hook(
        &mut self,
        emblem: compiler::effect::EmblemDescription,
    ) -> Result<ironsmith::effect::EmblemDescription, Self::Error> {
        let mut converted = ironsmith::effect::EmblemDescription::new(&emblem.name, &emblem.text);
        for ability in emblem.abilities {
            converted = converted.with_ability(runtime_ability_from_core_model(ability)?);
        }
        Ok(converted)
    }

    fn runtime_continuous_modification_hook(
        &mut self,
        modification: compiler::continuous::Modification,
    ) -> Result<ironsmith::continuous::Modification, Self::Error> {
        ironsmith::continuous::Modification::try_from_model(
            modification,
            runtime_static_ability,
            runtime_ability_from_core_model,
            runtime_ability_from_core_model,
        )
    }

    fn runtime_continuous_runtime_modification_hook(
        &mut self,
        modification: compiler::effects::continuous::RuntimeModification,
    ) -> Result<ironsmith::effects::continuous::RuntimeModification, Self::Error> {
        Ok(match modification {
            compiler::effects::continuous::RuntimeModification::ModifyPowerToughness {
                power,
                toughness,
            } => ironsmith::effects::continuous::RuntimeModification::ModifyPowerToughness {
                power,
                toughness,
            },
            compiler::effects::continuous::RuntimeModification::ChangeControllerToEffectController => {
                ironsmith::effects::continuous::RuntimeModification::ChangeControllerToEffectController
            }
            compiler::effects::continuous::RuntimeModification::ChangeControllerToPlayer(player) => {
                ironsmith::effects::continuous::RuntimeModification::ChangeControllerToPlayer(player)
            }
            compiler::effects::continuous::RuntimeModification::CopyOf {
                source,
                preserve_source_abilities,
                name_override,
                name_override_surface,
                add_supertypes,
                copy_exception_surface,
            } => ironsmith::effects::continuous::RuntimeModification::CopyOf {
                source,
                preserve_source_abilities,
                name_override,
                name_override_surface,
                add_supertypes,
                copy_exception_surface,
            },
            compiler::effects::continuous::RuntimeModification::CopyOfWithAbilities { source, preserve_source_abilities, name_override, name_override_surface, add_supertypes, copy_exception_surface, abilities } =>
                ironsmith::effects::continuous::RuntimeModification::CopyOfWithAbilities { source, preserve_source_abilities, name_override, name_override_surface, add_supertypes, copy_exception_surface,
                    abilities: abilities.into_iter().map(|ability| runtime_ability_from_core_model(ability)).collect::<Result<Vec<_>, _>>()?,
                },
            compiler::effects::continuous::RuntimeModification::RemoveAllAbilities => {
                ironsmith::effects::continuous::RuntimeModification::RemoveAllAbilities
            }
            compiler::effects::continuous::RuntimeModification::RemoveThisAbility => {
                ironsmith::effects::continuous::RuntimeModification::RemoveThisAbility
            }
            compiler::effects::continuous::RuntimeModification::SetAuraAttachmentFilter(filter) => {
                ironsmith::effects::continuous::RuntimeModification::SetAuraAttachmentFilter(filter)
            }
        })
    }

    fn runtime_grantable_hook(
        &mut self,
        grantable: compiler::grant::Grantable,
    ) -> Result<ironsmith::grant::Grantable, Self::Error> {
        Ok(match grantable {
            compiler::grant::Grantable::Ability(ability) => {
                ironsmith::grant::Grantable::Ability(runtime_static_ability(ability)?)
            }
            compiler::grant::Grantable::AlternativeCast(method) => {
                ironsmith::grant::Grantable::AlternativeCast(convert_alternative_cast(method)?)
            }
            compiler::grant::Grantable::PlayFrom => ironsmith::grant::Grantable::PlayFrom,
            compiler::grant::Grantable::AlternativePrice { costs, origin } => ironsmith::grant::Grantable::AlternativePrice {
                costs: costs.into_iter().map(runtime_cost_from_core_model).collect::<Result<_, _>>()?, origin,
            },
            compiler::grant::Grantable::DerivedAlternativeCast(spec) => {
                ironsmith::grant::Grantable::DerivedAlternativeCast(
                    convert_derived_alternative_cast(spec)?,
                )
            }
        })
    }

    fn runtime_grant_duration_hook(
        &mut self,
        duration: compiler::grant::GrantDuration,
    ) -> Result<ironsmith::grant::GrantDuration, Self::Error> {
        match duration {
            compiler::grant::GrantDuration::Forever => Ok(ironsmith::grant::GrantDuration::Forever),
            compiler::grant::GrantDuration::UntilEndOfTurn => {
                Ok(ironsmith::grant::GrantDuration::UntilEndOfTurn)
            }
            compiler::grant::GrantDuration::UntilYourNextTurn => {
                Ok(ironsmith::grant::GrantDuration::UntilYourNextTurn)
            }
            compiler::grant::GrantDuration::UntilYourNextTurnEnd => {
                Ok(ironsmith::grant::GrantDuration::UntilYourNextTurnEnd)
            }
        }
    }

    fn runtime_grant_spec_hook(
        &mut self,
        spec: compiler::grant::GrantSpec,
    ) -> Result<ironsmith::grant::GrantSpec, Self::Error> {
        Ok(ironsmith::grant::GrantSpec {
            grantable: self.runtime_grantable_hook(spec.grantable)?,
            filter: spec.filter,
            zone: spec.zone,
            additional_zones: spec.additional_zones,
            beneficiary: spec.beneficiary,
            usage_limit: spec.usage_limit,
            max_plays: spec.max_plays,
            cast_this_way_filter: spec.cast_this_way_filter,
            on_use_effects: spec.on_use_effects.into_iter().map(runtime_effect_from_core_model).collect::<Result<_, _>>()?,
            requires_linked_exile_pair: spec.requires_linked_exile_pair,
            may_look_at_linked_exile: spec.may_look_at_linked_exile,
            cast_mana_spend_mode: spec.cast_mana_spend_mode,
            linked_exile_pair: spec.linked_exile_pair,
            linked_exile_class_level: spec.linked_exile_class_level,
            source_exiled_surface: spec.source_exiled_surface,
            filtered_zone_surface: spec.filtered_zone_surface,
            top_card_only: spec.top_card_only,
            instant_timing: spec.instant_timing,
            may_look_at_top: spec.may_look_at_top,
            cast_this_way_grants: spec
                .cast_this_way_grants
                .into_iter()
                .map(|ability| self.runtime_static_ability_hook(ability))
                .collect::<Result<Vec<_>, _>>()?,
            permanent_this_way_grants: spec
                .permanent_this_way_grants
                .into_iter()
                .map(|ability| self.runtime_static_ability_hook(ability))
                .collect::<Result<Vec<_>, _>>()?,
        })
    }

    fn retain_runtime_effect_model_hook(
        &mut self,
        model: &compiler::effect::Effect,
        effect: ironsmith::effect::Effect,
    ) -> Result<ironsmith::effect::Effect, Self::Error> {
        let wire: ironsmith_compiled_artifact::WireEffect =
            serde_json::from_value(serde_json::to_value(model).map_err(|error| {
                CompilerIntegrationError::ArtifactEncoding {
                    detail: error.to_string(),
                }
            })?)
            .map_err(|error| CompilerIntegrationError::ArtifactEncoding {
                detail: error.to_string(),
            })?;
        let json = serde_json::to_string(&wire).map_err(|error| {
            CompilerIntegrationError::ArtifactEncoding {
                detail: error.to_string(),
            }
        })?;
        Ok(effect.with_serialized_model(json))
    }

    fn runtime_external_model_effect_hook(
        &mut self,
        effect: &compiler::effect::Effect,
    ) -> Result<Option<ironsmith::effect::Effect>, Self::Error> {
        if let Some(payload) =
            effect.downcast_ref::<compiler::effects::cards::ImprintFromHandEffect>()
        {
            return Ok(Some(ironsmith::effect::Effect::new(
                ironsmith::effects::cards::ImprintFromHandEffect::new(payload.filter.clone()),
            )));
        }
        if let Some(payload) = effect.downcast_ref::<compiler::effects::ScaleXValueEffect>() {
            return Ok(Some(ironsmith::effect::Effect::scale_x_value(
                payload.target.clone(),
                payload.multiplier,
            )));
        }
        Ok(None)
    }
}

fn runtime_effect_from_core_model(
    effect: compiler::effect::Effect,
) -> Result<ironsmith::effect::Effect, CompilerIntegrationError> {
    ironsmith::effect_model_interpreter::interpret_effect_model::<CompilerEffectModel, _>(
        effect,
        &mut CompilerEffectModelHooks,
    )
}

fn remove_redundant_target_only_effects_in_program(
    program: &mut ironsmith::resolution::ResolutionProgram,
) {
    ironsmith::effect_model_interpreter::prune_redundant_target_only_effects_in_program(program);
}

fn runtime_cost_from_core_model(
    cost: compiler::costs::Cost,
) -> Result<ironsmith::costs::Cost, CompilerIntegrationError> {
    let model = cost.try_map_effect(runtime_effect_from_core_model)?;
    ironsmith::costs::Cost::from_model(model)
        .map_err(|detail| CompilerIntegrationError::UnsupportedEffect { detail })
}

fn runtime_optional_cost_from_core_model(
    cost: compiler::cost::OptionalCost,
) -> Result<ironsmith::cost::OptionalCost, CompilerIntegrationError> {
    cost.try_map(runtime_cost_from_core_model)
}

fn convert_alternative_cast(
    method: compiler::alternative_cast::AlternativeCastingMethod,
) -> Result<ironsmith::alternative_cast::AlternativeCastingMethod, CompilerIntegrationError> {
    let mut method =
        method.try_map(runtime_effect_from_core_model, runtime_cost_from_core_model)?;
    if let ironsmith::alternative_cast::AlternativeCastingMethod::Overload { effects, .. } =
        &mut method
    {
        *effects = effects
            .drain(..)
            .filter_map(detarget_overload_effect)
            .collect();
    }
    Ok(method)
}

fn detarget_overload_effect(
    effect: ironsmith::effect::Effect,
) -> Option<ironsmith::effect::Effect> {
    if effect
        .downcast_ref::<ironsmith::effects::TargetOnlyEffect>()
        .is_some()
    {
        return None;
    }

    if let Some(tagged) = effect.downcast_ref::<ironsmith::effects::TaggedEffect>() {
        let inner = detarget_overload_effect((*tagged.effect).clone())?;
        return Some(ironsmith::effect::Effect::new(
            ironsmith::effects::TaggedEffect::new(tagged.tag.clone(), inner),
        ));
    }

    if let Some(apply) = effect.downcast_ref::<ironsmith::effects::ApplyContinuousEffect>()
        && let Some(ironsmith::target::ChooseSpec::Target(inner)) = &apply.target_spec
        && let ironsmith::target::ChooseSpec::Object(filter) = inner.as_ref()
    {
        let mut detargeted = apply.clone();
        detargeted.target = ironsmith::continuous::EffectTarget::Filter(filter.clone());
        detargeted.target_spec = Some(ironsmith::target::ChooseSpec::Object(filter.clone()));
        detargeted.require_creature_target = false;
        return Some(ironsmith::effect::Effect::new(detargeted));
    }

    Some(effect)
}

fn convert_derived_alternative_cast(
    spec: compiler::grant::DerivedAlternativeCast,
) -> Result<ironsmith::grant::DerivedAlternativeCast, CompilerIntegrationError> {
    Ok(match spec {
        compiler::grant::DerivedAlternativeCast::FlashbackFromCardManaCost { additional_costs } => {
            ironsmith::grant::DerivedAlternativeCast::FlashbackFromCardManaCost {
                additional_costs: additional_costs
                    .into_iter()
                    .map(runtime_cost_from_core_model)
                    .collect::<Result<Vec<_>, _>>()?,
            }
        }
        compiler::grant::DerivedAlternativeCast::EscapeFromCardManaCost { exile_count } => {
            ironsmith::grant::DerivedAlternativeCast::EscapeFromCardManaCost { exile_count }
        }
        compiler::grant::DerivedAlternativeCast::RetraceFromCardManaCost => {
            ironsmith::grant::DerivedAlternativeCast::RetraceFromCardManaCost
        }
        compiler::grant::DerivedAlternativeCast::BlitzFromCardManaCost => {
            ironsmith::grant::DerivedAlternativeCast::BlitzFromCardManaCost
        }
        compiler::grant::DerivedAlternativeCast::EmergeFromCardManaCost => {
            ironsmith::grant::DerivedAlternativeCast::EmergeFromCardManaCost
        }
        compiler::grant::DerivedAlternativeCast::MiracleFromCardManaCostReducedBy { reduction } => {
            ironsmith::grant::DerivedAlternativeCast::MiracleFromCardManaCostReducedBy { reduction }
        }
        compiler::grant::DerivedAlternativeCast::ManaValueAsGenericFromHand => {
            ironsmith::grant::DerivedAlternativeCast::ManaValueAsGenericFromHand
        }
        compiler::grant::DerivedAlternativeCast::MadnessFromCardManaCost => {
            ironsmith::grant::DerivedAlternativeCast::MadnessFromCardManaCost
        }
        compiler::grant::DerivedAlternativeCast::LifeEqualManaValueFromHand { usage_limit } => {
            ironsmith::grant::DerivedAlternativeCast::LifeEqualManaValueFromHand { usage_limit }
        }
        compiler::grant::DerivedAlternativeCast::LifeEqualManaValueFromZone {
            zone,
            usage_limit,
        } => ironsmith::grant::DerivedAlternativeCast::LifeEqualManaValueFromZone {
            zone,
            usage_limit,
        },
        compiler::grant::DerivedAlternativeCast::GraveyardCastFromCardManaCost {
            additional_costs,
            usage_limit,
            condition,
            exiles_after_resolution,
        } => ironsmith::grant::DerivedAlternativeCast::GraveyardCastFromCardManaCost {
            additional_costs: additional_costs
                .into_iter()
                .map(runtime_cost_from_core_model)
                .collect::<Result<Vec<_>, _>>()?,
            usage_limit,
            condition,
            exiles_after_resolution,
        },
    })
}

fn runtime_static_ability_model(
    ability: compiler::static_abilities::StaticAbility,
) -> Result<ironsmith::static_abilities::CompiledStaticAbility, CompilerIntegrationError> {
    ability.try_map(
        runtime_trigger_from_core_model,
        runtime_effect_from_core_model,
        runtime_cost_from_core_model,
        // Runtime abilities already carry resolved conditions.
        Ok,
    )
}

fn runtime_static_ability(
    ability: compiler::static_abilities::StaticAbility,
) -> Result<ironsmith::static_abilities::StaticAbility, CompilerIntegrationError> {
    Ok(ironsmith::static_abilities::StaticAbility::from_model(
        runtime_static_ability_model(ability)?,
    ))
}

fn runtime_trigger_from_core_model(
    trigger: compiler::triggers::Trigger,
) -> Result<ironsmith::triggers::Trigger, CompilerIntegrationError> {
    ironsmith::triggers::Trigger::from_model(trigger)
        .map_err(|err| CompilerIntegrationError::UnsupportedTrigger { detail: err.detail })
}

fn runtime_ability_from_core_model(
    ability: compiler::ability::Ability,
) -> Result<ironsmith::ability::Ability, CompilerIntegrationError> {
    let mut converted = ability.try_map(
        runtime_static_ability,
        runtime_trigger_from_core_model,
        runtime_effect_from_core_model,
        runtime_cost_from_core_model,
        // Runtime abilities already carry resolved conditions.
        Ok,
    )?;
    match &mut converted.kind {
        ironsmith::ability::AbilityKind::Triggered(triggered) => {
            remove_redundant_target_only_effects_in_program(&mut triggered.effects);
        }
        ironsmith::ability::AbilityKind::Activated(activated) => {
            remove_redundant_target_only_effects_in_program(&mut activated.effects);
        }
        ironsmith::ability::AbilityKind::Static(_) => {}
    }
    Ok(converted)
}

fn combine_level_ability_statics(
    abilities: Vec<ironsmith::ability::Ability>,
) -> Vec<ironsmith::ability::Ability> {
    let mut out = Vec::with_capacity(abilities.len());
    let mut groups: Vec<(
        Vec<ironsmith::zone::Zone>,
        Vec<ironsmith::ability::LevelAbility>,
    )> = Vec::new();

    for ability in abilities {
        let ironsmith::ability::AbilityKind::Static(static_ability) = &ability.kind else {
            out.push(ability);
            continue;
        };
        let Some(level_abilities) = static_ability.level_abilities() else {
            out.push(ability);
            continue;
        };
        // Combining level rows must not broaden or discard their source zones.
        if let Some((_, levels)) = groups
            .iter_mut()
            .find(|(zones, _)| *zones == ability.functional_zones)
        {
            levels.extend(level_abilities.iter().cloned());
        } else {
            groups.push((ability.functional_zones, level_abilities.to_vec()));
        }
    }

    for (zones, levels) in groups {
        out.push(
            ironsmith::ability::Ability::static_ability(
                ironsmith::static_abilities::StaticAbility::with_level_abilities(levels),
            )
            .in_zones(zones),
        );
    }
    out
}

const CLASS_LEVEL_MARKER_PREFIX: &str = "__ironsmith_class_level:";

fn class_level_marker(ability: &ironsmith::ability::ActivatedAbility) -> Option<u32> {
    if let Some(ironsmith_core::ActivatedAbilityKeyword::ClassLevel(level)) = ability.keyword {
        return Some(level);
    }
    // Previously admitted definitions retain their legacy runtime route. New
    // definition-local pairing consumes only the typed keyword above.
    ability
        .additional_restrictions
        .iter()
        .find_map(|restriction| restriction.strip_prefix(CLASS_LEVEL_MARKER_PREFIX))
        .and_then(|level| level.parse::<u32>().ok())
}

fn class_level_activation_condition(level: u32) -> ironsmith::ConditionExpr {
    // CR 716.2a: "Level N" can be activated only while the Class is level
    // N-1. Levels are a designation, not level counters (CR 716.4).
    let previous = level.saturating_sub(1).max(1);
    ironsmith::ConditionExpr::And(
        Box::new(ironsmith::ConditionExpr::SourceClassLevelAtLeast(previous)),
        Box::new(ironsmith::ConditionExpr::Not(Box::new(
            ironsmith::ConditionExpr::SourceClassLevelAtLeast(previous + 1),
        ))),
    )
}

fn and_condition(
    left: Option<ironsmith::ConditionExpr>,
    right: ironsmith::ConditionExpr,
) -> ironsmith::ConditionExpr {
    left.map(|left| ironsmith::ConditionExpr::And(Box::new(left), Box::new(right.clone())))
        .unwrap_or(right)
}

fn apply_class_level_runtime_gates(definition: &mut ironsmith::cards::CardDefinition) {
    if !definition
        .card
        .subtypes
        .contains(&ironsmith::Subtype::Class)
    {
        return;
    }

    let mut current_level = None;
    for ability in &mut definition.abilities {
        if let ironsmith::ability::AbilityKind::Activated(activated) = &mut ability.kind
            && let Some(level) = class_level_marker(activated)
        {
            activated.activation_condition = Some(and_condition(
                activated.activation_condition.take(),
                class_level_activation_condition(level),
            ));
            // The level ability sets the Class's level designation instead of
            // putting a level counter on it (CR 716.2b).
            activated.effects = vec![ironsmith::effect::Effect::new(
                ironsmith::effects::SetClassLevelEffect::new(level),
            )]
            .into();
            current_level = Some(level);
            continue;
        }

        let Some(level) = current_level else {
            continue;
        };
        if let ironsmith::ability::AbilityKind::Static(static_ability) = &mut ability.kind {
            // Classes start at level 1. Grant the entire static ability so its
            // existing conditions stay intact.
            *static_ability = ironsmith::static_abilities::StaticAbility::new(
                ironsmith::static_abilities::GrantAbility::source(static_ability.clone())
                    .with_condition(ironsmith::ConditionExpr::SourceClassLevelAtLeast(level)),
            );
        }
        if let ironsmith::ability::AbilityKind::Triggered(triggered) = &mut ability.kind
            && triggered.presentation_label.is_none()
        {
            triggered.presentation_label =
                Some(ironsmith::ability::PresentationLabel::from_ability_word(
                    format!("{CLASS_LEVEL_MARKER_PREFIX}{level}"),
                ));
        }
    }
}

fn runtime_definition_from_core_model(
    definition: compiler::CardDefinition,
) -> Result<ironsmith::cards::CardDefinition, CompilerIntegrationError> {
    let mut definition = definition.try_map(
        runtime_ability_from_core_model,
        runtime_effect_from_core_model,
        runtime_cost_from_core_model,
        convert_alternative_cast,
        runtime_optional_cost_from_core_model,
    )?;
    definition.abilities = combine_level_ability_statics(definition.abilities);
    apply_class_level_runtime_gates(&mut definition);
    if let Some(spell_effect) = &mut definition.spell_effect {
        remove_redundant_target_only_effects_in_program(spell_effect);
    }
    Ok(definition)
}

fn attach_rendered_presentation(
    mut definition: ironsmith::cards::CardDefinition,
) -> ironsmith::cards::CardDefinition {
    definition.canonical_text = ironsmith_text::compiled_text_lines(&definition).join("\n");
    definition.ability_labels = ironsmith_text::ability_surface_texts(&definition);
    definition
}

pub fn into_runtime_definition(
    definition: compiler::CardDefinition,
) -> Result<ironsmith::cards::CardDefinition, CompilerIntegrationError> {
    runtime_definition_from_core_model(definition).map(attach_rendered_presentation)
}

pub fn into_runtime_compiled_card_text(
    compiled: compiler::CompiledCardText<compiler::CardDefinition>,
) -> Result<compiler::CompiledCardText<ironsmith::cards::CardDefinition>, CompilerIntegrationError>
{
    Ok(compiler::CompiledCardText {
        definition: into_runtime_definition(compiled.definition)?,
        annotations: compiled.annotations,
    })
}

pub fn compile_to_runtime_definition(
    name: &str,
    text: impl Into<String>,
    allow_unsupported: bool,
) -> Result<ironsmith::cards::CardDefinition, CompilerIntegrationError> {
    let builder = compiler::CardDefinitionBuilder::new(ironsmith::ids::CardId::new(), name);
    compile_builder_to_runtime_definition(builder, text, allow_unsupported)
}

/// Compile source into the same typed transport artifact used by baked catalogs.
pub fn compile_to_artifact(
    name: &str,
    text: impl Into<String>,
    allow_unsupported: bool,
) -> Result<(CompiledCardArtifact, ironsmith::cards::CardDefinition), CompilerIntegrationError> {
    let builder = compiler::CardDefinitionBuilder::new(ironsmith::ids::CardId::new(), name);
    compile_builder_to_artifact(builder, text, allow_unsupported)
}

pub fn compile_builder_to_runtime_definition(
    builder: compiler::CardDefinitionBuilder,
    text: impl Into<String>,
    allow_unsupported: bool,
) -> Result<ironsmith::cards::CardDefinition, CompilerIntegrationError> {
    let text = text.into();
    let compiled = compile_builder_to_runtime_compiled_card_text(builder, text, allow_unsupported)?;
    Ok(compiled.definition)
}

/// Compile source at the compiler/runtime boundary and emit a deterministic
/// transport envelope beside the materialized runtime definition.
pub fn compile_builder_to_artifact(
    builder: compiler::CardDefinitionBuilder,
    text: impl Into<String>,
    allow_unsupported: bool,
) -> Result<(CompiledCardArtifact, ironsmith::cards::CardDefinition), CompilerIntegrationError> {
    let source = text.into();
    let compiled = compiler::CompilerFacade::new().compile_definition(
        builder,
        source.clone(),
        compiler::CompilePolicy { allow_unsupported },
    )?;
    let wire_definition =
        wire_definition_from_serializable(&compiled.definition).map_err(|error| {
            CompilerIntegrationError::ArtifactEncoding {
                detail: error.to_string(),
            }
        })?;
    let rendered_definition = into_runtime_definition(compiled.definition.clone())?;
    let canonical_text = rendered_definition.canonical_text.clone();
    let ability_labels = rendered_definition.ability_labels.clone();
    let linked_face_layout = match compiled.definition.card.linked_face_layout {
        ironsmith::card::LinkedFaceLayout::None => None,
        layout => Some(format!("{layout:?}")),
    };
    let mut artifact = CompiledCardArtifact::new(
        ArtifactCardIdentity {
            local_id: ArtifactCardId(1),
            name: compiled.definition.card.name.clone(),
            face_name: None,
            other_face: compiled
                .definition
                .card
                .other_face
                .map(|_| ArtifactCardId(2)),
            linked_face_layout,
        },
        CompiledCardPayload {
            definition: wire_definition,
            canonical_text,
            ability_labels,
        },
        concat!("ironsmith-compiler/", env!("CARGO_PKG_VERSION")),
        source.as_bytes(),
    );
    artifact.compiler_facts.insert(
        "allowUnsupported".to_string(),
        allow_unsupported.to_string(),
    );
    artifact.refresh_checksum();
    let definition = ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(
        &artifact,
    )
    .map_err(|error| CompilerIntegrationError::ArtifactMaterialization {
        detail: error.to_string(),
    })?;
    Ok((artifact, definition))
}

#[derive(Debug, Clone)]
pub struct RuntimeBuilderSnapshot {
    pub card: ironsmith::card::Card,
    pub has_fuse: bool,
}

impl RuntimeBuilderSnapshot {
    fn into_compiler_builder(self) -> compiler::CardDefinitionBuilder {
        let mut builder = compiler::CardDefinitionBuilder::new(self.card.id, self.card.name);

        if let Some(cost) = self.card.mana_cost {
            builder = builder.mana_cost(cost);
        }
        if let Some(colors) = self.card.color_indicator {
            builder = builder.color_indicator(colors);
        }
        builder = builder
            .supertypes(self.card.supertypes)
            .card_types(self.card.card_types)
            .subtypes(self.card.subtypes)
            .attraction_lights(self.card.attraction_lights)
            .linked_face_layout(self.card.linked_face_layout);
        if let Some(pt) = self.card.power_toughness {
            builder = builder.power_toughness(pt);
        }
        if let Some(loyalty) = self.card.loyalty {
            builder = builder.loyalty(loyalty);
        }
        if let Some(defense) = self.card.defense {
            builder = builder.defense(defense);
        }
        if let Some(face) = self.card.other_face {
            builder = builder.other_face(face);
        }
        if let Some(face_name) = self.card.other_face_name {
            builder = builder.other_face_name(face_name);
        }
        if self.card.is_token {
            builder = builder.token();
        }
        if self.has_fuse {
            builder = builder.has_fuse();
        }

        builder
    }
}

pub fn compile_runtime_builder_snapshot_to_runtime_definition(
    snapshot: RuntimeBuilderSnapshot,
    text: impl Into<String>,
    allow_unsupported: bool,
) -> Result<ironsmith::cards::CardDefinition, CompilerIntegrationError> {
    compile_builder_to_runtime_definition(snapshot.into_compiler_builder(), text, allow_unsupported)
}

pub fn compile_runtime_builder_snapshot_to_runtime_compiled_card_text(
    snapshot: RuntimeBuilderSnapshot,
    text: impl Into<String>,
    allow_unsupported: bool,
) -> Result<compiler::CompiledCardText<ironsmith::cards::CardDefinition>, CompilerIntegrationError>
{
    compile_builder_to_runtime_compiled_card_text(
        snapshot.into_compiler_builder(),
        text,
        allow_unsupported,
    )
}

pub fn compile_builder_to_runtime_compiled_card_text(
    builder: compiler::CardDefinitionBuilder,
    text: impl Into<String>,
    allow_unsupported: bool,
) -> Result<compiler::CompiledCardText<ironsmith::cards::CardDefinition>, CompilerIntegrationError>
{
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
    use ironsmith::ids::PlayerId;
    use ironsmith::types::CardType;
    use ironsmith::zone::Zone;

    #[test]
    fn compiled_leader_replacement_keeps_the_original_conniving_creature() {
        use ironsmith::ability::AbilityKind;
        use ironsmith::effects::{EffectContext, ResolvedTarget, execute_effect};
        use ironsmith::events::{KeywordActionEvent, KeywordActionKind};
        use ironsmith::object::CounterType;

        let (_, definition) = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(ironsmith::ids::CardId::new(), "Leader, Super-Genius"),
            "Mana cost: {2}{U}{U}\nType: Legendary Creature — Gamma Scientist Villain\nPower/Toughness: 1/3\nIf a creature you control would connive, instead you draw a card, then that creature connives.\nAt the beginning of combat on your turn, target creature you control connives.",
            false,
        )
        .expect("Leader should compile and materialize through its artifact");
        let trigger = definition
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Triggered(trigger) => Some(trigger),
                _ => None,
            })
            .expect("Leader has a beginning-of-combat trigger");
        let alice = PlayerId::from_index(0);

        for choose_leader in [false, true] {
            let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let leader = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            let creature_card =
                ironsmith::card::CardBuilder::new(ironsmith::ids::CardId::new(), "Other creature")
                    .card_types(vec![CardType::Creature])
                    .power_toughness(ironsmith::card::PowerToughness::fixed(2, 2))
                    .build();
            let other = game.create_object_from_card(&creature_card, alice, Zone::Battlefield);
            for _ in 0..3 {
                game.create_object_from_card(&creature_card, alice, Zone::Library);
            }
            let conniver = if choose_leader { leader } else { other };
            let bystander = if choose_leader { other } else { leader };
            let mut ctx = EffectContext::new_default(leader, alice)
                .with_targets(vec![ResolvedTarget::Object(conniver)]);
            let mut events = Vec::new();
            for effect in trigger.effects.all_effects() {
                events.extend(execute_effect(&mut game, effect, &mut ctx).unwrap().events);
            }

            let player = game.player(alice).unwrap();
            assert_eq!(
                player.library.len(),
                1,
                "replacement and connive each draw once"
            );
            assert_eq!(player.hand.len(), 1, "two draws and one discard");
            assert_eq!(player.graveyard.len(), 1, "connive discards exactly once");
            let connives = events
                .iter()
                .filter_map(|event| event.downcast::<KeywordActionEvent>())
                .filter(|event| event.action == KeywordActionKind::Connive)
                .map(|event| event.source)
                .collect::<Vec<_>>();
            assert_eq!(
                connives,
                vec![conniver],
                "only the selected creature connives"
            );
            assert_eq!(
                game.object(conniver)
                    .unwrap()
                    .counters
                    .get(&CounterType::PlusOnePlusOne),
                Some(&1),
                "the original conniver gets the discarded nonland's counter",
            );
            assert_eq!(
                game.object(bystander)
                    .unwrap()
                    .counters
                    .get(&CounterType::PlusOnePlusOne),
                None,
                "the unselected creature must not receive a connive counter",
            );
        }
    }

    #[test]
    fn converts_assign_no_combat_damage_effect_payload() {
        let compiler_effect = compiler::effect::Effect::assign_no_combat_damage(
            compiler::target::ChooseSpec::Source,
            compiler::effect::Until::EndOfTurn,
        );

        let runtime_effect = runtime_effect_from_core_model(compiler_effect)
            .expect("assignment suppression should cross the compiler/runtime bridge");
        let suppression = runtime_effect
            .downcast_ref::<ironsmith::effects::AssignNoCombatDamageEffect>()
            .expect("runtime payload should remain assignment suppression");

        assert_eq!(suppression.source, ironsmith::target::ChooseSpec::Source);
        assert_eq!(suppression.until, ironsmith::effect::Until::EndOfTurn);
    }

    #[test]
    fn converts_tag_other_block_participant_effect_payload() {
        let filter = compiler::target::ObjectFilter::creature();
        let compiler_effect = compiler::effect::Effect::tag_other_block_participant(
            "other_block_participant",
            Some(filter.clone()),
        );

        let runtime_effect = runtime_effect_from_core_model(compiler_effect)
            .expect("block-participant tagging should cross the compiler/runtime bridge");
        let tagging = runtime_effect
            .downcast_ref::<ironsmith::effects::TagOtherBlockParticipantEffect>()
            .expect("runtime payload should remain block-participant tagging");

        assert_eq!(tagging.tag.as_str(), "other_block_participant");
        assert_eq!(tagging.filter.as_ref(), Some(&filter));
    }

    fn cast_payment_probe(
        game: &mut ironsmith::GameState,
        spell: ironsmith::ids::ObjectId,
        method: ironsmith::alternative_cast::CastingMethod,
    ) {
        use ironsmith::game_loop::*;
        let from_zone = game.object(spell).unwrap().zone;
        let action = ironsmith::decision::LegalAction::CastSpell {
            spell_id: spell,
            from_zone,
            casting_method: method,
        };
        let mut state = PriorityLoopState::new(2);
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
        let mut progress = apply_priority_response_with_dm(
            game,
            &mut queue,
            &mut state,
            &PriorityResponse::PriorityAction(action.clone()),
            &mut dm,
        )
        .unwrap_or_else(|error| {
            panic!(
                "{action:?} ({:?}): {error:?}",
                game.object(spell).map(|object| &object.name)
            )
        });
        for _ in 0..30 {
            if state.pending_cast.is_none() && !game.stack.is_empty() {
                break;
            }
            if let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress {
                progress =
                    apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, &mut dm)
                        .unwrap();
            } else {
                break;
            }
        }
        assert!(state.pending_cast.is_none(), "casting must finish");
        assert_eq!(game.stack.len(), 1);
    }

    #[test]
    fn impending_and_web_slinging_execute_their_compiled_costs() {
        use ironsmith::{alternative_cast::CastingMethod, object::CounterType, types::CardType};
        let alice = PlayerId::from_index(0);
        let impending = compile_to_runtime_definition("Impending probe",
            "Mana cost: {0}\nType: Enchantment Creature — Avatar\nPower/Toughness: 4/4\nImpending 2—{0}", false).unwrap();
        for paid in [false, true] {
            let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            game.turn.phase = ironsmith::game_state::Phase::FirstMain;
            let spell = game.create_object_from_definition(&impending, alice, Zone::Hand);
            cast_payment_probe(
                &mut game,
                spell,
                if paid {
                    CastingMethod::Alternative(0)
                } else {
                    CastingMethod::Normal
                },
            );
            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
            let permanent = *game.battlefield.last().unwrap();
            assert_eq!(
                game.object(permanent)
                    .unwrap()
                    .counters
                    .get(&CounterType::Time)
                    .copied()
                    .unwrap_or(0),
                if paid { 2 } else { 0 }
            );
            assert_eq!(
                game.object_has_card_type(permanent, CardType::Creature),
                !paid
            );
        }
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.phase = ironsmith::game_state::Phase::FirstMain;
        let fodder = game.create_object_from_definition(&impending, alice, Zone::Battlefield);
        game.tap(fodder);
        let web = compile_to_runtime_definition(
            "Web probe",
            "Mana cost: {5}\nType: Creature — Human\nPower/Toughness: 2/2\nWeb-slinging {0}",
            false,
        )
        .unwrap();
        let spell = game.create_object_from_definition(&web, alice, Zone::Hand);
        cast_payment_probe(&mut game, spell, CastingMethod::Alternative(0));
        assert!(!game.battlefield.contains(&fodder));
        assert!(
            game.player(alice)
                .unwrap()
                .hand
                .iter()
                .any(|id| game.object(*id).unwrap().name == "Impending probe")
        );
    }

    #[test]
    fn mayhem_requires_this_turns_discard_and_does_not_exile_after_resolution() {
        use ironsmith::alternative_cast::CastingMethod;
        let alice = PlayerId::from_index(0);
        let definition = compile_to_runtime_definition(
            "Mayhem probe",
            "Mana cost: {5}\nType: Sorcery\nMayhem {0}\nYou gain 1 life.",
            false,
        )
        .unwrap();
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.phase = ironsmith::game_state::Phase::FirstMain;
        let ordinary = game.create_object_from_definition(&definition, alice, Zone::Graveyard);
        assert!(!ironsmith::decision::compute_legal_actions(&game, alice).expect("fixture has complete replacement state").iter().any(|action|
            matches!(action, ironsmith::decision::LegalAction::CastSpell { spell_id, .. } if *spell_id == ordinary)));
        let card = game.create_object_from_definition(&definition, alice, Zone::Hand);
        let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
        let mut ctx = ironsmith::effects::EffectContext::new(card, alice, &mut dm);
        ironsmith::effects::execute_effect(&mut game, &ironsmith::Effect::discard(1), &mut ctx)
            .unwrap();
        let discarded = *game.player(alice).unwrap().graveyard.last().unwrap();
        assert!(ironsmith::decision::compute_legal_actions(&game, alice).expect("fixture has complete replacement state").iter().any(|action|
            matches!(action, ironsmith::decision::LegalAction::CastSpell { spell_id, casting_method: CastingMethod::Alternative(0), .. } if *spell_id == discarded)));
        cast_payment_probe(&mut game, discarded, CastingMethod::Alternative(0));
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert!(game.exile.is_empty());
        assert_eq!(game.player(alice).unwrap().life, 21);

        let land =
            compile_to_runtime_definition("Mayhem land", "Type: Land\nMayhem", false).unwrap();
        let card = game.create_object_from_definition(&land, alice, Zone::Hand);
        let mut ctx = ironsmith::effects::EffectContext::new(card, alice, &mut dm);
        ironsmith::effects::execute_effect(&mut game, &ironsmith::Effect::discard(1), &mut ctx)
            .unwrap();
        let discarded_land = *game.player(alice).unwrap().graveyard.last().unwrap();
        assert!(ironsmith::decision::compute_legal_actions(&game, alice).expect("fixture has complete replacement state").iter().any(|action|
            matches!(action, ironsmith::decision::LegalAction::PlayLand { land_id } if *land_id == discarded_land)),
            "costless Mayhem also permits playing a discarded land");
        let mut next_turn = game.clone();
        next_turn.turn_store.turn_history.clear_for_new_turn();
        assert!(!ironsmith::decision::compute_legal_actions(&next_turn, alice).expect("fixture has complete replacement state").iter().any(|action|
            matches!(action, ironsmith::decision::LegalAction::PlayLand { land_id } if *land_id == discarded_land)));
        ironsmith::special_actions::perform(
            ironsmith::special_actions::SpecialAction::PlayLand {
                card_id: discarded_land,
            },
            &mut game,
            alice,
            &mut dm,
        )
        .unwrap();
        assert!(
            game.battlefield
                .iter()
                .any(|id| game.object(*id).unwrap().name == "Mayhem land")
        );
    }

    #[test]
    fn more_than_meets_the_eye_uses_the_linked_back_face() {
        use ironsmith::alternative_cast::CastingMethod;
        let alice = PlayerId::from_index(0);
        let mut front = compile_to_runtime_definition("Converted front", "Mana cost: {5}\nType: Creature — Robot\nPower/Toughness: 4/4\nMore than meets the eye {0}", false).unwrap();
        let back =
            compile_to_runtime_definition("Converted back", "Type: Artifact — Vehicle", false)
                .unwrap();
        front.card.other_face = Some(back.card.id);
        front.card.other_face_name = Some(back.card.name.to_string());
        front.card.linked_face_layout = ironsmith::card::LinkedFaceLayout::TransformLike;
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.phase = ironsmith::game_state::Phase::FirstMain;
        game.register_linked_face_definition(&back);
        let source = game.create_object_from_definition(&front, alice, Zone::Hand);
        assert!(ironsmith::decision::compute_legal_actions(&game, alice).expect("fixture has complete replacement state").iter().any(|action|
            matches!(action, ironsmith::decision::LegalAction::CastSpell { spell_id, casting_method: CastingMethod::Alternative(0), .. } if *spell_id == source)));
        cast_payment_probe(&mut game, source, CastingMethod::Alternative(0));
        let stack = game.stack.last().unwrap().object_id;
        assert_eq!(game.object(stack).unwrap().name, "Converted back");
        assert!(game.object_has_card_type(stack, ironsmith::types::CardType::Artifact));
        assert!(!game.object_has_card_type(stack, ironsmith::types::CardType::Creature));
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert!(
            game.battlefield
                .iter()
                .any(|id| game.object(*id).unwrap().name == "Converted back")
        );
    }

    #[test]
    fn offering_casts_a_creature_during_an_opponents_turn() {
        use ironsmith::alternative_cast::CastingMethod;
        let alice = PlayerId::from_index(0);
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = PlayerId::from_index(1);
        game.turn.priority_player = Some(alice);
        game.turn.phase = ironsmith::game_state::Phase::FirstMain;
        let fodder = compile_to_runtime_definition(
            "Offering fodder",
            "Mana cost: {2}\nType: Creature — Goblin\nPower/Toughness: 1/1",
            false,
        )
        .unwrap();
        let resource = game.create_object_from_definition(&fodder, alice, Zone::Battlefield);
        let spell = compile_to_runtime_definition(
            "Offering timing probe",
            "Mana cost: {2}\nType: Creature — Spirit\nPower/Toughness: 2/2\nGoblin offering",
            false,
        )
        .unwrap();
        let source = game.create_object_from_definition(&spell, alice, Zone::Hand);
        assert!(ironsmith::decision::compute_legal_actions(&game, alice).expect("fixture has complete replacement state").iter().any(|action|
            matches!(action, ironsmith::decision::LegalAction::CastSpell { spell_id, .. } if *spell_id == source)), "Offering permits the otherwise unaffordable creature outside sorcery timing");
        cast_payment_probe(&mut game, source, CastingMethod::Normal);
        assert!(!game.battlefield.contains(&resource));
        assert!(
            game.player(alice)
                .unwrap()
                .graveyard
                .iter()
                .any(|id| game.object(*id).unwrap().name == "Offering fodder")
        );
    }

    #[test]
    fn alternative_payment_keywords_lower_to_executable_costs() {
        for (keyword, expected) in [
            ("Web-slinging {1}{U}", "Web-slinging"),
            ("Mayhem {B}", "Mayhem"),
            ("More than meets the eye {2}{U}", "More than meets the eye"),
            ("Impending 4—{1}{G}", "Impending"),
            ("Emerge from artifact {4}{U}", "Emerge"),
        ] {
            let definition = compile_to_runtime_definition(
                "Payment keyword probe",
                format!("Mana cost: {{4}}\nType: Creature — Human\n{keyword}"),
                false,
            )
            .unwrap();
            assert_eq!(definition.alternative_casts.len(), 1);
            let method = &definition.alternative_casts[0];
            assert_eq!(method.name(), expected);
            if expected == "Web-slinging" {
                // Returning a chosen permanent lowers to selection followed by return.
                assert_eq!(method.non_mana_costs().len(), 2);
                assert!(format!("{:?}", method.non_mana_costs()).contains("tapped: true"));
            } else if expected == "Mayhem" {
                assert_eq!(method.cast_from_zone(), ironsmith::zone::Zone::Graveyard);
                assert!(method.cast_condition().is_some());
                assert!(!method.exiles_after_resolution());
            } else if expected == "Impending" {
                assert_eq!(definition.abilities.len(), 3);
            } else if expected == "More than meets the eye" {
                assert!(method.casts_transformed());
            }
        }
        let mayhem_land =
            compile_to_runtime_definition("Mayhem land probe", "Type: Land\nMayhem", false)
                .unwrap();
        assert_eq!(mayhem_land.abilities.len(), 1);
        assert!(
            format!("{:?}", mayhem_land.abilities)
                .contains("discarded_or_cycled_this_turn_by: Some(You)")
        );
        let offering = compile_to_runtime_definition(
            "Offering probe",
            "Mana cost: {5}{G}\nType: Creature — Spirit\nGoblin offering",
            false,
        )
        .unwrap();
        assert_eq!(offering.optional_costs.len(), 1);
        assert_eq!(
            offering.optional_costs[0].kind,
            ironsmith::cost::OptionalCostKind::Offering
        );
    }

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
            compiler::CardDefinitionBuilder::new(ironsmith::ids::CardId::new(), "Command Tower")
                .card_types(vec![CardType::Land]),
            "{T}: Add one mana of any color in your commander's color identity.",
            false,
        )
        .expect("command tower should compile through runtime compiler integration");

        assert!(definition.card.is_land());
        assert_eq!(definition.abilities.len(), 1);
    }

    #[test]
    fn compile_builder_to_runtime_definition_handles_cumulative_upkeep_payment_from_metadata_builder()
     {
        fn contains_effect<T: 'static>(effect: &ironsmith::effect::Effect) -> bool {
            if effect.downcast_ref::<T>().is_some() {
                return true;
            }
            let mut found = false;
            effect.visit_child_effects(&mut |child| {
                found |= contains_effect::<T>(child);
            });
            found
        }

        // The compiler's deeply nested typed parser legitimately needs more
        // than libtest's small worker-stack default for this long reminder
        // clause. Exercise the same integration on an explicitly sized test
        // thread so the assertion remains structural instead of depending on
        // platform test-runner stack limits.
        let definition = std::thread::Builder::new()
            .name("cumulative-upkeep-compiler-regression".to_string())
            .stack_size(16 * 1024 * 1024)
            .spawn(|| {
                compile_builder_to_runtime_definition(
                    compiler::CardDefinitionBuilder::new(
                        ironsmith::ids::CardId::new(),
                        "Jötun Grunt",
                    )
                    .mana_cost(ironsmith::mana::ManaCost::from_pips(vec![
                        vec![ironsmith::mana::ManaSymbol::Generic(1)],
                        vec![ironsmith::mana::ManaSymbol::White],
                    ]))
                    .card_types(vec![CardType::Creature])
                    .subtypes(vec![
                        ironsmith::types::Subtype::Giant,
                        ironsmith::types::Subtype::Soldier,
                    ])
                    .power_toughness(ironsmith::card::PowerToughness::fixed(4, 4)),
                    "Cumulative upkeep—Put two cards from a single graveyard on the bottom of their owner's library. (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)",
                    false,
                )
                .expect("Jötun Grunt should compile through runtime compiler integration")
            })
            .expect("cumulative-upkeep compiler test thread should start")
            .join()
            .expect("cumulative-upkeep compiler test thread should finish");

        let root_effects = definition
            .abilities
            .iter()
            .flat_map(|ability| match &ability.kind {
                ironsmith::ability::AbilityKind::Triggered(triggered) => {
                    triggered.effects.all_effects()
                }
                ironsmith::ability::AbilityKind::Activated(activated) => {
                    activated.effects.all_effects()
                }
                ironsmith::ability::AbilityKind::Static(_) => Vec::new(),
            });
        let root_effects = root_effects.collect::<Vec<_>>();
        assert!(
            root_effects.iter().any(|effect| contains_effect::<
                ironsmith::effects::CumulativeUpkeepEffect,
            >(effect))
        );
        assert!(
            root_effects
                .iter()
                .any(|effect| contains_effect::<ironsmith::effects::MoveToZoneEffect>(effect))
        );
    }

    #[test]
    fn supported_keyword_mechanics_do_not_lower_to_keyword_markers() {
        let cases = [
            (
                "Grapeshot",
                "Mana cost: {1}{R}\nType: Sorcery\nGrapeshot deals 1 damage to any target.\nStorm",
                "CopySpellEffect",
            ),
            (
                "Alive // Well",
                "Mana cost: {3}{G}\nType: Sorcery\nCreate a 3/3 green Centaur creature token.\nFuse",
                "has_fuse: true",
            ),
            (
                "Akrasan Squire",
                "Mana cost: {W}\nType: Creature — Human Soldier\nPower/Toughness: 1/1\nExalted",
                "exalted_attacker",
            ),
            (
                "Abstruse Interference",
                "Mana cost: {2}{U}\nType: Instant\nDevoid\nCounter target spell unless its controller pays {1}.",
                "MakeColorless",
            ),
            (
                "Accorder Paladin",
                "Mana cost: {1}{W}\nType: Creature — Human Knight\nPower/Toughness: 3/1\nBattle cry",
                "ModifyPowerToughnessEffect",
            ),
            (
                "Adaptive Snapjaw",
                "Mana cost: {4}{G}\nType: Creature — Lizard Beast\nPower/Toughness: 6/2\nEvolve",
                "EvolveEffect",
            ),
            (
                "Bilious Skulldweller",
                "Mana cost: {B}\nType: Creature — Phyrexian Insect\nPower/Toughness: 1/1\nDeathtouch\nToxic 1",
                "Toxic",
            ),
            (
                "Doomed Traveler",
                "Mana cost: {W}\nType: Creature — Human Soldier\nPower/Toughness: 1/1\nAfterlife 1",
                "CreateTokenEffect",
            ),
            (
                "Cached Defenses",
                "Mana cost: {2}{G}\nType: Sorcery\nBolster 3.",
                "BolsterEffect",
            ),
            (
                "Aquastrand Spider",
                "Mana cost: {1}{G}\nType: Creature — Spider Mutant\nPower/Toughness: 0/0\nGraft 2\n{G}: Target creature with a +1/+1 counter on it gains reach until end of turn.",
                "MoveCountersEffect",
            ),
            (
                "Arcbound Worker",
                "Mana cost: {1}\nType: Artifact Creature — Construct\nPower/Toughness: 0/0\nModular 1",
                "modular_triggering_object",
            ),
            (
                "Ronin Houndmaster",
                "Mana cost: {2}{R}\nType: Creature — Human Samurai\nPower/Toughness: 2/2\nBushido 1",
                "ModifyPowerToughnessEffect",
            ),
            (
                "Ulamog's Crusher",
                "Mana cost: {8}\nType: Creature — Eldrazi\nPower/Toughness: 8/8\nAnnihilator 2",
                "SacrificePlayerEffect",
            ),
            (
                "Teysa, Envoy of Ghosts",
                "Mana cost: {5}{W}{B}\nType: Legendary Creature — Human Advisor\nPower/Toughness: 4/4\nProtection from creatures",
                "Protection",
            ),
            (
                "Top Library Fixture",
                "Mana cost: {2}{G}\nType: Creature — Bird\nPower/Toughness: 2/3\nYou may look at the top card of your library any time.",
                "LookAtTopCardOfLibrary",
            ),
            (
                "Mystic Remora",
                "Mana cost: {U}\nType: Enchantment\nCumulative upkeep {1}",
                "CumulativeUpkeepEffect",
            ),
            (
                "Cumulative Discard Fixture",
                "Mana cost: {1}{B}\nType: Enchantment\nCumulative upkeep—Discard a card.",
                "DiscardEffect",
            ),
            (
                "Cumulative Choice Fixture",
                "Mana cost: {G}{W}\nType: Enchantment\nCumulative upkeep {G} or {W}",
                "UnlessActionEffect",
            ),
            (
                "Jötun Grunt",
                "Mana cost: {1}{W}\nType: Creature — Giant Soldier\nPower/Toughness: 4/4\nCumulative upkeep—Put two cards from a single graveyard on the bottom of their owner's library. (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)",
                "MoveToZoneEffect",
            ),
        ];

        for (name, text, expected_debug) in cases {
            let definition = compile_to_runtime_definition(name, text, false)
                .unwrap_or_else(|err| panic!("{name} should compile: {err}"));
            let debug = format!("{definition:#?}");
            assert!(
                !debug.contains("KeywordFallbackText"),
                "{name} should not lower supported mechanics to KeywordFallbackText:\n{debug}"
            );
            assert!(
                !debug.contains("RuleFallbackText"),
                "{name} should not lower supported mechanics to RuleFallbackText:\n{debug}"
            );
            assert!(
                debug.contains(expected_debug),
                "{name} should contain {expected_debug}, got:\n{debug}"
            );
        }
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
            ironsmith::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let object_id = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let object = game.object(object_id).expect("object should exist");

        assert_eq!(object.name, "Llanowar Elves");
        assert_eq!(object.abilities.len(), 1);
        assert!(object.abilities[0].is_mana_ability());
    }

    #[test]
    fn compiled_artifact_materializes_without_the_compiler_bridge() {
        let builder = compiler::CardDefinitionBuilder::new(
            ironsmith::ids::CardId::from_raw(90_001),
            "Artifact Bolt",
        );
        let source = "Mana cost: {R}\nType: Instant\nArtifact Bolt deals 3 damage to any target.";
        let (artifact, definition) = compile_builder_to_artifact(builder, source, false)
            .expect("artifact bolt should compile and materialize");

        artifact
            .validate()
            .expect("artifact checksum should validate");
        assert_eq!(
            artifact.payload.canonical_text,
            "Artifact Bolt deals 3 damage to any target."
        );
        assert!(artifact.payload.ability_labels.is_empty());
        assert_eq!(definition.canonical_text, artifact.payload.canonical_text);
        assert_eq!(definition.card.name, "Artifact Bolt");
        assert!(format!("{definition:#?}").contains("DealDamageEffect"));

        let mut registry = ironsmith::cards::CardRegistry::new();
        registry
            .register_compiled_artifact(&artifact)
            .expect("lean catalog should materialize and register the artifact");
        assert!(registry.get("Artifact Bolt").is_some());
    }

    #[test]
    fn artifact_carries_canonical_yawgmoth_text_into_runtime_display() {
        let builder = compiler::CardDefinitionBuilder::new(
            ironsmith::ids::CardId::from_raw(90_002),
            "Yawgmoth, Thran Physician",
        );
        let source = "Mana cost: {2}{B}{B}\nType: Legendary Creature — Human Cleric\nPower/Toughness: 2/4\nProtection from Humans\nPay 1 life, Sacrifice another creature: Put a -1/-1 counter on up to one target creature and draw a card.\n{B}{B}, Discard a card: Proliferate. (Choose any number of permanents and/or players, then give each another counter of each kind already there.)";
        let expected = "Protection from Humans\nPay 1 life, Sacrifice another creature: Put a -1/-1 counter on up to one target creature and draw a card.\n{B}{B}, Discard a card: Proliferate.";

        let (artifact, definition) = compile_builder_to_artifact(builder, source, false)
            .expect("Yawgmoth should compile and materialize");

        assert_eq!(artifact.payload.canonical_text, expected);
        assert_eq!(definition.canonical_text, expected);
        assert_eq!(
            ironsmith::runtime_display::compiled_text_lines(&definition).join("\n"),
            expected
        );
        assert_eq!(
            artifact.payload.ability_labels,
            expected.lines().map(str::to_string).collect::<Vec<_>>()
        );
        let runtime_action_labels = (0..definition.abilities.len())
            .map(|ability_index| {
                ironsmith::runtime_display::indexed_ability_surface_text(
                    &definition.abilities,
                    &definition.canonical_text,
                    ability_index,
                )
                .expect("each compiled ability should have an action label")
            })
            .collect::<Vec<_>>();
        assert_eq!(runtime_action_labels, artifact.payload.ability_labels);
        assert!(!artifact.payload.canonical_text.contains("Ability kind:"));
        assert!(
            !artifact
                .payload
                .canonical_text
                .contains("StaticAbilityModelInterpreter")
        );
    }

    #[test]
    fn megatron_tyrant_strict_compiles_without_keyword_fallback() {
        let definition = compile_to_runtime_definition(
            "Megatron, Tyrant // Megatron, Destructive Force",
            "Mana cost: {3}{R}{W}{B}\nType: Legendary Artifact Creature — Robot\nPower/Toughness: 7/5\nMore Than Meets the Eye {1}{R}{W}{B} (You may cast this card converted for {1}{R}{W}{B}.)\nYour opponents can't cast spells during combat.\nAt the beginning of each of your postcombat main phases, you may convert Megatron. If you do, add {C} for each 1 life your opponents have lost this turn.",
            false,
        )
        .expect("Megatron should strict-compile without unsupported keyword fallbacks");

        let debug = format!("{definition:#?}");
        assert!(
            !debug.contains("KeywordFallbackText") && !debug.contains("RuleFallbackText"),
            "Megatron should not lower to fallback static abilities:\n{debug}"
        );
        assert!(
            debug
                .to_ascii_lowercase()
                .contains("more than meets the eye {1}{r}{w}{b}")
                && (debug.contains("OpponentsCantCastSpells")
                    || (debug.contains("RuleRestriction") && debug.contains("CastSpellsMatching")))
                && debug.contains("Opponent")
                && debug.contains("ActivationTiming(DuringCombat)")
                && debug.contains("ConvertEffect")
                && debug.contains("AddScaledManaEffect")
                && debug.contains("LifeLostThisTurn"),
            "Megatron should preserve keyword marker and main ability semantics:\n{debug}"
        );
    }
}

#[cfg(test)]
mod functional_zone_tests;

#[cfg(test)]
mod replacement_controller_binding_integration_tests {
    use super::compile_to_runtime_definition;
    use ironsmith::events::ReplacementMatcher;
    use ironsmith::static_abilities::StaticAbilityId;
    use ironsmith::{GameState, PlayerId, Zone};

    #[test]
    fn compiled_confiscate_rebinds_levitation_before_replacement_matching() {
        // The named fixtures use the local cards.json Oracle and metadata.
        // All abilities cross the actual compiler/runtime bridge.
        let confiscate = compile_to_runtime_definition("Confiscate",
            "Mana cost: {4}{U}{U}\nType: Enchantment — Aura\nEnchant permanent\nYou control enchanted permanent.", false)
            .expect("Confiscate must compile through the real compiler/runtime bridge");
        let levitation = compile_to_runtime_definition("Levitation",
            "Mana cost: {2}{U}{U}\nType: Enchantment\nCreatures you control have flying.", false)
            .expect("Levitation must compile through the real compiler/runtime bridge");
        let recipient = compile_to_runtime_definition("Flying grant recipient",
            "Type: Creature\nPower/Toughness: 1/1", false)
            .expect("generic recipient must compile");
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let grantor = game.create_object_from_definition(&levitation, bob, Zone::Battlefield);
        let alice_creature = game.create_object_from_definition(&recipient, alice, Zone::Battlefield);
        let bob_creature = game.create_object_from_definition(&recipient, bob, Zone::Battlefield);
        let entrant = game.create_object_from_definition(&recipient, alice, Zone::Hand);
        let original = game.continuous_query_snapshot().expect("original grant query is finite");
        assert!(!original.current_has_static_ability_id(alice_creature, StaticAbilityId::Flying));
        assert!(original.current_has_static_ability_id(bob_creature, StaticAbilityId::Flying));
        let aura = game.create_object_from_definition(&confiscate, alice, Zone::Battlefield);
        game.object_mut(aura).unwrap().attached_to =
            Some(ironsmith::object::AttachmentTarget::Object(grantor));
        game.object_mut(grantor).unwrap().attachments.push(aura);
        let revision = game.effect_store.continuous_effects.revision();
        let query = game.continuous_query_snapshot().expect("compiled control graph is finite");
        assert!(query.current_has_static_ability_id(aura, StaticAbilityId::ControlAttachedPermanent),
            "the compiler must materialize the control ability");
        assert_eq!(query.current_controller(grantor), Some(alice));
        assert!(query.current_has_static_ability_id(alice_creature, StaticAbilityId::Flying));
        assert!(!query.current_has_static_ability_id(bob_creature, StaticAbilityId::Flying));
        let event = ironsmith::events::EnterBattlefieldEvent::new(entrant, Zone::Hand);
        let matcher = ironsmith::events::zones::matchers::WouldEnterBattlefieldMatcher::new(
            ironsmith::target::ObjectFilter::creature().with_static_ability(StaticAbilityId::Flying));
        let context = ironsmith::events::EventContext::for_controller(alice, &game);
        assert!(matcher.matches_event(&event, &context).expect("compiled entry matching query is finite"));
        assert_eq!(game.effect_store.continuous_effects.revision(), revision);
        assert_eq!(game.object(entrant).unwrap().zone, Zone::Hand);
        assert_eq!(game.object(grantor).unwrap().owner, bob);
        assert_eq!(game.object(aura).unwrap().owner, alice);
    }
}

#[cfg(test)]
mod replacement_source_condition_integration_tests {
    use super::compile_to_runtime_definition;
    use ironsmith::{GameState, PlayerId, Zone, CardType};

    #[test]
    fn compiled_confiscate_rebinds_living_metal_turn_condition() {
        let confiscate = compile_to_runtime_definition("Confiscate",
            "Mana cost: {4}{U}{U}\nType: Enchantment — Aura\nEnchant permanent\nYou control enchanted permanent.", false)
            .expect("real Confiscate must compile");
        let vehicle = compile_to_runtime_definition("Living metal condition recipient",
            "Type: Artifact — Vehicle\nPower/Toughness: 5/5\nLiving metal", false)
            .expect("existing living metal keyword must compile");
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        for active in [alice, bob] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            game.turn.active_player = active;
            let source = game.create_object_from_definition(&vehicle, bob, Zone::Battlefield);
            let original = game.continuous_query_snapshot().expect("original compiled keyword query is finite");
            assert_eq!(original.current_characteristics(source).unwrap().card_types.contains(&CardType::Creature), active == bob);
            let aura = game.create_object_from_definition(&confiscate, alice, Zone::Battlefield);
            game.object_mut(aura).unwrap().attached_to = Some(ironsmith::object::AttachmentTarget::Object(source));
            game.object_mut(source).unwrap().attachments.push(aura);
            let revision = game.effect_store.continuous_effects.revision();
            let query = game.continuous_query_snapshot().expect("compiled controlled keyword query is finite");
            assert_eq!(query.current_controller(source), Some(alice));
            assert_eq!(query.current_characteristics(source).unwrap().card_types.contains(&CardType::Creature), active == alice);
            assert!(query.current_characteristics(source).unwrap().card_types.contains(&CardType::Artifact));
            assert_eq!(game.object(source).unwrap().owner, bob);
            assert_eq!(game.effect_store.continuous_effects.revision(), revision);
        }
    }
}

#[cfg(test)]
mod replacement_spell_control_integration_tests {
    use super::compile_to_runtime_definition;
    use ironsmith::{GameState, PlayerId, Zone};

    #[test]
    fn compiled_aethersnatch_controls_spell_resolution_and_preserves_permanent_base() {
        // Full Oracle and metadata verified against local cards.json.
        let theft = compile_to_runtime_definition("Aethersnatch",
            "Mana cost: {4}{U}{U}\nType: Instant\nGain control of target spell. You may choose new targets for it. (If that spell becomes a permanent, it enters under your control.)", false)
            .expect("Aethersnatch must compile through the real bridge");
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        for permanent in [false, true] {
            let recipient = compile_to_runtime_definition("Compiled spell control recipient",
                if permanent { "Type: Artifact" } else { "Type: Sorcery\nYou gain 1 life." }, false)
                .expect("generic recipient must compile");
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let spell = game.create_object_from_definition(&recipient, alice, Zone::Stack);
            let stable_id = game.object(spell).unwrap().stable_id;
            game.push_to_stack(ironsmith::game_state::StackEntry::new(spell, bob));
            assert_eq!(game.current_controller(spell), Some(bob));
            let stealing_spell = game.create_object_from_definition(&theft, alice, Zone::Stack);
            let mut entry = ironsmith::game_state::StackEntry::new(stealing_spell, alice);
            entry.targets.push(ironsmith::Target::Object(spell));
            game.push_to_stack(entry);
            ironsmith::game_loop::resolve_stack_entry(&mut game).expect("compiled theft resolves");
            assert_eq!(game.current_controller(spell), Some(alice));
            assert_eq!(game.object(spell).unwrap().initial_controller, bob);
            assert_eq!(game.object(spell).unwrap().owner, alice);
            ironsmith::game_loop::resolve_stack_entry(&mut game).expect("controlled compiled recipient resolves");
            let object = game.find_object_by_stable_id(stable_id).unwrap();
            assert_eq!(game.object(object).unwrap().owner, alice);
            if permanent {
                assert_eq!(game.object(object).unwrap().zone, Zone::Battlefield);
                assert_eq!(game.object(object).unwrap().initial_controller, bob);
                assert_eq!(game.current_controller(object), Some(alice));
            } else {
                assert_eq!(game.player(alice).unwrap().life, 21);
                assert_eq!(game.player(bob).unwrap().life, 20);
                assert_eq!(game.object(object).unwrap().zone, Zone::Graveyard);
                assert_eq!(game.object(object).unwrap().initial_controller, alice);
            }
            assert!(game.stack.is_empty());
        }
    }
}

#[cfg(test)]
mod retained_effect_model_tests {
    use super::*;

    fn assert_tree_models(effect: &ironsmith::Effect) -> usize {
        let json = effect
            .serialized_model()
            .expect("every compiled nested effect retains its model");
        let _: ironsmith_compiled_artifact::WireEffect =
            serde_json::from_str(json).expect("canonical wire envelope");
        let mut count = 1;
        effect.visit_child_effects(&mut |child| count += assert_tree_models(child));
        count
    }

    #[test]
    fn retained_effect_model_restores_real_compiled_optional_life_effect() {
        let source_text = "Mana cost: {1}{W}\nType: Sorcery\nYou may gain 3 life.";
        // Exercise both the direct compiler bridge and the artifact decoder.
        let direct = compile_to_runtime_definition("Retained model fixture", source_text, false)
            .expect("strict compiler bridge");
        let (_, artifact) = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(
                ironsmith::ids::CardId::new(),
                "Retained artifact fixture",
            ),
            source_text,
            false,
        )
        .expect("strict artifact bridge");
        for definition in [direct, artifact] {
            let effects = definition
                .spell_effect
                .as_ref()
                .expect("sorcery program")
                .all_effects();
            assert_eq!(effects.len(), 1);
            let original = effects[0];
            let node_count = assert_tree_models(original);
            assert!(
                node_count >= 2,
                "optional composition includes the executable child"
            );
            let wire = serde_json::from_str(original.serialized_model().unwrap())
                .expect("retained envelope deserializes");
            let restored =
                ironsmith_runtime_catalog::artifact_materializer::materialize_effect(wire)
                    .expect("ordinary artifact service restores an executable effect");
            assert_eq!(assert_tree_models(&restored), node_count);
            assert_eq!(restored.serialized_model(), original.serialized_model());
            let alice = ironsmith::PlayerId::from_index(0);
            for effect in [original, &restored] {
                let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let source =
                    game.create_object_from_definition(&definition, alice, ironsmith::Zone::Stack);
                let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
                let mut ctx = ironsmith::effects::EffectContext::new(source, alice, &mut dm);
                ironsmith::effects::execute_effect(&mut game, effect, &mut ctx)
                    .expect("optional effect executes");
                assert_eq!(
                    game.player(alice).unwrap().life,
                    23,
                    "restored composition executes the same accepted instruction"
                );
            }
        }
    }
}


#[cfg(test)]
mod retained_ability_component_tests {
    use super::*;

    #[test]
    fn retained_ability_models_restore_compiled_trigger_and_pay_actual_costs() {
        let (_, definition) = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Retained ability fixture"),
            "Mana cost: {1}\nType: Artifact\nWhen this artifact enters, you gain 2 life.\n{T}, Pay 1 life: You gain 2 life.",
            false,
        ).expect("strict artifact fixture");
        let trigger = definition
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                ironsmith::ability::AbilityKind::Triggered(value) => Some(value),
                _ => None,
            })
            .expect("compiled entry trigger");
        let model = trigger
            .trigger
            .compiled_model()
            .expect("trigger retains full matcher model");
        let json = serde_json::to_string(model).expect("matcher model serializes");
        let decoded = serde_json::from_str(&json).expect("matcher model deserializes");
        let restored_trigger =
            ironsmith::triggers::Trigger::from_model(decoded).expect("matcher restores");
        assert_eq!(restored_trigger.compiled_model(), Some(model));
        assert_eq!(restored_trigger.display(), trigger.trigger.display());
        let activated = definition
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                ironsmith::ability::AbilityKind::Activated(value) => Some(value),
                _ => None,
            })
            .expect("compiled activated ability");
        let original_costs = activated.mana_cost.costs();
        assert!(!original_costs.is_empty());
        let restored_costs = original_costs
            .iter()
            .map(|cost| {
                let wire = cost
                    .compiled_model()
                    .expect("cost retains its full core model")
                    .clone()
                    .try_map_effect(|effect| {
                        serde_json::from_str::<ironsmith_compiled_artifact::WireEffect>(
                            effect
                                .serialized_model()
                                .expect("nested cost effect retains its model"),
                        )
                    })
                    .expect("every nested cost effect encodes");
                let json = serde_json::to_string(&wire).expect("cost serializes");
                let decoded: ironsmith_compiled_artifact::WireCost =
                    serde_json::from_str(&json).expect("cost deserializes");
                let native = decoded
                    .try_map_effect(
                        ironsmith_runtime_catalog::artifact_materializer::materialize_effect,
                    )
                    .expect("nested cost effects restore");
                let restored = ironsmith::costs::Cost::from_model(native).expect("payer restores");
                assert_eq!(restored.display(), cost.display());
                restored
            })
            .collect::<Vec<_>>();
        let alice = ironsmith::PlayerId::from_index(0);
        for costs in [original_costs, restored_costs.as_slice()] {
            let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let source = game.create_object_from_definition(
                &definition,
                alice,
                ironsmith::Zone::Battlefield,
            );
            let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
            let mut cost_ctx = ironsmith::costs::CostContext::new(source, alice, &mut dm);
            cost_ctx.reason = ironsmith::costs::PaymentReason::ActivateAbility;
            for cost in costs {
                cost.pay(&mut game, &mut cost_ctx)
                    .expect("actual cost payment");
            }
            assert!(game.is_tapped(source));
            assert_eq!(game.player(alice).unwrap().life, 19);
            drop(cost_ctx);
            let mut ctx = ironsmith::effects::EffectContext::new(source, alice, &mut dm);
            for effect in activated.effects.all_effects() {
                ironsmith::effects::execute_effect(&mut game, effect, &mut ctx)
                    .expect("ability effects execute");
            }
            assert_eq!(game.player(alice).unwrap().life, 21);
        }
    }
}

#[cfg(test)]
fn unmodeled_codec_payer() -> ironsmith::costs::Cost {
    #[derive(Debug, Clone)]
    struct Payer;
    impl ironsmith::costs::CostPayer for Payer {
        fn can_pay(&self, _game: &ironsmith::GameState, _ctx: &ironsmith::costs::CostContext)
            -> Result<(), ironsmith::cost::CostPaymentError> { Ok(()) }
        fn pay(&self, _game: &mut ironsmith::GameState, _ctx: &mut ironsmith::costs::CostContext)
            -> Result<ironsmith::costs::CostPaymentResult, ironsmith::cost::CostPaymentError> {
            Ok(ironsmith::costs::CostPaymentResult::Paid)
        }
        fn display(&self) -> String { "opaque native payer".into() }
        fn as_any(&self) -> &dyn std::any::Any { self }
    }
    ironsmith::costs::Cost::new(Payer)
}

#[cfg(test)]
fn unmodeled_codec_callback() -> ironsmith::effect::Effect {
    #[derive(Debug, Clone)]
    struct Callback;
    impl ironsmith::effects::EffectExecutor for Callback {
        fn execute(&self, _game: &mut ironsmith::GameState, _ctx: &mut ironsmith::effects::EffectContext)
            -> Result<ironsmith::effect::EffectOutcome, ironsmith::effects::ExecutionError> {
            Ok(ironsmith::effect::EffectOutcome::resolved())
        }
    }
    ironsmith::effect::Effect::new(Callback)
}

#[cfg(test)]
fn assert_native_gain_codec_ability(ability: &ironsmith::ability::Ability) {
    let program = match &ability.kind {
        ironsmith::ability::AbilityKind::Triggered(value) => &value.effects,
        ironsmith::ability::AbilityKind::Activated(value) => &value.effects,
        _ => panic!("expected executable ability"),
    };
    let effects = program.all_effects();
    assert_eq!(effects.len(), 1);
    let gain = effects[0].downcast_ref::<ironsmith::effects::GainLifeEffect>().expect("native gain executor survives nested restore");
    assert_eq!(gain.amount, ironsmith::effect::Value::Fixed(2));
    assert_eq!(gain.player, ironsmith::target::ChooseSpec::Player(ironsmith::target::PlayerFilter::You));
}

#[cfg(test)]
mod runtime_payload_codec_tests {
    use super::*;
    use ironsmith_runtime_catalog::artifact_materializer::{
        RuntimePayloadEncodingError, encode_runtime_ability, encode_runtime_effect,
        restore_runtime_ability,
    };

    fn fixture() -> ironsmith::cards::CardDefinition {
        compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Payload codec fixture"),
            "Mana cost: {1}\nType: Artifact\nWhen this artifact enters, you gain 2 life.\n{T}, Pay 1 life: You gain 2 life.",
            false,
        ).expect("strict compiled fixture").1
    }

    #[test]
    fn retained_ability_payload_codec_restores_complete_compiled_abilities_and_executes() {
        let definition = fixture();
        let alice = ironsmith::PlayerId::from_index(0);
        let mut abilities = definition.abilities.clone();
        abilities.push(ironsmith::ability::Ability::static_ability(
            ironsmith::static_abilities::StaticAbility::flying(),
        ));
        for mut original in abilities {
            if let ironsmith::ability::AbilityKind::Triggered(triggered) = &mut original.kind {
                triggered.trigger = triggered
                    .trigger
                    .clone()
                    .with_intro_surface(ironsmith::triggers::TriggerIntroSurface::Whenever);
            }
            let wire = encode_runtime_ability(original.clone()).expect("complete ability encodes");
            let json = serde_json::to_string(&wire).expect("complete ability serializes");
            let decoded = serde_json::from_str(&json).expect("complete ability deserializes");
            let restored =
                restore_runtime_ability(decoded).expect("exact runtime ability restores");
            assert_eq!(restored.functional_zones, original.functional_zones);
            assert_eq!(
                serde_json::to_value(encode_runtime_ability(restored.clone()).unwrap()).unwrap(),
                serde_json::to_value(&wire).unwrap(),
                "restoration must preserve every wire field"
            );
            for ability in [original, restored] {
                let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let source = game.create_object_from_definition(
                    &definition,
                    alice,
                    ironsmith::Zone::Battlefield,
                );
                let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
                match ability.kind {
                    ironsmith::ability::AbilityKind::Triggered(triggered) => {
                        assert_eq!(
                            triggered.trigger.intro_surface(),
                            Some(ironsmith::triggers::TriggerIntroSurface::Whenever)
                        );
                        let ctx = ironsmith::triggers::TriggerContext::new(
                            source,
                            alice,
                            ironsmith::target::FilterContext::new(alice),
                            &game,
                        );
                        let own_entry = ironsmith::events::Event::zone_change(
                            source,
                            ironsmith::Zone::Hand,
                            ironsmith::Zone::Battlefield,
                            ironsmith::events::EventCause::effect(),
                            None,
                        );
                        assert!(triggered.trigger.matches(&own_entry.0, &ctx));
                        let other_entry = ironsmith::events::Event::zone_change(
                            ironsmith::ObjectId::from_raw(9999),
                            ironsmith::Zone::Hand,
                            ironsmith::Zone::Battlefield,
                            ironsmith::events::EventCause::effect(),
                            None,
                        );
                        assert!(!triggered.trigger.matches(&other_entry.0, &ctx));
                        let mut ctx =
                            ironsmith::effects::EffectContext::new(source, alice, &mut dm);
                        for effect in triggered.effects.all_effects() {
                            ironsmith::effects::execute_effect(&mut game, effect, &mut ctx)
                                .expect("restored trigger executes");
                        }
                        assert_eq!(game.player(alice).unwrap().life, 22);
                    }
                    ironsmith::ability::AbilityKind::Activated(activated) => {
                        let mut ctx = ironsmith::costs::CostContext::new(source, alice, &mut dm);
                        ctx.reason = ironsmith::costs::PaymentReason::ActivateAbility;
                        for cost in activated.mana_cost.costs() {
                            cost.pay(&mut game, &mut ctx)
                                .expect("restored complete cost pays");
                        }
                        assert!(game.is_tapped(source));
                        assert_eq!(game.player(alice).unwrap().life, 19);
                        drop(ctx);
                        let mut ctx =
                            ironsmith::effects::EffectContext::new(source, alice, &mut dm);
                        for effect in activated.effects.all_effects() {
                            ironsmith::effects::execute_effect(&mut game, effect, &mut ctx)
                                .expect("restored activation executes");
                        }
                        assert_eq!(game.player(alice).unwrap().life, 21);
                    }
                    ironsmith::ability::AbilityKind::Static(ability) => {
                        assert!(ability.has_flying());
                    }
                }
            }
        }
    }

    #[test]
    fn retained_ability_payload_codec_rejects_missing_nested_and_invalid_models() {
        let definition = fixture();
        let mut ability = definition
            .abilities
            .iter()
            .find(|ability| matches!(ability.kind, ironsmith::ability::AbilityKind::Activated(_)))
            .expect("compiled activation")
            .clone();
        if let ironsmith::ability::AbilityKind::Activated(value) = &mut ability.kind {
            value.mana_cost =
                ironsmith::TotalCost::from_cost(ironsmith::costs::Cost::life(1));
        }
        let restored = restore_runtime_ability(encode_runtime_ability(ability.clone()).expect("standard native life cost encodes"))
            .expect("standard native life cost restores");
        let ironsmith::ability::AbilityKind::Activated(restored) = restored.kind else { panic!("activated cost owner"); };
        assert_eq!(restored.mana_cost.costs().len(), 1);
        assert_eq!(restored.mana_cost.costs()[0].life_amount(), Some(1));
        if let ironsmith::ability::AbilityKind::Activated(value) = &mut ability.kind {
            value.mana_cost = ironsmith::TotalCost::from_cost(unmodeled_codec_payer());
        }
        assert!(matches!(
            encode_runtime_ability(ability),
            Err(RuntimePayloadEncodingError::MissingModel { component: "cost" })
        ));
        let mut ability = definition
            .abilities
            .iter()
            .find(|ability| matches!(ability.kind, ironsmith::ability::AbilityKind::Triggered(_)))
            .expect("compiled trigger")
            .clone();
        if let ironsmith::ability::AbilityKind::Triggered(value) = &mut ability.kind {
            value.effects = ironsmith::resolution::ResolutionProgram::from_effects(vec![
                ironsmith::effect::Effect::gain_life(2),
            ]);
        }
        let restored = restore_runtime_ability(encode_runtime_ability(ability.clone()).expect("native gain nested ability encodes"))
            .expect("native gain nested ability restores");
        assert_native_gain_codec_ability(&restored);
        if let ironsmith::ability::AbilityKind::Triggered(value) = &mut ability.kind {
            value.effects = ironsmith::resolution::ResolutionProgram::from_effects(vec![unmodeled_codec_callback()]);
        }
        assert!(matches!(
            encode_runtime_ability(ability),
            Err(RuntimePayloadEncodingError::MissingModel {
                component: "effect"
            })
        ));
        assert!(matches!(
            encode_runtime_effect(
                ironsmith::effect::Effect::gain_life(2).with_serialized_model("invalid JSON")
            ),
            Err(RuntimePayloadEncodingError::InvalidEffectModel { .. })
        ));
    }
}

#[cfg(test)]
mod retained_copy_text_payload_tests {
    use super::*;
    use ironsmith_runtime_catalog::artifact_materializer::{
        RuntimePayloadEncodingError, encode_runtime_copy_values, encode_runtime_text_overlay,
        restore_runtime_copy_values, restore_runtime_text_overlay,
    };

    fn definitions() -> (
        ironsmith::cards::CardDefinition,
        ironsmith::cards::CardDefinition,
    ) {
        let source = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Copied program"),
            "Mana cost: {1}\nType: Artifact\nWhen this artifact enters, you gain 2 life.\n{T}, Pay 1 life: You gain 2 life.", false,
        ).expect("strict source fixture").1;
        let target = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Recipient"),
            "Mana cost: {4}\nType: Artifact",
            false,
        )
        .expect("strict recipient fixture")
        .1;
        (source, target)
    }

    #[test]
    fn retained_copy_text_payloads_restore_and_apply_compiled_abilities_to_recipient() {
        let (definition, target_definition) = definitions();
        let alice = ironsmith::PlayerId::from_index(0);
        let bob = ironsmith::PlayerId::from_index(1);
        let mut prototype = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source =
            prototype.create_object_from_definition(&definition, bob, ironsmith::Zone::Battlefield);
        let original_copy =
            ironsmith::snapshot::CopiableValues::from_object(prototype.object(source).unwrap());
        assert!(!original_copy.abilities.is_empty());
        let copy_wire =
            encode_runtime_copy_values(original_copy.clone()).expect("all copy abilities encode");
        let json = serde_json::to_string(&copy_wire).unwrap();
        let restored_copy = restore_runtime_copy_values(serde_json::from_str(&json).unwrap())
            .expect("complete copy restores");
        assert_eq!(
            serde_json::to_value(encode_runtime_copy_values(restored_copy.clone()).unwrap())
                .unwrap(),
            serde_json::to_value(copy_wire).unwrap()
        );
        let original_overlay = ironsmith::continuous::TextBoxOverlay::new(
            original_copy.compiled_card_text.clone(),
            original_copy.abilities.as_ref().clone(),
        )
        .with_ability_labels(original_copy.ability_labels.clone());
        let overlay_wire = encode_runtime_text_overlay(original_overlay.clone())
            .expect("all overlay abilities encode");
        let json = serde_json::to_string(&overlay_wire).unwrap();
        let restored_overlay = restore_runtime_text_overlay(serde_json::from_str(&json).unwrap())
            .expect("complete overlay restores");
        assert_eq!(
            serde_json::to_value(encode_runtime_text_overlay(restored_overlay.clone()).unwrap())
                .unwrap(),
            serde_json::to_value(overlay_wire).unwrap()
        );
        for copy_effect in [true, false] {
            for restored in [false, true] {
                let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let source = game.create_object_from_definition(
                    &definition,
                    bob,
                    ironsmith::Zone::Battlefield,
                );
                let recipient = game.create_object_from_definition(
                    &target_definition,
                    alice,
                    ironsmith::Zone::Battlefield,
                );
                let modification = if copy_effect {
                    ironsmith::continuous::Modification::CopyOf {
                        target_id: source,
                        copiable_values: Box::new(if restored {
                            restored_copy.clone()
                        } else {
                            original_copy.clone()
                        }),
                        preserve_source_abilities: false,
                        name_override: None,
                        name_override_surface: None,
                        add_supertypes: vec![],
                    }
                } else {
                    ironsmith::continuous::Modification::SetTextBox(if restored {
                        restored_overlay.clone()
                    } else {
                        original_overlay.clone()
                    })
                };
                game.effect_store.continuous_effects.add_effect(
                    ironsmith::continuous::ContinuousEffect::from_resolution(
                        recipient,
                        alice,
                        vec![recipient],
                        modification,
                    )
                    .until(ironsmith::Until::EndOfTurn),
                );
                game.refresh_continuous_state()
                    .expect("registered payload applies");
                let chars = game
                    .calculated_characteristics(recipient)
                    .expect("recipient characteristics");
                assert_eq!(
                    chars.name.to_owned_string(),
                    if copy_effect {
                        original_copy.name.clone()
                    } else {
                        "Recipient".into()
                    }
                );
                assert_eq!(
                    chars.compiled_card_text.to_string(),
                    original_copy.compiled_card_text
                );
                assert_eq!(chars.ability_labels.to_vec(), original_copy.ability_labels);
                let abilities = game
                    .current_abilities(recipient)
                    .expect("recipient abilities");
                let trigger = abilities
                    .iter()
                    .find_map(|ability| match &ability.kind {
                        ironsmith::ability::AbilityKind::Triggered(value) => Some(value),
                        _ => None,
                    })
                    .expect("copied/overlaid trigger remains executable");
                let ctx = ironsmith::triggers::TriggerContext::new(
                    recipient,
                    alice,
                    ironsmith::target::FilterContext::new(alice),
                    &game,
                );
                let own_entry = ironsmith::events::Event::zone_change(
                    recipient,
                    ironsmith::Zone::Hand,
                    ironsmith::Zone::Battlefield,
                    ironsmith::events::EventCause::effect(),
                    None,
                );
                let source_entry = ironsmith::events::Event::zone_change(
                    source,
                    ironsmith::Zone::Hand,
                    ironsmith::Zone::Battlefield,
                    ironsmith::events::EventCause::effect(),
                    None,
                );
                assert!(trigger.trigger.matches(&own_entry.0, &ctx));
                assert!(
                    !trigger.trigger.matches(&source_entry.0, &ctx),
                    "copied self reference binds to recipient"
                );
                let activated = abilities
                    .iter()
                    .find_map(|ability| match &ability.kind {
                        ironsmith::ability::AbilityKind::Activated(value) => Some(value),
                        _ => None,
                    })
                    .expect("copied/overlaid activation remains executable");
                let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
                let mut cost_ctx = ironsmith::costs::CostContext::new(recipient, alice, &mut dm);
                cost_ctx.reason = ironsmith::costs::PaymentReason::ActivateAbility;
                for cost in activated.mana_cost.costs() {
                    cost.pay(&mut game, &mut cost_ctx)
                        .expect("recipient pays retained costs");
                }
                assert!(game.is_tapped(recipient));
                assert!(!game.is_tapped(source));
                assert_eq!(game.player(alice).unwrap().life, 19);
                drop(cost_ctx);
                let mut ctx = ironsmith::effects::EffectContext::new(recipient, alice, &mut dm);
                for effect in activated.effects.all_effects() {
                    ironsmith::effects::execute_effect(&mut game, effect, &mut ctx)
                        .expect("recipient executes retained effects");
                }
                assert_eq!(game.player(alice).unwrap().life, 21);
                assert_eq!(
                    game.player(bob).unwrap().life,
                    20,
                    "copied You does not bind to original owner"
                );
                game.effect_store.continuous_effects.cleanup_end_of_turn();
                game.refresh_continuous_state().expect("payload expires");
                assert!(game.current_abilities(recipient).unwrap().is_empty());
                assert_eq!(
                    game.calculated_characteristics(recipient)
                        .unwrap()
                        .name
                        .to_owned_string(),
                    "Recipient"
                );
            }
        }
    }

    #[test]
    fn retained_copy_text_payloads_reject_unencodable_nested_ability() {
        let (definition, _) = definitions();
        let alice = ironsmith::PlayerId::from_index(0);
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source =
            game.create_object_from_definition(&definition, alice, ironsmith::Zone::Battlefield);
        let mut copy =
            ironsmith::snapshot::CopiableValues::from_object(game.object(source).unwrap());
        let mut abilities = copy.abilities.as_ref().clone();
        let broken = abilities
            .iter_mut()
            .find_map(|ability| match &mut ability.kind {
                ironsmith::ability::AbilityKind::Activated(value) => Some(value),
                _ => None,
            })
            .expect("compiled activation");
        broken.effects = ironsmith::resolution::ResolutionProgram::from_effects(vec![
            ironsmith::effect::Effect::gain_life(2),
        ]);
        let overlay = ironsmith::continuous::TextBoxOverlay::new(
            copy.compiled_card_text.clone(),
            abilities.clone(),
        )
        .with_ability_labels(copy.ability_labels.clone());
        copy.abilities = std::sync::Arc::new(abilities);
        let restored_copy = restore_runtime_copy_values(encode_runtime_copy_values(copy.clone()).expect("native gain copy encodes"))
            .expect("native gain copy restores");
        let restored_overlay = restore_runtime_text_overlay(encode_runtime_text_overlay(overlay.clone()).expect("native gain overlay encodes"))
            .expect("native gain overlay restores");
        for abilities in [restored_copy.abilities.as_slice(), restored_overlay.abilities.as_slice()] {
            let ability = abilities.iter().find(|ability| matches!(ability.kind, ironsmith::ability::AbilityKind::Activated(_)))
                .expect("restored activated program");
            assert_native_gain_codec_ability(ability);
        }
        let mut abilities = copy.abilities.as_ref().clone();
        let callback = abilities.iter_mut().find_map(|ability| match &mut ability.kind {
            ironsmith::ability::AbilityKind::Activated(value) => Some(value), _ => None,
        }).expect("callback host");
        callback.effects = ironsmith::resolution::ResolutionProgram::from_effects(vec![unmodeled_codec_callback()]);
        let overlay = ironsmith::continuous::TextBoxOverlay::new(copy.compiled_card_text.clone(), abilities.clone())
            .with_ability_labels(copy.ability_labels.clone());
        copy.abilities = std::sync::Arc::new(abilities);
        assert!(matches!(
            encode_runtime_copy_values(copy),
            Err(RuntimePayloadEncodingError::MissingModel {
                component: "effect"
            })
        ));
        assert!(matches!(
            encode_runtime_text_overlay(overlay),
            Err(RuntimePayloadEncodingError::MissingModel {
                component: "effect"
            })
        ));
    }
}

#[cfg(test)]
mod retained_restriction_model_codec_tests {
    use ironsmith_runtime_catalog::artifact_materializer::{
        encode_runtime_static_ability, restore_runtime_ability,
    };

    #[test]
    fn retained_restriction_models_wire_codec_preserves_flags_and_rule_outcomes() {
        use ironsmith::continuous::{RegisteredRestriction, RestrictionKind};
        let alice = ironsmith::PlayerId::from_index(0);
        for (kind, expected) in [
            (RestrictionKind::CantBeBlocked, [true, true, false]),
            (RestrictionKind::CantAttack, [false, true, true]),
            (RestrictionKind::CantBlock, [true, false, true]),
            (RestrictionKind::DoesntUntap, [true, true, true]),
        ] {
            let original = RegisteredRestriction::new(kind).ability().clone();
            let wire = encode_runtime_static_ability(original.clone())
                .expect("native restriction encodes");
            let json = serde_json::to_string(&wire).unwrap();
            let decoded = serde_json::from_str(&json).unwrap();
            let restored = restore_runtime_ability(
                ironsmith_compiled_artifact::WireAbility::static_ability(decoded),
            )
            .expect("restriction restores");
            let ironsmith::ability::AbilityKind::Static(restored) = restored.kind else {
                panic!("static kind lost")
            };
            assert_eq!(
                serde_json::to_value(encode_runtime_static_ability(restored.clone()).unwrap())
                    .unwrap(),
                serde_json::to_value(wire).unwrap()
            );
            assert_eq!(restored.has_defender(), original.has_defender());
            assert_eq!(restored.is_unblockable(), original.is_unblockable());
            assert_eq!(restored.affects_untap(), original.affects_untap());
            for ability in [original, restored] {
                let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let card = ironsmith::card::CardBuilder::new(
                    ironsmith::CardId::new(),
                    "Restriction codec fixture",
                )
                .card_types(vec![ironsmith::CardType::Creature])
                .build();
                let source =
                    game.create_object_from_card(&card, alice, ironsmith::Zone::Battlefield);
                ability.apply_restrictions(&mut game, source, alice);
                let tracker = &game.effect_store.cant_effects;
                assert_eq!(
                    [
                        tracker.can_attack(source),
                        tracker.can_block(source),
                        tracker.can_be_blocked(source)
                    ],
                    expected
                );
            }
        }
    }
}

#[cfg(test)]
mod retained_metadata_payload_codec_tests {
    use ironsmith::continuous::{
        ContinuousEffect, EffectTarget, Modification, RegisteredRestriction, RestrictionKind,
    };
    use ironsmith::object::{
        AttachmentTarget, AuraAttachmentFilter, AuraAttachmentFilterRuntimeExt,
        AuraAttachmentMetadata,
    };
    use ironsmith::static_abilities::StaticAbility;
    use ironsmith_runtime_catalog::artifact_materializer::{
        encode_runtime_aura_metadata, encode_runtime_restriction, encode_runtime_static_ability,
        restore_runtime_aura_metadata, restore_runtime_restriction,
    };

    #[test]
    fn retained_metadata_payload_codec_registered_restrictions_and_attachment_rules() {
        let alice = ironsmith::PlayerId::from_index(0);
        let bob = ironsmith::PlayerId::from_index(1);
        for (kind, expected) in [
            (RestrictionKind::CantBeBlocked, [true, true, false]),
            (RestrictionKind::CantAttack, [false, true, true]),
            (RestrictionKind::CantBlock, [true, false, true]),
            (RestrictionKind::DoesntUntap, [true, true, true]),
        ] {
            let original = RegisteredRestriction::new(kind);
            let wire = encode_runtime_restriction(original.clone())
                .expect("complete restriction encoding");
            let json = serde_json::to_value(&wire).unwrap();
            let restored =
                restore_runtime_restriction(serde_json::from_value(json.clone()).unwrap())
                    .expect("complete restriction restoration");
            assert_eq!(
                serde_json::to_value(encode_runtime_restriction(restored.clone()).unwrap())
                    .unwrap(),
                json
            );
            for restriction in [original, restored] {
                let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let card = ironsmith::card::CardBuilder::new(ironsmith::CardId::new(), "Recipient")
                    .card_types(vec![ironsmith::CardType::Creature])
                    .build();
                let recipient =
                    game.create_object_from_card(&card, alice, ironsmith::Zone::Battlefield);
                let expected_id = restriction.ability().id();
                let effect =
                    game.effect_store
                        .continuous_effects
                        .add_effect(ContinuousEffect::new(
                            recipient,
                            alice,
                            EffectTarget::Specific(recipient),
                            Modification::Restriction(restriction),
                        ));
                game.refresh_continuous_state()
                    .expect("registered payload refresh");
                let tracker = &game.effect_store.cant_effects;
                assert_eq!(
                    [
                        tracker.can_attack(recipient),
                        tracker.can_block(recipient),
                        tracker.can_be_blocked(recipient)
                    ],
                    expected
                );
                assert!(game.current_has_static_ability_id(recipient, expected_id));
                game.effect_store.continuous_effects.remove_effect(effect);
                game.refresh_continuous_state().expect("removal refresh");
                assert!(!game.current_has_static_ability_id(recipient, expected_id));
                assert!(game.effect_store.cant_effects.can_attack(recipient));
                assert!(game.effect_store.cant_effects.can_block(recipient));
                assert!(game.effect_store.cant_effects.can_be_blocked(recipient));
            }
        }
        let filter =
            AuraAttachmentFilter::from(ironsmith::target::ObjectFilter::creature().you_control());
        let original = AuraAttachmentMetadata::from(filter.clone());
        let wire =
            encode_runtime_aura_metadata(original.clone()).expect("complete enchant encoding");
        let json = serde_json::to_value(&wire).unwrap();
        let restored = restore_runtime_aura_metadata(serde_json::from_value(json.clone()).unwrap())
            .expect("matching enchant restoration");
        assert_eq!(
            serde_json::to_value(encode_runtime_aura_metadata(restored.clone()).unwrap()).unwrap(),
            json
        );
        for metadata in [original, restored] {
            let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let aura =
                ironsmith::card::CardBuilder::new(ironsmith::CardId::new(), "Attachment recipient")
                    .card_types(vec![ironsmith::CardType::Enchantment])
                    .subtypes(vec![ironsmith::Subtype::Aura])
                    .build();
            let aura = game.create_object_from_card(&aura, alice, ironsmith::Zone::Battlefield);
            let creature =
                ironsmith::card::CardBuilder::new(ironsmith::CardId::new(), "Creature target")
                    .card_types(vec![ironsmith::CardType::Creature])
                    .build();
            let own = game.create_object_from_card(&creature, alice, ironsmith::Zone::Battlefield);
            let other = game.create_object_from_card(&creature, bob, ironsmith::Zone::Battlefield);
            let effect = game
                .effect_store
                .continuous_effects
                .add_effect(ContinuousEffect::new(
                    aura,
                    alice,
                    EffectTarget::Specific(aura),
                    Modification::SetAuraAttachmentFilter(metadata),
                ));
            let effects = game.try_all_continuous_effects().expect("finite discovery");
            let chars = game
                .calculated_characteristics_with_effects(aura, &effects)
                .expect("aura exists");
            let actual = chars
                .aura_attach_filter
                .expect("registered filter retained");
            assert_eq!(actual, filter);
            let enchant = chars
                .static_abilities
                .iter()
                .find(|ability| ability.enchant_filter().is_some())
                .expect("enchant ability retained");
            assert_eq!(enchant.enchant_filter(), Some(&actual));
            let ctx = ironsmith::target::FilterContext::new(alice).with_source(aura);
            assert!(actual.matches_target(AttachmentTarget::Object(own), &ctx, &game));
            assert!(!actual.matches_target(AttachmentTarget::Object(other), &ctx, &game));
            assert!(!actual.matches_target(AttachmentTarget::Object(aura), &ctx, &game));
            assert!(!actual.matches_target(AttachmentTarget::Player(alice), &ctx, &game));
            game.effect_store.continuous_effects.remove_effect(effect);
            let effects = game
                .try_all_continuous_effects()
                .expect("finite removal discovery");
            let chars = game
                .calculated_characteristics_with_effects(aura, &effects)
                .expect("aura exists");
            assert!(chars.aura_attach_filter.is_none());
            assert!(
                chars
                    .static_abilities
                    .iter()
                    .all(|ability| ability.enchant_filter().is_none())
            );
        }
    }

    #[test]
    fn retained_metadata_payload_codec_rejects_mismatched_and_spoofed_payloads() {
        let mut restriction =
            encode_runtime_restriction(RegisteredRestriction::new(RestrictionKind::CantAttack))
                .unwrap();
        restriction.ability = encode_runtime_static_ability(StaticAbility::defender()).unwrap();
        assert!(restore_runtime_restriction(restriction).is_err());
        let mut spoof =
            encode_runtime_restriction(RegisteredRestriction::new(RestrictionKind::CantAttack))
                .unwrap();
        let mut enchant = encode_runtime_static_ability(StaticAbility::enchant(
            AuraAttachmentFilter::from(ironsmith::target::ObjectFilter::creature()),
        ))
        .unwrap();
        enchant.id = spoof.ability.id;
        spoof.ability = enchant;
        assert!(
            restore_runtime_restriction(spoof).is_err(),
            "same ID with unrelated payload must reject"
        );
        let filter = AuraAttachmentFilter::from(ironsmith::target::ObjectFilter::creature());
        let mut attachment =
            encode_runtime_aura_metadata(AuraAttachmentMetadata::from(filter)).unwrap();
        attachment.filter = AuraAttachmentFilter::from(ironsmith::target::ObjectFilter::land());
        assert!(
            restore_runtime_aura_metadata(attachment.clone()).is_err(),
            "enchant filter mismatch"
        );
        attachment.enchant_ability =
            encode_runtime_static_ability(StaticAbility::flying()).unwrap();
        assert!(
            restore_runtime_aura_metadata(attachment).is_err(),
            "missing enchant payload"
        );
        let mut spoof_attachment = encode_runtime_aura_metadata(AuraAttachmentMetadata::from(
            AuraAttachmentFilter::from(ironsmith::target::ObjectFilter::creature()),
        ))
        .unwrap();
        spoof_attachment.enchant_ability.id =
            encode_runtime_static_ability(StaticAbility::flying())
                .unwrap()
                .id;
        assert!(
            restore_runtime_aura_metadata(spoof_attachment).is_err(),
            "matching filter with wrong ability ID rejects"
        );
    }
}






#[cfg(test)]
mod retained_casting_payload_codec_tests {
    use super::*;
    use ironsmith_runtime_catalog::artifact_materializer::*;
    fn fixture() -> (
        ironsmith::cards::CardDefinition,
        ironsmith::costs::Cost,
        ironsmith::effect::Effect,
    ) {
        let definition = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Saved payable form"),
            "Type: Artifact\nPay 1 life: You gain 2 life.",
            false,
        )
        .expect("actual payable ability compiles")
        .1;
        let ability = definition
            .abilities
            .iter()
            .find_map(|ability| {
                if let ironsmith::ability::AbilityKind::Activated(ability) = &ability.kind {
                    Some(ability)
                } else {
                    None
                }
            })
            .unwrap();
        let cost = ability.mana_cost.costs()[0].clone();
        let effect = ability.effects.all_effects()[0].clone();
        (definition, cost, effect)
    }
    #[test]
    fn retained_casting_payload_codec_preserves_cost_branches_metadata_and_actual_payment() {
        let (definition, cost, _) = fixture();
        let branch = ironsmith::cost::TotalCost::from_cost(cost);
        let total = ironsmith::cost::TotalCost::one_of(vec![
            branch.clone(),
            ironsmith::cost::TotalCost::one_of(vec![branch.clone(), branch]),
        ]);
        let mut optional = ironsmith::cost::OptionalCost::buyback(total.clone()).repeatable();
        optional.source_label = "Retained cost label".into();
        let alternative = ironsmith::alternative_cast::AlternativeCastingMethod::Composed {
            name: "Saved alternative".into(),
            total_cost: total.clone(),
            condition: Some(ironsmith::static_abilities::ThisSpellCostCondition::YourTurn),
            prototype_power_toughness: Some(ironsmith::card::PowerToughness::fixed(2, 3)),
        };
        let total = encode_runtime_total_cost(total).unwrap();
        let optional = encode_runtime_optional_cost(optional).unwrap();
        let alternative = encode_runtime_alternative_cast(alternative).unwrap();
        let wire = serde_json::to_value((&total, &optional, &alternative)).unwrap();
        let (total, optional, alternative) = serde_json::from_value(wire.clone()).unwrap();
        let total = restore_runtime_total_cost(total).unwrap();
        let optional = restore_runtime_optional_cost(optional).unwrap();
        let alternative = restore_runtime_alternative_cast(alternative).unwrap();
        assert!(optional.repeatable && optional.returns_to_hand);
        assert_eq!(optional.source_label, "Retained cost label");
        assert_eq!(total.as_one_of().unwrap().len(), 2);
        assert_eq!(total.as_one_of().unwrap()[1].as_one_of().unwrap().len(), 2);
        assert_eq!(
            serde_json::to_value((
                encode_runtime_total_cost(total.clone()).unwrap(),
                encode_runtime_optional_cost(optional).unwrap(),
                encode_runtime_alternative_cast(alternative).unwrap()
            ))
            .unwrap(),
            wire
        );
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let source =
            game.create_object_from_definition(&definition, alice, ironsmith::Zone::Battlefield);
        let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
        let mut ctx = ironsmith::costs::CostContext::new(source, alice, &mut dm);
        for part in total.as_one_of().unwrap()[0].costs() {
            part.pay(&mut game, &mut ctx).unwrap();
        }
        assert_eq!(game.player(alice).unwrap().life, 19);
    }
    #[test]
    fn retained_casting_payload_codec_preserves_alternative_effect_program_without_normalizing() {
        let (definition, _, effect) = fixture();
        let method = ironsmith::alternative_cast::AlternativeCastingMethod::Overload {
            cost: ironsmith::mana::ManaCost::from_pips(vec![vec![
                ironsmith::mana::ManaSymbol::Blue,
            ]]),
            effects: vec![effect.clone(), effect],
        };
        let wire = encode_runtime_alternative_cast(method).unwrap();
        let json = serde_json::to_value(&wire).unwrap();
        let method =
            restore_runtime_alternative_cast(serde_json::from_value(json.clone()).unwrap())
                .unwrap();
        assert_eq!(
            serde_json::to_value(encode_runtime_alternative_cast(method.clone()).unwrap()).unwrap(),
            json
        );
        let ironsmith::alternative_cast::AlternativeCastingMethod::Overload { effects, .. } =
            method
        else {
            panic!("overload retained")
        };
        assert_eq!(effects.len(), 2);
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let source =
            game.create_object_from_definition(&definition, alice, ironsmith::Zone::Battlefield);
        let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
        let mut ctx = ironsmith::effects::EffectContext::new(source, alice, &mut dm);
        for effect in effects {
            ironsmith::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
        }
        assert_eq!(game.player(alice).unwrap().life, 24);
    }

    #[test]
    fn retained_casting_payload_codec_preserves_target_binding_in_retained_overload() {
        let definition = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(
                ironsmith::CardId::new(),
                "Retained target program",
            ),
            "Type: Instant\nTarget creature gets +1/+1 until end of turn.",
            false,
        )
        .expect("actual targeted program compiles")
        .1;
        let program = definition.spell_effect.unwrap();
        let effect = program
            .all_effects()
            .iter()
            .find(|effect| {
                matches!(
                    effect.0.get_target_spec(),
                    Some(ironsmith::target::ChooseSpec::Target(_))
                )
            })
            .expect("real compiled effect retains targeted binding")
            .clone();
        let expected = effect.0.get_target_spec().unwrap().clone();
        let method = ironsmith::alternative_cast::AlternativeCastingMethod::Overload {
            cost: ironsmith::mana::ManaCost::from_pips(vec![vec![
                ironsmith::mana::ManaSymbol::Blue,
            ]]),
            effects: vec![effect.clone()],
        };
        let wire = encode_runtime_alternative_cast(method).unwrap();
        let json = serde_json::to_value(&wire).unwrap();
        let restored =
            restore_runtime_alternative_cast(serde_json::from_value(json.clone()).unwrap())
                .unwrap();
        let ironsmith::alternative_cast::AlternativeCastingMethod::Overload { effects, .. } =
            &restored
        else {
            panic!("retained overload")
        };
        assert_eq!(effects[0].0.get_target_spec(), Some(&expected));
        assert_eq!(
            serde_json::to_value(encode_runtime_alternative_cast(restored).unwrap()).unwrap(),
            json
        );
    }
}


#[cfg(test)]
mod retained_registered_card_graph_codec_tests {
    use super::*;
    use ironsmith_runtime_catalog::artifact_materializer::*;
    fn fixture() -> (
        ironsmith::GameState,
        ironsmith::ObjectId,
        ironsmith::CardId,
        ironsmith::CardId,
    ) {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let card = ironsmith::card::CardBuilder::new(ironsmith::CardId::new(), "Origin carrier")
            .card_types(vec![ironsmith::types::CardType::Artifact])
            .build();
        let source = game.create_object_from_card(&card, alice, ironsmith::Zone::Battlefield);
        let face = ironsmith::CardId::new();
        let level_face = ironsmith::CardId::new();
        let flying = ironsmith::static_abilities::StaticAbility::flying();
        let mut effect = ironsmith::continuous::ContinuousEffect::new(
            source,
            alice,
            ironsmith::continuous::EffectTarget::Specific(source),
            ironsmith::continuous::Modification::SetColors(ironsmith::color::ColorSet::RED),
        );
        effect.originating_static_ability = Some(flying);
        effect.originating_ability = Some(Box::new(ironsmith::continuous::ContinuousAbilityOrigin {
            host: source,
            printed_face: Some(face),
            branch: 3,
            ability: ironsmith::continuous::AbilityOrigin::Level {
                printed_face: Some(level_face),
                parent: Box::new(ironsmith::continuous::AbilityOrigin::Printed(4)),
                tier: 5,
                slot: 6,
            },
        }));
        game.effect_store.continuous_effects.add_effect(effect);
        (game, source, face, level_face)
    }



}




#[cfg(test)]
mod compiled_entry_type_projection_tests {
    use super::*;
    use ironsmith::decision::DecisionMaker;
    use ironsmith::{GameState, PlayerId, Zone};
    use ironsmith::types::{CardType, Subtype, Supertype};
    use ironsmith::object::CounterType;
    use ironsmith::ability::{Ability, AbilityKind};
    use ironsmith::static_abilities::StaticAbility;
    use ironsmith::target::ObjectFilter;

    #[test]
    fn compiled_entry_type_projection_imposter_mech_preserves_copy_exception_and_replacement_applicability() {
        // Oracle and printed metadata match the local cards.json snapshot.
        let text = "Mana cost: {1}{U}\nType: Artifact — Vehicle\nPower/Toughness: 3/1\nYou may have this Vehicle enter as a copy of a creature an opponent controls, except it's a Vehicle artifact with crew 3 and it loses all other card types.\nCrew 3";
        let (artifact, direct) = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Imposter Mech"), text, false,
        ).expect("strict actual Imposter Mech compiler/artifact path");
        artifact.validate().unwrap();
        let mut registry = ironsmith::cards::CardRegistry::new();
        registry.register_compiled_artifact(&artifact).expect("actual catalog artifact decoder");
        let decoded = registry.get("Imposter Mech").unwrap().clone();
        for definition in [direct, decoded] {
            let spec = definition.abilities.iter().find_map(|ability| match &ability.kind {
                AbilityKind::Static(value) => value.enter_as_copy_as_enters(), _ => None,
            }).expect("compiled copy ability retains its semantic spec");
            assert!(spec.removes_other_card_types);
            assert_eq!(spec.added_card_types, vec![CardType::Artifact]);
            assert_eq!(spec.added_subtypes, vec![Subtype::Vehicle]);
            let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let source_definition = ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Copy type source")
                .card_types(vec![CardType::Artifact, CardType::Creature])
                .subtypes(vec![Subtype::Elf, Subtype::Equipment]).supertypes(vec![Supertype::Legendary])
                .power_toughness(ironsmith::card::PowerToughness::fixed(3, 4)).build();
            let source = game.create_object_from_definition(&source_definition, bob, Zone::Battlefield);
            let mut watcher = ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Type-filtered entry replacements")
                .card_types(vec![CardType::Enchantment]);
            for (filter, amount) in [
                (ObjectFilter::creature(), 1), (ObjectFilter::artifact(), 2),
                (ObjectFilter::permanent().with_subtype(Subtype::Elf), 4),
                (ObjectFilter::permanent().with_subtype(Subtype::Vehicle), 8),
                (ObjectFilter::permanent().with_subtype(Subtype::Equipment), 16),
            ] {
                watcher = watcher.with_ability(Ability::static_ability(StaticAbility::enters_with_counters_for_filter(
                    filter, CounterType::PlusOnePlusOne, amount,
                )));
            }
            game.create_object_from_definition(&watcher.build(), alice, Zone::Battlefield);
            let entrant = game.create_object_from_definition(&definition, alice, Zone::Hand);
            // Optional copy presents a legal no-copy option first. Select the
            // intended source by its typed UI identity, preserving that option.
            struct CopyChoice {
                source: ironsmith::ObjectId,
                selected: usize,
                fallback: ironsmith::decision::SelectFirstDecisionMaker,
            }
            impl DecisionMaker for CopyChoice {
                fn decide_options(&mut self, game: &GameState, ctx: &ironsmith::decisions::context::SelectOptionsContext) -> Vec<usize> {
                    if let Some(option) = ctx.options.iter().find(|option| option.legal &&
                        option.related_object_ids.as_ref().is_some_and(|objects| objects.contains(&self.source))) {
                        self.selected += 1;
                        vec![option.index]
                    } else { self.fallback.decide_options(game, ctx) }
                }
                fn decide_boolean(&mut self, game: &GameState, ctx: &ironsmith::decisions::context::BooleanContext) -> bool {
                    self.fallback.decide_boolean(game, ctx)
                }
                fn decide_objects(&mut self, game: &GameState, ctx: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ironsmith::ObjectId> {
                    self.fallback.decide_objects(game, ctx)
                }
            }
            let mut chooser = CopyChoice { source, selected: 0, fallback: ironsmith::decision::SelectFirstDecisionMaker };
            let receipt = game.move_object_with_etb_processing_with_dm(entrant, Zone::Battlefield, &mut chooser).unwrap();
            assert_eq!(chooser.selected, 1, "the actual optional-copy prompt must select the intended source once");
            assert!(!receipt.pending && receipt.programs.is_empty());
            let ironsmith::events::processing::EventOutcome::Proceed(result) = receipt.original else { panic!("compiled copy enters"); };
            assert_eq!(game.object(result.new_id).unwrap().name.as_ref(), "Copy type source");
            assert_eq!(game.calculated_card_types(result.new_id), vec![CardType::Artifact]);
            assert_eq!(game.calculated_subtypes(result.new_id), vec![Subtype::Equipment, Subtype::Vehicle]);
            assert_eq!(game.object(result.new_id).unwrap().counters.get(&CounterType::PlusOnePlusOne).copied(), Some(26));
            assert_eq!(game.current_controller(result.new_id), Some(alice));
            assert_eq!(game.object(source).unwrap().subtypes.as_slice(), &[Subtype::Elf, Subtype::Equipment]);
        }
    }
}

#[cfg(test)]
mod retained_copy_defense_payload_tests {
    use super::*;
    use ironsmith_runtime_catalog::artifact_materializer::{encode_runtime_copy_values, restore_runtime_copy_values};
    #[test]
    fn retained_copy_defense_payload_restores_compiled_battle_and_seeds_copy_entry() {
        let (artifact, direct) = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Copied defense payload").defense(5),
            "Type: Battle — Siege\nWhen this battle enters, you gain 2 life.", false,
        ).expect("strict battle fixture");
        artifact.validate().unwrap();
        let mut registry = ironsmith::cards::CardRegistry::new();
        registry.register_compiled_artifact(&artifact).unwrap();
        let decoded = registry.get("Copied defense payload").unwrap().clone();
        for definition in [&direct, &decoded] {
            assert_eq!(definition.card.defense, Some(5));
            for duration in [None, Some(ironsmith::effect::Until::EndOfTurn)] {
                let alice = ironsmith::PlayerId::from_index(0);
                let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let base = game.create_object_from_definition(definition, alice, ironsmith::Zone::Battlefield);
                game.object_mut(base).unwrap().counters.insert(ironsmith::object::CounterType::Defense, 2);
                let original = ironsmith::snapshot::CopiableValues::from_object(game.object(base).unwrap());
                assert_eq!(original.defense, Some(5));
                let wire = encode_runtime_copy_values(original).unwrap();
                let value = serde_json::to_value(&wire).unwrap();
                let restored = restore_runtime_copy_values(serde_json::from_value(value.clone()).unwrap()).unwrap();
                assert_eq!(restored.defense, Some(5));
                assert_eq!(serde_json::to_value(encode_runtime_copy_values(restored.clone()).unwrap()).unwrap(), value);
                let mut truncated = value;
                truncated.as_object_mut().unwrap().remove("defense");
                assert!(serde_json::from_value::<ironsmith::snapshot::RetainedCopiableValues<ironsmith_compiled_artifact::WireAbility>>(truncated).is_err(),
                    "missing defense cannot silently become no defense");
                let blank = ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Defense recipient")
                    .card_types(vec![ironsmith::types::CardType::Artifact]).build();
                let source = game.create_object_from_definition(&blank, alice, ironsmith::Zone::Battlefield);
                game.effect_store.continuous_effects.add_effect(ironsmith::continuous::ContinuousEffect::from_resolution(
                    source, alice, vec![source], ironsmith::continuous::Modification::CopyOf {
                        target_id: base, copiable_values: Box::new(restored), preserve_source_abilities: false,
                        name_override: None, name_override_surface: None, add_supertypes: Vec::new(),
                    },
                ).until(ironsmith::effect::Until::EndOfTurn));
                assert_eq!(game.object(source).unwrap().base_defense, None);
                assert_eq!(game.calculated_characteristics(source).unwrap().defense, Some(5));
                let entrant = game.create_object_from_definition(&blank, alice, ironsmith::Zone::Hand);
                game.effect_store.replacement_effects.add_resolution_effect(ironsmith::replacement::ReplacementEffect::with_matcher(
                    entrant, alice, ironsmith::events::zones::matchers::ThisWouldEnterBattlefieldMatcher,
                    ironsmith::replacement::ReplacementAction::EnterAsCopy {
                        source, enters_tapped: false, copy_duration: duration.clone(), linked_exile_objects: Vec::new(),
                        additional_counters: Vec::new(), name_override: None, added_colors: ironsmith::ColorSet::new(),
                        added_card_types: Vec::new(), removes_other_card_types: false,
                        added_supertypes: Vec::new(), removed_supertypes: Vec::new(), added_subtypes: Vec::new(),
                        added_abilities: Vec::new(), set_base_power_toughness: None, copy_followups: Vec::new(),
                    },
                ));
                let receipt = game.move_object_with_etb_processing(entrant, ironsmith::Zone::Battlefield).unwrap();
                assert!(!receipt.pending && receipt.programs.is_empty());
                let ironsmith::events::processing::EventOutcome::Proceed(result) = receipt.original else { panic!("compiled restored battle copy enters"); };
                assert_eq!(game.counter_count(result.new_id, ironsmith::object::CounterType::Defense), 5);
                assert_eq!(game.calculated_characteristics(result.new_id).unwrap().defense, Some(5));
                assert_eq!(game.counter_count(base, ironsmith::object::CounterType::Defense), 2);
            }
        }
    }
}

#[cfg(test)]
mod retained_intrinsic_starting_counter_rule_tests {
use super::*;
use ironsmith::ability::{Ability, AbilityKind};
use ironsmith::static_abilities::StaticAbility;
use ironsmith::{ObjectId, PlayerId};

#[test]
fn intrinsic_starting_counter_models_round_trip_with_rule_identity_and_action_payloads() {
    for rule in [ironsmith::static_abilities::IntrinsicStartingCounter::Loyalty, ironsmith::static_abilities::IntrinsicStartingCounter::Defense] {
        let original = StaticAbility::intrinsic_starting_counters(rule);
        let ability = Ability::static_ability(original.clone());
        let wire = ironsmith_runtime_catalog::artifact_materializer::encode_runtime_ability(ability).unwrap();
        let json = serde_json::to_value(&wire).unwrap();
        let decoded: ironsmith_compiled_artifact::WireAbility = serde_json::from_value(json.clone()).unwrap();
        let restored = ironsmith_runtime_catalog::artifact_materializer::restore_runtime_ability(decoded).unwrap();
        let AbilityKind::Static(restored) = restored.kind else { panic!("restored intrinsic static rule"); };
        assert_eq!(restored.intrinsic_starting_counter_rule(), Some(rule));
        assert_eq!(serde_json::to_value(ironsmith_runtime_catalog::artifact_materializer::encode_runtime_ability(
            Ability::static_ability(restored.clone())).unwrap()).unwrap(), json);
        let source = ObjectId::from_raw(752);
        let controller = PlayerId::from_index(0);
        let origin = ironsmith::continuous::AbilityOrigin::IntrinsicStartingCounters(rule);
        let first = original.generate_replacement_effect(source, controller).unwrap()
            .with_ability_origin(origin.clone(), None, 0);
        let second = restored.generate_replacement_effect(source, controller).unwrap()
            .with_ability_origin(origin, None, 0);
        assert_eq!(first.static_ability_instance, None, "rule identity does not use a synthetic native instance");
        assert_eq!(second.static_ability_instance, None);
        assert_eq!(first.application_key(), second.application_key(), "rule and host identity survives materialization");
        assert!(matches!(first.replacement,
            ironsmith::replacement::ReplacementAction::EnterWithIntrinsicStartingCounters(actual) if actual == rule));
    }
}

}

#[cfg(test)]
mod compiled_surviving_token_group_tests {
    use super::*;
    use ironsmith::effects::{EffectExecutor, CreateTokenEffect};
    use ironsmith::effects::EffectContext as ExecutionContext;
    use ironsmith::events::tokens::matchers::WouldCreateTokensUnderControlMatcher;
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect, EventModification};
    use ironsmith::target::{PlayerFilter, ObjectFilter};
    use ironsmith::{GameState, PlayerId, Zone, ObjectId};
    fn check_compiled_owner(name: &str, text: &str) {
        // Printed metadata and Oracle text match the local cards.json snapshot.
        let (artifact, direct) = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(ironsmith::CardId::new(), name), text, false).unwrap();
        artifact.validate().unwrap();
        let mut registry = ironsmith::cards::CardRegistry::new();
        registry.register_compiled_artifact(&artifact).unwrap();
        let decoded = registry.get(name).unwrap().clone();
        for definition in [direct, decoded] {
            let alice = PlayerId::from_index(0);
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            let source_def = ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Token group modifier")
                .card_types(vec![ironsmith::types::CardType::Artifact]).build();
            let adder = game.create_object_from_definition(&source_def, alice, Zone::Battlefield);
            let remover = game.create_object_from_definition(&source_def, alice, Zone::Battlefield);
            let matcher = || WouldCreateTokensUnderControlMatcher::new(PlayerFilter::Any).with_token_filter(ObjectFilter::creature());
            game.effect_store.replacement_effects.add_resolution_effect(ReplacementEffect::with_matcher(
                adder, alice, matcher(), ReplacementAction::AddTokens { token: serde_json::from_value(serde_json::json!("Treasure")).expect("typed AdditionalTokenKind wire enum"), count: 1 }));
            game.effect_store.replacement_effects.add_resolution_effect(ReplacementEffect::with_matcher(
                remover, alice, matcher(), ReplacementAction::Modify(EventModification::ReduceToZero)));
            struct Ordered(ObjectId, ObjectId);
            impl ironsmith::decision::DecisionMaker for Ordered {
                fn decide_options(&mut self, _game: &GameState, ctx: &ironsmith::decisions::context::SelectOptionsContext) -> Vec<usize> {
                    let option = [self.0, self.1].into_iter().find_map(|source|
                        ctx.options.iter().find(|option| option.legal && option.object_id == Some(source)))
                        .or_else(|| ctx.options.iter().find(|option| option.legal)).unwrap();
                    vec![option.index]
                }
            }
            let token = ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Original creature token")
                .token().card_types(vec![ironsmith::types::CardType::Creature])
                .power_toughness(ironsmith::card::PowerToughness::fixed(1, 1)).build();
            let mut chooser = Ordered(adder, remover);
            let mut ctx = ExecutionContext::new(adder, alice, &mut chooser);
            let outcome = CreateTokenEffect::you(token, 1).execute(&mut game, &mut ctx).unwrap();
            let ids = outcome.result_objects().unwrap();
            assert_eq!(ids.len(), 2, "actual compiled/catalog replacement must modify the surviving added Treasure group");
            assert!(ids.iter().all(|id| game.object(*id).unwrap().subtypes.contains(&ironsmith::types::Subtype::Treasure)));
            assert!(ids.iter().all(|id| game.current_controller(*id) == Some(alice)));
            assert_eq!(game.battlefield.iter().filter(|id| game.object(**id).unwrap().kind == ironsmith::object::ObjectKind::Token).count(), 2);
        }
    }
    #[test]
    fn compiled_parallel_lives_doubles_surviving_added_tokens_after_primary_removal() {
        check_compiled_owner("Parallel Lives", "Mana cost: {3}{G}\nType: Enchantment\nIf an effect would create one or more tokens under your control, it creates twice that many of those tokens instead.");
    }
    #[test]
    fn compiled_xorn_adds_to_surviving_treasure_group_after_primary_removal() {
        check_compiled_owner("Xorn", "Mana cost: {2}{R}\nType: Creature — Elemental\nPower/Toughness: 3/2\nIf you would create one or more Treasure tokens, instead create those tokens plus an additional Treasure token.");
    }
}

#[cfg(test)]
mod compiled_counter_transfer_mode_tests {
    use super::*;
    use ironsmith::{GameState, PlayerId, Zone};
    use ironsmith::effects::{MoveAllCountersEffect, MayEffect};
    use ironsmith::effects::EffectContext as ExecutionContext;
    use ironsmith::effect::Effect;
    use ironsmith::target::ChooseSpec;
    use ironsmith::object::CounterType;
    fn collect(effect: &Effect, out: &mut Vec<MoveAllCountersEffect>) {
        if let Some(value) = effect.downcast_ref::<MoveAllCountersEffect>() { out.push(value.clone()); }
        if let Some(value) = effect.downcast_ref::<MayEffect>() {
            for child in &value.effects { collect(child, out); }
        }
        if let Some(value) = effect.downcast_ref::<ironsmith::effects::TaggedEffect>() {
            collect(&value.effect, out);
        }
    }
    fn compiled_modes(name: &str, text: &str) -> Vec<Vec<MoveAllCountersEffect>> {
        let (artifact, direct) = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(ironsmith::CardId::new(), name), text, false).unwrap();
        artifact.validate().unwrap();
        let mut registry = ironsmith::cards::CardRegistry::new();
        registry.register_compiled_artifact(&artifact).unwrap();
        let decoded = registry.get(name).unwrap().clone();
        [direct, decoded].into_iter().map(|definition| {
            let mut out = Vec::new();
            if let Some(program) = &definition.spell_effect {
                for effect in program.all_effects() { collect(effect, &mut out); }
            }
            for ability in &definition.abilities {
                if let ironsmith::ability::AbilityKind::Triggered(value) = &ability.kind {
                    for effect in value.effects.all_effects() { collect(effect, &mut out); }
                }
            }
            out
        }).collect()
    }
    #[test]
    fn compiled_fate_transfer_is_explicit_movement_in_direct_and_catalog_payloads() {
        for modes in compiled_modes("Fate Transfer", "Mana cost: {1}{U/B}\nType: Instant\nMove all counters from target creature onto another target creature.") {
            assert_eq!(modes.len(), 1);
            assert!(modes[0].remove_from_source);
        }
    }
    #[test]
    fn compiled_ozolith_distinguishes_historical_placement_from_actual_move() {
        let text = "Mana cost: {1}\nType: Legendary Artifact\nWhenever a creature you control leaves the battlefield, if it had counters on it, put those counters on The Ozolith.\nAt the beginning of combat on your turn, if The Ozolith has counters on it, you may move all counters from The Ozolith onto target creature.";
        for modes in compiled_modes("The Ozolith", text) {
            assert_eq!(modes.len(), 2);
            assert!(!modes[0].remove_from_source);
            assert!(modes[1].remove_from_source);
            let alice = PlayerId::from_index(0);
            let artifact = ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Transfer artifact")
                .card_types(vec![ironsmith::types::CardType::Artifact]).build();
            let creature = ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Transfer creature")
                .card_types(vec![ironsmith::types::CardType::Creature]).build();
            // Execute the actual compiled historical placement with its tag and Source destination.
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let source = game.create_object_from_definition(&artifact, alice, Zone::Battlefield);
            let departed = game.create_object_from_definition(&creature, alice, Zone::Battlefield);
            game.object_mut(departed).unwrap().counters.insert(CounterType::Charge, 2);
            let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object(game.object(departed).unwrap(), &game);
            game.move_object_by_effect(departed, Zone::Graveyard).unwrap();
            let ChooseSpec::Tagged(tag) = modes[0].from.base() else { panic!("historical source must retain triggering-object reference: {:?}", modes[0].from); };
            let mut ctx = ExecutionContext::new_default(source, alice);
            ctx.set_tagged_objects(tag.clone(), vec![snapshot]);
            let out = ironsmith::effects::execute_effect(&mut game, &Effect::new(modes[0].clone()), &mut ctx).unwrap();
            assert_eq!(game.counter_count(source, CounterType::Charge), 2);
            assert_eq!(out.events_of_type::<ironsmith::events::MarkersChangedEvent>().count(), 1);
            // Execute actual compiled move: a departed Source must not become historical placement.
            let target = game.create_object_from_definition(&creature, alice, Zone::Battlefield);
            let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
            game.move_object_by_effect(source, Zone::Graveyard).unwrap();
            let mut ctx = ExecutionContext::new_default(source, alice).with_source_snapshot(snapshot);
            ctx.targets.push(ironsmith::effects::ResolvedTarget::Object(target));
            let out = ironsmith::effects::execute_effect(&mut game, &Effect::new(modes[1].clone()), &mut ctx).unwrap();
            assert_eq!(game.counter_count(target, CounterType::Charge), 0);
            assert_eq!(out.count_or_zero(), 0);
            assert_eq!(out.events_of_type::<ironsmith::events::MarkersChangedEvent>().count(), 0);
        }
    }
    #[test]
    fn compiled_fate_transfer_public_resolution_moves_only_between_live_target_roles() {
        let (artifact, direct) = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Fate Transfer"),
            "Mana cost: {1}{U/B}\nType: Instant\nMove all counters from target creature onto another target creature.", false).unwrap();
        let mut registry = ironsmith::cards::CardRegistry::new();
        registry.register_compiled_artifact(&artifact).unwrap();
        let decoded = registry.get("Fate Transfer").unwrap().clone();
        for definition in [direct, decoded] {
            for departed in [false, true] {
                let alice = PlayerId::from_index(0);
                let bob = PlayerId::from_index(1);
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let creature = ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Counter role creature")
                    .card_types(vec![ironsmith::types::CardType::Creature])
                    .power_toughness(ironsmith::card::PowerToughness::fixed(2, 2)).build();
                let from = game.create_object_from_definition(&creature, alice, Zone::Battlefield);
                let to = game.create_object_from_definition(&creature, bob, Zone::Battlefield);
                game.object_mut(from).unwrap().counters.insert(CounterType::Charge, 2);
                let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
                let mut entry = ironsmith::game_state::StackEntry::new(spell, alice);
                entry.targets = vec![ironsmith::Target::Object(from), ironsmith::Target::Object(to)];
                game.push_to_stack(entry);
                if departed { game.move_object_by_effect(from, Zone::Graveyard).unwrap(); }
                ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
                assert!(game.stack.is_empty());
                assert_eq!(game.counter_count(to, CounterType::Charge), if departed { 0 } else { 2 },
                    "actual compiled spell must retain distinct source and recipient roles and must not copy former counters");
                if !departed { assert_eq!(game.counter_count(from, CounterType::Charge), 0); }
            }
        }
    }
    #[test]
    fn compiled_fate_transfer_cast_declares_and_pays_for_two_distinct_target_roles() {
        use ironsmith::game_loop::*;
        let definition = compile_to_runtime_definition("Fate Transfer",
            "Mana cost: {1}{U/B}\nType: Instant\nMove all counters from target creature onto another target creature.", false).unwrap();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.phase = ironsmith::game_state::Phase::FirstMain;
        game.player_mut(alice).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Blue, 2);
        let creature = ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Cast target role creature")
            .card_types(vec![ironsmith::types::CardType::Creature])
            .power_toughness(ironsmith::card::PowerToughness::fixed(2, 2)).build();
        let from = game.create_object_from_definition(&creature, alice, Zone::Battlefield);
        let to = game.create_object_from_definition(&creature, bob, Zone::Battlefield);
        game.object_mut(from).unwrap().counters.insert(CounterType::Charge, 2);
        let spell = game.create_object_from_definition(&definition, alice, Zone::Hand);
        let requirements = extract_target_requirements_from_program_with_modes(
            &game, definition.spell_effect.as_ref().unwrap(), alice, Some(spell), None);
        assert_eq!(requirements.len(), 2, "both counter transfer endpoints must be announced while casting");
        assert!(requirements.iter().all(|r| r.min_targets == 1 && r.max_targets == Some(1)));
        let mut state = PriorityLoopState::new(2);
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
        let action = ironsmith::decision::LegalAction::CastSpell {
            spell_id: spell, from_zone: Zone::Hand,
            casting_method: ironsmith::alternative_cast::CastingMethod::Normal,
        };
        let mut progress = apply_priority_response_with_dm(&mut game, &mut queue, &mut state,
            &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
        for _ in 0..30 {
            if state.pending_cast.is_none() && !game.stack.is_empty() { break; }
            let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress else { break; };
            progress = if let ironsmith::decisions::context::DecisionContext::Targets(context) = &ctx {
                assert_eq!(context.requirements.len(), 2);
                apply_priority_response_with_dm(&mut game, &mut queue, &mut state,
                    &PriorityResponse::Targets(vec![ironsmith::Target::Object(from), ironsmith::Target::Object(to)]), &mut dm).unwrap()
            } else { apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &ctx, &mut dm).unwrap() };
        }
        assert!(state.pending_cast.is_none());
        assert_eq!(game.stack.len(), 1);
        assert_eq!(game.stack[0].targets, vec![ironsmith::Target::Object(from), ironsmith::Target::Object(to)]);
        assert_eq!(game.stack[0].target_assignments.len(), 2);
        assert_eq!(game.player(alice).unwrap().mana_pool.blue, 0);
        resolve_stack_entry(&mut game).unwrap();
        assert_eq!(game.counter_count(from, CounterType::Charge), 0);
        assert_eq!(game.counter_count(to, CounterType::Charge), 2);
    }
    #[test]
    fn compiled_bioshift_cast_selects_one_counter_between_same_controller_roles() {
        use ironsmith::game_loop::*;
        let definition = compile_to_runtime_definition("Bioshift",
            "Mana cost: {G/U}\nType: Instant\nMove any number of +1/+1 counters from target creature onto another target creature with the same controller.", false).unwrap();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.phase = ironsmith::game_state::Phase::FirstMain;
        game.player_mut(alice).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Green, 1);
        let creature = ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Cast target role creature")
            .card_types(vec![ironsmith::types::CardType::Creature])
            .power_toughness(ironsmith::card::PowerToughness::fixed(2, 2)).build();
        let from = game.create_object_from_definition(&creature, alice, Zone::Battlefield);
        let to = game.create_object_from_definition(&creature, alice, Zone::Battlefield);
        game.object_mut(from).unwrap().counters.insert(CounterType::PlusOnePlusOne, 2);
        let spell = game.create_object_from_definition(&definition, alice, Zone::Hand);
        let requirements = extract_target_requirements_from_program_with_modes(
            &game, definition.spell_effect.as_ref().unwrap(), alice, Some(spell), None);
        assert_eq!(requirements.len(), 2, "both counter transfer endpoints must be announced while casting");
        assert!(requirements.iter().all(|r| r.min_targets == 1 && r.max_targets == Some(1)));
        let mut state = PriorityLoopState::new(2);
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
        let action = ironsmith::decision::LegalAction::CastSpell {
            spell_id: spell, from_zone: Zone::Hand,
            casting_method: ironsmith::alternative_cast::CastingMethod::Normal,
        };
        let mut progress = apply_priority_response_with_dm(&mut game, &mut queue, &mut state,
            &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
        for _ in 0..30 {
            if state.pending_cast.is_none() && !game.stack.is_empty() { break; }
            let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress else { break; };
            progress = if let ironsmith::decisions::context::DecisionContext::Targets(context) = &ctx {
                assert_eq!(context.requirements.len(), 2);
                apply_priority_response_with_dm(&mut game, &mut queue, &mut state,
                    &PriorityResponse::Targets(vec![ironsmith::Target::Object(from), ironsmith::Target::Object(to)]), &mut dm).unwrap()
            } else { apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &ctx, &mut dm).unwrap() };
        }
        assert!(state.pending_cast.is_none());
        assert_eq!(game.stack.len(), 1);
        assert_eq!(game.stack[0].targets, vec![ironsmith::Target::Object(from), ironsmith::Target::Object(to)]);
        assert_eq!(game.stack[0].target_assignments.len(), 2);
        assert_eq!(game.player(alice).unwrap().mana_pool.green, 0);
        struct PickOne { choices: usize }
        impl ironsmith::decision::DecisionMaker for PickOne {
            fn decide_boolean(&mut self, _game: &GameState, _ctx: &ironsmith::decisions::context::BooleanContext) -> bool { true }
            fn decide_number(&mut self, _game: &GameState, ctx: &ironsmith::decisions::context::NumberContext) -> u32 {
                assert!(ctx.min <= 1 && ctx.max >= 1); self.choices += 1; 1
            }
            fn decide_counters(&mut self, _game: &GameState, ctx: &ironsmith::decisions::context::CountersContext) -> Vec<(CounterType, u32)> {
                assert!(ctx.available_counters.iter().any(|(kind, count)| *kind == CounterType::PlusOnePlusOne && *count >= 1));
                self.choices += 1; vec![(CounterType::PlusOnePlusOne, 1)]
            }
        }
        let mut chooser = PickOne { choices: 0 };
        resolve_stack_entry_with(&mut game, &mut chooser).unwrap();
        assert!(chooser.choices > 0, "actual any-number transfer must offer a legal subset choice");
        assert_eq!(game.counter_count(from, CounterType::PlusOnePlusOne), 1);
        assert_eq!(game.counter_count(to, CounterType::PlusOnePlusOne), 1);
    }
    #[test]
    fn compiled_bioshift_rejects_opposing_controller_endpoint_during_public_cast() {
        use ironsmith::game_loop::*;
        let definition = compile_to_runtime_definition("Bioshift",
            "Mana cost: {G/U}\nType: Instant\nMove any number of +1/+1 counters from target creature onto another target creature with the same controller.", false).unwrap();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.phase = ironsmith::game_state::Phase::FirstMain;
        game.player_mut(alice).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Green, 1);
        let creature = ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Cast target role creature")
            .card_types(vec![ironsmith::types::CardType::Creature])
            .power_toughness(ironsmith::card::PowerToughness::fixed(2, 2)).build();
        let from = game.create_object_from_definition(&creature, alice, Zone::Battlefield);
        let to = game.create_object_from_definition(&creature, bob, Zone::Battlefield);
        game.object_mut(from).unwrap().counters.insert(CounterType::PlusOnePlusOne, 2);
        let spell = game.create_object_from_definition(&definition, alice, Zone::Hand);
        let requirements = extract_target_requirements_from_program_with_modes(
            &game, definition.spell_effect.as_ref().unwrap(), alice, Some(spell), None);
        assert_eq!(requirements.len(), 2, "both counter transfer endpoints must be announced while casting");
        assert!(requirements.iter().all(|r| r.min_targets == 1 && r.max_targets == Some(1)));
        let mut state = PriorityLoopState::new(2);
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
        let action = ironsmith::decision::LegalAction::CastSpell {
            spell_id: spell, from_zone: Zone::Hand,
            casting_method: ironsmith::alternative_cast::CastingMethod::Normal,
        };
        let mut progress = apply_priority_response_with_dm(&mut game, &mut queue, &mut state,
            &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
        for _ in 0..30 {
            if state.pending_cast.is_none() && !game.stack.is_empty() { break; }
            let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress else { break; };
            progress = if let ironsmith::decisions::context::DecisionContext::Targets(context) = &ctx {
                assert_eq!(context.requirements.len(), 2);
                let rejected = apply_priority_response_with_dm(&mut game, &mut queue, &mut state,
                    &PriorityResponse::Targets(vec![ironsmith::Target::Object(from), ironsmith::Target::Object(to)]), &mut dm);
                assert!(rejected.is_err(), "same-controller movement must reject the opposing endpoint while announcing targets");
                assert!(game.stack.is_empty());
                assert_eq!(game.counter_count(from, CounterType::PlusOnePlusOne), 2);
                assert_eq!(game.counter_count(to, CounterType::PlusOnePlusOne), 0);
                return;
            } else { apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &ctx, &mut dm).unwrap() };
        }
        panic!("actual Bioshift casting must request both endpoint roles");
    }
    #[test]
    fn retained_counter_move_amount_preserves_exact_and_any_number_and_rejects_missing_mode() {
        use ironsmith::effects::MoveCountersEffect;
        use ironsmith::target::ChooseSpec;
        for model in [
            MoveCountersEffect::new(CounterType::PlusOnePlusOne, 2, ChooseSpec::Source, ChooseSpec::creature()),
            MoveCountersEffect::any_number(CounterType::PlusOnePlusOne, ChooseSpec::Source, ChooseSpec::creature()),
        ] {
            let wire=serde_json::to_value(&model).unwrap();
            let restored:MoveCountersEffect=serde_json::from_value(wire.clone()).unwrap();
            assert_eq!(restored,model);
            assert_eq!(serde_json::to_value(&restored).unwrap(),wire);
            let mut missing=wire.clone();missing.as_object_mut().unwrap().remove("count");
            assert!(serde_json::from_value::<MoveCountersEffect>(missing).is_err());
            let mut legacy=wire.clone();legacy["count"]=serde_json::json!({"Fixed":2});
            assert!(serde_json::from_value::<MoveCountersEffect>(legacy).is_err(),"an untyped legacy count cannot silently become a chosen movement amount");
            let mut unknown=wire;unknown["count"]=serde_json::json!({"Unknown":null});
            assert!(serde_json::from_value::<MoveCountersEffect>(unknown).is_err());
        }
    }
    #[test]
    fn compiled_fate_transfer_cast_preserves_empty_illegal_endpoint_roles() {
        use ironsmith::game_loop::*;
        for departed_source in [true, false] {
        let definition = compile_to_runtime_definition("Fate Transfer",
            "Mana cost: {1}{U/B}\nType: Instant\nMove all counters from target creature onto another target creature.", false).unwrap();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.phase = ironsmith::game_state::Phase::FirstMain;
        game.player_mut(alice).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Blue, 2);
        let creature = ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Cast target role creature")
            .card_types(vec![ironsmith::types::CardType::Creature])
            .power_toughness(ironsmith::card::PowerToughness::fixed(2, 2)).build();
        let from = game.create_object_from_definition(&creature, alice, Zone::Battlefield);
        let to = game.create_object_from_definition(&creature, bob, Zone::Battlefield);
        game.object_mut(from).unwrap().counters.insert(CounterType::Charge, 2);
        let spell = game.create_object_from_definition(&definition, alice, Zone::Hand);
        let requirements = extract_target_requirements_from_program_with_modes(
            &game, definition.spell_effect.as_ref().unwrap(), alice, Some(spell), None);
        assert_eq!(requirements.len(), 2, "both counter transfer endpoints must be announced while casting");
        assert!(requirements.iter().all(|r| r.min_targets == 1 && r.max_targets == Some(1)));
        let mut state = PriorityLoopState::new(2);
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
        let action = ironsmith::decision::LegalAction::CastSpell {
            spell_id: spell, from_zone: Zone::Hand,
            casting_method: ironsmith::alternative_cast::CastingMethod::Normal,
        };
        let mut progress = apply_priority_response_with_dm(&mut game, &mut queue, &mut state,
            &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
        for _ in 0..30 {
            if state.pending_cast.is_none() && !game.stack.is_empty() { break; }
            let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress else { break; };
            progress = if let ironsmith::decisions::context::DecisionContext::Targets(context) = &ctx {
                assert_eq!(context.requirements.len(), 2);
                apply_priority_response_with_dm(&mut game, &mut queue, &mut state,
                    &PriorityResponse::Targets(vec![ironsmith::Target::Object(from), ironsmith::Target::Object(to)]), &mut dm).unwrap()
            } else { apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &ctx, &mut dm).unwrap() };
        }
        assert!(state.pending_cast.is_none());
        assert_eq!(game.stack.len(), 1);
        assert_eq!(game.stack[0].targets, vec![ironsmith::Target::Object(from), ironsmith::Target::Object(to)]);
        assert_eq!(game.stack[0].target_assignments.len(), 2);
        assert_eq!(game.player(alice).unwrap().mana_pool.blue, 0);
        game.move_object_by_effect(if departed_source { from } else { to }, Zone::Graveyard).unwrap();
        resolve_stack_entry(&mut game).unwrap();
        assert_eq!(game.counter_count(from, CounterType::Charge), if departed_source { 0 } else { 2 },
            "surviving source must retain counters when recipient is illegal");
        assert_eq!(game.counter_count(to, CounterType::Charge), 0,
            "surviving recipient must not receive departed source counters");
        }

    }
    fn activated_counter_owner(name: &str, text: &str, single_any_kind: bool) {
        use ironsmith::game_loop::*;
        let (artifact, direct) = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(ironsmith::CardId::new(), name), text, false).unwrap();
        artifact.validate().unwrap();
        let mut registry = ironsmith::cards::CardRegistry::new();
        registry.register_compiled_artifact(&artifact).unwrap();
        let decoded = registry.get(name).unwrap().clone();
        for definition in [direct, decoded] {
            let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            game.turn.phase = ironsmith::game_state::Phase::FirstMain;
            let symbol = if single_any_kind { ironsmith::mana::ManaSymbol::Colorless } else { ironsmith::mana::ManaSymbol::Black };
            game.player_mut(alice).unwrap().mana_pool.add(symbol, if single_any_kind { 1 } else { 3 });
            let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            // Set up a surviving Daghatar after its printed four-counter entry.
            // This activation scenario does not claim to test its entry ability.
            if !single_any_kind { game.object_mut(source).unwrap().counters.insert(CounterType::PlusOnePlusOne, 4); }
            let creature = ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Activated endpoint creature")
                .card_types(vec![ironsmith::types::CardType::Creature])
                .power_toughness(ironsmith::card::PowerToughness::fixed(2, 2)).build();
            let from = game.create_object_from_definition(&creature, alice, Zone::Battlefield);
            let to = game.create_object_from_definition(&creature, bob, Zone::Battlefield);
            game.object_mut(from).unwrap().counters.insert(CounterType::PlusOnePlusOne, 2);
            let (ability_index, ability) = definition.abilities.iter().enumerate().find_map(|(index, ability)| {
                if let ironsmith::ability::AbilityKind::Activated(value) = &ability.kind {
                    value.mana_output.is_none().then_some((index, value))
                } else { None }
            }).expect("actual nonmana activated counter transfer");
            fn contains(effect: &Effect, single: bool) -> bool {
                if single && effect.downcast_ref::<ironsmith::effects::MoveOneCounterEffect>().is_some() { return true; }
                if !single && effect.downcast_ref::<ironsmith::effects::MoveCountersEffect>().is_some() { return true; }
                effect.transparent_child_effect().is_some_and(|child| contains(child, single))
            }
            assert!(ability.effects.all_effects().into_iter().any(|effect| contains(effect, single_any_kind)),
                "actual Oracle/card artifact must compile to expected counter owner");
            let requirements = extract_target_requirements_from_program_with_modes(&game, &ability.effects, alice, Some(source), None);
            assert_eq!(requirements.len(), 2);
            let mut state = PriorityLoopState::new(2); let mut queue = ironsmith::triggers::TriggerQueue::new();
            let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
            let action = ironsmith::decision::LegalAction::ActivateAbility { source, ability_index };
            let mut progress = apply_priority_response_with_dm(&mut game, &mut queue, &mut state,
                &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
            for _ in 0..30 {
                if state.pending_activation.is_none() && !game.stack.is_empty() { break; }
                let ironsmith::GameProgress::NeedsDecisionCtx(ctx) = progress else { break; };
                progress = if let ironsmith::decisions::context::DecisionContext::Targets(context) = &ctx {
                    assert_eq!(context.requirements.len(), 2);
                    apply_priority_response_with_dm(&mut game, &mut queue, &mut state,
                        &PriorityResponse::Targets(vec![ironsmith::Target::Object(from), ironsmith::Target::Object(to)]), &mut dm).unwrap()
                } else { apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &ctx, &mut dm).unwrap() };
            }
            assert!(state.pending_activation.is_none()); assert_eq!(game.stack.len(), 1);
            assert_eq!(game.stack[0].targets, vec![ironsmith::Target::Object(from), ironsmith::Target::Object(to)]);
            assert_eq!(game.stack[0].target_assignments.len(), 2);
            if single_any_kind {
                assert!(game.is_tapped(source)); assert_eq!(game.player(alice).unwrap().mana_pool.colorless, 0);
            } else { assert_eq!(game.player(alice).unwrap().mana_pool.black, 0); }
            resolve_stack_entry(&mut game).unwrap();
            assert_eq!(game.counter_count(from, CounterType::PlusOnePlusOne), 1);
            assert_eq!(game.counter_count(to, CounterType::PlusOnePlusOne), 1);
        }
    }
    #[test]
    fn compiled_nesting_grounds_activates_single_counter_between_two_roles() {
        activated_counter_owner("Nesting Grounds", "Type: Land\n{T}: Add {C}.\n{1}, {T}: Move a counter from target permanent you control onto a second target permanent. Activate only as a sorcery.", true);
    }
    #[test]
    fn compiled_daghatar_activates_fixed_counter_between_two_roles() {
        activated_counter_owner("Daghatar the Adamant", "Mana cost: {3}{W}\nType: Legendary Creature — Human Warrior\nPower/Toughness: 0/0\nVigilance\nDaghatar enters with four +1/+1 counters on it.\n{1}{B/G}{B/G}: Move a +1/+1 counter from target creature onto a second target creature.", false);
    }

    #[test]
    fn retained_counter_transfer_mode_is_required_and_preserved() {
        for remove in [true, false] {
            let payload = if remove { MoveAllCountersEffect::new(ChooseSpec::Source, ChooseSpec::Source) }
                else { MoveAllCountersEffect::put_referenced(ChooseSpec::Source, ChooseSpec::Source) };
            let mut wire = serde_json::to_value(&payload).unwrap();
            assert_eq!(wire["remove_from_source"], remove);
            let restored: MoveAllCountersEffect = serde_json::from_value(wire.clone()).unwrap();
            assert_eq!(restored, payload);
            wire.as_object_mut().unwrap().remove("remove_from_source");
            assert!(serde_json::from_value::<MoveAllCountersEffect>(wire).is_err(),
                "ambiguous retained intent must be rejected, not guessed from snapshot state");
        }
    }
}

#[cfg(test)]
mod retained_unsigned_quantity_codec_tests {
    use super::*;
    use ironsmith::{GameState,PlayerId,Zone};
    use ironsmith::effect::{EffectOutcome,EffectId,Value};
    use ironsmith::effects::{PutCountersEffect,RemoveCountersEffect,MoveAllCountersEffect};
    use ironsmith::target::ChooseSpec;
    use ironsmith::object::CounterType;
    fn object(game:&mut GameState,alice:PlayerId)->ironsmith::ObjectId {
        let card=ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::CardId::new(),"Retained quantity recipient")
            .card_types(vec![ironsmith::types::CardType::Artifact]).build();
        game.create_object_from_definition(&card,alice,Zone::Battlefield)
    }
    fn restored(kind:&str,payload:serde_json::Value)->ironsmith::Effect {
        let wire=ironsmith_compiled_artifact::WireEffect::new(kind,payload);
        let text=serde_json::to_string(&wire).unwrap();
        let restored=ironsmith_runtime_catalog::artifact_materializer::materialize_effect(serde_json::from_str(&text).unwrap()).unwrap();
        let retained:ironsmith_compiled_artifact::WireEffect=serde_json::from_str(restored.serialized_model().expect("materializer retains canonical executable model")).unwrap();
        assert_eq!(retained.kind(),wire.kind());assert_eq!(retained.payload(),wire.payload());restored
    }
    fn outcome(out:&EffectOutcome)->EffectOutcome {
        // EffectOutcome serialization is documented only for event-free receipts.
        // Trigger events are owned by the live execution stream, not this codec.
        assert!(!out.events.is_empty(), "real execution must produce its marker events");
        let mut receipt=out.clone();receipt.events.clear();
        let restored:EffectOutcome=serde_json::from_str(&serde_json::to_string(&receipt).unwrap()).unwrap();
        assert_eq!(restored,receipt);restored
    }
    #[test]
    fn retained_unsigned_quantity_literal_executes_through_materializer() {
        for amount in [i32::MAX as u32,i32::MAX as u32+1,u32::MAX] {
            let alice=PlayerId::from_index(0);let mut game=GameState::new(vec!["Alice".into(),"Bob".into()],20);let source=object(&mut game,alice);
            let effect=restored("PutCountersEffect",serde_json::to_value(&PutCountersEffect::new(CounterType::Charge,amount,ChooseSpec::SpecificObject(source))).unwrap());
            let mut ctx=ironsmith::effects::EffectContext::new_default(source,alice);
            let out=ironsmith::effects::execute_effect(&mut game,&effect,&mut ctx).unwrap();
            assert_eq!(game.counter_count(source,CounterType::Charge),amount);assert_eq!(outcome(&out).as_count(),Some(i64::from(amount)));
        }
    }
    #[test]
    fn retained_unsigned_quantity_receipt_drives_materialized_removal() {
        for amount in [i32::MAX as u32+1,u32::MAX] {
            let alice=PlayerId::from_index(0);let mut game=GameState::new(vec!["Alice".into(),"Bob".into()],20);let source=object(&mut game,alice);
            let put=restored("PutCountersEffect",serde_json::to_value(&PutCountersEffect::new(CounterType::Charge,amount,ChooseSpec::SpecificObject(source))).unwrap());
            let remove=restored("RemoveCountersEffect",serde_json::to_value(&RemoveCountersEffect::new(CounterType::Charge,Value::EffectValue(EffectId(23)),ChooseSpec::SpecificObject(source))).unwrap());
            let mut ctx=ironsmith::effects::EffectContext::new_default(source,alice);
            let placed=ironsmith::effects::execute_effect(&mut game,&put,&mut ctx).unwrap();ctx.store_outcome(EffectId(23),outcome(&placed));
            let removed=ironsmith::effects::execute_effect(&mut game,&remove,&mut ctx).unwrap();
            assert_eq!(game.counter_count(source,CounterType::Charge),0);assert_eq!(outcome(&removed).as_count(),Some(i64::from(amount)));
        }
    }
    #[test]
    fn retained_unsigned_quantity_mixed_counter_sum_drives_wide_arithmetic() {
        let alice=PlayerId::from_index(0);let mut game=GameState::new(vec!["Alice".into(),"Bob".into()],20);let source=object(&mut game,alice);let target=object(&mut game,alice);let following=object(&mut game,alice);
        let mut ctx=ironsmith::effects::EffectContext::new_default(source,alice);
        for kind in [CounterType::Charge,CounterType::PlusOnePlusOne] {
            let put=restored("PutCountersEffect",serde_json::to_value(&PutCountersEffect::new(kind,u32::MAX,ChooseSpec::SpecificObject(source))).unwrap());
            ironsmith::effects::execute_effect(&mut game,&put,&mut ctx).unwrap();
        }
        let movement=restored("MoveAllCountersEffect",serde_json::to_value(&MoveAllCountersEffect::new(ChooseSpec::SpecificObject(source),ChooseSpec::SpecificObject(target))).unwrap());
        let moved=ironsmith::effects::execute_effect(&mut game,&movement,&mut ctx).unwrap();
        assert_eq!(moved.as_count(),Some(2*i64::from(u32::MAX)));ctx.store_outcome(EffectId(23),outcome(&moved));
        for kind in [CounterType::Charge,CounterType::PlusOnePlusOne] {assert_eq!(game.counter_count(source,kind),0);assert_eq!(game.counter_count(target,kind),u32::MAX);}
        let put=restored("PutCountersEffect",serde_json::to_value(&PutCountersEffect::new(CounterType::Charge,Value::HalfRoundedDown(Box::new(Value::EffectValue(EffectId(23)))),ChooseSpec::SpecificObject(following))).unwrap());
        let out=ironsmith::effects::execute_effect(&mut game,&put,&mut ctx).unwrap();assert_eq!(game.counter_count(following,CounterType::Charge),u32::MAX);assert_eq!(outcome(&out).as_count(),Some(i64::from(u32::MAX)));
    }
}

#[cfg(test)]
mod retained_unsigned_counter_cost_codec_tests {
    use super::*;
    use ironsmith::{GameState,PlayerId,Zone};
    use ironsmith::object::CounterType;
    fn restored(cost:&ironsmith::costs::Cost)->ironsmith::costs::Cost {
        let wire=cost.compiled_model().expect("public counter cost retains canonical model").clone()
            .try_map_effect(|effect| serde_json::from_str::<ironsmith_compiled_artifact::WireEffect>(effect.serialized_model().expect("nested effect retains executable model"))).unwrap();
        let json=serde_json::to_string(&wire).unwrap();
        let decoded:ironsmith_compiled_artifact::WireCost=serde_json::from_str(&json).unwrap();
        assert_eq!(serde_json::to_value(&decoded).unwrap(),serde_json::to_value(&wire).unwrap());
        let native=decoded.try_map_effect(ironsmith_runtime_catalog::artifact_materializer::materialize_effect).unwrap();
        let result=ironsmith::costs::Cost::from_model(native).unwrap();assert_eq!(result.display(),cost.display());
        let again=result.compiled_model().unwrap().clone().try_map_effect(|effect| serde_json::from_str::<ironsmith_compiled_artifact::WireEffect>(effect.serialized_model().unwrap())).unwrap();
        assert_eq!(serde_json::to_value(&again).unwrap(),serde_json::to_value(&wire).unwrap());result
    }
    fn check(add:bool) {
        for amount in [i32::MAX as u32,i32::MAX as u32+1,u32::MAX] {
            let alice=PlayerId::from_index(0);let mut game=GameState::new(vec!["Alice".into(),"Bob".into()],20);
            let card=ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::CardId::new(),"Restored counter cost recipient").card_types(vec![ironsmith::types::CardType::Artifact]).build();
            let source=game.create_object_from_definition(&card,alice,Zone::Battlefield);
            let cost=if add {ironsmith::costs::Cost::add_counters(CounterType::Charge,amount)} else {game.object_mut(source).unwrap().counters.insert(CounterType::Charge,amount);ironsmith::costs::Cost::remove_counters(CounterType::Charge,amount)};
            let cost=restored(&cost);let mut dm=ironsmith::decision::SelectFirstDecisionMaker;let mut ctx=ironsmith::costs::CostContext::new(source,alice,&mut dm);
            assert_eq!(cost.pay(&mut game,&mut ctx).unwrap(),ironsmith::costs::CostPaymentResult::Paid);
            if !add {assert_eq!(ctx.x_value,Some(amount));}
            assert_eq!(game.counter_count(source,CounterType::Charge),if add {amount} else {0});
        }
    }
    #[test] fn retained_unsigned_counter_cost_removal_executes_after_roundtrip() {check(false);}
    #[test] fn retained_unsigned_counter_cost_placement_executes_after_roundtrip() {check(true);}
}

#[cfg(test)]
mod retained_unsigned_player_counter_codec_tests {
    use super::*;
    use ironsmith::{GameState,PlayerId,Zone};
    use ironsmith::effect::{EffectOutcome,EffectId,Value};
    use ironsmith::object::CounterType;
    use ironsmith::target::{ChooseSpec,PlayerFilter};
    fn restored(kind:&str,payload:serde_json::Value)->ironsmith::Effect {
        let wire=ironsmith_compiled_artifact::WireEffect::new(kind,payload);let text=serde_json::to_string(&wire).unwrap();
        let native=ironsmith_runtime_catalog::artifact_materializer::materialize_effect(serde_json::from_str(&text).unwrap()).unwrap();
        let retained:ironsmith_compiled_artifact::WireEffect=serde_json::from_str(native.serialized_model().unwrap()).unwrap();assert_eq!(retained.kind(),wire.kind());assert_eq!(retained.payload(),wire.payload());native
    }
    fn receipt(out:&EffectOutcome)->EffectOutcome {
        assert!(!out.events.is_empty());let mut receipt=out.clone();receipt.events.clear();
        let decoded:EffectOutcome=serde_json::from_str(&serde_json::to_string(&receipt).unwrap()).unwrap();assert_eq!(decoded,receipt);decoded
    }
    fn object(game:&mut GameState,alice:PlayerId)->ironsmith::ObjectId {
        let card=ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::CardId::new(),"Retained player quantity source").card_types(vec![ironsmith::types::CardType::Artifact]).build();game.create_object_from_definition(&card,alice,Zone::Battlefield)
    }
    fn check(kind:CounterType) {
        for amount in [i32::MAX as u32,i32::MAX as u32+1,u32::MAX] {
            let alice=PlayerId::from_index(0);let bob=PlayerId::from_index(1);let mut game=GameState::new(vec!["Alice".into(),"Bob".into()],20);let source=object(&mut game,alice);let following=object(&mut game,alice);
            let mut ctx=ironsmith::effects::EffectContext::new_default(source,alice);
            let prior=restored("PutCountersEffect",serde_json::to_value(ironsmith::effects::PutCountersEffect::new(CounterType::Charge,amount,ChooseSpec::SpecificObject(source))).unwrap());
            let placed=ironsmith::effects::execute_effect(&mut game,&prior,&mut ctx).unwrap();assert_eq!(placed.as_count(),Some(i64::from(amount)));ctx.store_outcome(EffectId(31),receipt(&placed));
            let value=Value::EffectValue(EffectId(31));let player=PlayerFilter::Specific(bob);
            let (class,payload)=match kind {
                CounterType::Energy=>("EnergyCountersEffect",serde_json::to_value(ironsmith::effects::EnergyCountersEffect::new(value,player)).unwrap()),
                CounterType::Experience=>("ExperienceCountersEffect",serde_json::json!({"count":value,"player":player})),
                CounterType::Poison=>("PoisonCountersEffect",serde_json::json!({"count":value,"player":player})),
                ticket if ticket==CounterType::Named("ticket".into())=>("TicketCountersEffect",serde_json::to_value(ironsmith::effects::TicketCountersEffect::new(value,player)).unwrap()),
                _=>("GivePlayerCountersEffect",serde_json::json!({"counter_type":kind,"count":value,"player":player})),
            };
            let effect=restored(class,payload);let out=ironsmith::effects::execute_effect(&mut game,&effect,&mut ctx).unwrap();assert_eq!(out.as_count(),Some(i64::from(amount)));assert_eq!(game.player(bob).unwrap().counter_count(kind),amount);assert_eq!(game.player(alice).unwrap().counter_count(kind),0);
            let marker=out.events.iter().find_map(|event|event.downcast::<ironsmith::events::MarkersChangedEvent>()).unwrap();assert_eq!(marker.amount,amount);assert_eq!(marker.count_after,Some(amount));assert_eq!(marker.location,ironsmith::marker::MarkerLocation::Player(bob));
            ctx.store_outcome(EffectId(57),receipt(&out));
            let follow=restored("PutCountersEffect",serde_json::to_value(ironsmith::effects::PutCountersEffect::new(CounterType::Charge,Value::EffectValue(EffectId(57)),ChooseSpec::SpecificObject(following))).unwrap());
            let out=ironsmith::effects::execute_effect(&mut game,&follow,&mut ctx).unwrap();assert_eq!(out.as_count(),Some(i64::from(amount)));assert_eq!(game.counter_count(following,CounterType::Charge),amount);
        }
    }
    #[test] fn retained_unsigned_player_counter_energy_executes_and_preserves_receipt() {check(CounterType::Energy);}
    #[test] fn retained_unsigned_player_counter_experience_executes_and_preserves_receipt() {check(CounterType::Experience);}
    #[test] fn retained_unsigned_player_counter_poison_executes_and_preserves_receipt() {check(CounterType::Poison);}
    #[test] fn retained_unsigned_player_counter_generic_executes_and_preserves_receipt() {check(CounterType::Rad);}
    #[test] fn retained_unsigned_player_counter_ticket_executes_and_preserves_receipt() {check(CounterType::Named("ticket".into()));}
}

#[cfg(test)]
mod retained_wide_removal_and_pending_result_contract_tests {
    use super::*;
    use ironsmith::{GameState,PlayerId,ObjectId,Zone};
    use ironsmith::effect::{EffectOutcome,EffectId,Value};
    use ironsmith::effects::{EffectContext,PutCountersEffect,GainLifeEffect};
    use ironsmith::object::CounterType;
    use ironsmith::target::{ChooseSpec,PlayerFilter,ObjectFilter};
    fn wire(kind:&str,payload:serde_json::Value)->ironsmith_compiled_artifact::WireEffect {ironsmith_compiled_artifact::WireEffect::new(kind,payload)}
    fn materialized(wire:ironsmith_compiled_artifact::WireEffect)->ironsmith::Effect {
        let json=serde_json::to_string(&wire).unwrap();let native=ironsmith_runtime_catalog::artifact_materializer::materialize_effect(serde_json::from_str(&json).unwrap()).unwrap();let retained:ironsmith_compiled_artifact::WireEffect=serde_json::from_str(native.serialized_model().unwrap()).unwrap();assert_eq!(retained.kind(),wire.kind());assert_eq!(retained.payload(),wire.payload());native
    }
    fn receipt(out:&EffectOutcome)->EffectOutcome {
        assert!(!out.events.is_empty());let mut value=out.clone();value.events.clear();let decoded:EffectOutcome=serde_json::from_str(&serde_json::to_string(&value).unwrap()).unwrap();assert_eq!(decoded,value);decoded
    }
    fn object(game:&mut GameState,alice:PlayerId)->ObjectId {
        let card=ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::CardId::new(),"Materialized removal result owner").card_types(vec![ironsmith::types::CardType::Artifact]).build();game.create_object_from_definition(&card,alice,Zone::Battlefield)
    }
    fn put(game:&mut GameState,ctx:&mut EffectContext,id:ObjectId,kind:CounterType,count:u32,result:u32) {
        let effect=materialized(wire("PutCountersEffect",serde_json::to_value(PutCountersEffect::new(kind,count,ChooseSpec::SpecificObject(id))).unwrap()));let out=ironsmith::effects::execute_effect(game,&effect,ctx).unwrap();assert_eq!(out.as_count(),Some(i64::from(count)));assert_eq!(game.counter_count(id,kind),count);ctx.store_outcome(EffectId(result),receipt(&out));
    }
    fn bounded(any:bool) {
        for amount in [i32::MAX as u32,i32::MAX as u32+1,u32::MAX] {
            let alice=PlayerId::from_index(0);let mut game=GameState::new(vec!["Alice".into(),"Bob".into()],20);let source=object(&mut game,alice);let target=object(&mut game,alice);let following=object(&mut game,alice);let mut ctx=EffectContext::new_default(source,alice);put(&mut game,&mut ctx,source,CounterType::Charge,amount,31);put(&mut game,&mut ctx,target,CounterType::Charge,3,32);
            let value=Value::EffectValue(EffectId(31));let payload=if any{serde_json::json!({"max_count":value,"target":ChooseSpec::SpecificObject(target),"up_to":true})}else{serde_json::json!({"counter_type":CounterType::Charge,"max_count":value,"target":ChooseSpec::SpecificObject(target)})};let effect=materialized(wire(if any{"RemoveUpToAnyCountersEffect"}else{"RemoveUpToCountersEffect"},payload));let out=ironsmith::effects::execute_effect(&mut game,&effect,&mut ctx).unwrap();assert_eq!(out.as_count(),Some(3));assert_eq!(game.counter_count(target,CounterType::Charge),0);assert_eq!(game.counter_count(source,CounterType::Charge),amount);ctx.store_outcome(EffectId(57),receipt(&out));
            let follow=materialized(wire("PutCountersEffect",serde_json::to_value(PutCountersEffect::new(CounterType::Charge,Value::EffectValue(EffectId(57)),ChooseSpec::SpecificObject(following))).unwrap()));assert_eq!(ironsmith::effects::execute_effect(&mut game,&follow,&mut ctx).unwrap().as_count(),Some(3));assert_eq!(game.counter_count(following,CounterType::Charge),3);
        }
    }
    #[test]fn materialized_typed_up_to_caps_unsigned_actual_prior(){bounded(false);}
    #[test]fn materialized_any_up_to_caps_unsigned_actual_prior(){bounded(true);}
    fn wide(typed:bool) {
        let alice=PlayerId::from_index(0);let mut game=GameState::new(vec!["Alice".into(),"Bob".into()],20);let source=object(&mut game,alice);let other=if typed{object(&mut game,alice)}else{source};let following=object(&mut game,alice);let mut ctx=EffectContext::new_default(source,alice);put(&mut game,&mut ctx,source,CounterType::Charge,u32::MAX,31);put(&mut game,&mut ctx,other,if typed{CounterType::Charge}else{CounterType::PlusOnePlusOne},u32::MAX,32);
        let mut filter=ObjectFilter::permanent().you_control();if !typed{filter.source=true;}
        let maximum=Value::Add(Box::new(Value::EffectValue(EffectId(31))),Box::new(Value::EffectValue(EffectId(32))));let payload=if typed{serde_json::json!({"counter_type":CounterType::Charge,"max_count":maximum,"target":ChooseSpec::All(filter)})}else{serde_json::json!({"max_count":maximum,"target":ChooseSpec::All(filter),"up_to":false})};let nested=wire(if typed{"RemoveUpToCountersEffect"}else{"RemoveUpToAnyCountersEffect"},payload);let effect=materialized(wire("WithIdEffect",serde_json::json!({"id":57,"effect":nested})));let out=ironsmith::effects::execute_effect(&mut game,&effect,&mut ctx).unwrap();assert_eq!(out.as_count(),Some(2*i64::from(u32::MAX)));assert_eq!(ctx.get_outcome(EffectId(57)).unwrap().as_count(),out.as_count());assert_eq!(game.counter_count(source,CounterType::Charge),0);assert_eq!(game.counter_count(other,if typed{CounterType::Charge}else{CounterType::PlusOnePlusOne}),0);assert_eq!(out.events_of_type::<ironsmith::events::MarkersChangedEvent>().count(),2);ctx.store_outcome(EffectId(57),receipt(&out));
        let follow=materialized(wire("PutCountersEffect",serde_json::to_value(PutCountersEffect::new(CounterType::Charge,Value::HalfRoundedDown(Box::new(Value::EffectValue(EffectId(57)))),ChooseSpec::SpecificObject(following))).unwrap()));assert_eq!(ironsmith::effects::execute_effect(&mut game,&follow,&mut ctx).unwrap().as_count(),Some(i64::from(u32::MAX)));assert_eq!(game.counter_count(following,CounterType::Charge),u32::MAX);
    }
    #[test]fn materialized_typed_all_wide_receipt_survives_codec_and_followup(){wide(true);}
    #[test]fn materialized_any_all_wide_receipt_survives_codec_and_followup(){wide(false);}
    struct Decisions{answer:Option<usize>,pending:bool,calls:usize}
    impl ironsmith::decision::DecisionMaker for Decisions {
        fn awaiting_choice(&self)->bool{self.pending}
        fn decide_options(&mut self,_game:&GameState,ctx:&ironsmith::decisions::context::SelectOptionsContext)->Vec<usize> {
            assert!(!self.pending);assert_eq!(ctx.player,PlayerId::from_index(1));assert_eq!(ctx.options.len(),2);self.calls+=1;if let Some(answer)=self.answer.take(){vec![answer]}else{self.pending=true;vec![]}
        }
    }
    fn pending(simultaneous:bool,already_pending:bool) {
        for previous in [false,true] {
            let alice=PlayerId::from_index(0);let bob=PlayerId::from_index(1);let mut game=GameState::new(vec!["Alice".into(),"Bob".into()],20);let source=object(&mut game,alice);let mut prior=EffectContext::new_default(source,alice);put(&mut game,&mut prior,source,CounterType::Charge,42,31);if previous{let gain=materialized(wire("GainLifeEffect",serde_json::to_value(GainLifeEffect::new(7,ChooseSpec::Player(PlayerFilter::You))).unwrap()));let out=ironsmith::effects::execute_effect(&mut game,&gain,&mut prior).unwrap();assert_eq!(out.as_count(),Some(7));prior.store_outcome(EffectId(57),receipt(&out));}let records=prior.effect_outcomes.clone();drop(prior);
            let mut shields=Vec::new();for extra in [0,1]{let replacement=ironsmith::static_abilities::StaticAbility::add_player_counters_placement_replacement(PlayerFilter::Specific(bob),Some(CounterType::Energy),extra,"Materialized result choice".into()).generate_replacement_effect(source,alice).unwrap();shields.push(game.effect_store.replacement_effects.add_one_shot_effect(replacement));}
            let nested=wire("GivePlayerCountersEffect",serde_json::json!({"counter_type":CounterType::Energy,"count":Value::from(1),"player":PlayerFilter::Specific(bob)}));let effect=materialized(wire("WithIdEffect",serde_json::json!({"id":57,"effect":nested})));let mut dm=Decisions{answer:None,pending:already_pending,calls:0};let mut ctx=EffectContext::new(source,alice,&mut dm);ctx.effect_outcomes=records.clone();let out=if simultaneous{assert!(effect.0.supports_simultaneous_player_action());effect.0.prepare_simultaneous_player_action(&game,&mut ctx).unwrap().commit(&mut game,&mut ctx).unwrap()}else{ironsmith::effects::execute_effect(&mut game,&effect,&mut ctx).unwrap()};assert!(ctx.decision_maker.awaiting_choice());assert_eq!(out.as_count(),Some(0));assert!(out.events.is_empty());assert_eq!(ctx.effect_outcomes,records);assert_eq!(game.player(bob).unwrap().energy_counters,0);assert_eq!(game.counter_count(source,CounterType::Charge),42);assert!(shields.iter().all(|id|game.effect_store.replacement_effects.get_effect(*id).is_some()));assert!(game.take_pending_trigger_events().is_empty());
            // Only event-free receipts are serialized here; this is not a
            // checkpoint/event-stream codec claim.
            assert!(ctx.effect_outcomes.values().all(|out|out.events.is_empty()));let text=serde_json::to_string(&ctx.effect_outcomes).unwrap();let saved:std::collections::HashMap<EffectId,EffectOutcome>=serde_json::from_str(&text).unwrap();assert_eq!(saved,records);drop(ctx);assert_eq!(dm.calls,usize::from(!already_pending));let mut replay=Decisions{answer:Some(0),pending:false,calls:0};let mut ctx=EffectContext::new(source,alice,&mut replay);ctx.effect_outcomes=saved;let out=if simultaneous{effect.0.prepare_simultaneous_player_action(&game,&mut ctx).unwrap().commit(&mut game,&mut ctx).unwrap()}else{ironsmith::effects::execute_effect(&mut game,&effect,&mut ctx).unwrap()};assert!(!ctx.decision_maker.awaiting_choice());assert_eq!(out.as_count(),Some(2));assert_eq!(ctx.get_outcome(EffectId(57)).unwrap().as_count(),Some(2));assert_eq!(ctx.get_outcome(EffectId(31)),records.get(&EffectId(31)));assert_eq!(game.player(bob).unwrap().energy_counters,2);assert!(shields.iter().all(|id|game.effect_store.replacement_effects.get_effect(*id).is_none()));assert_eq!(out.events_of_type::<ironsmith::events::MarkersChangedEvent>().count(),1);assert!(game.take_pending_trigger_events().is_empty());
        }
    }
    #[test]fn materialized_live_with_id_pending_keeps_result_map(){pending(false,false);}
    #[test]fn materialized_prepared_with_id_pending_keeps_result_map(){pending(true,false);}
    #[test]fn materialized_live_with_id_already_pending_keeps_result_map(){pending(false,true);}
    #[test]fn materialized_prepared_with_id_already_pending_keeps_result_map(){pending(true,true);}
}

#[cfg(test)]
mod compiled_distinct_player_clause_tests {
    use super::*;
    use ironsmith::{GameState, PlayerId, Zone, Target};
    use ironsmith::game_state::{StackEntry, TargetAssignment};

    #[test]
    fn verdant_command_direct_and_catalog_keep_player_modes_independent() {
        // Oracle and printed metadata match the local cards.json snapshot.
        let text = "Mana cost: {1}{G}\nType: Instant\nChoose two —\n• Target player creates two tapped 1/1 green Squirrel creature tokens.\n• Counter target loyalty ability of a planeswalker.\n• Exile target card from a graveyard.\n• Target player gains 3 life.";
        let (artifact, direct) = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Verdant Command"),
            text, false,
        ).expect("strict real-card compilation");
        artifact.validate().unwrap();
        let mut registry = ironsmith::cards::CardRegistry::new();
        registry.register_compiled_artifact(&artifact).unwrap();
        let decoded = registry.get("Verdant Command").unwrap().clone();
        for definition in [direct, decoded] {
            let alice = PlayerId::from_index(0);
            let bob = PlayerId::from_index(1);
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
            let modes = vec![0, 3];
            let requirements = ironsmith::game_loop::extract_target_requirements_from_program_with_modes(
                &game, definition.spell_effect.as_ref().unwrap(), alice, Some(spell), Some(&modes),
            );
            assert_eq!(requirements.len(), 2, "chosen modes announce two independent targets");
            for requirement in &requirements {
                assert_eq!(requirement.min_targets, 1);
                assert_eq!(requirement.max_targets, Some(1));
                assert!(requirement.legal_targets.contains(&Target::Player(alice)));
                assert!(requirement.legal_targets.contains(&Target::Player(bob)));
            }
            let assignments = requirements.into_iter().enumerate()
                .map(|(index, requirement)| TargetAssignment { spec: requirement.spec, range: index..index+1 })
                .collect();
            game.push_to_stack(StackEntry::new(spell, alice)
                .with_chosen_modes(Some(modes))
                .with_targets(vec![Target::Player(alice), Target::Player(bob)])
                .with_target_assignments(assignments));
            ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
            assert_eq!(game.battlefield.len(), 2);
            for &id in &game.battlefield {
                assert_eq!(game.controller_of_id(id), Some(alice));
                assert!(game.is_tapped(id));
            }
            assert_eq!(game.player(alice).unwrap().life, 20);
            assert_eq!(game.player(bob).unwrap().life, 23);
        }
    }
}

#[cfg(test)]
mod keyword_grant_materialization_tests;
