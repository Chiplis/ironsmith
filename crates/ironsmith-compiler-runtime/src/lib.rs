use ironsmith_compiled_artifact::{
    ArtifactCardId, ArtifactCardIdentity, CompiledCardArtifact, CompiledCardPayload,
    wire_definition_from_serializable,
};
use ironsmith_compiler as compiler;
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
            beneficiary: spec.beneficiary,
            usage_limit: spec.usage_limit,
            max_plays: spec.max_plays,
            cast_this_way_filter: spec.cast_this_way_filter,
            source_exiled_surface: spec.source_exiled_surface,
            filtered_zone_surface: spec.filtered_zone_surface,
            top_card_only: spec.top_card_only,
            instant_timing: spec.instant_timing,
            cast_this_way_grants: spec
                .cast_this_way_grants
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
                "PoisonCountersEffect",
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
mod retained_occurrence_table_tests {
    use ironsmith::ability::{Ability, AbilityKind};
    use ironsmith::continuous::{AbilityOrigin, ContinuousAbilityOrigin};
    use ironsmith::continuous::{
        ContinuousEffect, EffectTarget, Modification, RegisteredRestriction, RestrictionKind,
        TextBoxOverlay,
    };
    use ironsmith::object::{
        AuraAttachmentFilter, AuraAttachmentMetadata, RetainedAuraAttachmentMetadata,
    };
    use ironsmith::static_abilities::{StaticAbility, StaticAbilityId, StaticAbilityKind};
    use ironsmith_runtime_catalog::artifact_materializer::{
        OccurrenceBindingError, RetainedStaticAbilityTable, StaticAbilityOccurrenceDecoder,
        StaticAbilityOccurrenceEncoder, StaticAbilityOccurrenceRef,
    };

    fn static_payload(ability: Ability) -> StaticAbility {
        let AbilityKind::Static(ability) = ability.kind else {
            panic!("static ability lost")
        };
        ability
    }

    #[test]
    fn retained_occurrence_table_shared_payloads_preserve_aliases_and_independent_copies() {
        let original = RegisteredRestriction::new(RestrictionKind::CantAttack);
        let independent = RegisteredRestriction::new(RestrictionKind::CantAttack);
        assert_ne!(
            original.ability().instance_id(),
            independent.ability().instance_id()
        );
        let metadata = AuraAttachmentMetadata::from(AuraAttachmentFilter::from(
            ironsmith::target::ObjectFilter::creature(),
        ));
        let enchant = RetainedAuraAttachmentMetadata::from(metadata.clone()).enchant_ability;
        let original_ability = Ability::static_ability(original.ability().clone());
        let enchant_ability = Ability::static_ability(enchant.clone());
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        let ability = encoder.encode_ability(original_ability.clone()).unwrap();
        let restriction = encoder.encode_restriction(original.clone()).unwrap();
        let independent = encoder.encode_restriction(independent).unwrap();
        let enchant_ability_wire = encoder.encode_ability(enchant_ability.clone()).unwrap();
        let metadata = encoder.encode_aura_metadata(metadata).unwrap();
        let alice = ironsmith::PlayerId::from_index(0);
        let source = ironsmith::ObjectId::from_raw(9971);
        let mut descriptor = ContinuousEffect::new(
            source,
            alice,
            EffectTarget::Specific(source),
            Modification::AddAbility(original.ability().clone()),
        );
        descriptor.originating_static_ability = Some(original.ability().clone());
        let origin = ContinuousAbilityOrigin {
            host: source,
            printed_face: Some(ironsmith::CardId::from_raw(9972)),
            branch: 3,
            ability: AbilityOrigin::Borrowed {
                effect: (&descriptor).into(),
                source: ironsmith::ObjectId::from_raw(9973),
                origin: Box::new(AbilityOrigin::Effect {
                    effect: (&descriptor).into(),
                    slot: 2,
                }),
            },
        };
        let wire_origin = origin
            .clone()
            .try_map_static_instances(&mut |id| encoder.reference(id))
            .unwrap();
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let card =
            ironsmith::card::CardBuilder::new(ironsmith::CardId::new(), "Copied occurrence host")
                .card_types(vec![ironsmith::CardType::Creature])
                .build();
        let object = game.create_object_from_card(&card, alice, ironsmith::Zone::Battlefield);
        game.object_mut(object).unwrap().abilities =
            std::sync::Arc::new(vec![original_ability.clone(), enchant_ability.clone()]);
        let copy = encoder
            .encode_copy_values(ironsmith::snapshot::CopiableValues::from_object(
                game.object(object).unwrap(),
            ))
            .unwrap();
        let overlay = encoder
            .encode_text_overlay(TextBoxOverlay {
                compiled_card_text: "retained text".into(),
                abilities: vec![original_ability, enchant_ability],
                ability_labels: vec!["prohibition".into(), "enchant".into()].into(),
            })
            .unwrap();
        let table = encoder.into_table();
        assert_eq!(
            table.models.len(),
            3,
            "one prohibition, independent equal prohibition and one enchant"
        );
        let json = serde_json::json!({"table": table, "ability": ability, "restriction": restriction, "independent": independent,
            "enchant": enchant_ability_wire, "metadata": metadata, "copy": copy, "overlay": overlay, "origin": wire_origin});
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&json).unwrap()).unwrap();
        let table: RetainedStaticAbilityTable =
            serde_json::from_value(json["table"].clone()).unwrap();
        let peer = StaticAbilityOccurrenceDecoder::restore(table.clone()).unwrap();
        let decoder = StaticAbilityOccurrenceDecoder::restore(table).unwrap();
        let restored_ability = static_payload(
            decoder
                .restore_ability(serde_json::from_value(json["ability"].clone()).unwrap())
                .unwrap(),
        );
        let restored_restriction = decoder
            .restore_restriction(serde_json::from_value(json["restriction"].clone()).unwrap())
            .unwrap();
        let restored_independent = decoder
            .restore_restriction(serde_json::from_value(json["independent"].clone()).unwrap())
            .unwrap();
        assert_eq!(
            restored_ability.instance_id(),
            restored_restriction.ability().instance_id(),
            "one retained occurrence must be shared across ability and restriction carriers"
        );
        assert_ne!(
            restored_ability.instance_id(),
            restored_independent.ability().instance_id(),
            "equal independent models must stay distinct"
        );
        assert_ne!(
            restored_ability.instance_id(),
            original.ability().instance_id(),
            "fresh receiver uses its own identity"
        );
        let peer_ability = static_payload(
            peer.restore_ability(serde_json::from_value(json["ability"].clone()).unwrap())
                .unwrap(),
        );
        assert_ne!(
            peer_ability.instance_id(),
            restored_ability.instance_id(),
            "two receivers never import global IDs"
        );
        let restored_enchant = static_payload(
            decoder
                .restore_ability(serde_json::from_value(json["enchant"].clone()).unwrap())
                .unwrap(),
        );
        let restored_metadata = decoder
            .restore_aura_metadata(serde_json::from_value(json["metadata"].clone()).unwrap())
            .unwrap();
        assert_eq!(
            restored_enchant.instance_id(),
            RetainedAuraAttachmentMetadata::from(restored_metadata.clone())
                .enchant_ability
                .instance_id(),
            "one enchant occurrence must be shared across ability and attachment carriers"
        );
        let copy = decoder
            .restore_copy_values(serde_json::from_value(json["copy"].clone()).unwrap())
            .unwrap();
        let overlay = decoder
            .restore_text_overlay(serde_json::from_value(json["overlay"].clone()).unwrap())
            .unwrap();
        for abilities in [copy.abilities.as_slice(), overlay.abilities.as_slice()] {
            assert_eq!(
                static_payload(abilities[0].clone()).instance_id(),
                restored_ability.instance_id()
            );
            assert_eq!(
                static_payload(abilities[1].clone()).instance_id(),
                restored_enchant.instance_id()
            );
        }
        let wire_origin: ContinuousAbilityOrigin<StaticAbilityOccurrenceRef> =
            serde_json::from_value(json["origin"].clone()).unwrap();
        let mut visited = Vec::new();
        let restored_origin = wire_origin
            .try_map_static_instances(&mut |reference| {
                let id = decoder.instance_id(reference)?;
                visited.push(id);
                Ok::<_, OccurrenceBindingError>(id)
            })
            .unwrap();
        assert_eq!(visited, vec![restored_ability.instance_id(); 2]);
        assert_eq!(restored_origin.host, origin.host);
        assert_eq!(restored_origin.printed_face, origin.printed_face);
        assert_eq!(restored_origin.branch, origin.branch);
        game.object_mut(object).unwrap().abilities = std::sync::Arc::new(Vec::new());
        game.refresh_continuous_state()
            .expect("empty recipient refresh");
        assert!(game.effect_store.cant_effects.can_attack(object));
        let effect = game
            .effect_store
            .continuous_effects
            .add_effect(ContinuousEffect::new(
                object,
                alice,
                EffectTarget::Specific(object),
                Modification::Restriction(restored_restriction),
            ));
        game.refresh_continuous_state()
            .expect("decoded registered restriction refresh");
        assert!(!game.effect_store.cant_effects.can_attack(object));
        let effects = game
            .try_all_continuous_effects()
            .expect("finite decoded descriptor discovery");
        let chars = game
            .calculated_characteristics_with_effects(object, &effects)
            .expect("recipient exists");
        assert!(
            chars
                .static_abilities
                .iter()
                .any(|ability| ability.instance_id() == restored_ability.instance_id()),
            "registered layer carries the table-bound occurrence"
        );
        game.effect_store.continuous_effects.remove_effect(effect);
        game.refresh_continuous_state()
            .expect("decoded restriction removal refresh");
        assert!(game.effect_store.cant_effects.can_attack(object));
    }

    #[derive(Debug, Clone)]
    struct Unmodeled;
    impl StaticAbilityKind for Unmodeled {
        fn id(&self) -> StaticAbilityId {
            StaticAbilityId::CantAttack
        }
        fn display(&self) -> String {
            "Unmodeled fixture".into()
        }
    }

    #[test]
    fn retained_occurrence_table_rejects_conflicts_unknown_refs_and_rolls_back_failed_carriers() {
        let original = StaticAbility::cant_attack();
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        let reference = encoder.retain(original.clone()).unwrap();
        assert_eq!(encoder.retain(original.clone()).unwrap(), reference);
        let before = serde_json::to_value(encoder.clone().into_table()).unwrap();
        let mut conflicting = original.clone();
        conflicting.0 = StaticAbility::cant_block().0;
        assert!(matches!(
            encoder.retain(conflicting),
            Err(OccurrenceBindingError::InconsistentOccurrence { .. })
        ));
        assert_eq!(
            serde_json::to_value(encoder.clone().into_table()).unwrap(),
            before
        );
        let independent = StaticAbility::cant_block();
        let overlay = TextBoxOverlay {
            compiled_card_text: "two abilities".into(),
            abilities: vec![
                Ability::static_ability(independent.clone()),
                Ability::static_ability(StaticAbility::new(Unmodeled)),
            ],
            ability_labels: Vec::new().into(),
        };
        assert!(encoder.encode_text_overlay(overlay).is_err());
        assert_eq!(
            serde_json::to_value(encoder.clone().into_table()).unwrap(),
            before,
            "failed carrier must not retain its successful prefix"
        );
        assert!(matches!(
            encoder.reference(independent.instance_id()),
            Err(OccurrenceBindingError::UnboundNativeOccurrence)
        ));
        let valid = encoder
            .encode_restriction(RegisteredRestriction::new(RestrictionKind::CantAttack))
            .unwrap();
        let table = encoder.into_table();
        let mut missing = serde_json::to_value(table.clone()).unwrap();
        missing.as_object_mut().unwrap().remove("models");
        assert!(serde_json::from_value::<RetainedStaticAbilityTable>(missing).is_err());
        assert!(
            serde_json::from_value::<StaticAbilityOccurrenceRef>(serde_json::json!(4294967296u64))
                .is_err()
        );
        let decoder = StaticAbilityOccurrenceDecoder::restore(table).unwrap();
        let before_id = decoder.instance_id(reference).unwrap();
        assert!(matches!(
            decoder.ability(StaticAbilityOccurrenceRef(u32::MAX)),
            Err(OccurrenceBindingError::UnknownReference { .. })
        ));
        let mut invalid = valid.clone();
        invalid.ability = StaticAbilityOccurrenceRef(u32::MAX);
        assert!(decoder.restore_restriction(invalid).is_err());
        assert!(decoder.restore_restriction(valid).is_ok());
        assert_eq!(
            decoder.instance_id(reference).unwrap(),
            before_id,
            "invalid reference cannot rebind a valid occurrence"
        );
    }
}

#[cfg(test)]
mod retained_descriptor_codec_tests {
    use ironsmith::ability::Ability;
    use ironsmith::continuous::*;
    use ironsmith::object::{AuraAttachmentFilter, AuraAttachmentMetadata};
    use ironsmith::static_abilities::StaticAbility;
    use ironsmith_runtime_catalog::artifact_materializer::*;
    fn fixture() -> ironsmith::cards::CardDefinition {
        super::compile_builder_to_artifact(super::compiler::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Occurrence execution fixture"),
            "Mana cost: {1}\nType: Artifact\nWhen this artifact enters, you gain 2 life.\n{T}, Pay 1 life: You gain 2 life.", false).expect("strict fixture").1
    }
    #[test]
    fn retained_occurrence_table_complete_abilities_preserve_models_and_execute() {
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
            let mut table = StaticAbilityOccurrenceEncoder::default();
            let wire = table
                .encode_ability(original.clone())
                .expect("complete ability encodes");
            let json = serde_json::to_string(&(table.clone().into_table(), wire.clone()))
                .expect("complete table/ability serializes");
            let (models, decoded): (RetainedStaticAbilityTable, RetainedOccurrenceAbility) =
                serde_json::from_str(&json).expect("complete table/ability deserializes");
            let receiver = StaticAbilityOccurrenceDecoder::restore(models).expect("table restores");
            let restored = receiver
                .restore_ability(decoded)
                .expect("exact runtime ability restores");
            let mut reencoder = StaticAbilityOccurrenceEncoder::default();
            let reencoded = reencoder.encode_ability(restored.clone()).unwrap();
            assert_eq!(restored.functional_zones, original.functional_zones);
            assert_eq!(
                serde_json::to_value((reencoder.into_table(), reencoded)).unwrap(),
                serde_json::to_value((table.into_table(), wire)).unwrap(),
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
    fn retained_descriptor_codec_all_payload_families_state_and_forward_provenance_roundtrip() {
        let alice = ironsmith::PlayerId::from_index(0);
        let bob = ironsmith::PlayerId::from_index(1);
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let card = ironsmith::card::CardBuilder::new(ironsmith::CardId::new(), "Descriptor source")
            .card_types(vec![ironsmith::CardType::Artifact])
            .build();
        let source = game.create_object_from_card(&card, alice, ironsmith::Zone::Battlefield);
        let target = game.create_object_from_card(&card, bob, ironsmith::Zone::Battlefield);
        let shared = StaticAbility::cant_attack();
        let generic = Ability::static_ability(shared.clone());
        game.object_mut(target).unwrap().abilities = std::sync::Arc::new(vec![generic.clone()]);
        let copy = ironsmith::snapshot::CopiableValues::from_object(game.object(target).unwrap());
        let text = TextBoxOverlay {
            compiled_card_text: "captured text".into(),
            abilities: vec![generic.clone()],
            ability_labels: vec!["captured label".into()].into(),
        };
        let restriction = RegisteredRestriction::try_from(RetainedRestriction {
            kind: RestrictionKind::CantAttack,
            ability: shared.clone(),
        })
        .unwrap();
        let attachment = AuraAttachmentMetadata::from(AuraAttachmentFilter::from(
            ironsmith::target::ObjectFilter::creature(),
        ));
        let mut later = ContinuousEffect::new(
            source,
            alice,
            EffectTarget::Specific(target),
            Modification::AddAbility(shared.clone()),
        );
        later.originating_static_ability = Some(shared.clone());
        let origin = ContinuousAbilityOrigin {
            host: source,
            printed_face: Some(ironsmith::CardId::from_raw(9976)),
            branch: 2,
            ability: AbilityOrigin::Effect {
                effect: (&later).into(),
                slot: 1,
            },
        };
        let red = ironsmith::ColorSet::RED;
        let mut early = ContinuousEffect::new(
            source,
            alice,
            EffectTarget::Specific(source),
            Modification::SetColors(red),
        );
        early.originating_ability = Some(Box::new(origin));
        early.expires_end_of_turn = 9;
        early.condition = Some(ironsmith::ConditionExpr::YourTurn);
        game.effect_store.continuous_effects.add_effect(early);
        let removed = game
            .effect_store
            .continuous_effects
            .add_effect(ContinuousEffect::new(
                source,
                alice,
                EffectTarget::Specific(target),
                Modification::ChangeController(bob),
            ));
        game.effect_store.continuous_effects.remove_effect(removed);
        for modification in [
            Modification::AddAbilityGeneric(generic.clone()),
            Modification::SetAbilities(vec![generic.clone()]),
            Modification::RemoveAbility(shared.clone()),
            Modification::RemoveAbilityGeneric {
                ability: generic.clone(),
                mode: serde_json::from_value(serde_json::json!("Lose"))
                    .expect("ordinary loss mode fixture"),
            },
            Modification::CopyOf {
                target_id: target,
                copiable_values: Box::new(copy),
                preserve_source_abilities: true,
                name_override: Some("retained name".into()),
                name_override_surface: None,
                add_supertypes: vec![ironsmith::Supertype::Legendary],
            },
            Modification::SetTextBox(text),
            Modification::SetAuraAttachmentFilter(attachment),
            Modification::Restriction(restriction),
            Modification::ChangeControllerToEffectController,
        ] {
            game.effect_store
                .continuous_effects
                .add_effect(ContinuousEffect::new(
                    source,
                    alice,
                    EffectTarget::Specific(target),
                    modification,
                ));
        }
        let group = game.effect_store.continuous_effects.next_effect_group_id();
        game.effect_store
            .continuous_effects
            .add_effect(later.with_group(group));
        let expired = game.effect_store.continuous_effects.add_effect(
            ContinuousEffect::new(
                source,
                alice,
                EffectTarget::Specific(source),
                Modification::SetColors(ironsmith::ColorSet::BLUE),
            )
            .until(ironsmith::Until::YouStopControllingThis),
        );
        let mut state = game.effect_store.continuous_effects.registered_state();
        // A captured expired duration must stay expired even though its predicate is true again.
        assert_eq!(state.duration_latches.len(), 1);
        assert_eq!(state.duration_latches[0].0, expired);
        state.duration_latches[0].1 = ContinuousDurationLatch::Expired;
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        let wire = encoder
            .encode_registered_state(state.clone())
            .expect("forward provenance resolved after all payloads");
        let table = encoder.into_table();
        let json = serde_json::to_value((&table, &wire)).unwrap();
        let (table, wire): (
            RetainedStaticAbilityTable,
            RetainedOccurrenceRegisteredState,
        ) = serde_json::from_value(json.clone()).unwrap();
        let decoder = StaticAbilityOccurrenceDecoder::restore(table).unwrap();
        let restored = decoder
            .restore_registered_state(wire)
            .expect("complete native descriptors");
        assert_eq!(restored.next_id, state.next_id);
        assert_eq!(restored.next_group_id, state.next_group_id);
        assert_eq!(restored.duration_latches, state.duration_latches);
        assert_eq!(restored.timestamps, state.timestamps);
        let mut receiver = game.clone();
        receiver
            .effect_store
            .continuous_effects
            .restore_registered_state(restored.clone())
            .expect("manager validates restored registrations");
        let mut reencoder = StaticAbilityOccurrenceEncoder::default();
        let reencoded = reencoder.encode_registered_state(restored).unwrap();
        assert_eq!(
            serde_json::to_value((reencoder.into_table(), reencoded)).unwrap(),
            json,
            "all payload/scalar/captured context/provenance/allocator/chronology fields survive"
        );
        let effects = receiver
            .try_all_continuous_effects()
            .expect("finite restored discovery");
        let chars = receiver
            .calculated_characteristics_with_effects(source, &effects)
            .expect("source exists");
        assert_eq!(
            chars.colors, red,
            "real restored registered color effect applies"
        );
    }

    #[test]
    fn retained_descriptor_codec_failed_state_rolls_back_bindings_and_unknown_origin_rejects() {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = ironsmith::PlayerId::from_index(0);
        let source = ironsmith::ObjectId::from_raw(9978);
        let shared = StaticAbility::cant_attack();
        let missing = StaticAbility::cant_block();
        let mut generator = ContinuousEffect::new(
            source,
            alice,
            EffectTarget::Specific(source),
            Modification::AddAbility(missing.clone()),
        );
        generator.originating_static_ability = Some(missing);
        let mut effect = ContinuousEffect::new(
            source,
            alice,
            EffectTarget::Specific(source),
            Modification::AddAbility(shared.clone()),
        );
        effect.originating_ability = Some(Box::new(ContinuousAbilityOrigin {
            host: source,
            printed_face: None,
            branch: 1,
            ability: AbilityOrigin::Effect {
                effect: (&generator).into(),
                slot: 0,
            },
        }));
        game.effect_store.continuous_effects.add_effect(effect);
        let state = game.effect_store.continuous_effects.registered_state();
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        assert!(matches!(
            encoder.encode_registered_state(state),
            Err(OccurrenceBindingError::UnboundNativeOccurrence)
        ));
        assert!(
            encoder.clone().into_table().models.is_empty(),
            "failed second pass rolls back every retained descriptor payload"
        );
        assert!(encoder.reference(shared.instance_id()).is_err());
        let valid = game
            .effect_store
            .continuous_effects
            .registered_state()
            .try_map_effects(|mut effect| {
                effect.originating_ability = None;
                Ok::<_, OccurrenceBindingError>(effect)
            })
            .unwrap();
        let mut wire = encoder.encode_registered_state(valid).unwrap();
        let reference = wire.effects[0].modification.clone();
        let decoder = StaticAbilityOccurrenceDecoder::restore(encoder.into_table()).unwrap();
        wire.effects[0].originating_static_ability = Some(StaticAbilityOccurrenceRef(u32::MAX));
        assert!(
            decoder.restore_registered_state(wire).is_err(),
            "unknown generating occurrence rejects full state"
        );
        assert!(
            decoder.restore_modification(reference).is_ok(),
            "failed state restore cannot rebind valid payload references"
        );
    }
}

#[cfg(test)]
mod retained_temporary_grant_codec_tests {
    use ironsmith::object::{TemporaryStaticAbilityGrant, TemporaryStaticAbilityGrants};
    use ironsmith::static_abilities::StaticAbilityId;
    use ironsmith_runtime_catalog::artifact_materializer::StaticAbilityOccurrenceEncoder;
    #[test]
    fn retained_temporary_grant_codec_every_scalar_registration_has_canonical_model() {
        let mut missing = Vec::new();
        for ability in [
            StaticAbilityId::Deathtouch,
            StaticAbilityId::DoubleStrike,
            StaticAbilityId::FirstStrike,
            StaticAbilityId::Flying,
            StaticAbilityId::Haste,
            StaticAbilityId::Hexproof,
            StaticAbilityId::Indestructible,
            StaticAbilityId::Lifelink,
            StaticAbilityId::Menace,
            StaticAbilityId::Reach,
            StaticAbilityId::Trample,
            StaticAbilityId::Vigilance,
            StaticAbilityId::ReadAhead,
        ] {
            let mut grants = TemporaryStaticAbilityGrants::new(ironsmith::ObjectId::from_raw(9985));
            grants.push(TemporaryStaticAbilityGrant {
                ability,
                ability_payload: None,
                expires_end_of_turn: 3,
            });
            let mut encoder = StaticAbilityOccurrenceEncoder::default();
            if let Err(error) = encoder.encode_temporary_grants(grants) {
                missing.push((ability, error));
            }
        }
        assert!(
            missing.is_empty(),
            "registered scalar grants lack complete models: {missing:?}"
        );
    }

    #[test]
    fn retained_temporary_grant_codec_shared_table_restores_live_grants_expiry_and_next_registration()
     {
        use ironsmith_runtime_catalog::artifact_materializer::{
            RetainedStaticAbilityTable, StaticAbilityOccurrenceDecoder,
        };
        for ability in [
            StaticAbilityId::Deathtouch,
            StaticAbilityId::DoubleStrike,
            StaticAbilityId::FirstStrike,
            StaticAbilityId::Flying,
            StaticAbilityId::Haste,
            StaticAbilityId::Hexproof,
            StaticAbilityId::Indestructible,
            StaticAbilityId::Lifelink,
            StaticAbilityId::Menace,
            StaticAbilityId::Reach,
            StaticAbilityId::Trample,
            StaticAbilityId::Vigilance,
            StaticAbilityId::ReadAhead,
        ] {
            let alice = ironsmith::PlayerId::from_index(0);
            let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let card = ironsmith::card::CardBuilder::new(
                ironsmith::CardId::new(),
                "Temporary grant recipient",
            )
            .card_types(vec![ironsmith::CardType::Creature])
            .build();
            let object = game.create_object_from_card(&card, alice, ironsmith::Zone::Battlefield);
            let mut grants = TemporaryStaticAbilityGrants::new(object);
            for _ in 0..2 {
                grants.push(TemporaryStaticAbilityGrant {
                    ability,
                    ability_payload: None,
                    expires_end_of_turn: 2,
                });
            }
            let original_origins = (0..2)
                .map(|slot| grants.origin(slot).unwrap().clone())
                .collect::<Vec<_>>();
            let original_ids = grants
                .iter()
                .map(|grant| grant.materialize().unwrap().instance_id())
                .collect::<Vec<_>>();
            assert_ne!(original_ids[0], original_ids[1]);
            let mut encoder = StaticAbilityOccurrenceEncoder::default();
            let wire = encoder.encode_temporary_grants(grants.clone()).unwrap();
            let alias = encoder
                .encode_ability(ironsmith::ability::Ability::static_ability(
                    grants[0].materialize().unwrap(),
                ))
                .unwrap();
            let table = encoder.into_table();
            assert_eq!(table.models.len(), 2);
            let json = serde_json::to_value((&table, &wire)).unwrap();
            let (table, wire): (RetainedStaticAbilityTable, _) =
                serde_json::from_value(json.clone()).unwrap();
            let decoder = StaticAbilityOccurrenceDecoder::restore(table).unwrap();
            let restored = decoder.restore_temporary_grants(wire).unwrap();
            let alias = decoder.restore_ability(alias).unwrap();
            let ironsmith::ability::AbilityKind::Static(alias) = alias.kind else {
                panic!("static alias lost")
            };
            assert_eq!(
                restored[0].materialize().unwrap().instance_id(),
                alias.instance_id()
            );
            assert_ne!(
                restored[1].materialize().unwrap().instance_id(),
                alias.instance_id()
            );
            for slot in 0..2 {
                assert_eq!(restored.origin(slot), grants.origin(slot));
            }
            let mut reencoder = StaticAbilityOccurrenceEncoder::default();
            let reencoded = reencoder.encode_temporary_grants(restored.clone()).unwrap();
            assert_eq!(
                serde_json::to_value((reencoder.into_table(), reencoded)).unwrap(),
                json
            );
            game.object_mut(object)
                .unwrap()
                .temporary_static_ability_grants = restored;
            game.refresh_continuous_state()
                .expect("decoded temporary ability refresh");
            assert!(
                game.current_has_static_ability_id(object, ability),
                "decoded live {ability:?}"
            );
            game.next_turn();
            game.next_turn();
            assert_eq!(game.turn.turn_number, 3, "public turn transitions reach expiry");
            game.refresh_continuous_state()
                .expect("expired temporary ability refresh");
            assert!(
                !game.current_has_static_ability_id(object, ability),
                "expired {ability:?} cannot persist"
            );
            let grants = &mut game
                .object_mut(object)
                .unwrap()
                .temporary_static_ability_grants;
            grants.retain(|grant| !grant.is_expired(3));
            assert!(grants.is_empty());
            grants.push(TemporaryStaticAbilityGrant {
                ability,
                ability_payload: None,
                expires_end_of_turn: 4,
            });
            assert!(
                !original_origins.contains(grants.origin(0).unwrap()),
                "new registration cannot reuse an expired origin"
            );
        }
    }
}


#[cfg(test)]
mod retained_restore_state_codec_tests {
    use super::*;
    use ironsmith::object::{EntersAsCopyRestoreState, FaceDownCastState};
    use ironsmith_runtime_catalog::artifact_materializer::*;
    fn fixture() -> (ironsmith::cards::CardDefinition, FaceDownCastState) {
        let definition = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Saved executable form"),
            "Mana cost: {1}\nType: Artifact\nWhen this artifact enters, you gain 2 life.\n{T}, Pay 1 life: You gain 2 life.", false,
        ).expect("strict saved abilities compile").1;
        let spell = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Saved program"),
            "Type: Sorcery\nYou gain 2 life.",
            false,
        )
        .expect("strict saved program compiles")
        .1;
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let source =
            game.create_object_from_definition(&definition, alice, ironsmith::Zone::Battlefield);
        let object = game.object_mut(source).unwrap();
        let flying = ironsmith::ability::Ability::static_ability(
            ironsmith::static_abilities::StaticAbility::flying(),
        );
        object.abilities_mut().extend([flying.clone(), flying]);
        object.spell_effect = Some(
            spell
                .spell_effect
                .expect("actual compiled spell program")
                .into(),
        );
        object.aura_attach_filter = Some(
            ironsmith::object::AuraAttachmentFilter::from(
                ironsmith::target::ObjectFilter::creature(),
            )
            .into(),
        );
        assert!(object.apply_face_down_cast_overlay_with_disguise_ward(true));
        (
            definition,
            object
                .face_down_cast_state
                .as_ref()
                .unwrap()
                .as_ref()
                .clone(),
        )
    }

    #[test]
    fn retained_restore_state_codec_preserves_shared_payloads_and_executes_after_both_overlay_ends()
    {
        let (definition, saved) = fixture();
        assert!(saved.disguise_ward);
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        let alias = encoder
            .encode_ability(saved.abilities.last().unwrap().clone())
            .unwrap();
        let enclosing = EntersAsCopyRestoreState {
            printed: saved.clone(),
            other_face: Some(ironsmith::CardId::new()),
            other_face_name: Some("Linked saved face".into()),
            linked_face_layout: ironsmith::card::LinkedFaceLayout::Split,
            has_fuse: true,
        };
        let original_face = enclosing.other_face.unwrap();
        let wire = encoder
            .encode_enters_as_copy_restore_state(enclosing, |id| {
                assert_eq!(id, original_face);
                Ok("linked-face".to_string())
            })
            .unwrap();
        let direct = encoder.encode_face_down_state(saved).unwrap();
        let table = encoder.into_table();
        assert_eq!(
            table.models.len(),
            2,
            "one shared Flying occurrence and one Enchant occurrence"
        );
        let json = serde_json::to_value((&table, &wire, &direct)).unwrap();
        let (table, wire, direct): (
            RetainedStaticAbilityTable,
            RetainedOccurrenceEntersAsCopyRestoreState<String>,
            RetainedOccurrenceFaceDownCastState,
        ) = serde_json::from_value(json).unwrap();
        let decoder = StaticAbilityOccurrenceDecoder::restore(table).unwrap();
        let rebound_face = ironsmith::CardId::new();
        assert_ne!(rebound_face, original_face);
        let restored = decoder
            .restore_enters_as_copy_restore_state(wire, |reference| {
                assert_eq!(reference, "linked-face");
                Ok(rebound_face)
            })
            .unwrap();
        let direct = decoder.restore_face_down_state(direct).unwrap();
        let ironsmith::ability::AbilityKind::Static(alias) =
            decoder.restore_ability(alias).unwrap().kind
        else {
            panic!("static alias")
        };
        for state in [&restored.printed, &direct] {
            for ability in state.abilities.iter().rev().take(2) {
                let ironsmith::ability::AbilityKind::Static(ability) = &ability.kind else {
                    panic!("flying occurrence")
                };
                assert_eq!(ability.instance_id(), alias.instance_id());
            }
        }
        assert_eq!(restored.other_face, Some(rebound_face));
        assert!(restored.has_fuse);
        assert!(restored.printed.disguise_ward);
        let mut reencoder = StaticAbilityOccurrenceEncoder::default();
        let reencoded = reencoder
            .encode_enters_as_copy_restore_state(restored.clone(), |id| {
                assert_eq!(id, rebound_face);
                Ok("linked-face".to_string())
            })
            .unwrap();
        assert_eq!(
            reencoded.printed.ability_labels,
            restored.printed.ability_labels.to_vec()
        );
        for (copy, state) in [(true, restored.printed.clone()), (false, direct)] {
            let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = game.players[0].id;
            let source = game.create_object_from_definition(
                &definition,
                alice,
                ironsmith::Zone::Battlefield,
            );
            let object = game.object_mut(source).unwrap();
            object.name = "Current overlay".into();
            object.abilities = Vec::new().into();
            if copy {
                object.enters_as_copy_restore_state = Some(Box::new(restored.clone()));
                assert!(object.end_enters_as_copy_overlay());
                assert_eq!(object.other_face, Some(rebound_face));
            } else {
                object.face_down_cast_state = Some(Box::new(state));
                assert!(object.end_face_down_cast_overlay());
            }
            assert_eq!(object.name.as_ref(), "Saved executable form");
            let program = object.spell_effect_owned().unwrap();
            let activated = object
                .abilities
                .iter()
                .find_map(|ability| {
                    if let ironsmith::ability::AbilityKind::Activated(activated) = &ability.kind {
                        Some(activated.clone())
                    } else {
                        None
                    }
                })
                .unwrap();
            let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
            let mut ctx = ironsmith::effects::EffectContext::new(source, alice, &mut dm);
            for effect in program.all_effects() {
                ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
            }
            assert_eq!(game.player(alice).unwrap().life, 22);
            let mut cost = ironsmith::costs::CostContext::new(source, alice, &mut dm);
            cost.reason = ironsmith::costs::PaymentReason::ActivateAbility;
            for part in activated.mana_cost.costs() {
                part.pay(&mut game, &mut cost).unwrap();
            }
            assert!(game.is_tapped(source));
            assert_eq!(game.player(alice).unwrap().life, 21);
            let mut ctx = ironsmith::effects::EffectContext::new(source, alice, &mut dm);
            for effect in activated.effects.all_effects() {
                ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
            }
            assert_eq!(game.player(alice).unwrap().life, 23);
        }
    }

    #[derive(Debug, Clone)]
    struct Unmodeled;
    impl ironsmith::static_abilities::StaticAbilityKind for Unmodeled {
        fn id(&self) -> ironsmith::static_abilities::StaticAbilityId {
            ironsmith::static_abilities::StaticAbilityId::CantAttack
        }
        fn display(&self) -> String {
            "Unmodeled saved payload".into()
        }
    }
    #[test]
    fn retained_restore_state_codec_rejects_nested_payloads_links_and_refs_atomically() {
        let (_, saved) = fixture();
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        encoder
            .encode_ability(saved.abilities.last().unwrap().clone())
            .unwrap();
        let before = serde_json::to_value(encoder.clone().into_table()).unwrap();
        let enclosing = EntersAsCopyRestoreState {
            printed: saved.clone(),
            other_face: Some(ironsmith::CardId::new()),
            other_face_name: None,
            linked_face_layout: ironsmith::card::LinkedFaceLayout::TransformLike,
            has_fuse: false,
        };
        assert!(
            encoder
                .encode_enters_as_copy_restore_state(enclosing, |_| Err::<String, _>(
                    OccurrenceBindingError::InvalidModel {
                        detail: "unbound linked face".into()
                    }
                ))
                .is_err()
        );
        assert_eq!(
            serde_json::to_value(encoder.clone().into_table()).unwrap(),
            before
        );
        let mut bad = saved.clone();
        std::sync::Arc::make_mut(&mut bad.abilities).push(
            ironsmith::ability::Ability::static_ability(
                ironsmith::static_abilities::StaticAbility::new(Unmodeled),
            ),
        );
        assert!(matches!(
            encoder.encode_face_down_state(bad),
            Err(OccurrenceBindingError::Encoding(
                RuntimePayloadEncodingError::MissingModel {
                    component: "static ability"
                }
            ))
        ));
        assert_eq!(
            serde_json::to_value(encoder.clone().into_table()).unwrap(),
            before
        );
        let wire = encoder.encode_face_down_state(saved).unwrap();
        let decoder = StaticAbilityOccurrenceDecoder::restore(encoder.into_table()).unwrap();
        let mut bad = wire.clone();
        let ability = bad.abilities.last_mut().unwrap();
        ability.model = ability.model
            .clone()
            .try_map(
                |_| Ok::<_, ()>(StaticAbilityOccurrenceRef(u32::MAX)),
                Ok,
                Ok,
                Ok,
                Ok,
            )
            .unwrap();
        assert!(matches!(
            decoder.restore_face_down_state(bad),
            Err(OccurrenceBindingError::UnknownReference { .. })
        ));
        let mut bad = wire;
        bad.aura_attach_filter.as_mut().unwrap().enchant_ability =
            StaticAbilityOccurrenceRef(u32::MAX);
        assert!(matches!(
            decoder.restore_face_down_state(bad),
            Err(OccurrenceBindingError::UnknownReference { .. })
        ));
    }
}

#[cfg(test)]
mod retained_cast_overlay_codec_tests {
    use super::*;
    use ironsmith::object::{BestowCastState, SpliceCastState};
    use ironsmith_runtime_catalog::artifact_materializer::*;

    fn occurrence(
        value: &ironsmith::object::AuraAttachmentMetadata,
    ) -> ironsmith::static_abilities::StaticAbilityInstanceId {
        ironsmith::object::RetainedAuraAttachmentMetadata::from(value.clone())
            .enchant_ability
            .instance_id()
    }
    fn saved() -> BestowCastState {
        let spell = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Saved cast program"),
            "Type: Sorcery\nYou gain 2 life.",
            false,
        )
        .expect("saved program compiles")
        .1;
        BestowCastState {
            card_types: vec![ironsmith::types::CardType::Creature].into(),
            subtypes: vec![ironsmith::types::Subtype::Elf].into(),
            aura_attach_filter: Some(
                ironsmith::object::AuraAttachmentFilter::from(
                    ironsmith::target::ObjectFilter::creature(),
                )
                .into(),
            ),
            spell_effect: Some(spell.spell_effect.unwrap().into()),
        }
    }
    #[test]
    fn retained_cast_overlay_codec_preserves_alias_and_executes_restored_programs() {
        let bestow = saved();
        let splice = SpliceCastState {
            spell_effect: bestow.spell_effect.clone(),
        };
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        let alias = encoder
            .encode_aura_metadata(bestow.aura_attach_filter.clone().unwrap())
            .unwrap();
        let first = encoder.encode_bestow_state(bestow.clone()).unwrap();
        let second = encoder.encode_bestow_state(bestow).unwrap();
        let splice = encoder.encode_splice_state(splice).unwrap();
        let table = encoder.into_table();
        assert_eq!(table.models.len(), 1);
        let json = serde_json::to_value((&table, &alias, &first, &second, &splice)).unwrap();
        let (table, alias, first, second, splice): (
            RetainedStaticAbilityTable,
            ironsmith::object::RetainedAuraAttachmentMetadata<StaticAbilityOccurrenceRef>,
            RetainedOccurrenceBestowCastState,
            RetainedOccurrenceBestowCastState,
            RetainedOccurrenceSpliceCastState,
        ) = serde_json::from_value(json).unwrap();
        let decoder = StaticAbilityOccurrenceDecoder::restore(table).unwrap();
        let alias = decoder.restore_aura_metadata(alias).unwrap();
        let first = decoder.restore_bestow_state(first).unwrap();
        let second = decoder.restore_bestow_state(second).unwrap();
        let splice = decoder.restore_splice_state(splice).unwrap();
        for bestow in [&first, &second] {
            assert_eq!(
                occurrence(bestow.aura_attach_filter.as_ref().unwrap()),
                occurrence(&alias)
            );
        }
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let card =
            ironsmith::card::CardBuilder::new(ironsmith::CardId::new(), "Current form").build();
        for (bestow, state) in [(true, Some(first)), (false, None)] {
            let id = game.create_object_from_card(&card, alice, ironsmith::Zone::Battlefield);
            let object = game.object_mut(id).unwrap();
            object.spell_effect = None;
            if bestow {
                object.bestow_cast_state = state.map(Box::new);
                assert!(object.end_bestow_cast_overlay());
                assert_eq!(
                    object.card_types.as_slice(),
                    &[ironsmith::types::CardType::Creature]
                );
                assert_eq!(
                    object.subtypes.as_slice(),
                    &[ironsmith::types::Subtype::Elf]
                );
                assert_eq!(
                    occurrence(object.aura_attach_filter.as_ref().unwrap()),
                    occurrence(&alias)
                );
            } else {
                object.splice_cast_state = Some(Box::new(splice.clone()));
                assert!(object.end_splice_cast_overlay());
            }
            let program = object.spell_effect_owned().unwrap();
            let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
            let mut ctx = ironsmith::effects::EffectContext::new(id, alice, &mut dm);
            for effect in program.all_effects() {
                ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
            }
        }
        assert_eq!(game.player(alice).unwrap().life, 24);
    }
    #[derive(Debug, Clone)]
    struct Unmodeled;
    impl ironsmith::effects::EffectExecutor for Unmodeled {
        fn execute(
            &self,
            _game: &mut ironsmith::GameState,
            _ctx: &mut ironsmith::effects::EffectContext,
        ) -> Result<ironsmith::effect::EffectOutcome, ironsmith::effects::ExecutionError> {
            Err(ironsmith::effects::ExecutionError::InternalError(
                "must not execute unmodeled payload".into(),
            ))
        }
    }
    #[test]
    fn retained_cast_overlay_codec_rejects_unmodeled_programs_and_unknown_refs_atomically() {
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        encoder
            .retain(ironsmith::static_abilities::StaticAbility::flying())
            .unwrap();
        let before = serde_json::to_value(encoder.clone().into_table()).unwrap();
        let mut bad = saved();
        bad.spell_effect = Some(
            ironsmith::resolution::ResolutionProgram::from_effects(vec![
                ironsmith::effect::Effect::new(Unmodeled),
            ])
            .into(),
        );
        // The new Aura occurrence is retained before program encoding fails.
        assert!(encoder.encode_bestow_state(bad.clone()).is_err());
        assert_eq!(
            serde_json::to_value(encoder.clone().into_table()).unwrap(),
            before
        );
        assert!(
            encoder
                .encode_splice_state(SpliceCastState {
                    spell_effect: bad.spell_effect
                })
                .is_err()
        );
        assert_eq!(
            serde_json::to_value(encoder.clone().into_table()).unwrap(),
            before
        );
        let mut wire = encoder.encode_bestow_state(saved()).unwrap();
        wire.aura_attach_filter.as_mut().unwrap().enchant_ability =
            StaticAbilityOccurrenceRef(u32::MAX);
        let decoder = StaticAbilityOccurrenceDecoder::restore(encoder.into_table()).unwrap();
        assert!(matches!(
            decoder.restore_bestow_state(wire),
            Err(OccurrenceBindingError::UnknownReference { .. })
        ));
    }
    #[test]
    fn retained_cast_overlay_codec_keeps_explicit_absent_payloads() {
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        let bestow = encoder
            .encode_bestow_state(BestowCastState {
                card_types: vec![].into(),
                subtypes: vec![].into(),
                aura_attach_filter: None,
                spell_effect: None,
            })
            .unwrap();
        let splice = encoder
            .encode_splice_state(SpliceCastState { spell_effect: None })
            .unwrap();
        let table = encoder.into_table();
        assert!(table.models.is_empty());
        let decoder = StaticAbilityOccurrenceDecoder::restore(table).unwrap();
        let bestow = decoder.restore_bestow_state(bestow).unwrap();
        let splice = decoder.restore_splice_state(splice).unwrap();
        assert!(bestow.aura_attach_filter.is_none());
        assert!(bestow.spell_effect.is_none());
        assert!(splice.spell_effect.is_none());
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
mod retained_historical_snapshot_codec_tests {
    use super::*;
    use ironsmith_runtime_catalog::artifact_materializer::*;
    fn fixture() -> (ironsmith::GameState, ironsmith::snapshot::ObjectSnapshot) {
        let definition = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(
                ironsmith::CardId::new(),
                "Historical executable form",
            ),
            "Type: Artifact\nPay 1 life: You gain 2 life.",
            false,
        )
        .expect("historical executable compiles")
        .1;
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let id =
            game.create_object_from_definition(&definition, alice, ironsmith::Zone::Battlefield);
        let flying = ironsmith::ability::Ability::static_ability(
            ironsmith::static_abilities::StaticAbility::flying(),
        );
        game.object_mut(id).unwrap().abilities_mut().push(flying);
        let mut snapshot =
            ironsmith::snapshot::ObjectSnapshot::from_object(game.object(id).unwrap(), &game);
        snapshot.chosen_object = Some(Box::new(snapshot.clone()));
        snapshot
            .mana_sources_spent_to_cast
            .push(snapshot.chosen_object.as_ref().unwrap().as_ref().clone());
        snapshot
            .attachment_snapshots
            .push(snapshot.chosen_object.as_ref().unwrap().as_ref().clone());
        (game, snapshot)
    }
    #[test]
    fn retained_historical_snapshot_codec_rebinds_cards_shares_occurrences_and_executes_after_departure()
     {
        let (mut game, saved) = fixture();
        let original_card = saved.card.unwrap();
        let rebound = ironsmith::CardId::new();
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        let alias = encoder
            .encode_ability(saved.abilities.last().unwrap().clone())
            .unwrap();
        let wire = encoder
            .encode_snapshot(saved.clone(), |id| {
                assert_eq!(id, original_card);
                Ok("saved-definition".to_string())
            })
            .unwrap();
        let table = encoder.into_table();
        assert_eq!(table.models.len(), 1);
        let json = serde_json::to_value((&table, &wire)).unwrap();
        let (table, wire): (
            RetainedStaticAbilityTable,
            RetainedOccurrenceObjectSnapshot<String>,
        ) = serde_json::from_value(json).unwrap();
        let decoder = StaticAbilityOccurrenceDecoder::restore(table).unwrap();
        let restored = decoder
            .restore_snapshot(wire, |reference| {
                assert_eq!(reference, "saved-definition");
                Ok(rebound)
            })
            .unwrap();
        let ironsmith::ability::AbilityKind::Static(alias) =
            decoder.restore_ability(alias).unwrap().kind
        else {
            panic!("static alias")
        };
        for node in [
            &restored,
            restored.chosen_object.as_ref().unwrap().as_ref(),
            &restored.mana_sources_spent_to_cast[0],
            &restored.attachment_snapshots[0],
        ] {
            assert_eq!(node.card, Some(rebound));
            for ability in [
                node.abilities.last().unwrap(),
                node.copiable_values.abilities.last().unwrap(),
            ] {
                let ironsmith::ability::AbilityKind::Static(ability) = &ability.kind else {
                    panic!("static occurrence")
                };
                assert_eq!(ability.instance_id(), alias.instance_id());
            }
        }
        game.move_object(
            saved.object_id,
            ironsmith::Zone::Graveyard,
            ironsmith::events::cause::EventCause::effect(),
        )
        .unwrap();
        assert!(game.object(saved.object_id).is_none());
        let activated = restored
            .chosen_object
            .as_ref()
            .unwrap()
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
        let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
        let mut ctx = ironsmith::effects::EffectContext::new(
            restored.object_id,
            restored.controller,
            &mut dm,
        );
        for effect in activated.effects.all_effects() {
            ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        assert_eq!(game.player(restored.controller).unwrap().life, 22);
    }
    #[test]
    fn retained_historical_snapshot_codec_rolls_back_failed_graph_binding_and_rejects_nested_unknown_refs()
     {
        let (_, saved) = fixture();
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        encoder
            .retain(ironsmith::static_abilities::StaticAbility::cant_attack())
            .unwrap();
        let before = serde_json::to_value(encoder.clone().into_table()).unwrap();
        let mut calls = 0;
        assert!(
            encoder
                .encode_snapshot(saved.clone(), |_| {
                    calls += 1;
                    if calls == 2 {
                        Err(OccurrenceBindingError::InvalidModel {
                            detail: "missing historical definition".into(),
                        })
                    } else {
                        Ok(0u8)
                    }
                })
                .is_err()
        );
        assert_eq!(calls, 2);
        assert_eq!(
            serde_json::to_value(encoder.clone().into_table()).unwrap(),
            before
        );
        let mut wire = encoder.encode_snapshot(saved, |_| Ok(0u8)).unwrap();
        let nested = wire.attachment_snapshots[0].abilities.last_mut().unwrap();
        nested.model = nested.model
            .clone()
            .try_map(
                |_| Ok::<_, ()>(StaticAbilityOccurrenceRef(u32::MAX)),
                Ok,
                Ok,
                Ok,
                Ok,
            )
            .unwrap();
        let decoder = StaticAbilityOccurrenceDecoder::restore(encoder.into_table()).unwrap();
        assert!(matches!(
            decoder.restore_snapshot(wire, |_| Ok(ironsmith::CardId::new())),
            Err(OccurrenceBindingError::UnknownReference { .. })
        ));
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
    #[test]
    fn retained_registered_card_graph_codec_rebinds_faces_preserves_occurrences_and_applies_effect()
    {
        let (mut game, source, face, level_face) = fixture();
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        let wire = encoder
            .encode_registered_state_with_card_graph(
                game.effect_store.continuous_effects.registered_state(),
                |id| {
                    if id == face {
                        Ok("source-face".to_string())
                    } else {
                        assert_eq!(id, level_face);
                        Ok("level-face".to_string())
                    }
                },
            )
            .unwrap();
        let table = encoder.into_table();
        assert_eq!(table.models.len(), 1);
        let json = serde_json::to_value((&table, &wire)).unwrap();
        let (table, wire): (
            RetainedStaticAbilityTable,
            RetainedGraphRegisteredState<String>,
        ) = serde_json::from_value(json).unwrap();
        let decoder = StaticAbilityOccurrenceDecoder::restore(table).unwrap();
        let peer_face = ironsmith::CardId::new();
        let peer_level_face = ironsmith::CardId::new();
        let restored = decoder
            .restore_registered_state_with_card_graph(wire, |reference| match reference.as_str() {
                "source-face" => Ok(peer_face),
                "level-face" => Ok(peer_level_face),
                _ => panic!("unexpected graph ref"),
            })
            .unwrap();
        let origin = restored.effects[0].originating_ability.as_ref().unwrap();
        assert_eq!(origin.printed_face, Some(peer_face));
        let ironsmith::continuous::AbilityOrigin::Level {
            printed_face,
            tier,
            slot,
            ..
        } = &origin.ability
        else {
            panic!("level origin")
        };
        assert_eq!(*printed_face, Some(peer_level_face));
        assert_eq!((*tier, *slot), (5, 6));
        let mut reencoder = StaticAbilityOccurrenceEncoder::default();
        let reencoded = reencoder
            .encode_registered_state_with_card_graph(restored.clone(), |id| {
                if id == peer_face {
                    Ok("source-face".to_string())
                } else {
                    assert_eq!(id, peer_level_face);
                    Ok("level-face".to_string())
                }
            })
            .unwrap();
        assert_eq!(
            serde_json::to_value(&reencoded).unwrap(),
            serde_json::to_value(&encoder_wire_for_comparison(&game, face, level_face)).unwrap()
        );
        game.effect_store
            .continuous_effects
            .restore_registered_state(restored)
            .unwrap();
        game.refresh_continuous_state().unwrap();
        let effects = game.try_all_continuous_effects().unwrap();
        assert_eq!(
            game.calculated_characteristics_with_effects(source, &effects)
                .unwrap()
                .colors,
            ironsmith::color::ColorSet::RED
        );
    }
    fn encoder_wire_for_comparison(
        game: &ironsmith::GameState,
        face: ironsmith::CardId,
        level_face: ironsmith::CardId,
    ) -> RetainedGraphRegisteredState<String> {
        StaticAbilityOccurrenceEncoder::default()
            .encode_registered_state_with_card_graph(
                game.effect_store.continuous_effects.registered_state(),
                |id| {
                    if id == face {
                        Ok("source-face".to_string())
                    } else {
                        assert_eq!(id, level_face);
                        Ok("level-face".to_string())
                    }
                },
            )
            .unwrap()
    }
    #[test]
    fn retained_registered_card_graph_codec_rolls_back_occurrences_on_nested_face_failure() {
        let (game, _, face, level_face) = fixture();
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        encoder
            .retain(ironsmith::static_abilities::StaticAbility::cant_attack())
            .unwrap();
        let before = serde_json::to_value(encoder.clone().into_table()).unwrap();
        let mut calls = 0;
        assert!(
            encoder
                .encode_registered_state_with_card_graph(
                    game.effect_store.continuous_effects.registered_state(),
                    |_| {
                        calls += 1;
                        if calls == 2 {
                            Err(OccurrenceBindingError::InvalidModel {
                                detail: "missing printed face".into(),
                            })
                        } else {
                            Ok(0u8)
                        }
                    }
                )
                .is_err()
        );
        assert_eq!(calls, 2);
        assert_eq!(
            serde_json::to_value(encoder.clone().into_table()).unwrap(),
            before
        );
        // The old retry collapsed two distinct native faces into owner zero.
        // Keep that input as a negative, then retry with a valid graph mapping.
        assert!(
            encoder
                .encode_registered_state_with_card_graph(
                    game.effect_store.continuous_effects.registered_state(),
                    |_| Ok(0u8),
                )
                .is_err()
        );
        assert_eq!(
            serde_json::to_value(encoder.clone().into_table()).unwrap(),
            before
        );
        let wire = encoder
            .encode_registered_state_with_card_graph(
                game.effect_store.continuous_effects.registered_state(),
                |id| {
                    if id == face {
                        Ok(0u8)
                    } else {
                        assert_eq!(id, level_face);
                        Ok(1u8)
                    }
                },
            )
            .unwrap();
        let decoder = StaticAbilityOccurrenceDecoder::restore(encoder.into_table()).unwrap();
        assert!(
            decoder
                .restore_registered_state_with_card_graph(wire, |_| Err(
                    OccurrenceBindingError::InvalidModel {
                        detail: "unbound receiver face".into()
                    }
                ))
                .is_err()
        );
    }

}

#[cfg(test)]
mod retained_grant_permission_codec_tests {
    use super::*;
    use ironsmith_runtime_catalog::artifact_materializer::*;
    fn fixture() -> (
        Vec<ironsmith::grant_registry::GrantPermissionIdentity>,
        ironsmith::static_abilities::StaticAbility,
        ironsmith::CardId,
        ironsmith::CardId,
    ) {
        let source = ironsmith::ObjectId::from_raw(4444);
        let main = ironsmith::CardId::new();
        let linked = ironsmith::CardId::new();
        let flying = ironsmith::static_abilities::StaticAbility::flying();
        let mut effect = ironsmith::continuous::ContinuousEffect::new(
            source,
            ironsmith::PlayerId::from_index(0),
            ironsmith::continuous::EffectTarget::Specific(source),
            ironsmith::continuous::Modification::SetColors(ironsmith::color::ColorSet::RED),
        );
        effect.originating_static_ability = Some(flying.clone());
        let origin = ironsmith::continuous::AbilityOrigin::Borrowed {
            source,
            effect: ironsmith::continuous::AbilityEffectOrigin::from(&effect),
            origin: Box::new(ironsmith::continuous::AbilityOrigin::Level {
                printed_face: Some(linked),
                parent: Box::new(ironsmith::continuous::AbilityOrigin::Printed(3)),
                tier: 4,
                slot: 5,
            }),
        };
        (
            vec![
                ironsmith::grant_registry::GrantPermissionIdentity::Static {
                    source,
                    origin,
                    printed_face: Some(main),
                },
                ironsmith::grant_registry::GrantPermissionIdentity::LinkedFace {
                    source,
                    face: linked,
                    slot: 6,
                },
                ironsmith::grant_registry::GrantPermissionIdentity::Stored(7),
            ],
            flying,
            main,
            linked,
        )
    }
    #[test]
    fn retained_grant_permission_codec_rebinds_complete_identity_and_preserves_shared_budget_keys()
    {
        let (keys, flying, main, linked) = fixture();
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        let alias_carrier = encoder
            .encode_ability(ironsmith::ability::Ability::static_ability(flying))
            .unwrap();
        let wire = keys
            .into_iter()
            .map(|key| {
                encoder
                    .encode_permission_identity(key, |id| {
                        if id == main {
                            Ok("main".to_string())
                        } else {
                            assert_eq!(id, linked);
                            Ok("linked".to_string())
                        }
                    })
                    .unwrap()
            })
            .collect::<Vec<_>>();
        let json = serde_json::to_value((&encoder.clone().into_table(), &wire)).unwrap();
        let (table, decoded): (
            RetainedStaticAbilityTable,
            Vec<RetainedOccurrenceGrantPermission<String>>,
        ) = serde_json::from_value(json).unwrap();
        let decoder = StaticAbilityOccurrenceDecoder::restore(table).unwrap();
        let peer_main = ironsmith::CardId::new();
        let peer_linked = ironsmith::CardId::new();
        let restore = |key| {
            decoder
                .restore_permission_identity(key, |face: String| match face.as_str() {
                    "main" => Ok(peer_main),
                    "linked" => Ok(peer_linked),
                    _ => panic!("unexpected face ref"),
                })
                .unwrap()
        };
        let restored = decoded.clone().into_iter().map(restore).collect::<Vec<_>>();
        let same = decoded.into_iter().map(restore).collect::<Vec<_>>();
        let budget = restored
            .iter()
            .cloned()
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(budget.len(), 3);
        assert!(same.iter().all(|key| budget.contains(key)));
        let ironsmith::grant_registry::GrantPermissionIdentity::Static {
            printed_face,
            origin,
            ..
        } = &restored[0]
        else {
            panic!("static permission")
        };
        assert_eq!(*printed_face, Some(peer_main));
        let ironsmith::continuous::AbilityOrigin::Borrowed { effect, origin, .. } = origin else {
            panic!("borrowed identity")
        };
        let ironsmith::ability::AbilityKind::Static(alias_ability) =
            decoder.restore_ability(alias_carrier).unwrap().kind
        else {
            panic!("shared static alias")
        };
        assert_eq!(effect.static_ability(), Some(alias_ability.instance_id()));
        let ironsmith::continuous::AbilityOrigin::Level { printed_face, .. } = origin.as_ref()
        else {
            panic!("level identity")
        };
        assert_eq!(*printed_face, Some(peer_linked));
        let ironsmith::grant_registry::GrantPermissionIdentity::LinkedFace { face, slot, .. } =
            restored[1]
        else {
            panic!("linked permission")
        };
        assert_eq!((face, slot), (peer_linked, 6));
        let mut reencoder = StaticAbilityOccurrenceEncoder::default();
        reencoder.retain(alias_ability).unwrap();
        let reencoded = restored
            .into_iter()
            .map(|key| {
                reencoder
                    .encode_permission_identity(key, |id| {
                        if id == peer_main {
                            Ok("main".to_string())
                        } else {
                            assert_eq!(id, peer_linked);
                            Ok("linked".to_string())
                        }
                    })
                    .unwrap()
            })
            .collect::<Vec<_>>();
        assert_eq!(
            serde_json::to_value(reencoded).unwrap(),
            serde_json::to_value(wire).unwrap()
        );
    }
    #[test]
    fn retained_grant_permission_codec_rejects_unbound_occurrences_faces_and_unknown_refs() {
        let (keys, flying, main, linked) = fixture();
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        assert!(matches!(
            encoder.encode_permission_identity(keys[0].clone(), |_| Ok(0u8)),
            Err(OccurrenceBindingError::UnboundNativeOccurrence)
        ));
        encoder.retain(flying).unwrap();
        let before = serde_json::to_value(encoder.clone().into_table()).unwrap();
        assert!(
            encoder
                .encode_permission_identity(keys[0].clone(), |_| Err::<u8, _>(
                    OccurrenceBindingError::InvalidModel {
                        detail: "unbound face".into()
                    }
                ))
                .is_err()
        );
        assert_eq!(
            serde_json::to_value(encoder.clone().into_table()).unwrap(),
            before
        );
        // Preserve the original collapsed binder as an explicit negative case.
        assert!(encoder.encode_permission_identity(keys[0].clone(), |_| Ok(0u8)).is_err());
        assert_eq!(serde_json::to_value(encoder.clone().into_table()).unwrap(), before);
        let wire = encoder
            .encode_permission_identity(keys[0].clone(), |id| {
                if id == main { Ok(0u8) } else { assert_eq!(id, linked); Ok(1u8) }
            })
            .unwrap();
        let decoder = StaticAbilityOccurrenceDecoder::restore(encoder.into_table()).unwrap();
        assert!(
            decoder
                .restore_permission_identity(wire.clone(), |_| Err(
                    OccurrenceBindingError::InvalidModel {
                        detail: "missing receiver definition".into()
                    }
                ))
                .is_err()
        );
        let mut bad = wire;
        let ironsmith::grant_registry::RetainedGrantPermissionIdentity::Static { origin, .. } =
            &mut bad
        else {
            panic!("static key")
        };
        *origin = origin
            .clone()
            .try_map_static_instances(&mut |_| Ok::<_, ()>(StaticAbilityOccurrenceRef(u32::MAX)))
            .unwrap();
        assert!(matches!(
            decoder.restore_permission_identity(bad, |_| Ok(ironsmith::CardId::new())),
            Err(OccurrenceBindingError::UnknownReference { .. })
        ));
    }

    #[test]
    fn retained_grant_permission_codec_binding_failure_retry_and_fresh_receiver_roundtrip() {
        let (keys, flying, main, linked) = fixture();
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        let alias = encoder.encode_ability(ironsmith::ability::Ability::static_ability(flying)).unwrap();
        let before = serde_json::to_value(encoder.clone().into_table()).unwrap();
        assert!(encoder.encode_permission_identity(keys[0].clone(), |_| Ok(serde_json::Value::Null)).is_err());
        assert_eq!(serde_json::to_value(encoder.clone().into_table()).unwrap(), before);
        let wire = keys.into_iter().map(|key| encoder.encode_permission_identity(key, |id| {
            if id == main { Ok("main".to_string()) } else { assert_eq!(id, linked); Ok("linked".to_string()) }
        }).unwrap()).collect::<Vec<_>>();
        let accepted = serde_json::to_value(encoder.clone().into_table()).unwrap();
        assert!(encoder.encode_permission_identity(
            ironsmith::grant_registry::GrantPermissionIdentity::LinkedFace {
                source: ironsmith::ObjectId::from_raw(4444), face: main, slot: 9,
            }, |_| Ok("changed".to_string())).is_err());
        assert_eq!(serde_json::to_value(encoder.clone().into_table()).unwrap(), accepted);
        let json = serde_json::to_value((encoder.into_table(), wire.clone(), alias)).unwrap();
        let (table, parsed, alias): (RetainedStaticAbilityTable,
            Vec<RetainedOccurrenceGrantPermission<String>>, RetainedOccurrenceAbility) = serde_json::from_value(json.clone()).unwrap();
        let decoder = StaticAbilityOccurrenceDecoder::restore(table).unwrap();
        let peer_main = ironsmith::CardId::new(); let peer_linked = ironsmith::CardId::new();
        assert_ne!(peer_main, main); assert_ne!(peer_linked, linked); assert_ne!(peer_main, peer_linked);
        let restored = parsed.into_iter().map(|key| decoder.restore_permission_identity(key, |id| {
            match id.as_str() { "main" => Ok(peer_main), "linked" => Ok(peer_linked), _ => panic!("unknown face") }
        }).unwrap()).collect::<Vec<_>>();
        let alias = decoder.restore_ability(alias).unwrap();
        let mut receiver = StaticAbilityOccurrenceEncoder::default();
        let alias = receiver.encode_ability(alias).unwrap();
        let reencoded = restored.into_iter().map(|key| receiver.encode_permission_identity(key, |id| {
            if id == peer_main { Ok("main".to_string()) } else { assert_eq!(id, peer_linked); Ok("linked".to_string()) }
        }).unwrap()).collect::<Vec<_>>();
        assert_eq!(serde_json::to_value((receiver.into_table(), reencoded, alias)).unwrap(), json);
    }

}

#[cfg(test)]
mod retained_counter_store_codec_tests {
    use ironsmith::object::{CounterType, ObjectCounters, RetainedObjectCounters, RetainedCounterAbilitySlot};
    use ironsmith_runtime_catalog::artifact_materializer::{
        StaticAbilityOccurrenceEncoder, StaticAbilityOccurrenceDecoder,
        StaticAbilityOccurrenceRef, RetainedStaticAbilityTable,
    };

    #[test]
    fn retained_counter_store_codec_all_supported_payloads_survive_json_and_shared_aliases() {
        for kind in [CounterType::Deathtouch, CounterType::Flying, CounterType::FirstStrike,
            CounterType::DoubleStrike, CounterType::Hexproof, CounterType::Indestructible,
            CounterType::Lifelink, CounterType::Menace, CounterType::Reach, CounterType::Trample,
            CounterType::Vigilance, CounterType::Haste, CounterType::Decayed,
            CounterType::Named("shadow".into()), CounterType::Named("exalted".into())] {
            let mut counters = ObjectCounters::default(); counters.add(kind, 2);
            counters.add(CounterType::Charge, 3);
            let mut original_abilities = Vec::new();
            counters.retain_with_static_occurrences(|ability| {
                original_abilities.push(ability.clone()); Ok(())
            }).unwrap();
            if original_abilities.len() == 2 {
                assert_ne!(original_abilities[0].instance_id(), original_abilities[1].instance_id());
            }
            let mut encoder = StaticAbilityOccurrenceEncoder::default();
            let retained = encoder.encode_counter_store(&counters).unwrap();
            let aliases = original_abilities.into_iter().map(|ability| {
                encoder.encode_ability(ironsmith::ability::Ability::static_ability(ability)).unwrap()
            }).collect::<Vec<_>>();
            let table = encoder.into_table(); assert_eq!(table.models.len(), aliases.len());
            let json = serde_json::to_value((&table, &retained)).unwrap();
            let (table, retained): (RetainedStaticAbilityTable, RetainedObjectCounters<StaticAbilityOccurrenceRef>) =
                serde_json::from_value(json.clone()).unwrap();
            let decoder = StaticAbilityOccurrenceDecoder::restore(table).unwrap();
            let mut restored = decoder.restore_counter_store(retained).unwrap();
            let mut restored_ids = Vec::new();
            restored.retain_with_static_occurrences(|ability| {
                restored_ids.push(ability.instance_id()); Ok(())
            }).unwrap();
            let alias_ids = aliases.into_iter().map(|alias| {
                match decoder.restore_ability(alias).unwrap().kind {
                    ironsmith::ability::AbilityKind::Static(ability) => ability.instance_id(),
                    _ => panic!("alias must remain static"),
                }
            }).collect::<Vec<_>>();
            assert_eq!(restored_ids, alias_ids, "counter payloads must share the peer's occurrence table");
            if restored_ids.len() == 2 { assert_ne!(restored_ids[0], restored_ids[1]); }
            assert_eq!(restored.ability_state(), counters.ability_state());
            let mut reencoder = StaticAbilityOccurrenceEncoder::default();
            let reencoded = reencoder.encode_counter_store(&restored).unwrap();
            assert_eq!(serde_json::to_value((reencoder.into_table(), reencoded)).unwrap(), json);
            counters.insert(kind, 1); restored.insert(kind, 1);
            counters.add(kind, 1); restored.add(kind, 1);
            assert_eq!(restored.ability_state(), counters.ability_state());
        }
    }

    #[test]
    fn retained_counter_store_codec_rejects_shared_identity_between_independent_counters() {
        let mut counters = ObjectCounters::default(); counters.add(CounterType::Flying, 2);
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        let mut retained = encoder.encode_counter_store(&counters).unwrap();
        assert_ne!(retained.occurrences[0].slots, retained.occurrences[1].slots);
        retained.occurrences[1].slots = retained.occurrences[0].slots.clone();
        let decoder = StaticAbilityOccurrenceDecoder::restore(encoder.into_table()).unwrap();
        assert!(decoder.restore_counter_store(retained).is_err(),
            "independent counter registrations cannot share a static payload occurrence");
    }

    fn counter_attacker(kind: CounterType, count: u32, restore: bool) -> (ironsmith::GameState, ironsmith::ObjectId) {
        let alice = ironsmith::PlayerId::from_index(0);
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let card = ironsmith::card::CardBuilder::new(ironsmith::CardId::new(), "Counter attacker")
            .card_types(vec![ironsmith::CardType::Creature])
            .power_toughness(ironsmith::card::PowerToughness::fixed(1, 1)).build();
        let attacker = game.create_object_from_card(&card, alice, ironsmith::Zone::Battlefield);
        game.add_counters(attacker, kind, count).unwrap();
        if restore {
            let mut encoder = StaticAbilityOccurrenceEncoder::default();
            let wire = encoder.encode_counter_store(&game.object(attacker).unwrap().counters).unwrap();
            let json = serde_json::to_value((encoder.into_table(), wire)).unwrap();
            let (table, wire): (RetainedStaticAbilityTable, RetainedObjectCounters<StaticAbilityOccurrenceRef>) =
                serde_json::from_value(json).unwrap();
            let decoder = StaticAbilityOccurrenceDecoder::restore(table).unwrap();
            game.object_mut(attacker).unwrap().counters = decoder.restore_counter_store(wire).unwrap();
        }
        game.remove_summoning_sickness(attacker);
        game.turn.active_player = alice;
        game.turn.phase = ironsmith::game_state::Phase::Combat;
        game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
        (game, attacker)
    }

    fn attack(game: &mut ironsmith::GameState, attackers: &[ironsmith::ObjectId]) -> ironsmith::triggers::TriggerQueue {
        let mut combat = ironsmith::combat_state::CombatState::default();
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let declarations = attackers.iter().map(|attacker| ironsmith::AttackerDeclaration {
            creature: *attacker,
            target: ironsmith::combat_state::AttackTarget::Player(ironsmith::PlayerId::from_index(1)),
        }).collect::<Vec<_>>();
        ironsmith::game_loop::apply_attacker_declarations(game, &mut combat, &mut queue, &declarations).unwrap();
        queue
    }

    #[test]
    fn retained_counter_store_codec_exalted_triggers_resolve_for_each_surviving_counter() {
        for restored in [false, true] {
            for count in [1, 2, 3] {
                for remove_one in [false, true] {
                    let (mut game, attacker) = counter_attacker(CounterType::Named("exalted".into()), count, restored);
                    let remaining = count - u32::from(remove_one);
                    if remove_one { game.object_mut(attacker).unwrap().counters.insert(CounterType::Named("exalted".into()), remaining); }
                    let mut queue = attack(&mut game, &[attacker]);
                    assert_eq!(queue.entries.len(), remaining as usize);
                    ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
                    while !game.stack.is_empty() { ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap(); }
                    assert_eq!(game.calculated_power(attacker), Some(1 + remaining as i32));
                    assert_eq!(game.calculated_toughness(attacker), Some(1 + remaining as i32));
                }
            }
            let (mut game, attacker) = counter_attacker(CounterType::Named("exalted".into()), 2, restored);
            let card = ironsmith::card::CardBuilder::new(ironsmith::CardId::new(), "Second attacker")
                .card_types(vec![ironsmith::CardType::Creature])
                .power_toughness(ironsmith::card::PowerToughness::fixed(1, 1)).build();
            let other = game.create_object_from_card(&card, ironsmith::PlayerId::from_index(0), ironsmith::Zone::Battlefield);
            game.remove_summoning_sickness(other);
            assert!(attack(&mut game, &[attacker, other]).entries.is_empty(), "exalted requires attacking alone");
        }
    }

    #[test]
    fn retained_counter_store_codec_decayed_schedules_and_resolves_one_shot_sacrifice() {
        for restored in [false, true] {
            for count in [1, 2] {
                let (mut game, attacker) = counter_attacker(CounterType::Decayed, count, restored);
                assert!(game.object_has_static_ability_id(attacker, ironsmith::static_abilities::StaticAbilityId::CantBlock));
                let mut queue = attack(&mut game, &[attacker]);
                assert_eq!(queue.entries.len(), count as usize);
                ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
                while !game.stack.is_empty() { ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap(); }
                assert_eq!(game.object(attacker).unwrap().zone, ironsmith::Zone::Battlefield);
                assert_eq!(game.effect_store.delayed_triggers.len(), count as usize);
                let event = ironsmith::triggers::TriggerEvent::new_with_provenance(
                    ironsmith::events::EndOfCombatEvent::new(), ironsmith::provenance::ProvNodeId::default());
                for entry in ironsmith::triggers::check_delayed_triggers(&mut game, &event) { queue.add(entry); }
                assert_eq!(queue.entries.len(), count as usize);
                assert!(game.effect_store.delayed_triggers.is_empty());
                assert!(ironsmith::triggers::check_delayed_triggers(&mut game, &event).is_empty());
                ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
                while !game.stack.is_empty() { ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap(); }
                assert!(game.object(attacker).is_none_or(|object| object.zone == ironsmith::Zone::Graveyard));
                assert_eq!(game.player(ironsmith::PlayerId::from_index(0)).unwrap().graveyard.len(), 1,
                    "multiple decayed counters sacrifice the same creature only once");
            }
        }
    }

    #[test]
    fn retained_counter_store_codec_rejects_unknown_and_wrong_payload_references() {
        let mut counters = ObjectCounters::default(); counters.add(CounterType::Flying, 1);
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        let mut retained = encoder.encode_counter_store(&counters).unwrap();
        let decoder = StaticAbilityOccurrenceDecoder::restore(encoder.into_table()).unwrap();
        retained.occurrences[0].slots[0] = RetainedCounterAbilitySlot::Static(StaticAbilityOccurrenceRef(99));
        assert!(decoder.restore_counter_store(retained).is_err());
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        let mut retained = encoder.encode_counter_store(&counters).unwrap();
        let haste_alias = encoder.encode_ability(ironsmith::ability::Ability::static_ability(
            ironsmith::static_abilities::StaticAbility::haste())).unwrap();
        let _ = haste_alias;
        let decoder = StaticAbilityOccurrenceDecoder::restore(encoder.into_table()).unwrap();
        retained.occurrences[0].slots[0] = RetainedCounterAbilitySlot::Static(StaticAbilityOccurrenceRef(1));
        assert!(decoder.restore_counter_store(retained).is_err(), "a valid table reference must still match its counter type");
    }
}

#[cfg(test)]
mod retained_cast_payment_codec_tests {
    use super::*;
    use ironsmith_runtime_catalog::artifact_materializer::*;
    fn fixture() -> (ironsmith::GameState, ironsmith::ObjectId, ironsmith::object::NativeCastPaymentState, ironsmith::CardId, ironsmith::CardId) {
        let definition = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Historical payer"),
            "Type: Artifact\nPay 1 life: You gain 2 life.", false).unwrap().1;
        let alice = ironsmith::PlayerId::from_index(0);
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, alice, ironsmith::Zone::Battlefield);
        let flying = ironsmith::static_abilities::StaticAbility::flying();
        let mut abilities = game.object(source).unwrap().abilities.as_ref().clone();
        abilities.push(ironsmith::ability::Ability::static_ability(flying.clone()));
        game.object_mut(source).unwrap().abilities = std::sync::Arc::new(abilities);
        let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
        let main = snapshot.card.unwrap(); let linked = ironsmith::CardId::new();
        let cost = snapshot.abilities.iter().find_map(|ability| match &ability.kind {
            ironsmith::ability::AbilityKind::Activated(ability) => Some(ability.mana_cost.costs()[0].clone()), _ => None,
        }).unwrap();
        let total = ironsmith::cost::TotalCost::from_cost(cost);
        let method = ironsmith::alternative_cast::AlternativeCastingMethod::Composed {
            name: "Captured method".into(), total_cost: total.clone(),
            condition: Some(ironsmith::static_abilities::ThisSpellCostCondition::YourTurn),
            prototype_power_toughness: None,
        };
        let mut effect = ironsmith::continuous::ContinuousEffect::new(source, alice,
            ironsmith::continuous::EffectTarget::Specific(source),
            ironsmith::continuous::Modification::SetColors(ironsmith::color::ColorSet::RED));
        effect.originating_static_ability = Some(flying);
        let mut object = ironsmith::object::Object::new_hidden_card(ironsmith::ObjectId::from_raw(99234), alice, ironsmith::Zone::Stack);
        object.alternative_casts = vec![method.clone()].into();
        object.cast_alternative_method = Some(Box::new(method));
        object.additional_cost = total.clone().into();
        object.optional_costs = vec![ironsmith::cost::OptionalCost::buyback(ironsmith::cost::TotalCost::one_of(vec![total.clone(), total])).repeatable()].into();
        object.optional_costs_paid.costs = vec![(object.optional_costs[0].cost_ref().clone(), 2)];
        object.optional_costs_paid.branch_choices = vec![(0, 1)];
        object.optional_costs_paid.cast_at_sorcery_timing = true;
        object.cast_play_from_constraints = Some(Box::new((source, ironsmith::Zone::Exile,
            ironsmith::grant_registry::PlayFromConstraints { lands_enter_tapped: true, ..Default::default() })));
        object.cast_grant_usage_identity = Some(Box::new(ironsmith::grant_registry::GrantPermissionIdentity::Static {
            source, printed_face: Some(main),
            origin: ironsmith::continuous::AbilityOrigin::Borrowed { source,
                effect: ironsmith::continuous::AbilityEffectOrigin::from(&effect),
                origin: Box::new(ironsmith::continuous::AbilityOrigin::Level { printed_face: Some(linked),
                    parent: Box::new(ironsmith::continuous::AbilityOrigin::Printed(2)), tier: 3, slot: 4 }),
            },
        }));
        object.x_value = Some(9); object.mana_spent_to_cast.blue = 3; object.snow_mana_spent_to_cast.blue = 2;
        object.keyword_payment_contributions_to_cast.push(ironsmith::decision::KeywordPaymentContribution {
            permanent_id: source, effect: ironsmith::decision::AlternativePaymentEffect::Convoke,
        });
        object.cast_tagged_objects.insert("paid_card".into(), vec![snapshot]);
        game.move_object(source, ironsmith::Zone::Graveyard, ironsmith::events::cause::EventCause::effect()).unwrap();
        assert!(game.object(source).is_none());
        (game, source, ironsmith::object::NativeCastPaymentState::from(&object), main, linked)
    }

    #[test]
    fn retained_cast_payment_codec_restores_history_permissions_and_actual_life_payment() {
        let (mut game, source, state, main, linked) = fixture();
        let alice = ironsmith::PlayerId::from_index(0);
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        let wire = encoder.encode_cast_payment_state(state, |id| {
            if id == main { Ok("main".to_string()) } else { assert_eq!(id, linked); Ok("linked".to_string()) }
        }).unwrap();
        let table = encoder.into_table(); assert_eq!(table.models.len(), 1, "history seeds the permission's shared static occurrence");
        let json = serde_json::to_value((&table, &wire)).unwrap();
        let (table, wire): (RetainedStaticAbilityTable, RetainedOccurrenceCastPaymentState<String>) = serde_json::from_value(json.clone()).unwrap();
        let decoder = StaticAbilityOccurrenceDecoder::restore(table).unwrap();
        let peer_main = ironsmith::CardId::new(); let peer_linked = ironsmith::CardId::new();
        let restored = decoder.restore_cast_payment_state(wire, |reference| match reference.as_str() {
            "main" => Ok(peer_main), "linked" => Ok(peer_linked), _ => panic!("all graph references are explicit"),
        }).unwrap();
        let mut object = ironsmith::object::Object::new_hidden_card(ironsmith::ObjectId::from_raw(99235), alice, ironsmith::Zone::Stack);
        restored.apply_to(&mut object).unwrap();
        let historical = &object.cast_tagged_objects[&ironsmith::tag::TagKey::from("paid_card")][0];
        assert_eq!(historical.card, Some(peer_main));
        assert!(game.object(source).is_none());
        let ironsmith::ability::AbilityKind::Static(alias) = &historical.abilities.last().unwrap().kind else { panic!("historical static alias") };
        let ironsmith::grant_registry::GrantPermissionIdentity::Static { origin, printed_face, .. } = object.cast_grant_usage_identity.as_deref().unwrap() else { panic!("captured static permission") };
        assert_eq!(*printed_face, Some(peer_main));
        let ironsmith::continuous::AbilityOrigin::Borrowed { effect, origin, .. } = origin else { panic!("borrowed origin") };
        assert_eq!(effect.static_ability(), Some(alias.instance_id()));
        assert!(matches!(origin.as_ref(), ironsmith::continuous::AbilityOrigin::Level { printed_face: Some(face), .. } if *face == peer_linked));
        assert_eq!(object.x_value, Some(9)); assert_eq!(object.mana_spent_to_cast.blue, 3); assert_eq!(object.snow_mana_spent_to_cast.blue, 2);
        assert_eq!(object.optional_costs_paid.branch_choices, vec![(0, 1)]);
        assert!(object.optional_costs_paid.cast_at_sorcery_timing);
        assert_eq!(object.keyword_payment_contributions_to_cast[0].permanent_id, source);
        let mut reencoder = StaticAbilityOccurrenceEncoder::default();
        let wire = reencoder.encode_cast_payment_state(ironsmith::object::NativeCastPaymentState::from(&object), |id| {
            if id == peer_main { Ok("main".to_string()) } else { assert_eq!(id, peer_linked); Ok("linked".to_string()) }
        }).unwrap();
        assert_eq!(serde_json::to_value((reencoder.into_table(), wire)).unwrap(), json);
        let payer = game.create_object_from_card(&ironsmith::card::CardBuilder::new(ironsmith::CardId::new(), "Current payer").build(), alice, ironsmith::Zone::Battlefield);
        let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
        let mut context = ironsmith::costs::CostContext::new(payer, alice, &mut dm);
        for cost in object.additional_cost.costs() { cost.pay(&mut game, &mut context).unwrap(); }
        assert_eq!(game.player(alice).unwrap().life, 19, "restored additional cost executes");
        let chosen_branch = object.optional_costs_paid.branch_choices[0].1;
        let paid_count = object.optional_costs_paid.costs[0].1;
        assert_eq!(object.optional_costs_paid.costs[0].0, object.optional_costs[0].cost_ref());
        for _ in 0..paid_count {
            for cost in object.optional_costs[0].cost.as_one_of().unwrap()[chosen_branch].costs() {
                cost.pay(&mut game, &mut context).unwrap();
            }
        }
        assert_eq!(game.player(alice).unwrap().life, 17, "captured branch and repeat count drive restored optional payments");
        let historical = &object.cast_tagged_objects[&ironsmith::tag::TagKey::from("paid_card")][0];
        let activated = historical.abilities.iter().find_map(|ability| match &ability.kind {
            ironsmith::ability::AbilityKind::Activated(ability) => Some(ability), _ => None,
        }).unwrap();
        let mut context = ironsmith::effects::EffectContext::new_default(source, alice);
        for effect in activated.effects.all_effects() { ironsmith::effects::execute_effect(&mut game, effect, &mut context).unwrap(); }
        assert_eq!(game.player(alice).unwrap().life, 19, "historical program still executes after its source leaves");
    }

    #[test]
    fn retained_cast_payment_codec_rolls_back_failed_graph_binding_and_rejects_unknown_refs() {
        let (_, _, state, main, linked) = fixture();
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        assert!(encoder.encode_cast_payment_state(state.clone(), |id| {
            if id == main { Ok(0u8) } else { assert_eq!(id, linked); Err(OccurrenceBindingError::InvalidModel { detail: "missing linked face".into() }) }
        }).is_err());
        assert!(encoder.clone().into_table().models.is_empty(), "history retained before permission failure must roll back with the complete cast root");
        let mut wire = encoder.encode_cast_payment_state(state, |id| Ok(if id == main { 0u8 } else { assert_eq!(id, linked); 1u8 })).unwrap();
        let decoder = StaticAbilityOccurrenceDecoder::restore(encoder.into_table()).unwrap();
        assert!(decoder.restore_cast_payment_state(wire.clone(), |_| Err(OccurrenceBindingError::InvalidModel { detail: "unknown graph reference".into() })).is_err());
        let Some(ironsmith::grant_registry::RetainedGrantPermissionIdentity::Static { origin, .. }) = &mut wire.cast_grant_usage_identity else { panic!("static permission") };
        *origin = origin.clone().try_map_static_instances(&mut |_| Ok::<_, ()>(StaticAbilityOccurrenceRef(99))).unwrap();
        assert!(decoder.restore_cast_payment_state(wire, |_| Ok(ironsmith::CardId::new())).is_err());
    }
}

#[cfg(test)]
mod retained_live_object_codec_tests {
    use super::*;
    use ironsmith_runtime_catalog::artifact_materializer::*;
    fn fixture() -> (
        ironsmith::GameState,
        ironsmith::object::Object,
        ironsmith::CardId,
        ironsmith::CardId,
    ) {
        let definition = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Executable object"),
            "Type: Artifact\nPay 1 life: You gain 2 life.",
            false,
        )
        .unwrap()
        .1;
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = ironsmith::PlayerId::from_index(0);
        let id =
            game.create_object_from_definition(&definition, alice, ironsmith::Zone::Battlefield);
        let mut object = game.object(id).unwrap().clone();
        let main = object.card.unwrap();
        let linked = ironsmith::CardId::new();
        let activated = object
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                ironsmith::ability::AbilityKind::Activated(ability) => Some(ability.clone()),
                _ => None,
            })
            .unwrap();
        object.spell_effect = Some(activated.effects.clone().into());
        object.additional_cost = activated.mana_cost.clone().into();
        let flying = ironsmith::static_abilities::StaticAbility::flying();
        object
            .abilities_mut()
            .push(ironsmith::ability::Ability::static_ability(flying.clone()));
        object.temporary_static_ability_grants.push(
            ironsmith::object::TemporaryStaticAbilityGrant {
                ability: flying.id(),
                ability_payload: Some(flying),
                expires_end_of_turn: 3,
            },
        );
        object
            .counters
            .add(ironsmith::object::CounterType::Flying, 2);
        object
            .counters
            .insert(ironsmith::object::CounterType::Flying, 1);
        object.other_face = Some(linked);
        object.other_face_name = Some("Other face".into());
        object.linked_face_layout = ironsmith::card::LinkedFaceLayout::TransformLike;
        object.card_types =
            vec![ironsmith::CardType::Artifact, ironsmith::CardType::Creature].into();
        object.base_power = Some(ironsmith::card::PtValue::StarPlus(1));
        object.base_toughness = Some(ironsmith::card::PtValue::Fixed(4));
        object.first_printed_set_name = Some("Recorded set".into());
        object.hand_modifier = -2;
        object.life_modifier = 5;
        object.x_value = Some(7);
        object.aura_attach_filter = Some(
            ironsmith::object::AuraAttachmentFilter::Object(
                ironsmith::target::ObjectFilter::default(),
            )
            .into(),
        );
        object.cast_grant_usage_identity = Some(Box::new(
            ironsmith::grant_registry::GrantPermissionIdentity::LinkedFace {
                source: id,
                face: linked,
                slot: 2,
            },
        ));
        *game.object_mut(id).unwrap() = object.clone();
        let snapshot =
            ironsmith::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                &object, &game,
            );
        object
            .cast_tagged_objects
            .insert("paid_source".into(), vec![snapshot]);
        (game, object, main, linked)
    }
    fn encode(
        object: ironsmith::object::Object,
        main: ironsmith::CardId,
        linked: ironsmith::CardId,
    ) -> (RetainedStaticAbilityTable, RetainedOccurrenceLiveObject<u8>) {
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        let object = encoder
            .encode_live_object(object, |face| {
                if face == main {
                    Ok(0u8)
                } else {
                    assert_eq!(face, linked);
                    Ok(1u8)
                }
            })
            .unwrap();
        (encoder.into_table(), object)
    }
    fn restore(
        table: RetainedStaticAbilityTable,
        object: RetainedOccurrenceLiveObject<u8>,
        main: ironsmith::CardId,
        linked: ironsmith::CardId,
    ) -> ironsmith::object::Object {
        StaticAbilityOccurrenceDecoder::restore(table)
            .unwrap()
            .restore_live_object(object, |reference| match reference {
                0 => Ok(main),
                1 => Ok(linked),
                _ => panic!("complete graph references"),
            })
            .unwrap()
    }
    fn execute_program(
        game: &mut ironsmith::GameState,
        source: ironsmith::ObjectId,
        program: &ironsmith::resolution::ResolutionProgram,
    ) {
        let mut context = ironsmith::effects::EffectContext::new_default(
            source,
            ironsmith::PlayerId::from_index(0),
        );
        for effect in program.all_effects() {
            ironsmith::effects::execute_effect(game, effect, &mut context).unwrap();
        }
    }

    #[test]
    fn retained_live_object_codec_native_disguise_ward_cost_pays_after_restore() {
        let (mut game, mut object, main, linked) = fixture();
        assert!(object.apply_face_down_cast_overlay_with_disguise_ward(true));
        let (table, wire) = encode(object, main, linked);
        let (table, wire): (RetainedStaticAbilityTable, RetainedOccurrenceLiveObject<u8>) =
            serde_json::from_value(serde_json::to_value((table, wire)).unwrap()).unwrap();
        let restored = restore(table, wire, main, linked);
        assert!(
            restored
                .face_down_cast_state
                .as_ref()
                .unwrap()
                .disguise_ward
        );
        let ward = restored
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                ironsmith::ability::AbilityKind::Static(ability) => ability.ward_cost(),
                _ => None,
            })
            .expect("restored disguise retains native Ward2 cost");
        let alice = ironsmith::PlayerId::from_index(0);
        game.player_mut(alice).unwrap().mana_pool.colorless = 2;
        let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
        let mut context = ironsmith::costs::CostContext::new(restored.id, alice, &mut dm);
        for cost in ward.costs() {
            cost.pay(&mut game, &mut context).unwrap();
        }
        assert_eq!(game.player(alice).unwrap().mana_pool.colorless, 0);
        assert_eq!(game.player(alice).unwrap().life, 20);
    }

    #[test]
    fn retained_live_object_codec_shares_all_carriers_and_restores_actual_program_and_cost() {
        let (mut game, mut object, main, linked) = fixture();
        object.capture_enters_as_copy_restore_state();
        let printed = object
            .enters_as_copy_restore_state
            .as_ref()
            .unwrap()
            .printed
            .clone();
        object.face_down_cast_state = Some(Box::new(printed.clone()));
        object.bestow_cast_state = Some(Box::new(ironsmith::object::BestowCastState {
            card_types: object.card_types.clone(),
            subtypes: object.subtypes.clone(),
            aura_attach_filter: object.aura_attach_filter.clone(),
            spell_effect: object.spell_effect.clone(),
        }));
        object.splice_cast_state = Some(Box::new(ironsmith::object::SpliceCastState {
            spell_effect: object.spell_effect.clone(),
        }));
        object.prototype_cast_state = Some(ironsmith::object::PrototypeCastState {
            mana_cost: object.mana_cost.clone(),
            color_override: object.color_override,
            base_power: object.base_power,
            base_toughness: object.base_toughness,
        });
        let id = object.id;
        let stable_id = object.stable_id;
        let original_counts = object.counters.ability_state();
        let (table, wire) = encode(object, main, linked);
        assert_eq!(
            table.models.len(),
            3,
            "printed/granted flying, independent counter flying and shared enchant remain three occurrences"
        );
        let json = serde_json::to_value((&table, &wire)).unwrap();
        let (table, wire): (RetainedStaticAbilityTable, RetainedOccurrenceLiveObject<u8>) =
            serde_json::from_value(json.clone()).unwrap();
        let peer_main = ironsmith::CardId::new();
        let peer_linked = ironsmith::CardId::new();
        let restored = restore(table, wire, peer_main, peer_linked);
        assert_eq!(restored.id, id);
        assert_eq!(restored.stable_id, stable_id);
        assert_eq!(restored.card, Some(peer_main));
        assert_eq!(restored.other_face, Some(peer_linked));
        assert_eq!(
            restored
                .enters_as_copy_restore_state
                .as_ref()
                .unwrap()
                .other_face,
            Some(peer_linked)
        );
        assert_eq!(restored.counters.ability_state(), original_counts);
        let ironsmith::ability::AbilityKind::Static(printed) =
            &restored.abilities.last().unwrap().kind
        else {
            panic!("printed flying")
        };
        let printed_id = printed.instance_id();
        assert_eq!(
            restored.temporary_static_ability_grants[0]
                .materialize()
                .unwrap()
                .instance_id(),
            printed_id
        );
        for saved in [
            restored.face_down_cast_state.as_ref().unwrap().as_ref(),
            &restored
                .enters_as_copy_restore_state
                .as_ref()
                .unwrap()
                .printed,
        ] {
            let ironsmith::ability::AbilityKind::Static(ability) =
                &saved.abilities.last().unwrap().kind
            else {
                panic!("saved flying")
            };
            assert_eq!(ability.instance_id(), printed_id);
        }
        let history =
            &restored.cast_tagged_objects[&ironsmith::tag::TagKey::from("paid_source")][0];
        assert_eq!(history.card, Some(peer_main));
        assert!(history.abilities.iter().any(|ability| matches!(&ability.kind, ironsmith::ability::AbilityKind::Static(ability) if ability.instance_id() == printed_id)));
        let mut counter_ids = Vec::new();
        restored
            .counters
            .retain_with_static_occurrences(|ability| {
                counter_ids.push(ability.instance_id());
                Ok(())
            })
            .unwrap();
        assert_eq!(counter_ids.len(), 1);
        assert_ne!(counter_ids[0], printed_id);
        assert!(history.abilities.iter().any(|ability| matches!(&ability.kind, ironsmith::ability::AbilityKind::Static(ability) if ability.instance_id() == counter_ids[0])));
        let aura = ironsmith::object::RetainedAuraAttachmentMetadata::from(
            restored.aura_attach_filter.clone().unwrap(),
        );
        let saved_aura = ironsmith::object::RetainedAuraAttachmentMetadata::from(
            restored
                .bestow_cast_state
                .as_ref()
                .unwrap()
                .aura_attach_filter
                .clone()
                .unwrap(),
        );
        assert_eq!(
            aura.enchant_ability.instance_id(),
            saved_aura.enchant_ability.instance_id()
        );
        assert_eq!(
            serde_json::to_value(encode(restored.clone(), peer_main, peer_linked)).unwrap(),
            json
        );
        let alice = ironsmith::PlayerId::from_index(0);
        let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
        let mut context = ironsmith::costs::CostContext::new(id, alice, &mut dm);
        for cost in restored.additional_cost.costs() {
            cost.pay(&mut game, &mut context).unwrap();
        }
        assert_eq!(game.player(alice).unwrap().life, 19);
        execute_program(&mut game, id, restored.spell_effect.as_ref().unwrap());
        assert_eq!(game.player(alice).unwrap().life, 21);
        game.move_object(
            id,
            ironsmith::Zone::Graveyard,
            ironsmith::events::cause::EventCause::effect(),
        )
        .unwrap();
        assert!(game.object(id).is_none());
        let activated = history
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                ironsmith::ability::AbilityKind::Activated(ability) => Some(ability),
                _ => None,
            })
            .unwrap();
        execute_program(&mut game, id, &activated.effects);
        assert_eq!(
            game.player(alice).unwrap().life,
            23,
            "historical ability executes after source departure"
        );
    }

    #[test]
    fn retained_live_object_codec_saved_overlays_end_through_native_transitions() {
        for overlay in 0..5 {
            let (mut game, mut object, main, linked) = fixture();
            let original = object.clone();
            match overlay {
                0 => {
                    object.capture_enters_as_copy_restore_state();
                    object.name = "Copied name".into();
                    object.base_power = Some(ironsmith::card::PtValue::Fixed(8));
                }
                1 => {
                    assert!(object.apply_face_down_cast_overlay_with_disguise_ward(true));
                }
                2 => {
                    object.apply_bestow_cast_overlay();
                }
                3 => {
                    assert!(object.apply_prototype_cast_overlay(
                        ironsmith::mana::ManaCost::from_pips(Vec::new()),
                        ironsmith::card::PowerToughness::fixed(2, 3)
                    ));
                }
                4 => {
                    object.splice_cast_state = Some(Box::new(ironsmith::object::SpliceCastState {
                        spell_effect: object.spell_effect.clone(),
                    }));
                    object.spell_effect = Some(
                        ironsmith::resolution::ResolutionProgram::from_effects(Vec::new()).into(),
                    );
                }
                _ => unreachable!(),
            }
            let (table, wire) = encode(object, main, linked);
            let (table, wire): (RetainedStaticAbilityTable, RetainedOccurrenceLiveObject<u8>) =
                serde_json::from_value(serde_json::to_value((table, wire)).unwrap()).unwrap();
            let mut restored = restore(table, wire, main, linked);
            assert!(match overlay {
                0 => restored.end_enters_as_copy_overlay(),
                1 => restored.end_face_down_cast_overlay(),
                2 => restored.end_bestow_cast_overlay(),
                3 => restored.end_prototype_cast_overlay(),
                4 => restored.end_splice_cast_overlay(),
                _ => unreachable!(),
            });
            assert_eq!(restored.name, original.name);
            assert_eq!(restored.base_power, original.base_power);
            assert_eq!(restored.base_toughness, original.base_toughness);
            assert_eq!(restored.card_types, original.card_types);
            assert_eq!(restored.mana_cost, original.mana_cost);
            execute_program(
                &mut game,
                restored.id,
                restored.spell_effect.as_ref().unwrap(),
            );
            assert_eq!(
                game.player(ironsmith::PlayerId::from_index(0))
                    .unwrap()
                    .life,
                22
            );
        }
    }

    #[test]
    fn retained_live_object_codec_rejects_bad_refs_and_rolls_back_complete_root() {
        let (_, mut object, main, linked) = fixture();
        object.capture_enters_as_copy_restore_state();
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        let mut calls = 0;
        assert!(
            encoder
                .encode_live_object(object.clone(), |face| {
                    calls += 1;
                    if calls <= 2 {
                        Ok(if face == main { 0u8 } else { 1u8 })
                    } else {
                        Err(OccurrenceBindingError::InvalidModel {
                            detail: "unbound saved face".into(),
                        })
                    }
                })
                .is_err()
        );
        assert!(calls > 2);
        assert!(encoder.clone().into_table().models.is_empty());
        let (table, mut wire) = encode(object, main, linked);
        let decoder = StaticAbilityOccurrenceDecoder::restore(table).unwrap();
        assert!(
            decoder
                .restore_live_object(wire.clone(), |_| Err(
                    OccurrenceBindingError::InvalidModel {
                        detail: "unknown face".into()
                    }
                ))
                .is_err()
        );
        wire.counters.occurrences[0].slots[0] =
            ironsmith::object::RetainedCounterAbilitySlot::Static(StaticAbilityOccurrenceRef(99));
        assert!(
            decoder
                .restore_live_object(wire, |reference| Ok(if reference == 0 {
                    main
                } else {
                    linked
                }))
                .is_err()
        );
    }
}

#[cfg(test)]
mod retained_definition_codec_tests {
    use super::*;
    use ironsmith_runtime_catalog::artifact_materializer::*;
    fn fixture() -> (
        ironsmith::cards::CardDefinition,
        ironsmith::cards::CardDefinition,
    ) {
        let mut front = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Graph front"),
            "Type: Artifact\nPay 1 life: You gain 2 life.",
            false,
        )
        .unwrap()
        .1;
        let mut back = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Graph back"),
            "Type: Artifact\nPay 1 life: You gain 3 life.",
            false,
        )
        .unwrap()
        .1;
        front.card.other_face = Some(back.card.id);
        front.card.other_face_name = Some(back.card.name.clone());
        back.card.other_face = Some(front.card.id);
        back.card.other_face_name = Some(front.card.name.clone());
        front.card.linked_face_layout = ironsmith::card::LinkedFaceLayout::TransformLike;
        back.card.linked_face_layout = ironsmith::card::LinkedFaceLayout::TransformLike;
        front.card.transforming_dfc = true;
        back.card.transforming_dfc = true;
        front.card.first_printed_set_name = Some("Graph set".into());
        front.card.attraction_lights = vec![2, 3];
        front.card.hand_modifier = -1;
        front.card.life_modifier = 4;
        front
            .abilities
            .push(ironsmith::ability::Ability::static_ability(
                ironsmith::static_abilities::StaticAbility::flying(),
            ));
        front.ability_labels.push("Flying".into());
        front.canonical_text.push_str("\nFlying");
        assert!(!front.canonical_text.is_empty());
        assert!(!front.ability_labels.is_empty());
        (front, back)
    }
    fn program(
        definition: &ironsmith::cards::CardDefinition,
    ) -> &ironsmith::ability::ActivatedAbility {
        definition
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                ironsmith::ability::AbilityKind::Activated(ability) => Some(ability),
                _ => None,
            })
            .unwrap()
    }
    #[test]
    fn retained_definition_codec_preserves_metadata_cyclic_faces_and_shared_live_abilities() {
        let (front, back) = fixture();
        let front_id = front.card.id;
        let back_id = back.card.id;
        let bind = |id| {
            if id == front_id {
                Ok(0u8)
            } else {
                assert_eq!(id, back_id);
                Ok(1u8)
            }
        };
        let object = ironsmith::object::Object::from_card_definition(
            ironsmith::ObjectId::from_raw(66771),
            &front,
            ironsmith::PlayerId::from_index(0),
            ironsmith::Zone::Battlefield,
        );
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        let front_wire = encoder.encode_card_definition(front.clone(), bind).unwrap();
        let back_wire = encoder.encode_card_definition(back.clone(), bind).unwrap();
        let object_wire = encoder.encode_live_object(object, bind).unwrap();
        let table = encoder.into_table();
        assert_eq!(table.models.len(), 1);
        let json = serde_json::to_value((&table, &front_wire, &back_wire, &object_wire)).unwrap();
        let (table, front_wire, back_wire, object_wire): (
            RetainedStaticAbilityTable,
            RetainedOccurrenceCardDefinition<u8>,
            RetainedOccurrenceCardDefinition<u8>,
            RetainedOccurrenceLiveObject<u8>,
        ) = serde_json::from_value(json.clone()).unwrap();
        let decoder = StaticAbilityOccurrenceDecoder::restore(table).unwrap();
        let peer_front = ironsmith::CardId::new();
        let peer_back = ironsmith::CardId::new();
        let unbind = |reference| match reference {
            0 => Ok(peer_front),
            1 => Ok(peer_back),
            _ => panic!("explicit complete definition graph"),
        };
        let restored_front = decoder.restore_card_definition(front_wire, unbind).unwrap();
        let restored_back = decoder.restore_card_definition(back_wire, unbind).unwrap();
        let restored_object = decoder.restore_live_object(object_wire, unbind).unwrap();
        assert_eq!(restored_front.card.id, peer_front);
        assert_eq!(restored_front.card.other_face, Some(peer_back));
        assert_eq!(restored_back.card.id, peer_back);
        assert_eq!(restored_back.card.other_face, Some(peer_front));
        assert!(restored_front.card.transforming_dfc && restored_back.card.transforming_dfc);
        assert_eq!(restored_front.canonical_text, front.canonical_text);
        assert_eq!(restored_front.ability_labels, front.ability_labels);
        assert_eq!(restored_front.card.attraction_lights, vec![2, 3]);
        assert_eq!(
            restored_front.card.first_printed_set_name,
            front.card.first_printed_set_name
        );
        assert_eq!(restored_front.card.hand_modifier, -1);
        assert_eq!(restored_front.card.life_modifier, 4);
        let ironsmith::ability::AbilityKind::Static(definition_alias) =
            &restored_front.abilities.last().unwrap().kind
        else {
            panic!("definition flying")
        };
        let ironsmith::ability::AbilityKind::Static(object_alias) =
            &restored_object.abilities.last().unwrap().kind
        else {
            panic!("object flying")
        };
        assert_eq!(definition_alias.instance_id(), object_alias.instance_id());
        let mut reencoder = StaticAbilityOccurrenceEncoder::default();
        let bind_peer = |id| {
            if id == peer_front {
                Ok(0u8)
            } else {
                assert_eq!(id, peer_back);
                Ok(1u8)
            }
        };
        let front_wire = reencoder
            .encode_card_definition(restored_front.clone(), bind_peer)
            .unwrap();
        let back_wire = reencoder
            .encode_card_definition(restored_back.clone(), bind_peer)
            .unwrap();
        let object_wire = reencoder
            .encode_live_object(restored_object, bind_peer)
            .unwrap();
        assert_eq!(
            serde_json::to_value((reencoder.into_table(), front_wire, back_wire, object_wire))
                .unwrap(),
            json
        );
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = ironsmith::PlayerId::from_index(0);
        game.register_linked_face_definition(&restored_front);
        game.register_linked_face_definition(&restored_back);
        let source = game.create_object_from_definition(
            &restored_front,
            alice,
            ironsmith::Zone::Battlefield,
        );
        let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
        let mut cost_context = ironsmith::costs::CostContext::new(source, alice, &mut dm);
        for cost in program(&restored_front).mana_cost.costs() {
            cost.pay(&mut game, &mut cost_context).unwrap();
        }
        let mut context = ironsmith::effects::EffectContext::new_default(source, alice);
        for effect in program(&restored_front).effects.all_effects() {
            ironsmith::effects::execute_effect(&mut game, effect, &mut context).unwrap();
        }
        assert_eq!(game.player(alice).unwrap().life, 21);
        assert!(game.transform_permanent(source).unwrap());
        let shown_back = game.object(source).unwrap();
        // Transformation preserves the physical card while installing the other face.
        assert_eq!(shown_back.card, Some(peer_front));
        assert_eq!(shown_back.name.as_ref(), restored_back.card.name);
        assert_eq!(shown_back.other_face, Some(peer_front));
        assert!(game.is_transformed_permanent(source));
        let back_program = shown_back
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                ironsmith::ability::AbilityKind::Activated(program) => Some(program.clone()),
                _ => None,
            })
            .unwrap();
        for effect in back_program.effects.all_effects() {
            ironsmith::effects::execute_effect(&mut game, effect, &mut context).unwrap();
        }
        assert_eq!(game.player(alice).unwrap().life, 24);
        assert!(game.transform_permanent(source).unwrap());
        let shown_front = game.object(source).unwrap();
        assert_eq!(shown_front.card, Some(peer_front));
        assert_eq!(shown_front.name.as_ref(), restored_front.card.name);
        assert_eq!(shown_front.other_face, Some(peer_back));
        assert!(!game.is_transformed_permanent(source));
        let front_program = shown_front
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                ironsmith::ability::AbilityKind::Activated(program) => Some(program.clone()),
                _ => None,
            })
            .unwrap();
        for effect in front_program.effects.all_effects() {
            ironsmith::effects::execute_effect(&mut game, effect, &mut context).unwrap();
        }
        assert_eq!(game.player(alice).unwrap().life, 26);
    }

    #[test]
    fn retained_definition_codec_distinguishes_same_name_linked_graph_nodes() {
        let (front, back) = fixture();
        let (other_front, mut other_back) = fixture();
        // Independent graph nodes can have the same name and different runtime payloads.
        other_back.abilities = other_front.abilities.clone();
        other_back.card.life_modifier = 42;
        let definitions = [front, back, other_front, other_back];
        let original_ids = definitions.each_ref().map(|definition| definition.card.id);
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        let wires = definitions.each_ref().map(|definition| {
            encoder
                .encode_card_definition(definition.clone(), |id| {
                    original_ids
                        .iter()
                        .position(|candidate| *candidate == id)
                        .map(|index| index as u8)
                        .ok_or_else(|| OccurrenceBindingError::InvalidModel {
                            detail: "unbound definition graph node".into(),
                        })
                })
                .unwrap()
        });
        let (table, wires): (
            RetainedStaticAbilityTable,
            [RetainedOccurrenceCardDefinition<u8>; 4],
        ) = serde_json::from_value(serde_json::to_value((encoder.into_table(), wires)).unwrap())
            .unwrap();
        let peer_ids = original_ids.map(|_| ironsmith::CardId::new());
        let decoder = StaticAbilityOccurrenceDecoder::restore(table).unwrap();
        let restored = wires.map(|wire| {
            decoder
                .restore_card_definition(wire, |index| {
                    peer_ids.get(index as usize).copied().ok_or_else(|| {
                        OccurrenceBindingError::InvalidModel {
                            detail: "unknown definition graph node".into(),
                        }
                    })
                })
                .unwrap()
        });
        for registration_order in [[0, 1, 2, 3], [2, 3, 0, 1]] {
            let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = ironsmith::PlayerId::from_index(0);
            for index in registration_order {
                game.register_linked_face_definition(&restored[index]);
            }
            // Displayed-face lookup can intentionally use an original physical card id.
            let mut displayed = restored[1].clone();
            displayed.card.id = ironsmith::CardId::new();
            displayed.card.name = "Distinct displayed face".into();
            game.register_linked_face_definition(&displayed);
            assert_eq!(
                game.linked_face_definition_by_name_or_id(
                    Some(&displayed.card.name),
                    Some(peer_ids[0]),
                )
                .unwrap()
                .card
                .id,
                displayed.card.id,
            );
            let mut front_objects = Vec::new();
            for (front_index, back_index, expected_gain) in [(0, 1, 3), (2, 3, 2)] {
                let source = game.create_object_from_definition(
                    &restored[front_index],
                    alice,
                    ironsmith::Zone::Battlefield,
                );
                assert!(!game.is_transformed_permanent(source));
                assert!(game.transform_permanent(source).unwrap());
                assert!(game.is_transformed_permanent(source));
                let object = game.object(source).unwrap();
                assert_eq!(object.other_face, Some(peer_ids[front_index]));
                assert_eq!(
                    object.life_modifier,
                    restored[back_index].card.life_modifier
                );
                let activated = object
                    .abilities
                    .iter()
                    .find_map(|ability| match &ability.kind {
                        ironsmith::ability::AbilityKind::Activated(program) => {
                            Some(program.clone())
                        }
                        _ => None,
                    })
                    .unwrap();
                let before = game.player(alice).unwrap().life;
                let mut context = ironsmith::effects::EffectContext::new_default(source, alice);
                for effect in activated.effects.all_effects() {
                    ironsmith::effects::execute_effect(&mut game, effect, &mut context).unwrap();
                }
                assert_eq!(game.player(alice).unwrap().life, before + expected_gain);
                assert!(game.transform_permanent(source).unwrap());
                assert_eq!(
                    game.object(source).unwrap().other_face,
                    Some(peer_ids[back_index])
                );
                front_objects.push((source, front_index, back_index));
            }
            // Registering the second same-name graph must not change the first's status.
            for (source, front_index, back_index) in front_objects {
                assert!(!game.is_transformed_permanent(source));
                assert_eq!(
                    game.displayed_face_definition(game.object(source).unwrap())
                        .unwrap()
                        .card
                        .id,
                    peer_ids[front_index]
                );
                assert!(game.transform_permanent(source).unwrap());
                assert!(game.is_transformed_permanent(source));
                assert_eq!(
                    game.displayed_face_definition(game.object(source).unwrap())
                        .unwrap()
                        .card
                        .id,
                    peer_ids[back_index]
                );
                let moved = game
                    .move_object(
                        source,
                        ironsmith::Zone::Graveyard,
                        ironsmith::events::cause::EventCause::effect(),
                    )
                    .unwrap();
                let departed = game.object(moved).unwrap();
                assert_eq!(departed.card, Some(peer_ids[front_index]));
                assert_eq!(departed.name.as_ref(), restored[front_index].card.name);
                assert_eq!(departed.other_face, Some(peer_ids[back_index]));
                assert!(!game.is_transformed_permanent(moved));
            }
            for (front_index, back_index, conflicting_back_index) in [(0, 1, 3), (2, 3, 1)] {
                // An entry authored back-face-up still has its physical front identity.
                let source = game.create_object_from_definition(
                    &restored[back_index],
                    alice,
                    ironsmith::Zone::Battlefield,
                );
                game.object_mut(source).unwrap().card = Some(peer_ids[front_index]);
                game.register_linked_face_definition(&restored[conflicting_back_index]);
                assert!(game.is_transformed_permanent(source));
                let mut context = ironsmith::effects::EffectContext::new_default(source, alice);
                context.triggering_event =
                    Some(ironsmith::triggers::TriggerEvent::new_with_provenance(
                        ironsmith::events::zones::EnterBattlefieldEvent::new(
                            source,
                            ironsmith::Zone::Hand,
                        ),
                        ironsmith::ProvNodeId::default(),
                    ));
                assert!(
                    ironsmith::condition_eval::evaluate_condition_resolution(
                        &game,
                        &ironsmith::effect::Condition::TriggeringObjectEnteredTransformed,
                        &context,
                    )
                    .unwrap()
                );
                game.mark_transformed(source);
                assert!(
                    !ironsmith::condition_eval::evaluate_condition_resolution(
                        &game,
                        &ironsmith::effect::Condition::TriggeringObjectEnteredTransformed,
                        &context,
                    )
                    .unwrap()
                );
            }
        }
    }

    fn assert_nested_definition_requires_graph_binding(text: &str) {
        let root_id = ironsmith::CardId::new();
        let definition = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(root_id, "Nested graph producer"),
            text,
            false,
        )
        .unwrap()
        .1;
        assert_eq!(definition.card.id, root_id);
        let effects = match definition.spell_effect.as_ref() {
            Some(program) => program.all_effects(),
            None => program(&definition).effects.all_effects(),
        };
        assert_eq!(effects.len(), 1);
        let wire_effect = encode_runtime_effect(effects[0].clone()).unwrap();
        // Spell lowering retains its outcome tag around the token producer.
        // Inspect that typed child without removing the executable wrapper.
        let token_effect = if wire_effect.kind() == "TaggedEffect" {
            serde_json::from_value::<
                compiler::effects::CoreTaggedEffect<ironsmith_compiled_artifact::WireEffect>,
            >(wire_effect.payload().clone())
            .unwrap()
            .effect
        } else {
            Box::new(wire_effect)
        };
        assert_eq!(token_effect.kind(), "CreateTokenEffect");
        let template: ironsmith_compiled_artifact::WireCardDefinition =
            serde_json::from_value(token_effect.payload()["token"].clone()).unwrap();
        assert!(template.card.is_token);
        assert_ne!(template.card.id, root_id);
        // Prove this is an executable token producer, rather than an empty parser fixture.
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = ironsmith::PlayerId::from_index(0);
        let source = game.create_object_from_definition(&definition, alice, ironsmith::Zone::Stack);
        let before = game.battlefield.len();
        let mut context = ironsmith::effects::EffectContext::new_default(source, alice);
        for effect in &effects {
            ironsmith::effects::execute_effect(&mut game, effect, &mut context).unwrap();
        }
        assert_eq!(game.battlefield.len(), before + 1);
        let token = game
            .battlefield
            .iter()
            .find_map(|id| game.object(*id))
            .unwrap();
        assert_eq!(token.kind, ironsmith::object::ObjectKind::Token);
        assert_eq!(token.base_power, Some(ironsmith::card::PtValue::Fixed(1)));
        assert_eq!(
            token.base_toughness,
            Some(ironsmith::card::PtValue::Fixed(1))
        );
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        let mut rejected_ids = Vec::new();
        let result = encoder.encode_card_definition(definition, |id| {
            if id == root_id {
                Ok(0u32)
            } else {
                rejected_ids.push(id);
                Err(OccurrenceBindingError::InvalidModel {
                    detail: "nested token definition is not in the owning graph".into(),
                })
            }
        });
        assert!(
            result.is_err(),
            "unbound nested definitions must not bypass the owning graph"
        );
        assert!(
            !rejected_ids.is_empty(),
            "nested definition IDs must reach the graph binder"
        );
        assert!(
            encoder.into_table().models.is_empty(),
            "failed root must not publish occurrences"
        );
    }

    #[test]
    fn retained_definition_codec_spell_token_template_requires_owning_graph_binding() {
        assert_nested_definition_requires_graph_binding(
            "Type: Sorcery\nCreate a 1/1 white Soldier creature token.",
        );
    }

    #[test]
    fn retained_definition_codec_activated_token_template_requires_owning_graph_binding() {
        assert_nested_definition_requires_graph_binding(
            "Type: Artifact\nPay 1 life: Create a 1/1 white Soldier creature token.",
        );
    }

    fn template_id(effect: &ironsmith::effect::Effect) -> ironsmith::CardId {
        let wire = encode_runtime_effect(effect.clone()).unwrap();
        let token_effect = if wire.kind() == "TaggedEffect" {
            serde_json::from_value::<
                compiler::effects::CoreTaggedEffect<ironsmith_compiled_artifact::WireEffect>,
            >(wire.payload().clone())
            .unwrap()
            .effect
        } else {
            Box::new(wire)
        };
        let template: ironsmith_compiled_artifact::WireCardDefinition =
            serde_json::from_value(token_effect.payload()["token"].clone()).unwrap();
        template.card.id
    }

    #[test]
    fn retained_definition_codec_nested_templates_restore_fresh_peer_and_execute() {
        for text in [
            "Type: Sorcery\nCreate a 1/1 white Soldier creature token.",
            "Type: Artifact\nPay 1 life: Create a 1/1 white Soldier creature token.",
        ] {
            let definition = compile_builder_to_artifact(
                compiler::CardDefinitionBuilder::new(
                    ironsmith::CardId::new(),
                    "Bound token producer",
                ),
                text,
                false,
            )
            .unwrap()
            .1;
            let original_effects = match definition.spell_effect.as_ref() {
                Some(program) => program.all_effects(),
                None => program(&definition).effects.all_effects(),
            };
            let original_ids = [definition.card.id, template_id(original_effects[0])];
            let bind = |id| {
                original_ids
                    .iter()
                    .position(|candidate| *candidate == id)
                    .map(|index| index as u32)
                    .ok_or_else(|| OccurrenceBindingError::InvalidModel {
                        detail: "unknown source graph node".into(),
                    })
            };
            let mut encoder = StaticAbilityOccurrenceEncoder::default();
            let wire = encoder.encode_card_definition(definition, bind).unwrap();
            let table = encoder.into_table();
            assert_eq!(table.payload_card_references, vec![serde_json::json!(1u32)]);
            assert!(StaticAbilityOccurrenceDecoder::restore(table.clone()).is_err());
            let json = serde_json::to_value((&table, &wire)).unwrap();
            let (table, wire): (
                RetainedStaticAbilityTable,
                RetainedOccurrenceCardDefinition<u32>,
            ) = serde_json::from_value(json.clone()).unwrap();
            let peer = [ironsmith::CardId::new(), ironsmith::CardId::new()];
            let bind_peer = |index: u32| {
                peer.get(index as usize).copied().ok_or_else(|| {
                    OccurrenceBindingError::InvalidModel {
                        detail: "unknown receiver graph node".into(),
                    }
                })
            };
            assert!(
                StaticAbilityOccurrenceDecoder::restore_with_card_graph::<u32>(
                    table.clone(),
                    |_| {
                        Err(OccurrenceBindingError::InvalidModel {
                            detail: "missing receiver template node".into(),
                        })
                    }
                )
                .is_err()
            );
            let mut malformed = table.clone();
            malformed
                .payload_card_references
                .push(serde_json::json!(1u32));
            assert!(
                StaticAbilityOccurrenceDecoder::restore_with_card_graph::<u32>(
                    malformed, bind_peer
                )
                .is_err()
            );
            let mut missing = serde_json::to_value(&table).unwrap();
            missing
                .as_object_mut()
                .unwrap()
                .remove("payload_card_references");
            assert!(serde_json::from_value::<RetainedStaticAbilityTable>(missing).is_err());
            let decoder =
                StaticAbilityOccurrenceDecoder::restore_with_card_graph::<u32>(table, bind_peer)
                    .unwrap();
            let restored = decoder.restore_card_definition(wire, bind_peer).unwrap();
            let restored_effects = match restored.spell_effect.as_ref() {
                Some(program) => program.all_effects(),
                None => program(&restored).effects.all_effects(),
            };
            assert_eq!(template_id(restored_effects[0]), peer[1]);
            assert_ne!(peer[1], original_ids[1]);
            let mut reencoder = StaticAbilityOccurrenceEncoder::default();
            let rewired = reencoder
                .encode_card_definition(restored.clone(), |id| {
                    peer.iter()
                        .position(|candidate| *candidate == id)
                        .map(|index| index as u32)
                        .ok_or_else(|| OccurrenceBindingError::InvalidModel {
                            detail: "unbound restored node".into(),
                        })
                })
                .unwrap();
            assert_eq!(
                serde_json::to_value((reencoder.into_table(), rewired)).unwrap(),
                json
            );
            let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = ironsmith::PlayerId::from_index(0);
            let source =
                game.create_object_from_definition(&restored, alice, ironsmith::Zone::Stack);
            let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
            if restored.spell_effect.is_none() {
                let mut context = ironsmith::costs::CostContext::new(source, alice, &mut dm);
                for cost in program(&restored).mana_cost.costs() {
                    cost.pay(&mut game, &mut context).unwrap();
                }
            }
            let mut context = ironsmith::effects::EffectContext::new_default(source, alice);
            for effect in restored_effects {
                ironsmith::effects::execute_effect(&mut game, effect, &mut context).unwrap();
            }
            assert_eq!(game.battlefield.len(), 1);
            let token = game
                .object(*game.battlefield.iter().next().unwrap())
                .unwrap();
            assert_eq!(token.kind, ironsmith::object::ObjectKind::Token);
            assert_eq!(token.base_power, Some(ironsmith::card::PtValue::Fixed(1)));
            assert_eq!(
                token.base_toughness,
                Some(ironsmith::card::PtValue::Fixed(1))
            );
            assert_eq!(
                game.player(alice).unwrap().life,
                if restored.spell_effect.is_some() {
                    20
                } else {
                    19
                }
            );
        }
    }

    #[test]
    fn retained_definition_codec_nested_graph_failure_rolls_back_model_and_graph_prefixes() {
        let mut definition = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Two template producer"),
            "Type: Sorcery\nCreate a 1/1 white Soldier creature token. Create a 2/2 black Zombie creature token.", false,
        ).unwrap().1;
        let effects = definition.spell_effect.as_ref().unwrap().all_effects();
        assert_eq!(effects.len(), 2);
        let ids = [
            definition.card.id,
            template_id(effects[0]),
            template_id(effects[1]),
        ];
        assert_ne!(ids[1], ids[2]);
        let flying = ironsmith::static_abilities::StaticAbility::flying();
        definition
            .abilities
            .push(ironsmith::ability::Ability::static_ability(flying.clone()));
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        encoder
            .retain(ironsmith::static_abilities::StaticAbility::haste())
            .unwrap();
        let before = serde_json::to_value(encoder.clone().into_table()).unwrap();
        let mut seen = Vec::new();
        assert!(
            encoder
                .encode_card_definition(definition.clone(), |id| {
                    seen.push(id);
                    if id == ids[0] {
                        Ok(0u32)
                    } else if id == ids[1] {
                        Ok(1u32)
                    } else {
                        Err(OccurrenceBindingError::InvalidModel {
                            detail: "late second template graph failure".into(),
                        })
                    }
                })
                .is_err()
        );
        assert!(seen.contains(&ids[1]) && seen.contains(&ids[2]));
        assert_eq!(
            serde_json::to_value(encoder.clone().into_table()).unwrap(),
            before
        );
        assert!(encoder.reference(flying.instance_id()).is_err());
        // A new namespace is valid after rollback; failed binding must not remain cached.
        encoder
            .encode_card_definition(definition, |id| {
                ids.iter()
                    .position(|candidate| *candidate == id)
                    .map(|index| index as u32 + 7)
                    .ok_or_else(|| OccurrenceBindingError::InvalidModel {
                        detail: "unknown retry node".into(),
                    })
            })
            .unwrap();
        assert_eq!(
            encoder.into_table().payload_card_references,
            vec![serde_json::json!(8u32), serde_json::json!(9u32)]
        );
    }

    fn static_template_effect(
        ability: &ironsmith::static_abilities::StaticAbility,
    ) -> ironsmith::effect::Effect {
        let mut effects = Vec::new();
        ability
            .compiled_model()
            .unwrap()
            .clone()
            .try_map(
                Ok::<_, std::convert::Infallible>,
                |effect| {
                    effects.push(effect.clone());
                    Ok(effect)
                },
                Ok,
                Ok,
            )
            .unwrap();
        assert_eq!(effects.len(), 1, "expected one granted token producer");
        effects.pop().unwrap()
    }

    fn static_template_fixture() -> (
        ironsmith::cards::CardDefinition,
        ironsmith::static_abilities::StaticAbility,
        ironsmith::CardId,
    ) {
        let definition = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Template ability grant"),
            "Type: Artifact\nCreatures you control have \"Pay 1 life: Create a 1/1 white Soldier creature token.\".",
            false,
        ).unwrap().1;
        let ability = definition
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                ironsmith::ability::AbilityKind::Static(ability) => Some(ability.clone()),
                _ => None,
            })
            .unwrap();
        let template = template_id(&static_template_effect(&ability));
        (definition, ability, template)
    }

    #[test]
    fn retained_definition_codec_preseeded_static_template_must_bind_owning_graph() {
        let (definition, ability, template) = static_template_fixture();
        let root = definition.card.id;
        assert_ne!(root, template);
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        let reference = encoder.retain(ability.clone()).unwrap();
        let before = serde_json::to_value(encoder.clone().into_table()).unwrap();
        let mut seen = Vec::new();
        let result = encoder.encode_card_definition(definition, |id| {
            seen.push(id);
            if id == root {
                Ok(0u32)
            } else {
                Err(OccurrenceBindingError::InvalidModel {
                    detail: "unbound preseeded template".into(),
                })
            }
        });
        assert!(
            result.is_err(),
            "preseeded static models must not bypass graph binding"
        );
        assert!(seen.contains(&template));
        assert_eq!(
            serde_json::to_value(encoder.clone().into_table()).unwrap(),
            before
        );
        assert_eq!(encoder.reference(ability.instance_id()).unwrap(), reference);
    }

    #[test]
    fn retained_definition_codec_shared_static_templates_restore_and_execute() {
        let (first, grant, template) = static_template_fixture();
        let mut second = first.clone();
        second.card.id = ironsmith::CardId::new();
        second.card.name = "Second template grant".into();
        let ids = [first.card.id, template, second.card.id];
        let bind = |id| {
            ids.iter()
                .position(|candidate| *candidate == id)
                .map(|index| index as u32)
                .ok_or_else(|| OccurrenceBindingError::InvalidModel {
                    detail: "unknown shared graph node".into(),
                })
        };
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        let reference = encoder.retain(grant.clone()).unwrap();
        let first_wire = encoder.encode_card_definition(first, bind).unwrap();
        let second_wire = encoder.encode_card_definition(second, bind).unwrap();
        assert_eq!(encoder.retain(grant).unwrap(), reference);
        let table = encoder.into_table();
        assert_eq!(table.models.len(), 1);
        assert_eq!(
            table.model_card_references,
            vec![RetainedModelCardReferences::Bound]
        );
        assert_eq!(table.payload_card_references, vec![serde_json::json!(1u32)]);
        let json = serde_json::to_value((&table, &first_wire, &second_wire)).unwrap();
        let (table, first_wire, second_wire): (
            RetainedStaticAbilityTable,
            RetainedOccurrenceCardDefinition<u32>,
            RetainedOccurrenceCardDefinition<u32>,
        ) = serde_json::from_value(json.clone()).unwrap();
        let peer = [
            ironsmith::CardId::new(),
            ironsmith::CardId::new(),
            ironsmith::CardId::new(),
        ];
        let receiver = |reference: u32| {
            peer.get(reference as usize).copied().ok_or_else(|| {
                OccurrenceBindingError::InvalidModel {
                    detail: "unknown peer".into(),
                }
            })
        };
        let decoder =
            StaticAbilityOccurrenceDecoder::restore_with_card_graph::<u32>(table, receiver)
                .unwrap();
        let first = decoder
            .restore_card_definition(first_wire, receiver)
            .unwrap();
        let second = decoder
            .restore_card_definition(second_wire, receiver)
            .unwrap();
        let static_of = |definition: &ironsmith::cards::CardDefinition| {
            definition
                .abilities
                .iter()
                .find_map(|ability| match &ability.kind {
                    ironsmith::ability::AbilityKind::Static(ability) => Some(ability.clone()),
                    _ => None,
                })
                .unwrap()
        };
        assert_eq!(
            static_of(&first).instance_id(),
            static_of(&second).instance_id()
        );
        assert_eq!(
            template_id(&static_template_effect(&static_of(&first))),
            peer[1]
        );
        let mut reencoder = StaticAbilityOccurrenceEncoder::default();
        let rebind = |id| {
            peer.iter()
                .position(|candidate| *candidate == id)
                .map(|index| index as u32)
                .ok_or_else(|| OccurrenceBindingError::InvalidModel {
                    detail: "unknown restored node".into(),
                })
        };
        let a = reencoder
            .encode_card_definition(first.clone(), rebind)
            .unwrap();
        let b = reencoder.encode_card_definition(second, rebind).unwrap();
        assert_eq!(
            serde_json::to_value((reencoder.into_table(), a, b)).unwrap(),
            json
        );
        let creature = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Grant recipient"),
            "Type: Creature — Human\nPower/Toughness: 1/1",
            false,
        )
        .unwrap()
        .1;
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = ironsmith::PlayerId::from_index(0);
        game.create_object_from_definition(&first, alice, ironsmith::Zone::Battlefield);
        let source =
            game.create_object_from_definition(&creature, alice, ironsmith::Zone::Battlefield);
        let abilities = game.current_abilities(source).unwrap();
        let activated = abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                ironsmith::ability::AbilityKind::Activated(ability) => Some(ability.clone()),
                _ => None,
            })
            .expect("restored static grant must give the creature its activated ability");
        assert_eq!(template_id(activated.effects.all_effects()[0]), peer[1]);
        let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
        let mut context = ironsmith::costs::CostContext::new(source, alice, &mut dm);
        for cost in activated.mana_cost.costs() {
            cost.pay(&mut game, &mut context).unwrap();
        }
        let mut context = ironsmith::effects::EffectContext::new_default(source, alice);
        for effect in activated.effects.all_effects() {
            ironsmith::effects::execute_effect(&mut game, effect, &mut context).unwrap();
        }
        assert_eq!(game.player(alice).unwrap().life, 19);
        let tokens: Vec<_> = game
            .battlefield
            .iter()
            .filter_map(|id| game.object(*id))
            .filter(|object| object.kind == ironsmith::object::ObjectKind::Token)
            .collect();
        assert_eq!(tokens.len(), 1);
        // A token has no physical card; its producer template is checked above.
        assert_eq!(tokens[0].card, None);
        assert_eq!(
            tokens[0].base_power,
            Some(ironsmith::card::PtValue::Fixed(1))
        );
    }

    #[test]
    fn retained_definition_codec_preseeded_static_prefix_rolls_back_and_retries() {
        let (root, first, first_template) = static_template_fixture();
        let (_, second, second_template) = static_template_fixture();
        assert_ne!(first_template, second_template);
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        let first_ref = encoder.retain(first.clone()).unwrap();
        let second_ref = encoder.retain(second.clone()).unwrap();
        let before = serde_json::to_value(encoder.clone().into_table()).unwrap();
        let mut seen = Vec::new();
        let rejected = encoder.encode_card_definition(root.clone(), |id| {
            seen.push(id);
            if id == root.card.id {
                Ok(0u32)
            } else if id == first_template {
                Ok(1u32)
            } else {
                assert_eq!(id, second_template);
                Err(OccurrenceBindingError::InvalidModel {
                    detail: "late static prefix failure".into(),
                })
            }
        });
        assert!(rejected.is_err());
        assert!(seen.contains(&first_template) && seen.contains(&second_template));
        assert_eq!(
            serde_json::to_value(encoder.clone().into_table()).unwrap(),
            before
        );
        assert_eq!(encoder.reference(first.instance_id()).unwrap(), first_ref);
        assert_eq!(encoder.reference(second.instance_id()).unwrap(), second_ref);
        let ids = [root.card.id, first_template, second_template];
        encoder
            .encode_card_definition(root, |id| {
                ids.iter()
                    .position(|candidate| *candidate == id)
                    .map(|index| index as u32 + 7)
                    .ok_or_else(|| OccurrenceBindingError::InvalidModel {
                        detail: "unknown retry node".into(),
                    })
            })
            .unwrap();
        let table = encoder.into_table();
        assert_eq!(
            table.model_card_references,
            vec![RetainedModelCardReferences::Bound; 2]
        );
        assert_eq!(
            table.payload_card_references,
            vec![serde_json::json!(8u32), serde_json::json!(9u32)]
        );
    }

    #[test]
    fn retained_definition_codec_shared_static_reference_modes_are_required_and_exact() {
        let (_, grant, _) = static_template_fixture();
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        encoder.retain(grant).unwrap();
        let table = encoder.into_table();
        let value = serde_json::to_value(&table).unwrap();
        for invalid in [serde_json::Value::Null, serde_json::json!(["Unknown"])] {
            let mut malformed = value.clone();
            malformed["model_card_references"] = invalid;
            assert!(serde_json::from_value::<RetainedStaticAbilityTable>(malformed).is_err());
        }
        let mut missing = value;
        missing
            .as_object_mut()
            .unwrap()
            .remove("model_card_references");
        assert!(serde_json::from_value::<RetainedStaticAbilityTable>(missing).is_err());
        for modes in [vec![], vec![RetainedModelCardReferences::Bound; 2]] {
            let mut malformed = table.clone();
            malformed.model_card_references = modes;
            let mut calls = 0;
            assert!(
                StaticAbilityOccurrenceDecoder::restore_with_card_graph::<u32>(malformed, |_| {
                    calls += 1;
                    Ok(ironsmith::CardId::new())
                })
                .is_err()
            );
            assert_eq!(
                calls, 0,
                "bad mode counts must fail before receiver binding"
            );
        }
    }

    #[test]
    fn retained_definition_codec_live_object_token_template_requires_owning_graph_binding() {
        let definition = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Live graph producer"),
            "Type: Artifact\nPay 1 life: Create a 1/1 white Soldier creature token.",
            false,
        )
        .unwrap()
        .1;
        let root = definition.card.id;
        let template = template_id(program(&definition).effects.all_effects()[0]);
        assert_ne!(root, template);
        let alice = ironsmith::PlayerId::from_index(0);
        let object = ironsmith::object::Object::from_card_definition(
            ironsmith::ObjectId::from_raw(68771),
            &definition,
            alice,
            ironsmith::Zone::Battlefield,
        );
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source =
            game.create_object_from_definition(&definition, alice, ironsmith::Zone::Battlefield);
        let mut context = ironsmith::effects::EffectContext::new_default(source, alice);
        for effect in program(&definition).effects.all_effects() {
            ironsmith::effects::execute_effect(&mut game, effect, &mut context).unwrap();
        }
        assert_eq!(
            game.battlefield
                .iter()
                .filter_map(|id| game.object(*id))
                .filter(|object| object.kind == ironsmith::object::ObjectKind::Token)
                .count(),
            1
        );
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        let before = serde_json::to_value(encoder.clone().into_table()).unwrap();
        let mut seen = Vec::new();
        let result = encoder.encode_live_object(object, |id| {
            seen.push(id);
            if id == root {
                Ok(0u32)
            } else {
                assert_eq!(id, template);
                Err(OccurrenceBindingError::InvalidModel {
                    detail: "unbound live template".into(),
                })
            }
        });
        assert!(
            result.is_err(),
            "live executable payload templates must pass through the owning graph binder"
        );
        assert!(seen.contains(&template));
        assert_eq!(serde_json::to_value(encoder.into_table()).unwrap(), before);
    }

    #[test]
    fn retained_definition_codec_live_history_and_capture_templates_restore_and_execute() {
        let definition = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Live template carrier"),
            "Type: Artifact\nPay 1 life: Create a 1/1 white Soldier creature token.",
            false,
        )
        .unwrap()
        .1;
        let ids = [
            definition.card.id,
            template_id(program(&definition).effects.all_effects()[0]),
        ];
        let alice = ironsmith::PlayerId::from_index(0);
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source =
            game.create_object_from_definition(&definition, alice, ironsmith::Zone::Battlefield);
        let mut object = game.object(source).unwrap().clone();
        object.spell_effect = Some(program(&definition).effects.clone().into());
        object.splice_cast_state = Some(Box::new(ironsmith::object::SpliceCastState {
            spell_effect: object.spell_effect.clone(),
        }));
        let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object(&object, &game);
        object
            .cast_tagged_objects
            .insert("historical_producer".into(), vec![snapshot.clone()]);
        let capture = ironsmith::object::NativeCastPaymentState::from(&object);
        let bind = |id| {
            ids.iter()
                .position(|candidate| *candidate == id)
                .map(|index| index as u32)
                .ok_or_else(|| OccurrenceBindingError::InvalidModel {
                    detail: "unknown live graph node".into(),
                })
        };
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        let live = encoder.encode_live_object(object, bind).unwrap();
        let history = encoder.encode_snapshot(snapshot, bind).unwrap();
        let capture = encoder.encode_cast_payment_state(capture, bind).unwrap();
        let table = encoder.into_table();
        assert_eq!(table.payload_card_references, vec![serde_json::json!(1u32)]);
        let json = serde_json::to_value((&table, &live, &history, &capture)).unwrap();
        let (table, live, history, capture): (
            RetainedStaticAbilityTable,
            RetainedOccurrenceLiveObject<u32>,
            RetainedOccurrenceObjectSnapshot<u32>,
            RetainedOccurrenceCastPaymentState<u32>,
        ) = serde_json::from_value(json.clone()).unwrap();
        let peer = [ironsmith::CardId::new(), ironsmith::CardId::new()];
        let receiver = |slot: u32| {
            peer.get(slot as usize)
                .copied()
                .ok_or_else(|| OccurrenceBindingError::InvalidModel {
                    detail: "unknown live receiver node".into(),
                })
        };
        let decoder =
            StaticAbilityOccurrenceDecoder::restore_with_card_graph::<u32>(table, receiver)
                .unwrap();
        let restored = decoder.restore_live_object(live, receiver).unwrap();
        let history = decoder.restore_snapshot(history, receiver).unwrap();
        let capture = decoder
            .restore_cast_payment_state(capture, receiver)
            .unwrap();
        assert_eq!(restored.card, Some(peer[0]));
        let mut reencoder = StaticAbilityOccurrenceEncoder::default();
        let rebind = |id| {
            peer.iter()
                .position(|candidate| *candidate == id)
                .map(|index| index as u32)
                .ok_or_else(|| OccurrenceBindingError::InvalidModel {
                    detail: "unknown restored live node".into(),
                })
        };
        let a = reencoder
            .encode_live_object(restored.clone(), rebind)
            .unwrap();
        let b = reencoder.encode_snapshot(history.clone(), rebind).unwrap();
        let c = reencoder
            .encode_cast_payment_state(capture.clone(), rebind)
            .unwrap();
        assert_eq!(
            serde_json::to_value((reencoder.into_table(), a, b, c)).unwrap(),
            json
        );
        let captured = &capture.cast_tagged_objects[0].1[0];
        for abilities in [
            &restored.abilities[..],
            &history.abilities[..],
            &captured.abilities[..],
        ] {
            let activated = abilities
                .iter()
                .find_map(|ability| match &ability.kind {
                    ironsmith::ability::AbilityKind::Activated(ability) => Some(ability),
                    _ => None,
                })
                .unwrap();
            assert_eq!(template_id(activated.effects.all_effects()[0]), peer[1]);
            let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let source = game.create_object_from_definition(
                &definition,
                alice,
                ironsmith::Zone::Battlefield,
            );
            let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
            let mut context = ironsmith::costs::CostContext::new(source, alice, &mut dm);
            for cost in activated.mana_cost.costs() {
                cost.pay(&mut game, &mut context).unwrap();
            }
            let mut context = ironsmith::effects::EffectContext::new_default(source, alice);
            for effect in activated.effects.all_effects() {
                ironsmith::effects::execute_effect(&mut game, effect, &mut context).unwrap();
            }
            assert_eq!(game.player(alice).unwrap().life, 19);
            assert_eq!(
                game.battlefield
                    .iter()
                    .filter_map(|id| game.object(*id))
                    .filter(|object| object.kind == ironsmith::object::ObjectKind::Token)
                    .count(),
                1
            );
        }
        for saved in [
            restored.spell_effect.as_ref().unwrap(),
            restored
                .splice_cast_state
                .as_ref()
                .unwrap()
                .spell_effect
                .as_ref()
                .unwrap(),
        ] {
            assert_eq!(template_id(saved.all_effects()[0]), peer[1]);
            let mut context = ironsmith::effects::EffectContext::new_default(source, alice);
            let before = game.battlefield.len();
            for effect in saved.all_effects() {
                ironsmith::effects::execute_effect(&mut game, effect, &mut context).unwrap();
            }
            assert_eq!(game.battlefield.len(), before + 1);
        }
    }

    #[test]
    fn retained_definition_codec_nested_history_binding_restores_old_static_prefix_on_failure() {
        let (definition, grant, template) = static_template_fixture();
        let alice = ironsmith::PlayerId::from_index(0);
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source =
            game.create_object_from_definition(&definition, alice, ironsmith::Zone::Battlefield);
        let object = game.object(source).unwrap().clone();
        let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object(&object, &game);
        let mut unknown = snapshot.clone();
        let missing = ironsmith::CardId::new();
        unknown.card = Some(missing);
        let mut capture = ironsmith::object::NativeCastPaymentState::from(&object);
        capture.cast_tagged_objects = vec![
            ("first".into(), vec![snapshot]),
            ("later".into(), vec![unknown]),
        ];
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        let reference = encoder.retain(grant.clone()).unwrap();
        let before = serde_json::to_value(encoder.clone().into_table()).unwrap();
        let mut seen = Vec::new();
        let failed = encoder.encode_cast_payment_state(capture.clone(), |id| {
            seen.push(id);
            if id == definition.card.id {
                Ok(0u32)
            } else if id == template {
                Ok(1u32)
            } else {
                assert_eq!(id, missing);
                Err(OccurrenceBindingError::InvalidModel {
                    detail: "later history failed".into(),
                })
            }
        });
        assert!(failed.is_err());
        assert!(seen.contains(&template) && seen.contains(&missing));
        assert_eq!(
            serde_json::to_value(encoder.clone().into_table()).unwrap(),
            before
        );
        assert_eq!(encoder.reference(grant.instance_id()).unwrap(), reference);
        encoder
            .encode_cast_payment_state(capture, |id| {
                if id == definition.card.id {
                    Ok(7u32)
                } else if id == template {
                    Ok(8u32)
                } else {
                    assert_eq!(id, missing);
                    Ok(9u32)
                }
            })
            .unwrap();
        let table = encoder.into_table();
        assert_eq!(
            table.model_card_references,
            vec![RetainedModelCardReferences::Bound]
        );
        assert_eq!(table.payload_card_references, vec![serde_json::json!(8u32)]);
    }

    fn assert_registered_template_requires_graph(generic: bool) {
        let (definition, grant, template) = static_template_fixture();
        let root = definition.card.id;
        let alice = ironsmith::PlayerId::from_index(0);
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source =
            game.create_object_from_definition(&definition, alice, ironsmith::Zone::Battlefield);
        let modification = if generic {
            let definition = compile_builder_to_artifact(
                compiler::CardDefinitionBuilder::new(
                    ironsmith::CardId::new(),
                    "Registered inline producer",
                ),
                "Type: Artifact\nPay 1 life: Create a 1/1 white Soldier creature token.",
                false,
            )
            .unwrap()
            .1;
            ironsmith::continuous::Modification::AddAbilityGeneric(
                definition
                    .abilities
                    .iter()
                    .find(|ability| {
                        matches!(&ability.kind, ironsmith::ability::AbilityKind::Activated(_))
                    })
                    .unwrap()
                    .clone(),
            )
        } else {
            ironsmith::continuous::Modification::AddAbility(grant.clone())
        };
        let executable = match &modification {
            ironsmith::continuous::Modification::AddAbilityGeneric(ability) => {
                let ironsmith::ability::AbilityKind::Activated(activated) = &ability.kind else {
                    panic!("activated producer")
                };
                activated.effects.all_effects()[0].clone()
            }
            _ => static_template_effect(&grant),
        };
        let expected_template = template_id(&executable);
        if !generic {
            assert_eq!(expected_template, template);
        }
        let mut context = ironsmith::effects::EffectContext::new_default(source, alice);
        ironsmith::effects::execute_effect(&mut game, &executable, &mut context).unwrap();
        assert_eq!(
            game.battlefield
                .iter()
                .filter_map(|id| game.object(*id))
                .filter(|object| object.kind == ironsmith::object::ObjectKind::Token)
                .count(),
            1
        );
        let mut effect = ironsmith::continuous::ContinuousEffect::new(
            source,
            alice,
            ironsmith::continuous::EffectTarget::Specific(source),
            modification,
        );
        effect.originating_ability =
            Some(Box::new(ironsmith::continuous::ContinuousAbilityOrigin {
                host: source,
                printed_face: Some(root),
                branch: 0,
                ability: ironsmith::continuous::AbilityOrigin::Printed(0),
            }));
        game.effect_store.continuous_effects.add_effect(effect);
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        let before = serde_json::to_value(encoder.clone().into_table()).unwrap();
        let mut seen = Vec::new();
        let result = encoder.encode_registered_state_with_card_graph(
            game.effect_store.continuous_effects.registered_state(),
            |id| {
                seen.push(id);
                if id == root {
                    Ok(0u32)
                } else {
                    assert_eq!(id, expected_template);
                    Err(OccurrenceBindingError::InvalidModel {
                        detail: "unbound registered executable template".into(),
                    })
                }
            },
        );
        assert!(
            result.is_err(),
            "registered executable templates must pass through the owning graph binder; generic={generic}"
        );
        assert!(seen.contains(&expected_template));
        assert_eq!(serde_json::to_value(encoder.into_table()).unwrap(), before);
    }

    #[test]
    fn retained_definition_codec_registered_static_template_requires_owning_graph_binding() {
        assert_registered_template_requires_graph(false);
    }

    #[test]
    fn retained_definition_codec_registered_inline_template_requires_owning_graph_binding() {
        assert_registered_template_requires_graph(true);
    }

    #[test]
    fn retained_definition_codec_registered_templates_restore_and_grant_executable_abilities() {
        for generic in [false, true] {
            let (definition, grant, _) = static_template_fixture();
            let root = definition.card.id;
            let alice = ironsmith::PlayerId::from_index(0);
            let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let source = game.create_object_from_definition(
                &definition,
                alice,
                ironsmith::Zone::Battlefield,
            );
            game.object_mut(source).unwrap().abilities_mut().clear();
            let creature = compile_builder_to_artifact(
                compiler::CardDefinitionBuilder::new(
                    ironsmith::CardId::new(),
                    "Registered grant recipient",
                ),
                "Type: Creature — Human\nPower/Toughness: 1/1",
                false,
            )
            .unwrap()
            .1;
            let recipient =
                game.create_object_from_definition(&creature, alice, ironsmith::Zone::Battlefield);
            let (modification, expected_template) = if generic {
                let definition = compile_builder_to_artifact(
                    compiler::CardDefinitionBuilder::new(
                        ironsmith::CardId::new(),
                        "Registered generic program",
                    ),
                    "Type: Artifact\nPay 1 life: Create a 1/1 white Soldier creature token.",
                    false,
                )
                .unwrap()
                .1;
                (
                    ironsmith::continuous::Modification::AddAbilityGeneric(
                        definition
                            .abilities
                            .iter()
                            .find(|ability| {
                                matches!(
                                    &ability.kind,
                                    ironsmith::ability::AbilityKind::Activated(_)
                                )
                            })
                            .unwrap()
                            .clone(),
                    ),
                    template_id(program(&definition).effects.all_effects()[0]),
                )
            } else {
                (
                    ironsmith::continuous::Modification::AddAbility(grant.clone()),
                    template_id(&static_template_effect(&grant)),
                )
            };
            let target = if generic { recipient } else { source };
            let mut effect = ironsmith::continuous::ContinuousEffect::new(
                source,
                alice,
                ironsmith::continuous::EffectTarget::Specific(target),
                modification,
            );
            effect.originating_ability =
                Some(Box::new(ironsmith::continuous::ContinuousAbilityOrigin {
                    host: source,
                    printed_face: Some(root),
                    branch: 0,
                    ability: ironsmith::continuous::AbilityOrigin::Printed(0),
                }));
            game.effect_store.continuous_effects.add_effect(effect);
            let ids = [root, expected_template];
            let mut encoder = StaticAbilityOccurrenceEncoder::default();
            let wire = encoder
                .encode_registered_state_with_card_graph(
                    game.effect_store.continuous_effects.registered_state(),
                    |id| {
                        ids.iter()
                            .position(|candidate| *candidate == id)
                            .map(|index| index as u32)
                            .ok_or_else(|| OccurrenceBindingError::InvalidModel {
                                detail: "unknown registered graph node".into(),
                            })
                    },
                )
                .unwrap();
            let table = encoder.into_table();
            assert_eq!(table.payload_card_references, vec![serde_json::json!(1u32)]);
            assert!(
                table
                    .model_card_references
                    .iter()
                    .all(|mode| *mode == RetainedModelCardReferences::Bound)
            );
            let json = serde_json::to_value((&table, &wire)).unwrap();
            let (table, wire): (
                RetainedStaticAbilityTable,
                RetainedGraphRegisteredState<u32>,
            ) = serde_json::from_value(json.clone()).unwrap();
            let peer = [ironsmith::CardId::new(), ironsmith::CardId::new()];
            let receiver = |slot: u32| {
                peer.get(slot as usize).copied().ok_or_else(|| {
                    OccurrenceBindingError::InvalidModel {
                        detail: "unknown registered receiver node".into(),
                    }
                })
            };
            let decoder =
                StaticAbilityOccurrenceDecoder::restore_with_card_graph::<u32>(table, receiver)
                    .unwrap();
            let restored = decoder
                .restore_registered_state_with_card_graph(wire, receiver)
                .unwrap();
            assert_eq!(
                restored.effects[0]
                    .originating_ability
                    .as_ref()
                    .unwrap()
                    .printed_face,
                Some(peer[0])
            );
            let mut reencoder = StaticAbilityOccurrenceEncoder::default();
            let reencoded = reencoder
                .encode_registered_state_with_card_graph(restored.clone(), |id| {
                    peer.iter()
                        .position(|candidate| *candidate == id)
                        .map(|index| index as u32)
                        .ok_or_else(|| OccurrenceBindingError::InvalidModel {
                            detail: "unknown restored descriptor node".into(),
                        })
                })
                .unwrap();
            assert_eq!(
                serde_json::to_value((reencoder.into_table(), reencoded)).unwrap(),
                json
            );
            game.object_mut(source).unwrap().card = Some(peer[0]);
            game.effect_store
                .continuous_effects
                .restore_registered_state(restored)
                .unwrap();
            game.refresh_continuous_state().unwrap();
            let abilities = game.current_abilities(recipient).unwrap();
            let activated = abilities
                .iter()
                .find_map(|ability| match &ability.kind {
                    ironsmith::ability::AbilityKind::Activated(ability) => Some(ability),
                    _ => None,
                })
                .expect(
                    "restored registered descriptor must grant the recipient an executable ability",
                );
            assert_eq!(template_id(activated.effects.all_effects()[0]), peer[1]);
            let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
            let mut context = ironsmith::costs::CostContext::new(recipient, alice, &mut dm);
            for cost in activated.mana_cost.costs() {
                cost.pay(&mut game, &mut context).unwrap();
            }
            let mut context = ironsmith::effects::EffectContext::new_default(recipient, alice);
            for effect in activated.effects.all_effects() {
                ironsmith::effects::execute_effect(&mut game, effect, &mut context).unwrap();
            }
            assert_eq!(game.player(alice).unwrap().life, 19);
            assert_eq!(
                game.battlefield
                    .iter()
                    .filter_map(|id| game.object(*id))
                    .filter(|object| object.kind == ironsmith::object::ObjectKind::Token)
                    .count(),
                1
            );
        }
    }

    #[test]
    fn retained_definition_codec_late_native_inline_id_cannot_alias_payload_slot() {
        let owner = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(
                ironsmith::CardId::new(),
                "Existing framed producer",
            ),
            "Type: Sorcery\nCreate a 1/1 white Soldier creature token.",
            false,
        )
        .unwrap()
        .1;
        let ids = [
            owner.card.id,
            template_id(owner.spell_effect.as_ref().unwrap().all_effects()[0]),
        ];
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        encoder
            .encode_card_definition(owner, |id| {
                ids.iter()
                    .position(|candidate| *candidate == id)
                    .map(|index| index as u32)
                    .ok_or_else(|| OccurrenceBindingError::InvalidModel {
                        detail: "unknown initial graph node".into(),
                    })
            })
            .unwrap();
        let definition = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(
                ironsmith::CardId::new(),
                "Late native inline producer",
            ),
            "Type: Artifact\nPay 1 life: Create a 1/1 white Soldier creature token.",
            false,
        )
        .unwrap()
        .1;
        let mut activated = program(&definition).clone();
        activated.effects = activated
            .effects
            .try_map_effects(|effect| {
                Ok::<_, std::convert::Infallible>(replace_token_template_id(
                    effect,
                    ironsmith::CardId::from_raw(0),
                ))
            })
            .unwrap();
        assert_eq!(
            template_id(activated.effects.all_effects()[0]),
            ironsmith::CardId::from_raw(0)
        );
        let mut late = definition.abilities.iter().find(|ability| matches!(
            &ability.kind, ironsmith::ability::AbilityKind::Activated(_)
        )).unwrap().clone();
        late.kind = ironsmith::ability::AbilityKind::Activated(activated);
        let raw = encoder.encode_ability(late).unwrap();
        let table = encoder.into_table();
        assert_eq!(table.payload_card_references, vec![serde_json::json!(1u32)]);
        let peer = ironsmith::CardId::new();
        let decoder =
            StaticAbilityOccurrenceDecoder::restore_with_card_graph::<u32>(table, |slot| {
                assert_eq!(slot, 1);
                Ok(peer)
            })
            .unwrap();
        let result = decoder.restore_ability(raw);
        assert!(
            result.is_err(),
            "standalone native inline CardId zero must not alias framed slot zero"
        );
    }

    #[test]
    fn retained_definition_codec_inline_reference_modes_bind_explicitly_and_execute() {
        let definition = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Explicit inline graph"),
            "Type: Artifact\nPay 1 life: Create a 1/1 white Soldier creature token.",
            false,
        )
        .unwrap()
        .1;
        let mut native = definition
            .abilities
            .iter()
            .find(|ability| matches!(&ability.kind, ironsmith::ability::AbilityKind::Activated(_)))
            .unwrap()
            .clone();
        let ironsmith::ability::AbilityKind::Activated(activated) = &mut native.kind else {
            panic!("activated")
        };
        activated.effects = activated
            .effects
            .clone()
            .try_map_effects(|effect| {
                Ok::<_, std::convert::Infallible>(replace_token_template_id(
                    effect,
                    ironsmith::CardId::from_raw(0),
                ))
            })
            .unwrap();
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        let static_ref = encoder
            .retain(ironsmith::static_abilities::StaticAbility::haste())
            .unwrap();
        let before = serde_json::to_value(encoder.clone().into_table()).unwrap();
        assert!(
            encoder
                .encode_ability_with_card_graph(native.clone(), |id| {
                    assert_eq!(id, ironsmith::CardId::from_raw(0));
                    Err::<u32, _>(OccurrenceBindingError::InvalidModel {
                        detail: "missing standalone owner".into(),
                    })
                })
                .is_err()
        );
        assert_eq!(
            serde_json::to_value(encoder.clone().into_table()).unwrap(),
            before
        );
        let raw = encoder.encode_ability(native.clone()).unwrap();
        assert_eq!(raw.card_references, RetainedModelCardReferences::Native);
        let wire = encoder
            .encode_ability_with_card_graph(native, |id| {
                assert_eq!(id, ironsmith::CardId::from_raw(0));
                Ok(7u32)
            })
            .unwrap();
        assert_eq!(wire.card_references, RetainedModelCardReferences::Bound);
        let table = encoder.into_table();
        assert_eq!(table.payload_card_references, vec![serde_json::json!(7u32)]);
        let json = serde_json::to_value((&table, &wire)).unwrap();
        let (table, wire): (RetainedStaticAbilityTable, RetainedOccurrenceAbility) =
            serde_json::from_value(json.clone()).unwrap();
        let peer = ironsmith::CardId::new();
        let decoder =
            StaticAbilityOccurrenceDecoder::restore_with_card_graph::<u32>(table, |owner| {
                assert_eq!(owner, 7);
                Ok(peer)
            })
            .unwrap();
        assert!(decoder.restore_ability(raw).is_err());
        let value = serde_json::to_value(&wire).unwrap();
        assert_eq!(value.as_object().unwrap().len(), 3);
        for field in ["card_references", "model", "embedded_definitions"] {
            let mut missing = value.clone();
            missing.as_object_mut().unwrap().remove(field);
            assert!(serde_json::from_value::<RetainedOccurrenceAbility>(missing).is_err());
        }
        for mode in [serde_json::Value::Null, serde_json::json!("Unknown")] {
            let mut invalid = value.clone();
            invalid["card_references"] = mode;
            assert!(serde_json::from_value::<RetainedOccurrenceAbility>(invalid).is_err());
        }
        let mut unbound = wire.clone();
        unbound.card_references = RetainedModelCardReferences::Native;
        assert!(decoder.restore_ability(unbound).is_err());
        let restored = decoder.restore_ability(wire).unwrap();
        let ironsmith::ability::AbilityKind::Activated(activated) = &restored.kind else {
            panic!("activated")
        };
        assert_eq!(template_id(activated.effects.all_effects()[0]), peer);
        let mut reencoder = StaticAbilityOccurrenceEncoder::default();
        assert_eq!(
            reencoder
                .retain(decoder.ability(static_ref).unwrap())
                .unwrap(),
            static_ref
        );
        let reencoded = reencoder
            .encode_ability_with_card_graph(restored.clone(), |id| {
                assert_eq!(id, peer);
                Ok(7u32)
            })
            .unwrap();
        assert_eq!(
            serde_json::to_value((reencoder.into_table(), reencoded)).unwrap(),
            json
        );
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = ironsmith::PlayerId::from_index(0);
        let source =
            game.create_object_from_definition(&definition, alice, ironsmith::Zone::Battlefield);
        let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
        let mut context = ironsmith::costs::CostContext::new(source, alice, &mut dm);
        for cost in activated.mana_cost.costs() {
            cost.pay(&mut game, &mut context).unwrap();
        }
        let mut context = ironsmith::effects::EffectContext::new_default(source, alice);
        for effect in activated.effects.all_effects() {
            ironsmith::effects::execute_effect(&mut game, effect, &mut context).unwrap();
        }
        assert_eq!(game.player(alice).unwrap().life, 19);
        assert_eq!(
            game.battlefield
                .iter()
                .filter_map(|id| game.object(*id))
                .filter(|object| object.kind == ironsmith::object::ObjectKind::Token)
                .count(),
            1
        );
    }

    #[test]
    fn retained_definition_codec_standalone_saved_program_requires_owning_graph_binding() {
        let owner = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(
                ironsmith::CardId::new(),
                "Existing saved program graph",
            ),
            "Type: Sorcery\nCreate a 1/1 white Soldier creature token.",
            false,
        )
        .unwrap()
        .1;
        let ids = [
            owner.card.id,
            template_id(owner.spell_effect.as_ref().unwrap().all_effects()[0]),
        ];
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        encoder
            .encode_card_definition(owner, |id| {
                ids.iter()
                    .position(|candidate| *candidate == id)
                    .map(|index| index as u32)
                    .ok_or_else(|| OccurrenceBindingError::InvalidModel {
                        detail: "unknown saved root".into(),
                    })
            })
            .unwrap();
        let definition = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(
                ironsmith::CardId::new(),
                "Standalone saved producer",
            ),
            "Type: Artifact\nPay 1 life: Create a 1/1 white Soldier creature token.",
            false,
        )
        .unwrap()
        .1;
        let saved = program(&definition)
            .effects
            .clone()
            .try_map_effects(|effect| {
                Ok::<_, std::convert::Infallible>(replace_token_template_id(
                    effect,
                    ironsmith::CardId::from_raw(0),
                ))
            })
            .unwrap();
        assert_eq!(
            template_id(saved.all_effects()[0]),
            ironsmith::CardId::from_raw(0)
        );
        let wire = encoder
            .encode_splice_state(ironsmith::object::SpliceCastState {
                spell_effect: Some(saved.into()),
            })
            .unwrap();
        let table = encoder.into_table();
        assert_eq!(table.payload_card_references, vec![serde_json::json!(1u32)]);
        let peer = ironsmith::CardId::new();
        let decoder =
            StaticAbilityOccurrenceDecoder::restore_with_card_graph::<u32>(table, |reference| {
                assert_eq!(reference, 1);
                Ok(peer)
            })
            .unwrap();
        let result = decoder.restore_splice_state(wire);
        if let Ok(restored) = &result {
            assert_eq!(
                template_id(restored.spell_effect.as_ref().unwrap().all_effects()[0]),
                ironsmith::CardId::from_raw(0)
            );
            assert_ne!(peer, ironsmith::CardId::from_raw(0));
        }
        assert!(
            result.is_err(),
            "standalone saved executable native references must not bypass the owning graph"
        );
    }

    #[test]
    fn retained_definition_codec_saved_program_modes_bind_and_execute_after_splice_ends() {
        let definition = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(
                ironsmith::CardId::new(),
                "Saved template program",
            ),
            "Type: Artifact\nPay 1 life: Create a 1/1 white Soldier creature token.",
            false,
        )
        .unwrap()
        .1;
        let saved = program(&definition)
            .effects
            .clone()
            .try_map_effects(|effect| {
                Ok::<_, std::convert::Infallible>(replace_token_template_id(
                    effect,
                    ironsmith::CardId::from_raw(0),
                ))
            })
            .unwrap();
        let modified = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(
                ironsmith::CardId::new(),
                "Modified current program",
            ),
            "Type: Sorcery\nYou gain 2 life.",
            false,
        )
        .unwrap()
        .1
        .spell_effect
        .unwrap();
        let alice = ironsmith::PlayerId::from_index(0);
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source =
            game.create_object_from_definition(&definition, alice, ironsmith::Zone::Battlefield);
        let object = game.object_mut(source).unwrap();
        object.spell_effect = Some(saved.clone().into());
        assert!(object.begin_splice_cast_overlay());
        object.spell_effect = Some(modified.into());
        let state = object.splice_cast_state.as_deref().unwrap().clone();
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        let static_ref = encoder
            .retain(ironsmith::static_abilities::StaticAbility::haste())
            .unwrap();
        let before = serde_json::to_value(encoder.clone().into_table()).unwrap();
        assert!(
            encoder
                .encode_splice_state_with_card_graph(state.clone(), |id| {
                    assert_eq!(id, ironsmith::CardId::from_raw(0));
                    Err::<u32, _>(OccurrenceBindingError::InvalidModel {
                        detail: "missing saved template owner".into(),
                    })
                })
                .is_err()
        );
        assert_eq!(
            serde_json::to_value(encoder.clone().into_table()).unwrap(),
            before
        );
        let raw = encoder.encode_splice_state(state.clone()).unwrap();
        assert_eq!(
            raw.spell_effect.as_ref().unwrap().card_references,
            RetainedModelCardReferences::Native
        );
        let bind = |id| {
            assert_eq!(id, ironsmith::CardId::from_raw(0));
            Ok(9u32)
        };
        let wire = encoder
            .encode_splice_state_with_card_graph(state, bind)
            .unwrap();
        let direct = encoder.encode_program_with_card_graph(saved, bind).unwrap();
        assert_eq!(
            wire.spell_effect.as_ref().unwrap().card_references,
            RetainedModelCardReferences::Bound
        );
        assert_eq!(direct.card_references, RetainedModelCardReferences::Bound);
        let table = encoder.into_table();
        assert_eq!(table.payload_card_references, vec![serde_json::json!(9u32)]);
        let json = serde_json::to_value((&table, &wire, &direct)).unwrap();
        let (table, wire, direct): (
            RetainedStaticAbilityTable,
            RetainedOccurrenceSpliceCastState,
            RetainedOccurrenceProgram,
        ) = serde_json::from_value(json.clone()).unwrap();
        let peer = ironsmith::CardId::new();
        let decoder =
            StaticAbilityOccurrenceDecoder::restore_with_card_graph::<u32>(table, |owner| {
                assert_eq!(owner, 9);
                Ok(peer)
            })
            .unwrap();
        assert!(decoder.restore_splice_state(raw).is_err());
        let program_json = serde_json::to_value(&direct).unwrap();
        assert_eq!(program_json.as_object().unwrap().len(), 3);
        for field in ["card_references", "model", "embedded_definitions"] {
            let mut missing = program_json.clone();
            missing.as_object_mut().unwrap().remove(field);
            assert!(serde_json::from_value::<RetainedOccurrenceProgram>(missing).is_err());
        }
        for mode in [serde_json::Value::Null, serde_json::json!("Unknown")] {
            let mut invalid = program_json.clone();
            invalid["card_references"] = mode;
            assert!(serde_json::from_value::<RetainedOccurrenceProgram>(invalid).is_err());
        }
        let mut unbound = direct.clone();
        unbound.card_references = RetainedModelCardReferences::Native;
        assert!(decoder.restore_program(unbound).is_err());
        let restored = decoder.restore_splice_state(wire).unwrap();
        let direct = decoder.restore_program(direct).unwrap();
        assert_eq!(
            template_id(restored.spell_effect.as_ref().unwrap().all_effects()[0]),
            peer
        );
        assert_eq!(template_id(direct.all_effects()[0]), peer);
        let mut reencoder = StaticAbilityOccurrenceEncoder::default();
        assert_eq!(
            reencoder
                .retain(decoder.ability(static_ref).unwrap())
                .unwrap(),
            static_ref
        );
        let rebind = |id| {
            assert_eq!(id, peer);
            Ok(9u32)
        };
        let a = reencoder
            .encode_splice_state_with_card_graph(restored.clone(), rebind)
            .unwrap();
        let b = reencoder
            .encode_program_with_card_graph(direct.clone(), rebind)
            .unwrap();
        assert_eq!(
            serde_json::to_value((reencoder.into_table(), a, b)).unwrap(),
            json
        );
        let mut context = ironsmith::effects::EffectContext::new_default(source, alice);
        let current = game
            .object(source)
            .unwrap()
            .spell_effect
            .as_ref()
            .unwrap()
            .clone();
        for effect in current.all_effects() {
            ironsmith::effects::execute_effect(&mut game, effect, &mut context).unwrap();
        }
        assert_eq!(game.player(alice).unwrap().life, 22);
        game.object_mut(source).unwrap().splice_cast_state = Some(Box::new(restored));
        assert!(game.object_mut(source).unwrap().end_splice_cast_overlay());
        let restored_current = game
            .object(source)
            .unwrap()
            .spell_effect
            .as_ref()
            .unwrap()
            .clone();
        assert_eq!(template_id(restored_current.all_effects()[0]), peer);
        for effect in restored_current.all_effects() {
            ironsmith::effects::execute_effect(&mut game, effect, &mut context).unwrap();
        }
        for effect in direct.all_effects() {
            ironsmith::effects::execute_effect(&mut game, effect, &mut context).unwrap();
        }
        assert_eq!(game.player(alice).unwrap().life, 22);
        let tokens: Vec<_> = game
            .battlefield
            .iter()
            .filter_map(|id| game.object(*id))
            .filter(|object| object.kind == ironsmith::object::ObjectKind::Token)
            .collect();
        assert_eq!(tokens.len(), 2);
        assert!(
            tokens
                .iter()
                .all(|token| token.base_power == Some(ironsmith::card::PtValue::Fixed(1)))
        );
    }

    #[test]
    fn retained_definition_codec_unbound_captured_alternative_cannot_alias_payload_slot() {
        let owner = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(
                ironsmith::CardId::new(),
                "Captured namespace owner",
            ),
            "Type: Sorcery\nCreate a 1/1 white Soldier creature token.",
            false,
        )
        .unwrap()
        .1;
        let ids = [
            owner.card.id,
            template_id(owner.spell_effect.as_ref().unwrap().all_effects()[0]),
        ];
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        encoder
            .encode_card_definition(owner, |id| {
                ids.iter()
                    .position(|candidate| *candidate == id)
                    .map(|index| index as u32)
                    .ok_or_else(|| OccurrenceBindingError::InvalidModel {
                        detail: "unknown captured owner node".into(),
                    })
            })
            .unwrap();
        let definition = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(
                ironsmith::CardId::new(),
                "Captured inline producer",
            ),
            "Type: Artifact\nPay 1 life: Create a 1/1 white Soldier creature token.",
            false,
        )
        .unwrap()
        .1;
        let native = replace_token_template_id(
            program(&definition).effects.all_effects()[0].clone(),
            ironsmith::CardId::from_raw(0),
        );
        assert_eq!(template_id(&native), ironsmith::CardId::from_raw(0));
        let method = ironsmith::alternative_cast::AlternativeCastingMethod::Overload {
            cost: ironsmith::mana::ManaCost::from_pips(vec![vec![
                ironsmith::mana::ManaSymbol::Blue,
            ]]),
            effects: vec![native],
        };
        let object = ironsmith::object::Object::from_card_definition(
            ironsmith::ObjectId::from_raw(69771),
            &definition,
            ironsmith::PlayerId::from_index(0),
            ironsmith::Zone::Stack,
        );
        let mut wire = encoder
            .encode_cast_payment_state(
                ironsmith::object::NativeCastPaymentState::from(&object),
                |_| {
                    Err::<u32, _>(OccurrenceBindingError::InvalidModel {
                        detail: "unexpected capture reference".into(),
                    })
                },
            )
            .unwrap();
        wire.alternative_casts.push(RetainedCardPayload {
            card_references: RetainedModelCardReferences::Native,
            model: encode_runtime_alternative_cast(method).unwrap(),
            embedded_definitions: Vec::new(),
        });
        let table = encoder.into_table();
        assert_eq!(table.payload_card_references, vec![serde_json::json!(1u32)]);
        let peer = ironsmith::CardId::new();
        let decoder =
            StaticAbilityOccurrenceDecoder::restore_with_card_graph::<u32>(table, |slot| {
                assert_eq!(slot, 1);
                Ok(peer)
            })
            .unwrap();
        let result = decoder.restore_cast_payment_state(wire, |_| {
            Err::<ironsmith::CardId, _>(OccurrenceBindingError::InvalidModel {
                detail: "unexpected receiver capture reference".into(),
            })
        });
        if let Ok(state) = &result {
            let ironsmith::alternative_cast::AlternativeCastingMethod::Overload { effects, .. } =
                &state.alternative_casts[0]
            else {
                panic!("retained overload")
            };
            assert_eq!(template_id(&effects[0]), peer);
            assert_ne!(peer, ironsmith::CardId::from_raw(0));
        }
        assert!(
            result.is_err(),
            "unbound captured alternative native CardId zero must not alias payload slot zero"
        );
    }

    #[test]
    fn retained_definition_codec_captured_cost_modes_bind_roundtrip_pay_and_execute() {
        let owner = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Existing cost frame"),
            "Type: Sorcery\nCreate a 1/1 white Soldier creature token.",
            false,
        )
        .unwrap()
        .1;
        let mut definition = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(
                ironsmith::CardId::new(),
                "Captured cost producer",
            ),
            "Type: Artifact\nPay 1 life: Create a 1/1 white Soldier creature token.",
            false,
        )
        .unwrap()
        .1;
        let native_effect = replace_token_template_id(
            program(&definition).effects.all_effects()[0].clone(),
            ironsmith::CardId::from_raw(0),
        );
        let method = ironsmith::alternative_cast::AlternativeCastingMethod::Overload {
            cost: ironsmith::mana::ManaCost::new(),
            effects: vec![native_effect],
        };
        let total = ironsmith::cost::TotalCost::from_cost(
            program(&definition).mana_cost.costs()[0].clone(),
        );
        let optional = ironsmith::cost::OptionalCost::buyback(total.clone()).repeatable();
        let ids = [
            owner.card.id,
            template_id(owner.spell_effect.as_ref().unwrap().all_effects()[0]),
            ironsmith::CardId::from_raw(0),
            definition.card.id,
            template_id(program(&definition).effects.all_effects()[0]),
        ];
        assert!(
            ids.iter()
                .enumerate()
                .all(|(index, id)| !ids[..index].contains(id))
        );
        let bind = |id| {
            ids.iter()
                .position(|candidate| *candidate == id)
                .map(|index| index as u32)
                .ok_or_else(|| OccurrenceBindingError::InvalidModel {
                    detail: "unknown cost graph node".into(),
                })
        };
        let alice = ironsmith::PlayerId::from_index(0);
        let object = ironsmith::object::Object::from_card_definition(
            ironsmith::ObjectId::from_raw(70771),
            &definition,
            alice,
            ironsmith::Zone::Stack,
        );
        let mut capture = ironsmith::object::NativeCastPaymentState::from(&object);
        capture.alternative_casts = vec![method.clone()];
        capture.cast_alternative_method = Some(method.clone());
        capture.optional_costs = vec![optional.clone()];
        capture.additional_cost = total.clone();
        definition.alternative_casts = vec![method.clone()];
        definition.optional_costs = vec![optional.clone()];
        definition.additional_cost = total.clone();
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        encoder.encode_card_definition(owner, bind).unwrap();
        let before = serde_json::to_value(encoder.clone().into_table()).unwrap();
        assert!(
            encoder
                .encode_cast_payment_state(capture.clone(), |id| {
                    assert_eq!(id, ironsmith::CardId::from_raw(0));
                    Err::<u32, _>(OccurrenceBindingError::InvalidModel {
                        detail: "missing captured template owner".into(),
                    })
                })
                .is_err()
        );
        assert_eq!(
            serde_json::to_value(encoder.clone().into_table()).unwrap(),
            before
        );
        let wire = encoder.encode_cast_payment_state(capture, bind).unwrap();
        let root = encoder
            .encode_card_definition(definition.clone(), bind)
            .unwrap();
        let direct_method = encoder
            .encode_alternative_cast_with_card_graph(method, bind)
            .unwrap();
        let direct_optional = encoder
            .encode_optional_cost_with_card_graph(optional, bind)
            .unwrap();
        let direct_total = encoder
            .encode_total_cost_with_card_graph(total, bind)
            .unwrap();
        assert_eq!(
            serde_json::to_value(&direct_method).unwrap(),
            serde_json::to_value(&wire.alternative_casts[0]).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&direct_optional).unwrap(),
            serde_json::to_value(&wire.optional_costs[0]).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&direct_total).unwrap(),
            serde_json::to_value(&wire.additional_cost).unwrap()
        );
        let table = encoder.into_table();
        assert_eq!(
            table.payload_card_references,
            vec![
                serde_json::json!(1u32),
                serde_json::json!(2u32),
                serde_json::json!(4u32)
            ]
        );
        let json = serde_json::to_value((&table, &wire, &root)).unwrap();
        let (table, wire, root): (
            RetainedStaticAbilityTable,
            RetainedOccurrenceCastPaymentState<u32>,
            RetainedOccurrenceCardDefinition<u32>,
        ) = serde_json::from_value(json.clone()).unwrap();
        let peer = [
            ironsmith::CardId::new(),
            ironsmith::CardId::new(),
            ironsmith::CardId::new(),
            ironsmith::CardId::new(),
            ironsmith::CardId::new(),
        ];
        let receiver = |slot: u32| {
            peer.get(slot as usize)
                .copied()
                .ok_or_else(|| OccurrenceBindingError::InvalidModel {
                    detail: "unknown cost receiver node".into(),
                })
        };
        let decoder =
            StaticAbilityOccurrenceDecoder::restore_with_card_graph::<u32>(table, receiver)
                .unwrap();
        for value in [
            serde_json::to_value(&direct_method).unwrap(),
            serde_json::to_value(&direct_optional).unwrap(),
            serde_json::to_value(&direct_total).unwrap(),
        ] {
            assert_eq!(value["card_references"], serde_json::json!("Bound"));
            assert_eq!(value.as_object().unwrap().len(), 3);
        }
        let mut raw = direct_method.clone();
        raw.card_references = RetainedModelCardReferences::Native;
        assert!(decoder.restore_alternative_cast(raw).is_err());
        for mode in [serde_json::Value::Null, serde_json::json!("Unknown")] {
            let mut invalid = serde_json::to_value(&direct_method).unwrap();
            invalid["card_references"] = mode;
            assert!(serde_json::from_value::<RetainedOccurrenceAlternativeCast>(invalid).is_err());
        }
        for field in ["card_references", "model", "embedded_definitions"] {
            let mut missing = serde_json::to_value(&direct_method).unwrap();
            missing.as_object_mut().unwrap().remove(field);
            assert!(serde_json::from_value::<RetainedOccurrenceAlternativeCast>(missing).is_err());
            let mut missing = serde_json::to_value(&direct_optional).unwrap();
            missing.as_object_mut().unwrap().remove(field);
            assert!(serde_json::from_value::<RetainedOccurrenceOptionalCost>(missing).is_err());
            let mut missing = serde_json::to_value(&direct_total).unwrap();
            missing.as_object_mut().unwrap().remove(field);
            assert!(serde_json::from_value::<RetainedOccurrenceTotalCost>(missing).is_err());
        }
        let restored = decoder.restore_cast_payment_state(wire, receiver).unwrap();
        let root = decoder.restore_card_definition(root, receiver).unwrap();
        assert_eq!(root.card.id, peer[3]);
        assert_eq!(
            template_id(program(&root).effects.all_effects()[0]),
            peer[4]
        );
        for candidate in [
            &restored.alternative_casts[0],
            restored.cast_alternative_method.as_ref().unwrap(),
            &root.alternative_casts[0],
        ] {
            let ironsmith::alternative_cast::AlternativeCastingMethod::Overload { effects, .. } =
                candidate
            else {
                panic!("overload")
            };
            assert_eq!(template_id(&effects[0]), peer[2]);
            assert_ne!(template_id(&effects[0]), peer[1]);
        }
        let mut reencoder = StaticAbilityOccurrenceEncoder::default();
        // Preserve the existing unused frame too by reconstructing its owning root.
        let retained_owner = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(peer[0], "Existing cost frame"),
            "Type: Sorcery\nCreate a 1/1 white Soldier creature token.",
            false,
        )
        .unwrap()
        .1;
        let mut retained_owner = retained_owner;
        retained_owner.spell_effect = Some(
            retained_owner
                .spell_effect
                .take()
                .unwrap()
                .try_map_effects(|effect| {
                    Ok::<_, std::convert::Infallible>(replace_token_template_id(effect, peer[1]))
                })
                .unwrap(),
        );
        let rebind = |id| {
            peer.iter()
                .position(|candidate| *candidate == id)
                .map(|index| index as u32)
                .ok_or_else(|| OccurrenceBindingError::InvalidModel {
                    detail: "unknown restored cost node".into(),
                })
        };
        reencoder
            .encode_card_definition(retained_owner, rebind)
            .unwrap();
        let a = reencoder
            .encode_cast_payment_state(restored.clone(), rebind)
            .unwrap();
        let b = reencoder.encode_card_definition(root, rebind).unwrap();
        assert_eq!(
            serde_json::to_value((reencoder.into_table(), a, b)).unwrap(),
            json
        );
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, alice, ironsmith::Zone::Stack);
        let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
        let mut context = ironsmith::costs::CostContext::new(source, alice, &mut dm);
        let direct_total = decoder.restore_total_cost(direct_total).unwrap();
        let direct_optional = decoder.restore_optional_cost(direct_optional).unwrap();
        for cost in direct_total.costs() {
            cost.pay(&mut game, &mut context).unwrap();
        }
        for cost in direct_optional.cost.costs() {
            cost.pay(&mut game, &mut context).unwrap();
        }
        assert_eq!(game.player(alice).unwrap().life, 18);
        let method = decoder.restore_alternative_cast(direct_method).unwrap();
        let ironsmith::alternative_cast::AlternativeCastingMethod::Overload { effects, .. } =
            method
        else {
            panic!("overload")
        };
        let mut context = ironsmith::effects::EffectContext::new_default(source, alice);
        for effect in effects {
            ironsmith::effects::execute_effect(&mut game, &effect, &mut context).unwrap();
        }
        assert_eq!(game.battlefield.len(), 1);
        assert_eq!(
            game.object(game.battlefield[0]).unwrap().kind,
            ironsmith::object::ObjectKind::Token
        );
    }

    fn embedded_template_static(effect: &ironsmith::effect::Effect) -> Option<ironsmith::static_abilities::StaticAbility> {
        if let Some(token) = effect.downcast_ref::<ironsmith::effects::CreateTokenEffect>() {
            return token.token.abilities.iter().find_map(|ability| {
                if let ironsmith::ability::AbilityKind::Static(value) = &ability.kind { Some(value.clone()) } else { None }
            });
        }
        let mut result = None;
        effect.visit_child_effects(&mut |child| { if result.is_none() { result = embedded_template_static(child); } });
        result
    }

    #[test]
    fn retained_definition_codec_nested_template_static_occurrence_shares_origin_identity() {
        let definition = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Nested occurrence owner"),
            "Type: Sorcery\nCreate a 1/1 white Soldier creature token with flying.", false,
        ).unwrap().1;
        let effect = definition.spell_effect.as_ref().unwrap().all_effects()[0];
        let flying = embedded_template_static(effect).expect("actual native flying template");
        let ids = [definition.card.id, template_id(effect)];
        let alice = ironsmith::PlayerId::from_index(0);
        let mut native_game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()],20);
        let source = native_game.create_object_from_definition(&definition,alice,ironsmith::Zone::Stack);
        let mut context = ironsmith::effects::EffectContext::new_default(source,alice);
        ironsmith::effects::execute_effect(&mut native_game,effect,&mut context).unwrap();
        assert_eq!(native_game.battlefield.len(),1);
        assert!(native_game.object(native_game.battlefield[0]).unwrap().abilities.iter().any(|ability| {
            matches!(&ability.kind, ironsmith::ability::AbilityKind::Static(value) if value.instance_id()==flying.instance_id())
        }), "native template and created token must share the occurrence before transport");
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        let wire = encoder.encode_card_definition(definition,|id| {
            ids.iter().position(|candidate| *candidate==id).map(|index| index as u32)
                .ok_or_else(|| OccurrenceBindingError::InvalidModel { detail:"unknown nested alias graph node".into() })
        }).unwrap();
        let reference = encoder.reference(flying.instance_id());
        assert!(reference.is_ok(), "nested executable template static occurrences must join the shared table: {:?}",reference.err());
        let reference = reference.unwrap();
        let json = serde_json::to_value((encoder.into_table(),wire)).unwrap();
        let (table,wire): (RetainedStaticAbilityTable,RetainedOccurrenceCardDefinition<u32>) = serde_json::from_value(json).unwrap();
        let peers = [ironsmith::CardId::new(),ironsmith::CardId::new()];
        let decoder = StaticAbilityOccurrenceDecoder::restore_with_card_graph::<u32>(table,|index| {
            peers.get(index as usize).copied().ok_or_else(|| OccurrenceBindingError::InvalidModel {detail:"missing receiver graph node".into()})
        }).unwrap();
        let restored = decoder.restore_card_definition(wire,|index| {
            peers.get(index as usize).copied().ok_or_else(|| OccurrenceBindingError::InvalidModel {detail:"missing receiver graph node".into()})
        }).unwrap();
        let restored_static = embedded_template_static(restored.spell_effect.as_ref().unwrap().all_effects()[0]).unwrap();
        assert_eq!(restored_static.instance_id(),decoder.instance_id(reference).unwrap(),"nested restored template must retain the same occurrence used by external origins");
    }



    #[test]
    fn retained_definition_codec_equal_template_models_preserve_independent_occurrences() {
        let mut definition = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(
                ironsmith::CardId::new(),
                "Independent nested occurrences",
            ),
            "Type: Sorcery\nCreate a 1/1 white Soldier creature token with flying.",
            false,
        )
        .unwrap()
        .1;
        let first = definition.spell_effect.as_ref().unwrap().all_effects()[0].clone();
        let model =
            ironsmith_runtime_catalog::artifact_materializer::encode_runtime_effect(first.clone())
                .unwrap();
        let second =
            ironsmith_runtime_catalog::artifact_materializer::materialize_effect(model.clone())
                .unwrap();
        assert_eq!(
            ironsmith_runtime_catalog::artifact_materializer::encode_runtime_effect(second.clone())
                .unwrap(),
            model
        );
        let originals = [
            embedded_template_static(&first).unwrap().instance_id(),
            embedded_template_static(&second).unwrap().instance_id(),
        ];
        assert_ne!(originals[0], originals[1]);
        assert_eq!(template_id(&first), template_id(&second));
        let ids = [definition.card.id, template_id(&first)];
        definition.spell_effect = Some(ironsmith::resolution::ResolutionProgram::from_effects(
            vec![first, second],
        ));
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        let wire = encoder
            .encode_card_definition(definition, |id| {
                ids.iter()
                    .position(|candidate| *candidate == id)
                    .map(|index| index as u32)
                    .ok_or_else(|| OccurrenceBindingError::InvalidModel {
                        detail: "unknown template graph node".into(),
                    })
            })
            .unwrap();
        let references = originals.map(|id| encoder.reference(id).unwrap());
        assert_ne!(references[0], references[1]);
        let json = serde_json::to_value((encoder.into_table(), wire)).unwrap();
        let (table, wire): (
            RetainedStaticAbilityTable,
            RetainedOccurrenceCardDefinition<u32>,
        ) = serde_json::from_value(json).unwrap();
        let peers = [ironsmith::CardId::new(), ironsmith::CardId::new()];
        let decoder =
            StaticAbilityOccurrenceDecoder::restore_with_card_graph::<u32>(table, |index| {
                peers.get(index as usize).copied().ok_or_else(|| {
                    OccurrenceBindingError::InvalidModel {
                        detail: "missing receiver".into(),
                    }
                })
            })
            .unwrap();
        let restored = decoder
            .restore_card_definition(wire, |index| {
                peers.get(index as usize).copied().ok_or_else(|| {
                    OccurrenceBindingError::InvalidModel {
                        detail: "missing receiver".into(),
                    }
                })
            })
            .unwrap();
        let restored_ids = references.map(|reference| decoder.instance_id(reference).unwrap());
        assert_ne!(restored_ids[0], restored_ids[1]);
        let alice = ironsmith::PlayerId::from_index(0);
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&restored, alice, ironsmith::Zone::Stack);
        let effects = restored.spell_effect.as_ref().unwrap().all_effects();
        assert_eq!(effects.len(), 2);
        for (index, effect) in effects.into_iter().enumerate() {
            assert_eq!(
                embedded_template_static(effect).unwrap().instance_id(),
                restored_ids[index]
            );
            let mut context = ironsmith::effects::EffectContext::new_default(source, alice);
            ironsmith::effects::execute_effect(&mut game, effect, &mut context).unwrap();
            assert_eq!(game.battlefield.len(), index + 1);
            let token = game.object(*game.battlefield.last().unwrap()).unwrap();
            assert!(token.abilities.iter().any(|ability| matches!(&ability.kind, ironsmith::ability::AbilityKind::Static(value) if value.instance_id() == restored_ids[index])));
            assert!(!token.abilities.iter().any(|ability| matches!(&ability.kind, ironsmith::ability::AbilityKind::Static(value) if value.instance_id() == restored_ids[1-index])));
        }
    }

    #[test]
    fn retained_definition_codec_embedded_associations_reject_missing_extra_mismatch_and_unknown_occurrences()
     {
        let definition = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(
                ironsmith::CardId::new(),
                "Malformed nested association",
            ),
            "Type: Sorcery\nCreate a 1/1 white Soldier creature token with flying.",
            false,
        )
        .unwrap()
        .1;
        let native = definition.spell_effect.clone().unwrap();
        let template = template_id(native.all_effects()[0]);
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        let wire = encoder
            .encode_program_with_card_graph(native, |id| {
                assert_eq!(id, template);
                Ok(0u32)
            })
            .unwrap();
        assert_eq!(wire.embedded_definitions.len(), 1);
        let json = serde_json::to_value((encoder.into_table(), wire)).unwrap();
        let (table, wire): (RetainedStaticAbilityTable, RetainedOccurrenceProgram) =
            serde_json::from_value(json).unwrap();
        let peer = ironsmith::CardId::new();
        let decoder =
            StaticAbilityOccurrenceDecoder::restore_with_card_graph::<u32>(table, |index| {
                assert_eq!(index, 0);
                Ok(peer)
            })
            .unwrap();
        let mut missing = wire.clone();
        missing.embedded_definitions.clear();
        assert!(matches!(
            decoder.restore_program(missing),
            Err(OccurrenceBindingError::InvalidModel { .. })
        ));
        let mut extra = wire.clone();
        extra
            .embedded_definitions
            .push(wire.embedded_definitions[0].clone());
        assert!(matches!(
            decoder.restore_program(extra),
            Err(OccurrenceBindingError::InvalidModel { .. })
        ));
        let mut mismatch = wire.clone();
        mismatch.embedded_definitions[0]
            .model
            .canonical_text
            .push_str(" altered association");
        assert!(matches!(
            decoder.restore_program(mismatch),
            Err(OccurrenceBindingError::InvalidModel { .. })
        ));
        let mut unknown = wire.clone();
        let ability = &mut unknown.embedded_definitions[0].snapshot.abilities[0];
        ability.model = ability.model.clone().try_map(
            |_| Ok::<_, std::convert::Infallible>(StaticAbilityOccurrenceRef(u32::MAX)),
            Ok, Ok, Ok, Ok,
        ).unwrap();
        assert!(matches!(
            decoder.restore_program(unknown),
            Err(OccurrenceBindingError::UnknownReference {
                reference: StaticAbilityOccurrenceRef(u32::MAX)
            })
        ));
        let mut absent = serde_json::to_value(&wire).unwrap();
        absent
            .as_object_mut()
            .unwrap()
            .remove("embedded_definitions");
        assert!(serde_json::from_value::<RetainedOccurrenceProgram>(absent).is_err());
        let restored = decoder.restore_program(wire).unwrap();
        assert_eq!(template_id(restored.all_effects()[0]), peer);
        let alice = ironsmith::PlayerId::from_index(0);
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, alice, ironsmith::Zone::Stack);
        let mut context = ironsmith::effects::EffectContext::new_default(source, alice);
        ironsmith::effects::execute_effect(&mut game, restored.all_effects()[0], &mut context)
            .unwrap();
        assert_eq!(game.battlefield.len(), 1);
    }



    #[test]
    fn retained_definition_codec_alternative_template_occurrence_survives_every_owner() {
        let mut definition = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(
                ironsmith::CardId::new(),
                "Alternative occurrence owner",
            ),
            "Type: Sorcery\nCreate a 1/1 white Soldier creature token with flying.",
            false,
        )
        .unwrap()
        .1;
        let effect = definition.spell_effect.as_ref().unwrap().all_effects()[0].clone();
        let flying = embedded_template_static(&effect).unwrap();
        let ids = [definition.card.id, template_id(&effect)];
        let method = ironsmith::alternative_cast::AlternativeCastingMethod::Overload {
            cost: ironsmith::mana::ManaCost::new(),
            effects: vec![effect],
        };
        let alice = ironsmith::PlayerId::from_index(0);
        definition.alternative_casts = vec![method.clone()];
        let object = ironsmith::object::Object::from_card_definition(
            ironsmith::ObjectId::from_raw(80123),
            &definition,
            alice,
            ironsmith::Zone::Stack,
        );
        let mut capture = ironsmith::object::NativeCastPaymentState::from(&object);
        capture.cast_alternative_method = Some(method.clone());
        let bind = |id| {
            ids.iter()
                .position(|candidate| *candidate == id)
                .map(|index| index as u32)
                .ok_or_else(|| OccurrenceBindingError::InvalidModel {
                    detail: "unknown alternative graph node".into(),
                })
        };
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        let before = serde_json::to_value(encoder.clone().into_table()).unwrap();
        assert!(
            encoder
                .encode_alternative_cast_with_card_graph(method.clone(), |_| Err::<u32, _>(
                    OccurrenceBindingError::InvalidModel {
                        detail: "missing alternative graph binder".into()
                    }
                ))
                .is_err()
        );
        assert_eq!(
            serde_json::to_value(encoder.clone().into_table()).unwrap(),
            before
        );
        let direct = encoder
            .encode_alternative_cast_with_card_graph(method.clone(), bind)
            .unwrap();
        let reference = encoder.reference(flying.instance_id());
        assert!(
            reference.is_ok(),
            "alternative effect template must join shared occurrence table: {:?}",
            reference.err()
        );
        let reference = reference.unwrap();
        let root = encoder.encode_card_definition(definition, bind).unwrap();
        let capture = encoder.encode_cast_payment_state(capture, bind).unwrap();
        let json = serde_json::to_value((encoder.into_table(), direct, root, capture)).unwrap();
        let (table, direct, root, capture): (
            RetainedStaticAbilityTable,
            RetainedOccurrenceAlternativeCast,
            RetainedOccurrenceCardDefinition<u32>,
            RetainedOccurrenceCastPaymentState<u32>,
        ) = serde_json::from_value(json.clone()).unwrap();
        let peers = [ironsmith::CardId::new(), ironsmith::CardId::new()];
        let receive = |index: u32| {
            peers
                .get(index as usize)
                .copied()
                .ok_or_else(|| OccurrenceBindingError::InvalidModel {
                    detail: "missing alternative receiver graph node".into(),
                })
        };
        let decoder =
            StaticAbilityOccurrenceDecoder::restore_with_card_graph::<u32>(table, receive).unwrap();
        let identity = decoder.instance_id(reference).unwrap();
        let mut missing = direct.clone();
        missing.embedded_definitions.clear();
        assert!(matches!(decoder.restore_alternative_cast(missing), Err(OccurrenceBindingError::InvalidModel { .. })));
        let mut extra = direct.clone();
        extra.embedded_definitions.push(direct.embedded_definitions[0].clone());
        assert!(matches!(decoder.restore_alternative_cast(extra), Err(OccurrenceBindingError::InvalidModel { .. })));
        let mut mismatched = direct.clone();
        mismatched.embedded_definitions[0].model.canonical_text.push_str(" mismatched method template");
        assert!(matches!(decoder.restore_alternative_cast(mismatched), Err(OccurrenceBindingError::InvalidModel { .. })));
        let mut unknown = direct.clone();
        let ability = &mut unknown.embedded_definitions[0].snapshot.abilities[0];
        ability.model = ability.model.clone().try_map(
            |_| Ok::<_, std::convert::Infallible>(StaticAbilityOccurrenceRef(u32::MAX)), Ok, Ok, Ok, Ok,
        ).unwrap();
        assert!(matches!(decoder.restore_alternative_cast(unknown), Err(OccurrenceBindingError::UnknownReference { reference: StaticAbilityOccurrenceRef(u32::MAX) })));
        let direct = decoder.restore_alternative_cast(direct).unwrap();
        let root = decoder.restore_card_definition(root, receive).unwrap();
        let capture = decoder
            .restore_cast_payment_state(capture, receive)
            .unwrap();
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&root, alice, ironsmith::Zone::Stack);
        for (index, method) in [
            &direct,
            &root.alternative_casts[0],
            &capture.alternative_casts[0],
            capture.cast_alternative_method.as_ref().unwrap(),
        ]
        .into_iter()
        .enumerate()
        {
            let effect = &method.overload_effects().unwrap()[0];
            assert_eq!(template_id(effect), peers[1]);
            assert_eq!(
                embedded_template_static(effect).unwrap().instance_id(),
                identity
            );
            let mut context = ironsmith::effects::EffectContext::new_default(source, alice);
            ironsmith::effects::execute_effect(&mut game, effect, &mut context).unwrap();
            assert_eq!(game.battlefield.len(), index + 1);
            assert!(game.object(*game.battlefield.last().unwrap()).unwrap().abilities.iter().any(|ability| matches!(&ability.kind,ironsmith::ability::AbilityKind::Static(value) if value.instance_id()==identity)));
        }
        let mut reencoder = StaticAbilityOccurrenceEncoder::default();
        let rebind = |id| {
            peers
                .iter()
                .position(|candidate| *candidate == id)
                .map(|index| index as u32)
                .ok_or_else(|| OccurrenceBindingError::InvalidModel {
                    detail: "unexpected reencoded graph node".into(),
                })
        };
        let direct = reencoder
            .encode_alternative_cast_with_card_graph(direct, rebind)
            .unwrap();
        let root = reencoder.encode_card_definition(root, rebind).unwrap();
        let capture = reencoder
            .encode_cast_payment_state(capture, rebind)
            .unwrap();
        assert_eq!(
            serde_json::to_value((reencoder.into_table(), direct, root, capture)).unwrap(),
            json
        );
    }



    #[test]
    fn retained_definition_codec_cost_payloads_reject_unused_template_records_then_pay() {
        let producer = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Cost association seed"),
            "Type: Sorcery\nCreate a 1/1 white Soldier creature token with flying.",
            false,
        )
        .unwrap()
        .1;
        let template = template_id(producer.spell_effect.as_ref().unwrap().all_effects()[0]);
        let payer = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(
                ironsmith::CardId::new(),
                "Cost association payer",
            ),
            "Type: Artifact\nPay 1 life: You gain 2 life.",
            false,
        )
        .unwrap()
        .1;
        let cost =
            ironsmith::cost::TotalCost::from_cost(program(&payer).mana_cost.costs()[0].clone());
        let optional = ironsmith::cost::OptionalCost::buyback(cost.clone());
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        let seed = encoder
            .encode_program_with_card_graph(producer.spell_effect.unwrap(), |id| {
                assert_eq!(id, template);
                Ok(0u32)
            })
            .unwrap();
        let total = encoder
            .encode_total_cost_with_card_graph(cost, |_| {
                Err::<u32, _>(OccurrenceBindingError::InvalidModel {
                    detail: "ordinary life cost has no card references".into(),
                })
            })
            .unwrap();
        let optional = encoder
            .encode_optional_cost_with_card_graph(optional, |_| {
                Err::<u32, _>(OccurrenceBindingError::InvalidModel {
                    detail: "ordinary optional life cost has no card references".into(),
                })
            })
            .unwrap();
        assert!(total.embedded_definitions.is_empty());
        assert!(optional.embedded_definitions.is_empty());
        let json = serde_json::to_value((encoder.into_table(), seed, total, optional)).unwrap();
        let (table, seed, total, optional): (
            RetainedStaticAbilityTable,
            RetainedOccurrenceProgram,
            RetainedOccurrenceTotalCost,
            RetainedOccurrenceOptionalCost,
        ) = serde_json::from_value(json).unwrap();
        let peer = ironsmith::CardId::new();
        let decoder =
            StaticAbilityOccurrenceDecoder::restore_with_card_graph::<u32>(table, |index| {
                assert_eq!(index, 0);
                Ok(peer)
            })
            .unwrap();
        let mut poisoned_total = total.clone();
        poisoned_total.embedded_definitions = seed.embedded_definitions.clone();
        assert!(matches!(
            decoder.restore_total_cost(poisoned_total),
            Err(OccurrenceBindingError::InvalidModel { .. })
        ));
        let mut poisoned_optional = optional.clone();
        poisoned_optional.embedded_definitions = seed.embedded_definitions.clone();
        assert!(matches!(
            decoder.restore_optional_cost(poisoned_optional),
            Err(OccurrenceBindingError::InvalidModel { .. })
        ));
        let total = decoder.restore_total_cost(total).unwrap();
        let optional = decoder.restore_optional_cost(optional).unwrap();
        let alice = ironsmith::PlayerId::from_index(0);
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source =
            game.create_object_from_definition(&payer, alice, ironsmith::Zone::Battlefield);
        let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
        let mut context = ironsmith::costs::CostContext::new(source, alice, &mut dm);
        for cost in total.costs() {
            cost.pay(&mut game, &mut context).unwrap();
        }
        assert_eq!(game.player(alice).unwrap().life, 19);
        for cost in optional.cost.costs() {
            cost.pay(&mut game, &mut context).unwrap();
        }
        assert_eq!(game.player(alice).unwrap().life, 18);
    }



    #[test]
    fn retained_definition_codec_shared_static_template_preserves_nested_occurrence() {
        let definition = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Nested shared static owner"),
            "Type: Artifact\nCreatures you control have \"Pay 1 life: Create a 1/1 white Soldier creature token with flying.\".",false,
        ).unwrap().1;
        let grant = definition
            .abilities
            .iter()
            .find_map(|ability| {
                if let ironsmith::ability::AbilityKind::Static(value) = &ability.kind {
                    Some(value.clone())
                } else {
                    None
                }
            })
            .unwrap();
        let effect = static_template_effect(&grant);
        let flying = embedded_template_static(&effect).unwrap();
        let ids = [definition.card.id, template_id(&effect)];
        let creature = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(
                ironsmith::CardId::new(),
                "Nested shared static recipient",
            ),
            "Type: Creature — Human\nPower/Toughness: 1/1",
            false,
        )
        .unwrap()
        .1;
        let alice = ironsmith::PlayerId::from_index(0);
        let mut native_game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        native_game.create_object_from_definition(&definition, alice, ironsmith::Zone::Battlefield);
        let native_source = native_game.create_object_from_definition(
            &creature,
            alice,
            ironsmith::Zone::Battlefield,
        );
        let native_abilities = native_game.current_abilities(native_source).unwrap();
        let native_activated = native_abilities
            .iter()
            .find_map(|ability| {
                if let ironsmith::ability::AbilityKind::Activated(value) = &ability.kind {
                    Some(value.clone())
                } else {
                    None
                }
            })
            .unwrap();
        assert_eq!(
            embedded_template_static(native_activated.effects.all_effects()[0])
                .unwrap()
                .instance_id(),
            flying.instance_id(),
            "native granted executor must share compiled template occurrence before transport"
        );
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        let parent_ref = encoder.retain(grant.clone()).unwrap();
        let root = encoder
            .encode_card_definition(definition, |id| {
                ids.iter()
                    .position(|candidate| *candidate == id)
                    .map(|index| index as u32)
                    .ok_or_else(|| OccurrenceBindingError::InvalidModel {
                        detail: "unknown shared static graph node".into(),
                    })
            })
            .unwrap();
        let nested_ref = encoder.reference(flying.instance_id());
        assert!(
            nested_ref.is_ok(),
            "shared static effect templates must enroll nested occurrences: {:?}",
            nested_ref.err()
        );
        let nested_ref = nested_ref.unwrap();
        assert_ne!(parent_ref, nested_ref);
        let json = serde_json::to_value((encoder.into_table(), root)).unwrap();
        let (table, root): (
            RetainedStaticAbilityTable,
            RetainedOccurrenceCardDefinition<u32>,
        ) = serde_json::from_value(json.clone()).unwrap();
        let peers = [ironsmith::CardId::new(), ironsmith::CardId::new()];
        let receive = |index: u32| {
            peers
                .get(index as usize)
                .copied()
                .ok_or_else(|| OccurrenceBindingError::InvalidModel {
                    detail: "missing shared static receiver".into(),
                })
        };
        assert_eq!(table.embedded_definitions.len(),table.models.len());
        let mut absent = serde_json::to_value(&table).unwrap(); absent.as_object_mut().unwrap().remove("embedded_definitions");
        assert!(serde_json::from_value::<RetainedStaticAbilityTable>(absent).is_err());
        let mut width = table.clone(); width.embedded_definitions.clear();
        assert!(matches!(StaticAbilityOccurrenceDecoder::restore_with_card_graph::<u32>(width,receive),Err(OccurrenceBindingError::InvalidModel{..})));
        let mut missing = table.clone(); missing.embedded_definitions[parent_ref.0 as usize].clear();
        assert!(matches!(StaticAbilityOccurrenceDecoder::restore_with_card_graph::<u32>(missing,receive),Err(OccurrenceBindingError::InvalidModel{..})));
        let mut extra = table.clone();
        let association = extra.embedded_definitions[parent_ref.0 as usize][0].clone();
        extra.embedded_definitions[parent_ref.0 as usize].push(association);
        assert!(matches!(StaticAbilityOccurrenceDecoder::restore_with_card_graph::<u32>(extra,receive),Err(OccurrenceBindingError::InvalidModel{..})));
        let mut mismatch = table.clone(); mismatch.embedded_definitions[parent_ref.0 as usize][0].model.canonical_text.push_str(" mismatched shared template");
        assert!(matches!(StaticAbilityOccurrenceDecoder::restore_with_card_graph::<u32>(mismatch,receive),Err(OccurrenceBindingError::InvalidModel{..})));
        for (reference,cycle) in [(StaticAbilityOccurrenceRef(u32::MAX),false),(parent_ref,true)] {
            let mut invalid = table.clone();
            let ability = &mut invalid.embedded_definitions[parent_ref.0 as usize][0].snapshot.abilities[0];
            ability.model = ability.model.clone().try_map(|_|Ok::<_,std::convert::Infallible>(reference),Ok,Ok,Ok,Ok).unwrap();
            let result = StaticAbilityOccurrenceDecoder::restore_with_card_graph::<u32>(invalid,receive);
            if cycle { assert!(matches!(result,Err(OccurrenceBindingError::InvalidModel{detail}) if detail.contains("cyclic shared occurrence dependencies"))); }
            else { assert!(matches!(result,Err(OccurrenceBindingError::UnknownReference{reference:StaticAbilityOccurrenceRef(u32::MAX)}))); }
        }
        let decoder = StaticAbilityOccurrenceDecoder::restore_with_card_graph::<u32>(table,receive).unwrap();
        let root = decoder.restore_card_definition(root, receive).unwrap();
        let identity = decoder.instance_id(nested_ref).unwrap();
        let restored_grant = root
            .abilities
            .iter()
            .find_map(|ability| {
                if let ironsmith::ability::AbilityKind::Static(value) = &ability.kind {
                    Some(value.clone())
                } else {
                    None
                }
            })
            .unwrap();
        assert_eq!(
            restored_grant.instance_id(),
            decoder.instance_id(parent_ref).unwrap()
        );
        assert_eq!(
            embedded_template_static(&static_template_effect(&restored_grant))
                .unwrap()
                .instance_id(),
            identity
        );
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.create_object_from_definition(&root, alice, ironsmith::Zone::Battlefield);
        let source =
            game.create_object_from_definition(&creature, alice, ironsmith::Zone::Battlefield);
        let abilities = game.current_abilities(source).unwrap();
        let activated = abilities
            .iter()
            .find_map(|ability| {
                if let ironsmith::ability::AbilityKind::Activated(value) = &ability.kind {
                    Some(value.clone())
                } else {
                    None
                }
            })
            .unwrap();
        assert_eq!(
            embedded_template_static(activated.effects.all_effects()[0])
                .unwrap()
                .instance_id(),
            identity
        );
        let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
        let mut cost_context = ironsmith::costs::CostContext::new(source, alice, &mut dm);
        for cost in activated.mana_cost.costs() {
            cost.pay(&mut game, &mut cost_context).unwrap();
        }
        assert_eq!(game.player(alice).unwrap().life, 19);
        let mut context = ironsmith::effects::EffectContext::new_default(source, alice);
        for effect in activated.effects.all_effects() {
            ironsmith::effects::execute_effect(&mut game, effect, &mut context).unwrap();
        }
        let tokens: Vec<_> = game
            .battlefield
            .iter()
            .filter_map(|id| game.object(*id))
            .filter(|object| object.kind == ironsmith::object::ObjectKind::Token)
            .collect();
        assert_eq!(tokens.len(), 1);
        assert!(tokens[0].abilities.iter().any(|ability|matches!(&ability.kind,ironsmith::ability::AbilityKind::Static(value) if value.instance_id()==identity)));
        let mut reencoder = StaticAbilityOccurrenceEncoder::default();
        assert_eq!(reencoder.retain(restored_grant).unwrap(), parent_ref);
        let root = reencoder
            .encode_card_definition(root, |id| {
                peers
                    .iter()
                    .position(|candidate| *candidate == id)
                    .map(|index| index as u32)
                    .ok_or_else(|| OccurrenceBindingError::InvalidModel {
                        detail: "unknown shared static reencoding node".into(),
                    })
            })
            .unwrap();
        assert_eq!(
            serde_json::to_value((reencoder.into_table(), root)).unwrap(),
            json
        );
    }



    #[test]
    fn retained_definition_codec_equal_shared_static_models_keep_independent_template_occurrences()
    {
        let first = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(ironsmith::CardId::new(),"Independent shared grant"),
            "Type: Artifact\nCreatures you control have \"Pay 1 life: Create a 1/1 white Soldier creature token with flying.\".",false,
        ).unwrap().1;
        let grant = first
            .abilities
            .iter()
            .find_map(|ability| {
                if let ironsmith::ability::AbilityKind::Static(value) = &ability.kind {
                    Some(value.clone())
                } else {
                    None
                }
            })
            .unwrap();
        let model = encode_runtime_static_ability(grant.clone()).unwrap();
        let independent = restore_runtime_ability(
            ironsmith_compiled_artifact::WireAbility::static_ability(model.clone()),
        )
        .unwrap();
        let ironsmith::ability::AbilityKind::Static(second_grant) = &independent.kind else {
            panic!("static grant")
        };
        assert_eq!(
            encode_runtime_static_ability(second_grant.clone()).unwrap(),
            model
        );
        assert_ne!(grant.instance_id(), second_grant.instance_id());
        let originals = [
            embedded_template_static(&static_template_effect(&grant))
                .unwrap()
                .instance_id(),
            embedded_template_static(&static_template_effect(second_grant))
                .unwrap()
                .instance_id(),
        ];
        assert_ne!(originals[0], originals[1]);
        let mut second = first.clone();
        second.abilities = vec![independent];
        let ids = [first.card.id, template_id(&static_template_effect(&grant))];
        let bind = |id| {
            ids.iter()
                .position(|candidate| *candidate == id)
                .map(|index| index as u32)
                .ok_or_else(|| OccurrenceBindingError::InvalidModel {
                    detail: "unknown independent shared graph node".into(),
                })
        };
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        let first = encoder.encode_card_definition(first, bind).unwrap();
        let second = encoder.encode_card_definition(second, bind).unwrap();
        let references = originals.map(|identity| encoder.reference(identity).unwrap());
        assert_ne!(references[0], references[1]);
        let json = serde_json::to_value((encoder.into_table(), first, second)).unwrap();
        let (table, first, second): (
            RetainedStaticAbilityTable,
            RetainedOccurrenceCardDefinition<u32>,
            RetainedOccurrenceCardDefinition<u32>,
        ) = serde_json::from_value(json.clone()).unwrap();
        let peers = [ironsmith::CardId::new(), ironsmith::CardId::new()];
        let receive = |index: u32| {
            peers
                .get(index as usize)
                .copied()
                .ok_or_else(|| OccurrenceBindingError::InvalidModel {
                    detail: "missing independent shared peer".into(),
                })
        };
        let decoder =
            StaticAbilityOccurrenceDecoder::restore_with_card_graph::<u32>(table, receive).unwrap();
        let first = decoder.restore_card_definition(first, receive).unwrap();
        let second = decoder.restore_card_definition(second, receive).unwrap();
        let restored_ids = references.map(|reference| decoder.instance_id(reference).unwrap());
        assert_ne!(restored_ids[0], restored_ids[1]);
        let creature = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(
                ironsmith::CardId::new(),
                "Independent shared recipient",
            ),
            "Type: Creature — Human\nPower/Toughness: 1/1",
            false,
        )
        .unwrap()
        .1;
        let alice = ironsmith::PlayerId::from_index(0);
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.create_object_from_definition(&first, alice, ironsmith::Zone::Battlefield);
        game.create_object_from_definition(&second, alice, ironsmith::Zone::Battlefield);
        let source =
            game.create_object_from_definition(&creature, alice, ironsmith::Zone::Battlefield);
        let activated: Vec<_> = game
            .current_abilities(source)
            .unwrap()
            .into_iter()
            .filter_map(|ability| {
                if let ironsmith::ability::AbilityKind::Activated(value) = ability.kind {
                    Some(value)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(
            activated.len(),
            2,
            "independent static grants must both remain usable"
        );
        let identities: std::collections::HashSet<_> = activated
            .iter()
            .map(|ability| {
                embedded_template_static(ability.effects.all_effects()[0])
                    .unwrap()
                    .instance_id()
            })
            .collect();
        assert_eq!(identities, restored_ids.into_iter().collect());
        for ability in &activated {
            let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
            let mut context = ironsmith::costs::CostContext::new(source, alice, &mut dm);
            for cost in ability.mana_cost.costs() {
                cost.pay(&mut game, &mut context).unwrap();
            }
            let identity = embedded_template_static(ability.effects.all_effects()[0])
                .unwrap()
                .instance_id();
            let mut context = ironsmith::effects::EffectContext::new_default(source, alice);
            for effect in ability.effects.all_effects() {
                ironsmith::effects::execute_effect(&mut game, effect, &mut context).unwrap();
            }
            let token = game.object(*game.battlefield.last().unwrap()).unwrap();
            assert!(token.abilities.iter().any(|ability|matches!(&ability.kind,ironsmith::ability::AbilityKind::Static(value) if value.instance_id()==identity)));
        }
        assert_eq!(game.player(alice).unwrap().life, 18);
        assert_eq!(
            game.battlefield
                .iter()
                .filter_map(|id| game.object(*id))
                .filter(|object| object.kind == ironsmith::object::ObjectKind::Token)
                .count(),
            2
        );
        let mut reencoder = StaticAbilityOccurrenceEncoder::default();
        let rebind = |id| {
            peers
                .iter()
                .position(|candidate| *candidate == id)
                .map(|index| index as u32)
                .ok_or_else(|| OccurrenceBindingError::InvalidModel {
                    detail: "unknown independent shared reencode node".into(),
                })
        };
        let first = reencoder.encode_card_definition(first, rebind).unwrap();
        let second = reencoder.encode_card_definition(second, rebind).unwrap();
        assert_eq!(
            serde_json::to_value((reencoder.into_table(), first, second)).unwrap(),
            json
        );
    }





    fn check_owned_cost_template_carrier(delayed: bool) {
        use ironsmith_compiled_artifact::{WireCost, WireEffect};
        fn owner_cost(
            effect: &ironsmith::effect::Effect,
            delayed: bool,
        ) -> &ironsmith::cost::TotalCost {
            if delayed {
                &effect
                    .downcast_ref::<ironsmith::effects::ScheduleDelayedTriggerEffect>()
                    .unwrap()
                    .prepayment
                    .as_ref()
                    .unwrap()
                    .cost
            } else {
                &effect
                    .downcast_ref::<ironsmith::effects::UnlessPaysEffect>()
                    .unwrap()
                    .cost
            }
        }
        fn branch_flying(
            cost: &ironsmith::cost::TotalCost,
        ) -> Vec<ironsmith::static_abilities::StaticAbility> {
            cost.as_one_of()
                .unwrap()
                .iter()
                .map(|branch| {
                    branch
                        .costs()
                        .iter()
                        .find_map(|cost| {
                            cost.compiled_model()
                                .and_then(|model| model.effect_ref())
                                .and_then(embedded_template_static)
                        })
                        .unwrap()
                })
                .collect()
        }

        fn branch_template_ids(cost: &ironsmith::cost::TotalCost) -> Vec<ironsmith::CardId> {
            cost.as_one_of()
                .unwrap()
                .iter()
                .map(|branch| {
                    let may = branch
                        .costs()
                        .iter()
                        .find_map(|cost| {
                            cost.compiled_model()
                                .and_then(|model| model.effect_ref())
                                .and_then(|effect| {
                                    effect.downcast_ref::<ironsmith::effects::MayEffect>()
                                })
                        })
                        .unwrap();
                    let mut effect = &may.effects[0];
                    while let Some(child) = effect.transparent_child_effect() {
                        effect = child;
                    }
                    effect
                        .downcast_ref::<ironsmith::effects::CreateTokenEffect>()
                        .unwrap()
                        .token
                        .card
                        .id
                })
                .collect()
        }

        fn execute_owner(
            game: &mut ironsmith::GameState,
            effect: &ironsmith::effect::Effect,
            source: ironsmith::ObjectId,
            alice: ironsmith::PlayerId,
            delayed: bool,
        ) {
            let mut context = ironsmith::effects::EffectContext::new_default(source, alice);
            ironsmith::effects::execute_effect(game, effect, &mut context).unwrap();
            if delayed {
                assert_eq!(game.effect_store.delayed_triggers.len(), 1);
                game.turn.priority_player = Some(alice);
                let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
                ironsmith::special_actions::perform(
                    ironsmith::special_actions::SpecialAction::PayDelayedTrigger {
                        delayed_trigger_index: 0,
                    },
                    game,
                    alice,
                    &mut dm,
                )
                .unwrap();
                assert!(game.effect_store.delayed_triggers.is_empty());
            }
            assert_eq!(game.player(alice).unwrap().life, 19);
            assert_eq!(game.battlefield.len(), 1);
        }
        let mut definition = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(
                ironsmith::CardId::new(),
                "Owned cost template carrier",
            ),
            "Type: Sorcery\nCreate a 1/1 white Soldier creature token with flying.",
            false,
        )
        .unwrap()
        .1;
        let token = encode_runtime_effect(
            definition.spell_effect.as_ref().unwrap().all_effects()[0].clone(),
        )
        .unwrap();
        let template = template_id(definition.spell_effect.as_ref().unwrap().all_effects()[0]);

        let optional = WireEffect::new(
            "MayEffect",
            serde_json::json!({"decider":null,"effects":[token]}),
        );
        let branch =
            serde_json::json!({"kind":{"All":[WireCost::life(1),WireCost::Effect(optional)]}});
        let cost = serde_json::json!({"kind":{"OneOf":[branch.clone(),branch]}});
        let wire = if delayed {
            fn delayed_model(effect: &ironsmith::effect::Effect) -> Option<WireEffect> {
                if effect
                    .downcast_ref::<ironsmith::effects::ScheduleDelayedTriggerEffect>()
                    .is_some()
                {
                    return Some(encode_runtime_effect(effect.clone()).unwrap());
                }
                let mut found = None;
                effect.visit_child_effects(&mut |child| {
                    if found.is_none() {
                        found = delayed_model(child)
                    }
                });
                found
            }
            let producer = compile_builder_to_artifact(
                compiler::CardDefinitionBuilder::new(
                    ironsmith::CardId::new(),
                    "Compiled prepayment owner",
                ),
                "Type: Sorcery\nAt the beginning of the next end step, you gain 1 life.",
                false,
            )
            .unwrap()
            .1;
            let model = delayed_model(producer.spell_effect.as_ref().unwrap().all_effects()[0])
                .expect("actual compiled delayed owner");
            let mut payload = model.payload().clone();
            payload["effects"] = serde_json::json!([]);
            payload["prepayment"] =
                serde_json::json!({"player":ironsmith::target::PlayerFilter::You,"cost":cost});
            WireEffect::new(model.kind(), payload)
        } else {
            WireEffect::new(
                "UnlessPaysEffect",
                serde_json::json!({"player":ironsmith::target::PlayerFilter::You,"effects":[],"cost":cost,"leading_surface":false,"before_delayed_step":false}),
            )
        };
        let effect =
            ironsmith_runtime_catalog::artifact_materializer::materialize_effect(wire).unwrap();
        let originals = branch_flying(owner_cost(&effect, delayed));
        assert_eq!(
            branch_template_ids(owner_cost(&effect, delayed)),
            vec![template, template]
        );
        assert_ne!(originals[0].instance_id(), originals[1].instance_id());
        definition.spell_effect = Some(ironsmith::resolution::ResolutionProgram::from_effects(
            vec![effect.clone()],
        ));
        let alice = ironsmith::PlayerId::from_index(0);
        let mut native = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source =
            native.create_object_from_definition(&definition, alice, ironsmith::Zone::Stack);
        execute_owner(&mut native, &effect, source, alice, delayed);
        assert!(native.object(native.battlefield[0]).unwrap().abilities.iter().any(|ability|matches!(&ability.kind,ironsmith::ability::AbilityKind::Static(value)if value.instance_id()==originals[0].instance_id())));
        let ids = [definition.card.id, template];
        let bind = |id| {
            ids.iter()
                .position(|value| *value == id)
                .map(|slot| slot as u32)
                .ok_or_else(|| OccurrenceBindingError::InvalidModel {
                    detail: "unknown owned cost graph node".into(),
                })
        };
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        let root = encoder.encode_card_definition(definition, bind);
        assert!(
            root.is_ok(),
            "owned cost templates must enroll, delayed={delayed}: {:?}",
            root.as_ref().err()
        );
        let root = root.unwrap();
        let references = originals
            .iter()
            .map(|ability| encoder.reference(ability.instance_id()).unwrap())
            .collect::<Vec<_>>();
        assert_ne!(references[0], references[1]);
        let json = serde_json::to_value((encoder.into_table(), root)).unwrap();
        let (table, root): (
            RetainedStaticAbilityTable,
            RetainedOccurrenceCardDefinition<u32>,
        ) = serde_json::from_value(json.clone()).unwrap();
        let peers = [ironsmith::CardId::new(), ironsmith::CardId::new()];
        let receive = |slot: u32| {
            peers
                .get(slot as usize)
                .copied()
                .ok_or_else(|| OccurrenceBindingError::InvalidModel {
                    detail: "unknown owned cost peer".into(),
                })
        };
        let decoder =
            StaticAbilityOccurrenceDecoder::restore_with_card_graph::<u32>(table, receive).unwrap();
        let root = decoder.restore_card_definition(root, receive).unwrap();
        let effect = root.spell_effect.as_ref().unwrap().all_effects()[0];
        let restored = branch_flying(owner_cost(effect, delayed));
        assert_eq!(
            branch_template_ids(owner_cost(effect, delayed)),
            vec![peers[1], peers[1]]
        );
        for index in 0..2 {
            assert_eq!(
                restored[index].instance_id(),
                decoder.instance_id(references[index]).unwrap()
            );
        }
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&root, alice, ironsmith::Zone::Stack);
        execute_owner(&mut game, effect, source, alice, delayed);
        let token = game.object(game.battlefield[0]).unwrap();
        assert_eq!(token.card, None, "created tokens are not physical cards");
        assert!(token.abilities.iter().any(|ability|matches!(&ability.kind,ironsmith::ability::AbilityKind::Static(value)if value.instance_id()==restored[0].instance_id())));
        let mut reencoder = StaticAbilityOccurrenceEncoder::default();
        let root = reencoder
            .encode_card_definition(root, |id| {
                peers
                    .iter()
                    .position(|value| *value == id)
                    .map(|slot| slot as u32)
                    .ok_or_else(|| OccurrenceBindingError::InvalidModel {
                        detail: "unknown owned cost reencode node".into(),
                    })
            })
            .unwrap();
        assert_eq!(
            serde_json::to_value((reencoder.into_table(), root)).unwrap(),
            json
        );
    }

    #[test]
    fn retained_definition_codec_unless_payment_cost_templates_preserve_occurrences() {
        check_owned_cost_template_carrier(false);
    }
    #[test]
    fn retained_definition_codec_delayed_prepayment_templates_preserve_occurrences() {
        check_owned_cost_template_carrier(true);
    }

    #[test]
    fn retained_definition_codec_native_template_contradictions_rollback_before_valid_retry() {
        fn contradict(
            effect: &ironsmith::effect::Effect,
            field: &str,
        ) -> ironsmith::effect::Effect {
            let rewritten = if let Some(token) =
                effect.downcast_ref::<ironsmith::effects::CreateTokenEffect>()
            {
                let mut token = token.clone();
                match field {
                    "name" => token.token.card.name.push_str(" forged"),
                    "power" => {
                        token.token.card.power_toughness =
                            Some(ironsmith::card::PowerToughness::fixed(99, 99))
                    }
                    "text" => token.token.canonical_text.push_str(" forged"),
                    _ => unreachable!(),
                }
                ironsmith::effect::Effect::new(token)
            } else {
                let tagged = effect
                    .downcast_ref::<ironsmith::effects::TaggedEffect>()
                    .expect("compiled token fixture uses token or tag");
                ironsmith::effect::Effect::new(
                    tagged.with_effect(contradict(&tagged.effect, field)),
                )
            };
            rewritten.with_serialized_model(effect.serialized_model().unwrap().to_owned())
        }
        let definition = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(
                ironsmith::CardId::new(),
                "Native template consistency",
            ),
            "Type: Sorcery\nCreate a 1/1 white Soldier creature token with flying.",
            false,
        )
        .unwrap()
        .1;
        let effect = definition.spell_effect.as_ref().unwrap().all_effects()[0];
        let flying = embedded_template_static(effect).unwrap();
        let ids = [definition.card.id, template_id(effect)];
        let bind = |id| {
            ids.iter()
                .position(|value| *value == id)
                .map(|slot| slot as u32)
                .ok_or_else(|| OccurrenceBindingError::InvalidModel {
                    detail: "unknown native consistency node".into(),
                })
        };
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        let seed = encoder
            .retain(ironsmith::static_abilities::StaticAbility::haste())
            .unwrap();
        let before = serde_json::to_value(encoder.clone().into_table()).unwrap();
        for field in ["name", "power", "text"] {
            let mut invalid = definition.clone();
            invalid.spell_effect = Some(ironsmith::resolution::ResolutionProgram::from_effects(
                vec![contradict(effect, field)],
            ));
            assert!(
                matches!(
                    encoder.encode_card_definition(invalid, bind),
                    Err(OccurrenceBindingError::InvalidModel { .. })
                ),
                "native {field} cannot contradict retained executable model"
            );
            assert_eq!(
                serde_json::to_value(encoder.clone().into_table()).unwrap(),
                before,
                "failed native consistency export rolls back {field}"
            );
            assert!(matches!(
                encoder.reference(flying.instance_id()),
                Err(OccurrenceBindingError::UnboundNativeOccurrence)
            ));
        }
        let root = encoder
            .encode_card_definition(definition.clone(), bind)
            .unwrap();
        let reference = encoder.reference(flying.instance_id()).unwrap();
        let json = serde_json::to_value((encoder.into_table(), root)).unwrap();
        let (table, root): (
            RetainedStaticAbilityTable,
            RetainedOccurrenceCardDefinition<u32>,
        ) = serde_json::from_value(json.clone()).unwrap();
        let peers = [ironsmith::CardId::new(), ironsmith::CardId::new()];
        let receive = |slot: u32| {
            peers
                .get(slot as usize)
                .copied()
                .ok_or_else(|| OccurrenceBindingError::InvalidModel {
                    detail: "unknown native consistency peer".into(),
                })
        };
        let decoder =
            StaticAbilityOccurrenceDecoder::restore_with_card_graph::<u32>(table, receive).unwrap();
        let identity = decoder.instance_id(reference).unwrap();
        let root = decoder.restore_card_definition(root, receive).unwrap();
        let alice = ironsmith::PlayerId::from_index(0);
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&root, alice, ironsmith::Zone::Stack);
        let mut context = ironsmith::effects::EffectContext::new_default(source, alice);
        for effect in root.spell_effect.as_ref().unwrap().all_effects() {
            ironsmith::effects::execute_effect(&mut game, effect, &mut context).unwrap();
        }
        assert_eq!(game.battlefield.len(), 1);
        let token = game.object(game.battlefield[0]).unwrap();
        assert_eq!(token.base_power, Some(ironsmith::card::PtValue::Fixed(1)));
        assert!(token.abilities.iter().any(|ability|matches!(&ability.kind,ironsmith::ability::AbilityKind::Static(value) if value.instance_id()==identity)));
        let mut reencoder = StaticAbilityOccurrenceEncoder::default();
        assert_eq!(
            reencoder.retain(decoder.ability(seed).unwrap()).unwrap(),
            seed
        );
        let root = reencoder
            .encode_card_definition(root, |id| {
                peers
                    .iter()
                    .position(|value| *value == id)
                    .map(|slot| slot as u32)
                    .ok_or_else(|| OccurrenceBindingError::InvalidModel {
                        detail: "unknown native consistency reencode node".into(),
                    })
            })
            .unwrap();
        assert_eq!(
            serde_json::to_value((reencoder.into_table(), root)).unwrap(),
            json
        );
    }

    #[test]
    fn retained_definition_codec_effect_granted_program_preserves_template_occurrence() {
        let definition=compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(ironsmith::CardId::new(),"Effect-owned grant template"),
            "Type: Instant\nTarget creature gains \"Pay 1 life: Create a 1/1 white Soldier creature token with flying.\" until end of turn.",false,
        ).unwrap().1;
        let recipient = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(
                ironsmith::CardId::new(),
                "Effect-owned grant recipient",
            ),
            "Type: Creature — Human\nPower/Toughness: 1/1",
            false,
        )
        .unwrap()
        .1;
        let alice = ironsmith::PlayerId::from_index(0);
        let mut native = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let native_source =
            native.create_object_from_definition(&definition, alice, ironsmith::Zone::Stack);
        let native_target =
            native.create_object_from_definition(&recipient, alice, ironsmith::Zone::Battlefield);
        let mut context = ironsmith::effects::EffectContext::new_default(native_source, alice)
            .with_targets(vec![ironsmith::effects::ResolvedTarget::Object(
                native_target,
            )]);
        for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
            ironsmith::effects::execute_effect(&mut native, effect, &mut context).unwrap();
        }
        let activated = native
            .current_abilities(native_target)
            .unwrap()
            .into_iter()
            .find_map(|ability| {
                if let ironsmith::ability::AbilityKind::Activated(value) = ability.kind {
                    Some(value)
                } else {
                    None
                }
            })
            .expect("actual native resolution must grant activated token program");
        let flying = embedded_template_static(activated.effects.all_effects()[0]).unwrap();
        let ids = [
            definition.card.id,
            template_id(activated.effects.all_effects()[0]),
        ];
        let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
        let mut cost_context = ironsmith::costs::CostContext::new(native_target, alice, &mut dm);
        for cost in activated.mana_cost.costs() {
            cost.pay(&mut native, &mut cost_context).unwrap();
        }
        let mut context = ironsmith::effects::EffectContext::new_default(native_target, alice);
        for effect in activated.effects.all_effects() {
            ironsmith::effects::execute_effect(&mut native, effect, &mut context).unwrap();
        }
        assert_eq!(native.player(alice).unwrap().life, 19);
        assert!(native.object(*native.battlefield.last().unwrap()).unwrap().abilities.iter().any(|ability|matches!(&ability.kind,ironsmith::ability::AbilityKind::Static(value)if value.instance_id()==flying.instance_id())));
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        let root = encoder
            .encode_card_definition(definition, |id| {
                ids.iter()
                    .position(|candidate| *candidate == id)
                    .map(|index| index as u32)
                    .ok_or_else(|| OccurrenceBindingError::InvalidModel {
                        detail: "unknown effect grant graph node".into(),
                    })
            })
            .unwrap();
        let reference = encoder.reference(flying.instance_id());
        assert!(
            reference.is_ok(),
            "effect-owned granted program must retain its native template occurrence: {:?}",
            reference.err()
        );
        let reference = reference.unwrap();
        let json = serde_json::to_value((encoder.into_table(), root)).unwrap();
        let (table, root): (
            RetainedStaticAbilityTable,
            RetainedOccurrenceCardDefinition<u32>,
        ) = serde_json::from_value(json.clone()).unwrap();
        let peers = [ironsmith::CardId::new(), ironsmith::CardId::new()];
        let receive = |index: u32| {
            peers
                .get(index as usize)
                .copied()
                .ok_or_else(|| OccurrenceBindingError::InvalidModel {
                    detail: "missing effect grant peer".into(),
                })
        };
        let decoder =
            StaticAbilityOccurrenceDecoder::restore_with_card_graph::<u32>(table, receive).unwrap();

        // Reject broken associations at the owning spell-program boundary
        // before the unchanged valid receiver/gameplay path is exercised.
        assert_eq!(
            root.spell_effect
                .as_ref()
                .unwrap()
                .embedded_definitions
                .len(),
            1
        );
        let mut missing = root.clone();
        missing
            .spell_effect
            .as_mut()
            .unwrap()
            .embedded_definitions
            .clear();
        assert!(matches!(
            decoder.restore_card_definition(missing, receive),
            Err(OccurrenceBindingError::InvalidModel { .. })
        ));
        let mut extra = root.clone();
        let record = extra.spell_effect.as_ref().unwrap().embedded_definitions[0].clone();
        extra
            .spell_effect
            .as_mut()
            .unwrap()
            .embedded_definitions
            .push(record);
        assert!(matches!(
            decoder.restore_card_definition(extra, receive),
            Err(OccurrenceBindingError::InvalidModel { .. })
        ));
        let mut mismatch = root.clone();
        mismatch.spell_effect.as_mut().unwrap().embedded_definitions[0]
            .model
            .canonical_text
            .push_str(" wrong effect-owned template");
        assert!(matches!(
            decoder.restore_card_definition(mismatch, receive),
            Err(OccurrenceBindingError::InvalidModel { .. })
        ));
        let mut unknown = root.clone();
        let ability = &mut unknown.spell_effect.as_mut().unwrap().embedded_definitions[0]
            .snapshot
            .abilities[0];
        ability.model = ability
            .model
            .clone()
            .try_map(
                |_| Ok::<_, std::convert::Infallible>(StaticAbilityOccurrenceRef(u32::MAX)),
                Ok,
                Ok,
                Ok,
                Ok,
            )
            .unwrap();
        assert!(matches!(
            decoder.restore_card_definition(unknown, receive),
            Err(OccurrenceBindingError::UnknownReference {
                reference: StaticAbilityOccurrenceRef(u32::MAX)
            })
        ));

        for field in ["name", "power", "text"] {
            let mut invalid = root.clone();
            let snapshot =
                &mut invalid.spell_effect.as_mut().unwrap().embedded_definitions[0].snapshot;
            match field {
                "name" => snapshot.card.name.push_str(" forged"),
                "power" => {
                    snapshot.card.power_toughness = Some(ironsmith::card::PowerToughness::fixed(99, 99))
                }
                "text" => snapshot.canonical_text.push_str(" forged"),
                _ => unreachable!(),
            }
            assert!(
                matches!(
                    decoder.restore_card_definition(invalid, receive),
                    Err(OccurrenceBindingError::InvalidModel { .. })
                ),
                "retained {field} cannot contradict executable template"
            );
        }

        let identity = decoder.instance_id(reference).unwrap();
        let root = decoder.restore_card_definition(root, receive).unwrap();
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&root, alice, ironsmith::Zone::Stack);
        let target =
            game.create_object_from_definition(&recipient, alice, ironsmith::Zone::Battlefield);
        let mut context = ironsmith::effects::EffectContext::new_default(source, alice)
            .with_targets(vec![ironsmith::effects::ResolvedTarget::Object(target)]);
        for effect in root.spell_effect.as_ref().unwrap().all_effects() {
            ironsmith::effects::execute_effect(&mut game, effect, &mut context).unwrap();
        }
        let activated = game
            .current_abilities(target)
            .unwrap()
            .into_iter()
            .find_map(|ability| {
                if let ironsmith::ability::AbilityKind::Activated(value) = ability.kind {
                    Some(value)
                } else {
                    None
                }
            })
            .unwrap();
        assert_eq!(template_id(activated.effects.all_effects()[0]), peers[1]);
        assert_eq!(
            embedded_template_static(activated.effects.all_effects()[0])
                .unwrap()
                .instance_id(),
            identity
        );
        let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
        let mut cost_context = ironsmith::costs::CostContext::new(target, alice, &mut dm);
        for cost in activated.mana_cost.costs() {
            cost.pay(&mut game, &mut cost_context).unwrap();
        }
        let mut context = ironsmith::effects::EffectContext::new_default(target, alice);
        for effect in activated.effects.all_effects() {
            ironsmith::effects::execute_effect(&mut game, effect, &mut context).unwrap();
        }
        assert_eq!(game.player(alice).unwrap().life, 19);
        assert!(game.object(*game.battlefield.last().unwrap()).unwrap().abilities.iter().any(|ability|matches!(&ability.kind,ironsmith::ability::AbilityKind::Static(value)if value.instance_id()==identity)));
        let mut reencoder = StaticAbilityOccurrenceEncoder::default();
        let root = reencoder
            .encode_card_definition(root, |id| {
                peers
                    .iter()
                    .position(|candidate| *candidate == id)
                    .map(|index| index as u32)
                    .ok_or_else(|| OccurrenceBindingError::InvalidModel {
                        detail: "unknown effect grant reencode node".into(),
                    })
            })
            .unwrap();
        assert_eq!(
            serde_json::to_value((reencoder.into_table(), root)).unwrap(),
            json
        );
    }

    #[test]
    fn retained_definition_codec_nested_definition_metadata_survives_graph_roundtrip() {
        let (artifact, native) = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Nested metadata producer"),
            "Type: Artifact\nPay 1 life: You gain 2 life.", false,
        ).unwrap();
        assert!(!native.canonical_text.is_empty()); assert!(!native.ability_labels.is_empty());
        let mut template = artifact.payload.definition;
        template.card.is_token = true;
        template.canonical_text = native.canonical_text.clone();
        template.ability_labels = native.ability_labels.clone();
        let native_id = template.card.id;
        let raw = serde_json::to_value(&template).unwrap();
        for field in ["canonical_text", "ability_labels"] {
            let mut invalid = raw.clone(); invalid.as_object_mut().unwrap().remove(field);
            assert!(serde_json::from_value::<ironsmith_compiled_artifact::WireCardDefinition>(invalid).is_err(), "missing definition metadata must not invent an empty value");
        }
        assert_eq!(raw["canonical_text"], serde_json::json!(native.canonical_text), "nested executable definitions cannot drop canonical text");
        assert_eq!(raw["ability_labels"], serde_json::json!(native.ability_labels), "nested executable definitions cannot drop ability labels");
        let producer = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Metadata token instruction"),
            "Type: Sorcery\nCreate a 1/1 white Soldier creature token.", false,
        ).unwrap().1;
        let base = encode_runtime_effect(producer.spell_effect.unwrap().all_effects()[0].clone()).unwrap();
        let mut payload = base.payload().clone();
        let kind;
        if base.kind() == "TaggedEffect" {
            let mut child: ironsmith_compiled_artifact::WireEffect = serde_json::from_value(payload["effect"].clone()).unwrap();
            let mut token = child.payload().clone(); token["token"] = raw;
            child = ironsmith_compiled_artifact::WireEffect::new(child.kind(), token);
            payload["effect"] = serde_json::to_value(child).unwrap(); kind = base.kind().to_string();
        } else {
            payload["token"] = raw; kind = base.kind().to_string();
        }
        let effect = materialize_effect(ironsmith_compiled_artifact::WireEffect::new(kind, payload)).unwrap();
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        let wire = encoder.encode_program_with_card_graph(ironsmith::resolution::ResolutionProgram::from_effects(vec![effect]), |id| {
            assert_eq!(id, native_id); Ok(23u32)
        }).unwrap();
        let json = serde_json::to_value((encoder.into_table(), wire)).unwrap();
        let (table, wire): (RetainedStaticAbilityTable, RetainedOccurrenceProgram) = serde_json::from_value(json).unwrap();
        let peer = ironsmith::CardId::new();
        let decoder = StaticAbilityOccurrenceDecoder::restore_with_card_graph::<u32>(table, |id| { assert_eq!(id,23); Ok(peer) }).unwrap();
        let program = decoder.restore_program(wire).unwrap();
        assert_eq!(template_id(program.all_effects()[0]), peer);
        let alice = ironsmith::PlayerId::from_index(0);
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()],20);
        let source = game.create_object_from_definition(&native, alice, ironsmith::Zone::Stack);
        let mut context = ironsmith::effects::EffectContext::new_default(source,alice);
        for effect in program.all_effects() { ironsmith::effects::execute_effect(&mut game,effect,&mut context).unwrap(); }
        assert_eq!(game.battlefield.len(),1);
        let token = game.object(game.battlefield[0]).unwrap();
        assert_eq!(token.compiled_card_text.as_ref(),native.canonical_text);
        assert_eq!(token.ability_labels,native.ability_labels);
        let activated = token.abilities.iter().find_map(|a| if let ironsmith::ability::AbilityKind::Activated(v)=&a.kind {Some(v.clone())}else{None}).unwrap();
        let recipient = token.id;
        let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
        let mut cost = ironsmith::costs::CostContext::new(recipient,alice,&mut dm);
        for c in activated.mana_cost.costs() { c.pay(&mut game,&mut cost).unwrap(); }
        assert_eq!(game.player(alice).unwrap().life,19); drop(cost);
        let mut context = ironsmith::effects::EffectContext::new(recipient,alice,&mut dm);
        for e in activated.effects.all_effects() { ironsmith::effects::execute_effect(&mut game,e,&mut context).unwrap(); }
        assert_eq!(game.player(alice).unwrap().life,21);
    }

    fn standalone_copy_graph_roundtrip(kind: u8) {
        let definition = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Copy graph producer"),
            "Type: Artifact\nPay 1 life: Create a 1/1 white Soldier creature token.", false,
        ).unwrap().1;
        let alice = ironsmith::PlayerId::from_index(0);
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, alice, ironsmith::Zone::Battlefield);
        let native = definition.abilities.iter().find(|a| matches!(a.kind, ironsmith::ability::AbilityKind::Activated(_))).unwrap().clone();
        let ironsmith::ability::AbilityKind::Activated(activated) = &native.kind else { panic!("activated") };
        let mut context = ironsmith::effects::EffectContext::new_default(source, alice);
        for effect in activated.effects.all_effects() {
            ironsmith::effects::execute_effect(&mut game, effect, &mut context).unwrap();
        }
        assert_eq!(game.battlefield.len(), 2, "native token control must execute");
        let copy = ironsmith::snapshot::CopiableValues::from_object(game.object(source).unwrap());
        let text = ironsmith::continuous::TextBoxOverlay::new(copy.compiled_card_text.clone(), copy.abilities.as_ref().clone())
            .with_ability_labels(copy.ability_labels.clone());
        let native_template = template_id(activated.effects.all_effects()[0]);
        let modification = ironsmith::continuous::Modification::SetTextBox(text.clone());
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        let static_ref = encoder.retain(ironsmith::static_abilities::StaticAbility::haste()).unwrap();
        let before = serde_json::to_value(encoder.clone().into_table()).unwrap();
        let mut missing = |id| {
            assert_eq!(id, native_template);
            Err::<u32, _>(OccurrenceBindingError::InvalidModel { detail: "missing owner template".into() })
        };
        let rejected = match kind {
            0 => encoder.encode_copy_values_with_card_graph(copy.clone(), &mut missing).map(|_| ()),
            1 => encoder.encode_text_overlay_with_card_graph(text.clone(), &mut missing).map(|_| ()),
            _ => encoder.encode_modification_with_card_graph(modification.clone(), &mut missing).map(|_| ()),
        };
        assert!(rejected.is_err());
        assert_eq!(serde_json::to_value(encoder.clone().into_table()).unwrap(), before);
        let raw = match kind {
            0 => serde_json::to_value(encoder.encode_copy_values(copy.clone()).unwrap()).unwrap(),
            1 => serde_json::to_value(encoder.encode_text_overlay(text.clone()).unwrap()).unwrap(),
            _ => serde_json::to_value(encoder.encode_modification(modification.clone()).unwrap()).unwrap(),
        };
        let mut bind = |id| { assert_eq!(id, native_template); Ok(17u32) };
        let payload = match kind {
            0 => serde_json::to_value(encoder.encode_copy_values_with_card_graph(copy, &mut bind).unwrap()).unwrap(),
            1 => serde_json::to_value(encoder.encode_text_overlay_with_card_graph(text, &mut bind).unwrap()).unwrap(),
            _ => serde_json::to_value(encoder.encode_modification_with_card_graph(modification, &mut bind).unwrap()).unwrap(),
        };
        let accepted = serde_json::to_value(encoder.clone().into_table()).unwrap();
        assert_eq!(accepted["payload_card_references"], serde_json::json!([17]));
        // A later inconsistent owner must not rewrite an already accepted frame.
        let conflict = ironsmith::continuous::TextBoxOverlay::new("", vec![native]);
        assert!(encoder.encode_text_overlay_with_card_graph(conflict, |_| Ok(18u32)).is_err());
        assert_eq!(serde_json::to_value(encoder.clone().into_table()).unwrap(), accepted);
        let json = serde_json::to_value((encoder.into_table(), payload)).unwrap();
        let (table, payload): (RetainedStaticAbilityTable, serde_json::Value) = serde_json::from_value(json.clone()).unwrap();
        let peer = ironsmith::CardId::new(); assert_ne!(peer, native_template);
        let decoder = StaticAbilityOccurrenceDecoder::restore_with_card_graph::<u32>(table, |id| {
            assert_eq!(id, 17); Ok(peer)
        }).unwrap();
        let raw_rejected = match kind {
            0 => decoder.restore_copy_values(serde_json::from_value(raw).unwrap()).map(|_| ()),
            1 => decoder.restore_text_overlay(serde_json::from_value(raw).unwrap()).map(|_| ()),
            _ => decoder.restore_modification(serde_json::from_value(raw).unwrap()).map(|_| ()),
        };
        assert!(raw_rejected.is_err(), "native IDs must remain rejected even with a populated frame");
        let mut reencoder = StaticAbilityOccurrenceEncoder::default();
        assert_eq!(reencoder.retain(decoder.ability(static_ref).unwrap()).unwrap(), static_ref);
        let mut rebind = |id| { assert_eq!(id, peer); Ok(17u32) };
        let (applied, reencoded) = match kind {
            0 => {
                let restored = decoder.restore_copy_values(serde_json::from_value(payload).unwrap()).unwrap();
                let wire = reencoder.encode_copy_values_with_card_graph(restored.clone(), &mut rebind).unwrap();
                (ironsmith::continuous::Modification::CopyOf {
                    target_id: source, copiable_values: Box::new(restored), preserve_source_abilities: false,
                    name_override: None, name_override_surface: None, add_supertypes: vec![],
                }, serde_json::to_value(wire).unwrap())
            },
            1 => {
                let restored = decoder.restore_text_overlay(serde_json::from_value(payload).unwrap()).unwrap();
                let wire = reencoder.encode_text_overlay_with_card_graph(restored.clone(), &mut rebind).unwrap();
                (ironsmith::continuous::Modification::SetTextBox(restored), serde_json::to_value(wire).unwrap())
            },
            _ => {
                let restored = decoder.restore_modification(serde_json::from_value(payload).unwrap()).unwrap();
                let wire = reencoder.encode_modification_with_card_graph(restored.clone(), &mut rebind).unwrap();
                (restored, serde_json::to_value(wire).unwrap())
            },
        };
        assert_eq!(serde_json::to_value((reencoder.into_table(), reencoded)).unwrap(), json);
        let target_definition = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Copy graph recipient"), "Type: Artifact", false,
        ).unwrap().1;
        let recipient = game.create_object_from_definition(&target_definition, alice, ironsmith::Zone::Battlefield);
        game.effect_store.continuous_effects.add_effect(ironsmith::continuous::ContinuousEffect::from_resolution(
            source, alice, vec![recipient], applied,
        ).until(ironsmith::Until::EndOfTurn));
        game.refresh_continuous_state().unwrap();
        let abilities = game.current_abilities(recipient).unwrap();
        let activated = abilities.iter().find_map(|ability| {
            if let ironsmith::ability::AbilityKind::Activated(value) = &ability.kind { Some(value.clone()) } else { None }
        }).unwrap();
        assert_eq!(template_id(activated.effects.all_effects()[0]), peer);
        let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
        let mut context = ironsmith::costs::CostContext::new(recipient, alice, &mut dm);
        for cost in activated.mana_cost.costs() { cost.pay(&mut game, &mut context).unwrap(); }
        assert_eq!(game.player(alice).unwrap().life, 19);
        drop(context);
        let previous = game.battlefield.len();
        let mut context = ironsmith::effects::EffectContext::new(recipient, alice, &mut dm);
        for effect in activated.effects.all_effects() {
            ironsmith::effects::execute_effect(&mut game, effect, &mut context).unwrap();
        }
        assert_eq!(game.battlefield.len(), previous + 1);
        let token = game.object(*game.battlefield.last().unwrap()).unwrap();
        assert_eq!(token.kind, ironsmith::object::ObjectKind::Token);
        assert_eq!(token.base_power, Some(ironsmith::card::PtValue::Fixed(1)));
        assert_eq!(token.base_toughness, Some(ironsmith::card::PtValue::Fixed(1)));
    }
    #[test]
    fn retained_definition_codec_standalone_copy_supports_template_graph() { standalone_copy_graph_roundtrip(0); }
    #[test]
    fn retained_definition_codec_standalone_text_supports_template_graph() { standalone_copy_graph_roundtrip(1); }
    #[test]
    fn retained_definition_codec_standalone_modification_supports_template_graph() { standalone_copy_graph_roundtrip(2); }

    fn assert_permission_graph_binding_is_consistent(changed: bool) {
        let definition = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(
                ironsmith::CardId::new(),
                "Permission graph owner",
            ),
            "Type: Sorcery\nCreate a 1/1 white Soldier creature token.",
            false,
        )
        .unwrap()
        .1;
        let ids = [
            definition.card.id,
            template_id(definition.spell_effect.as_ref().unwrap().all_effects()[0]),
        ];
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        encoder
            .encode_card_definition(definition, |id| {
                ids.iter()
                    .position(|candidate| *candidate == id)
                    .map(|index| index as u32)
                    .ok_or_else(|| OccurrenceBindingError::InvalidModel {
                        detail: "unknown permission owner node".into(),
                    })
            })
            .unwrap();
        let face = if changed {
            ids[0]
        } else {
            ironsmith::CardId::new()
        };
        let permission = ironsmith::grant_registry::GrantPermissionIdentity::LinkedFace {
            source: ironsmith::ObjectId::from_raw(71771),
            face,
            slot: 2,
        };
        let before = serde_json::to_value(encoder.clone().into_table()).unwrap();
        let result = encoder.encode_permission_identity(permission, |id| {
            assert_eq!(id, face);
            Ok(if changed { 1u32 } else { 0u32 })
        });
        assert!(
            result.is_err(),
            "permission card references must use shared graph consistency checks; changed={changed}"
        );
        assert_eq!(serde_json::to_value(encoder.into_table()).unwrap(), before);
    }

    #[test]
    fn retained_definition_codec_permission_cannot_change_existing_card_graph_binding() {
        assert_permission_graph_binding_is_consistent(true);
    }

    #[test]
    fn retained_definition_codec_permission_cannot_collapse_distinct_card_graph_nodes() {
        assert_permission_graph_binding_is_consistent(false);
    }


    fn check_stack_program_cache_identity(captured: bool) {
        let id = ironsmith::CardId::new();
        let first = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(id, "Independent printed program"),
            "Type: Sorcery\nCreate a 1/1 white Soldier creature token with flying.",
            false,
        )
        .unwrap()
        .1;
        let mut second = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(id, "Independent printed program"),
            "Type: Sorcery\nCreate a 1/1 white Soldier creature token with flying.",
            false,
        )
        .unwrap()
        .1;
        let original = first.spell_effect.as_ref().unwrap();
        let token_id = template_id(original.all_effects()[0]);
        second.spell_effect = Some(
            second
                .spell_effect
                .take()
                .unwrap()
                .try_map_effects(|effect| {
                    Ok::<_, std::convert::Infallible>(replace_token_template_id(effect, token_id))
                })
                .unwrap(),
        );
        let flying = embedded_template_static(original.all_effects()[0]).unwrap();
        let other_flying =
            embedded_template_static(second.spell_effect.as_ref().unwrap().all_effects()[0])
                .unwrap();
        assert_ne!(flying.instance_id(), other_flying.instance_id());
        assert_eq!(
            encode_runtime_effect(original.all_effects()[0].clone()).unwrap(),
            encode_runtime_effect(second.spell_effect.as_ref().unwrap().all_effects()[0].clone())
                .unwrap()
        );
        for method_index in 0..3 {
            let alice = ironsmith::PlayerId::from_index(0);
            let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let source = game.create_object_from_definition(&first, alice, ironsmith::Zone::Stack);
            game.create_object_from_definition(&second, alice, ironsmith::Zone::Hand);
            let object = game.object_mut(source).unwrap();
            let effects = vec![ironsmith::effect::Effect::gain_life(2)];
            object.cast_alternative_method = Some(Box::new(match method_index {
                0 => ironsmith::alternative_cast::AlternativeCastingMethod::Overload {
                    cost: ironsmith::mana::ManaCost::new(),
                    effects,
                },
                1 => ironsmith::alternative_cast::AlternativeCastingMethod::Cleave {
                    cost: ironsmith::mana::ManaCost::new(),
                    effects,
                },
                _ => ironsmith::alternative_cast::AlternativeCastingMethod::Awaken {
                    amount: 3,
                    cost: ironsmith::mana::ManaCost::new(),
                    effects,
                },
            }));
            if captured {
                assert!(object.begin_splice_cast_overlay());
                object.spell_effect = Some(
                    ironsmith::resolution::ResolutionProgram::from_effects(vec![
                        ironsmith::effect::Effect::gain_life(2),
                    ])
                    .into(),
                );
            }
            let destination = game
                .move_object(
                    source,
                    ironsmith::Zone::Graveyard,
                    ironsmith::events::cause::EventCause::effect(),
                )
                .unwrap();
            let restored = game.object(destination).unwrap();
            assert!(restored.cast_alternative_method.is_none());
            assert!(restored.splice_cast_state.is_none());
            let effect = restored.spell_effect.as_ref().unwrap().all_effects()[0].clone();
            assert_eq!(
                embedded_template_static(&effect).unwrap().instance_id(),
                flying.instance_id(),
                "stack exit must restore this object's own program, not the latest same-CardId cache entry (method {method_index}, captured {captured})"
            );
            let mut context = ironsmith::effects::EffectContext::new_default(destination, alice);
            ironsmith::effects::execute_effect(&mut game, &effect, &mut context).unwrap();
            assert_eq!(game.battlefield.len(), 1);
            let token = game.object(game.battlefield[0]).unwrap();
            assert!(token.abilities.iter().any(|ability|matches!(&ability.kind,
                ironsmith::ability::AbilityKind::Static(value) if value.instance_id()==flying.instance_id())));
            assert_eq!(game.player(alice).unwrap().life, 20);
        }
    }

    #[test]
    fn retained_definition_codec_stack_exit_preserves_native_program_without_snapshot() {
        check_stack_program_cache_identity(false);
    }

    #[test]
    fn retained_definition_codec_stack_exit_preserves_captured_native_program() {
        check_stack_program_cache_identity(true);
    }

    fn check_registry_native_definition_identity(linked: bool) {
        let first = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(
                ironsmith::CardId::new(),
                "Independent registered producer",
            ),
            "Type: Sorcery\nCreate a 1/1 white Soldier creature token with flying.",
            false,
        )
        .unwrap()
        .1;
        let second = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(
                ironsmith::CardId::new(),
                "Independent registered producer",
            ),
            "Type: Sorcery\nCreate a 1/1 white Soldier creature token with flying.",
            false,
        )
        .unwrap()
        .1;
        let first_flying =
            embedded_template_static(first.spell_effect.as_ref().unwrap().all_effects()[0])
                .unwrap();
        let second_flying =
            embedded_template_static(second.spell_effect.as_ref().unwrap().all_effects()[0])
                .unwrap();
        assert_ne!(first_flying.instance_id(), second_flying.instance_id());
        let mut registry = ironsmith::cards::CardRegistry::new();
        registry.register(first.clone());
        registry.register(second.clone());
        assert_eq!(
            registry.get(&first.card.name).unwrap().card.id,
            second.card.id,
            "name lookup selects latest definition"
        );
        registry.register_alias("Registered alias", first.card.name.clone());
        assert_eq!(
            registry.get("Registered alias").unwrap().card.id,
            second.card.id
        );
        let restored = if linked {
            registry
                .linked_face_definition_by_name_or_id(Some(&first.card.name), Some(first.card.id))
                .unwrap()
        } else {
            registry.get_by_id(first.card.id).unwrap()
        };
        assert_eq!(
            restored.card.id, first.card.id,
            "identity lookup cannot return another same-name definition"
        );
        assert_eq!(
            embedded_template_static(restored.spell_effect.as_ref().unwrap().all_effects()[0])
                .unwrap()
                .instance_id(),
            first_flying.instance_id()
        );
        assert_eq!(
            registry.get_by_id(second.card.id).unwrap().card.id,
            second.card.id
        );
        let alice = ironsmith::PlayerId::from_index(0);
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(restored, alice, ironsmith::Zone::Stack);
        let effect = restored.spell_effect.as_ref().unwrap().all_effects()[0].clone();
        let mut context = ironsmith::effects::EffectContext::new_default(source, alice);
        ironsmith::effects::execute_effect(&mut game, &effect, &mut context).unwrap();
        assert_eq!(game.battlefield.len(), 1);
        assert!(game.object(game.battlefield[0]).unwrap().abilities.iter().any(|ability|matches!(&ability.kind,
            ironsmith::ability::AbilityKind::Static(value) if value.instance_id()==first_flying.instance_id())));
        let snapshot = registry.clone();
        let mut renamed = first.clone();
        renamed.card.name = "Renamed registered producer".into();
        renamed.spell_effect = Some(ironsmith::resolution::ResolutionProgram::from_effects(
            vec![ironsmith::effect::Effect::gain_life(3)],
        ));
        registry.register(renamed.clone());
        assert_eq!(
            registry.get_by_id(first.card.id).unwrap().card.name,
            renamed.card.name,
            "re-registering ID updates its definition despite name change"
        );
        assert_eq!(
            snapshot.get_by_id(first.card.id).unwrap().card.name,
            first.card.name,
            "cloned registry retains original native definition"
        );
        assert_eq!(
            embedded_template_static(
                snapshot
                    .get_by_id(first.card.id)
                    .unwrap()
                    .spell_effect
                    .as_ref()
                    .unwrap()
                    .all_effects()[0]
            )
            .unwrap()
            .instance_id(),
            first_flying.instance_id()
        );
        assert_eq!(
            registry
                .linked_face_definition_by_name_or_id(Some(&second.card.name), Some(first.card.id))
                .unwrap()
                .card
                .id,
            second.card.id,
            "a different explicit face name selects the requested face"
        );
    }

    #[test]
    fn retained_definition_codec_registry_id_lookup_retains_independent_native_programs() {
        check_registry_native_definition_identity(false);
    }

    #[test]
    fn retained_definition_codec_registry_linked_lookup_retains_independent_native_programs() {
        check_registry_native_definition_identity(true);
    }
    fn replace_token_template_id(
        effect: ironsmith::effect::Effect,
        id: ironsmith::CardId,
    ) -> ironsmith::effect::Effect {
        let wire = encode_runtime_effect(effect).unwrap();
        let replaced = if wire.kind() == "TaggedEffect" {
            let mut tagged: compiler::effects::CoreTaggedEffect<
                ironsmith_compiled_artifact::WireEffect,
            > = serde_json::from_value(wire.payload().clone()).unwrap();
            let mut token: compiler::effects::CoreCreateTokenEffect<
                ironsmith_compiled_artifact::WireCardDefinition,
            > = serde_json::from_value(tagged.effect.payload().clone()).unwrap();
            token.token.card.id = id;
            tagged.effect = Box::new(ironsmith_compiled_artifact::WireEffect::new(
                "CreateTokenEffect",
                serde_json::to_value(token).unwrap(),
            ));
            ironsmith_compiled_artifact::WireEffect::new(
                "TaggedEffect",
                serde_json::to_value(tagged).unwrap(),
            )
        } else {
            assert_eq!(wire.kind(), "CreateTokenEffect");
            let mut token: compiler::effects::CoreCreateTokenEffect<
                ironsmith_compiled_artifact::WireCardDefinition,
            > = serde_json::from_value(wire.payload().clone()).unwrap();
            token.token.card.id = id;
            ironsmith_compiled_artifact::WireEffect::new(
                "CreateTokenEffect",
                serde_json::to_value(token).unwrap(),
            )
        };
        materialize_effect(replaced).unwrap()
    }

    #[test]
    fn retained_definition_codec_late_native_static_id_cannot_alias_payload_slot() {
        let definition = compile_builder_to_artifact(
            compiler::CardDefinitionBuilder::new(ironsmith::CardId::new(), "Bound producer"),
            "Type: Sorcery\nCreate a 1/1 white Soldier creature token.",
            false,
        )
        .unwrap()
        .1;
        let ids = [
            definition.card.id,
            template_id(definition.spell_effect.as_ref().unwrap().all_effects()[0]),
        ];
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        encoder
            .encode_card_definition(definition.clone(), |id| {
                ids.iter()
                    .position(|candidate| *candidate == id)
                    .map(|index| index as u32)
                    .ok_or_else(|| OccurrenceBindingError::InvalidModel {
                        detail: "unbound producer node".into(),
                    })
            })
            .unwrap();
        let (_, grant, _) = static_template_fixture();
        let model = grant
            .compiled_model()
            .unwrap()
            .clone()
            .try_map(
                Ok::<_, std::convert::Infallible>,
                |effect| {
                    Ok(replace_token_template_id(
                        effect,
                        ironsmith::CardId::from_raw(0),
                    ))
                },
                Ok,
                Ok,
            )
            .unwrap();
        let late = ironsmith::static_abilities::StaticAbility::from_model(model);
        assert_eq!(
            template_id(&static_template_effect(&late)),
            ironsmith::CardId::from_raw(0)
        );
        let reference = encoder.retain(late).unwrap();
        let table = encoder.clone().into_table();
        assert_eq!(table.payload_card_references, vec![serde_json::json!(1u32)]);
        let peer = ironsmith::CardId::new();
        let result =
            StaticAbilityOccurrenceDecoder::restore_with_card_graph::<u32>(table, |reference| {
                assert_eq!(reference, 1);
                Ok(peer)
            });
        assert!(
            result.is_err(),
            "a late native CardId zero must not be interpreted as framed slot zero"
        );
        // A later owning export can bind that exact native node explicitly.
        encoder
            .encode_card_definition(definition, |id| {
                if id == ironsmith::CardId::from_raw(0) {
                    Ok(2u32)
                } else {
                    ids.iter()
                        .position(|candidate| *candidate == id)
                        .map(|index| index as u32)
                        .ok_or_else(|| OccurrenceBindingError::InvalidModel {
                            detail: "unknown late graph node".into(),
                        })
                }
            })
            .unwrap();
        let table = encoder.into_table();
        assert_eq!(
            table.model_card_references,
            vec![RetainedModelCardReferences::Bound]
        );
        assert_eq!(
            table.payload_card_references,
            vec![serde_json::json!(1u32), serde_json::json!(2u32)]
        );
        let late_peer = ironsmith::CardId::new();
        assert_ne!(peer, late_peer);
        let decoder = StaticAbilityOccurrenceDecoder::restore_with_card_graph::<u32>(
            table,
            |slot| match slot {
                1 => Ok(peer),
                2 => Ok(late_peer),
                _ => Err(OccurrenceBindingError::InvalidModel {
                    detail: "unknown late receiver node".into(),
                }),
            },
        )
        .unwrap();
        assert_eq!(
            template_id(&static_template_effect(
                &decoder.ability(reference).unwrap()
            )),
            late_peer
        );
    }
    #[test]
    fn retained_definition_codec_requires_all_fields_and_rejects_bad_graph_or_payload() {
        let (front, back) = fixture();
        let front_id = front.card.id;
        let back_id = back.card.id;
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        let mut wire = encoder
            .encode_card_definition(front.clone(), |id| {
                Ok(if id == front_id {
                    0u8
                } else {
                    assert_eq!(id, back_id);
                    1u8
                })
            })
            .unwrap();
        let value = serde_json::to_value(&wire).unwrap();
        assert_eq!(value.as_object().unwrap().len(), 11);
        assert_eq!(value["card"].as_object().unwrap().len(), 20);
        for field in value.as_object().unwrap().keys() {
            let mut missing = value.clone();
            missing.as_object_mut().unwrap().remove(field);
            assert!(
                serde_json::from_value::<RetainedOccurrenceCardDefinition<u8>>(missing).is_err(),
                "missing definition {field} must reject"
            );
        }
        for field in value["card"].as_object().unwrap().keys() {
            let mut missing = value.clone();
            missing["card"].as_object_mut().unwrap().remove(field);
            assert!(
                serde_json::from_value::<RetainedOccurrenceCardDefinition<u8>>(missing).is_err(),
                "missing card {field} must reject"
            );
        }
        for field in [
            "first_printed_set_name",
            "mana_cost",
            "color_indicator",
            "power_toughness",
            "loyalty",
            "defense",
            "other_face",
            "other_face_name",
        ] {
            let mut absent = value.clone();
            absent["card"][field] = serde_json::Value::Null;
            let restored: RetainedOccurrenceCardDefinition<u8> =
                serde_json::from_value(absent.clone()).unwrap();
            assert_eq!(serde_json::to_value(restored).unwrap(), absent);
        }
        for field in ["spell_effect", "aura_attach_filter"] {
            let mut absent = value.clone();
            absent[field] = serde_json::Value::Null;
            let restored: RetainedOccurrenceCardDefinition<u8> =
                serde_json::from_value(absent.clone()).unwrap();
            assert_eq!(serde_json::to_value(restored).unwrap(), absent);
        }
        let decoder = StaticAbilityOccurrenceDecoder::restore(encoder.into_table()).unwrap();
        assert!(
            decoder
                .restore_card_definition(wire.clone(), |_| Err(
                    OccurrenceBindingError::InvalidModel {
                        detail: "unknown face".into()
                    }
                ))
                .is_err()
        );
        let last = wire.abilities.last_mut().unwrap();
        last.model = last.model
            .clone()
            .try_map(
                |_| Ok::<_, ()>(StaticAbilityOccurrenceRef(99)),
                Ok,
                Ok,
                Ok,
                Ok,
            )
            .unwrap();
        assert!(
            decoder
                .restore_card_definition(wire, |reference| Ok(if reference == 0 {
                    front_id
                } else {
                    back_id
                }))
                .is_err()
        );
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        encoder
            .encode_ability(ironsmith::ability::Ability::static_ability(
                ironsmith::static_abilities::StaticAbility::haste(),
            ))
            .unwrap();
        let before = serde_json::to_value(encoder.clone().into_table()).unwrap();
        let mut malformed = front;
        let mut stale =
            ironsmith::costs::Cost::mana(ironsmith::mana::ManaCost::from_pips(Vec::new()));
        stale.0 = ironsmith::costs::Cost::life(1).0;
        malformed.additional_cost = ironsmith::cost::TotalCost::from_cost(stale);
        assert!(
            encoder
                .encode_card_definition(malformed, |id| Ok(if id == front_id { 0u8 } else { 1u8 }))
                .is_err()
        );
        assert_eq!(
            serde_json::to_value(encoder.into_table()).unwrap(),
            before,
            "later cost failure rolls back new definition ability occurrences"
        );
    }
}

#[cfg(test)]
mod retained_replacement_identity_codec_tests {
    use ironsmith::continuous::{AbilityEffectOrigin, AbilityOrigin, ContinuousAbilityOrigin,
        ContinuousEffect, EffectTarget, Modification};
    use ironsmith::events::WouldGainLifeMatcher;
    use ironsmith::replacement::{ReplacementAbilityOrigin, ReplacementAction, ReplacementEffect,
        ReplacementEffectKey};
    use ironsmith::static_abilities::StaticAbility;
    use ironsmith::{CardId, ObjectId, PlayerId};
    use ironsmith_runtime_catalog::artifact_materializer::{OccurrenceBindingError,
        RetainedOccurrenceReplacementKey, RetainedOccurrenceReplacementOrigin,
        RetainedStaticAbilityTable, StaticAbilityOccurrenceEncoder,
        StaticAbilityOccurrenceDecoder, StaticAbilityOccurrenceRef};

    fn fixture() -> (ReplacementAbilityOrigin, [StaticAbility; 2], [CardId; 2]) {
        let source = ObjectId::from_raw(71);
        let abilities = [StaticAbility::hexproof(), StaticAbility::hexproof()];
        let faces = [CardId::new(), CardId::new()];
        let inner = ContinuousEffect::new(source, PlayerId::from_index(1),
            EffectTarget::AllPermanents, Modification::AddAbility(abilities[1].clone()))
            .with_originating_static_ability(abilities[1].clone());
        let mut outer = ContinuousEffect::new(source, PlayerId::from_index(1),
            EffectTarget::AllPermanents, Modification::AddAbility(abilities[0].clone()))
            .with_originating_static_ability(abilities[0].clone());
        outer.originating_ability = Some(Box::new(ContinuousAbilityOrigin {
            host: ObjectId::from_raw(72),
            ability: AbilityOrigin::Level { printed_face: Some(faces[1]),
                parent: Box::new(AbilityOrigin::Effect { effect: AbilityEffectOrigin::from(&inner), slot: 8 }),
                tier: 2, slot: 3 }, printed_face: Some(faces[0]), branch: 5,
        }));
        (ReplacementAbilityOrigin {
            ability: AbilityOrigin::Borrowed { effect: AbilityEffectOrigin::from(&outer),
                source: ObjectId::from_raw(73), origin: Box::new(AbilityOrigin::Level {
                    printed_face: Some(faces[1]), parent: Box::new(AbilityOrigin::Printed(4)), tier: 6, slot: 7 }) },
            printed_face: Some(faces[0]), branch: 9,
        }, abilities, faces)
    }

    fn descriptor(origin: ReplacementAbilityOrigin) -> ReplacementEffect {
        ReplacementEffect::with_matcher(ObjectId::from_raw(74), PlayerId::from_index(1),
            WouldGainLifeMatcher::you(), ReplacementAction::Double)
            .with_ability_origin(origin.ability, origin.printed_face, origin.branch).optional()
    }

    fn bind(face: CardId, faces: [CardId; 2]) -> Result<String, OccurrenceBindingError> {
        if face == faces[0] { Ok("main".into()) }
        else if face == faces[1] { Ok("level".into()) }
        else { Err(OccurrenceBindingError::InvalidModel { detail: "unknown native face".into() }) }
    }

    fn restore(face: String, faces: [CardId; 2]) -> Result<CardId, OccurrenceBindingError> {
        match face.as_str() { "main" => Ok(faces[0]), "level" => Ok(faces[1]),
            _ => Err(OccurrenceBindingError::InvalidModel { detail: "unknown owning face".into() }) }
    }

    #[test]
    fn retained_replacement_identity_codec_uses_shared_native_table_for_parent_and_decline() {
        let (origin, abilities, faces) = fixture();
        let original = descriptor(origin.clone());
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        let first = encoder.retain(abilities[0].clone()).unwrap();
        let second = encoder.retain(abilities[1].clone()).unwrap();
        assert_ne!(first, second, "equal native abilities are independent occurrences");
        let wire_origin = encoder.encode_replacement_origin(origin, |face| bind(face, faces)).unwrap();
        let key = encoder.encode_replacement_key(original.application_key(), |face| bind(face, faces)).unwrap();
        let decline = encoder.encode_replacement_key(original.optional_decline_effect().unwrap().application_key(),
            |face| bind(face, faces)).unwrap();
        assert_eq!(decline, key);
        let bytes = serde_json::to_vec(&(encoder.into_table(), wire_origin, key.clone(), decline)).unwrap();
        let (table, wire_origin, wire_key, wire_decline): (RetainedStaticAbilityTable,
            RetainedOccurrenceReplacementOrigin<String>, RetainedOccurrenceReplacementKey<String>,
            RetainedOccurrenceReplacementKey<String>) = serde_json::from_slice(&bytes).unwrap();
        let decoder = StaticAbilityOccurrenceDecoder::restore(table).unwrap();
        assert_ne!(decoder.instance_id(first).unwrap(), abilities[0].instance_id());
        assert_ne!(decoder.instance_id(first).unwrap(), decoder.instance_id(second).unwrap());
        assert_eq!(decoder.ability(first).unwrap().display(), abilities[0].display());
        let fresh_faces = [CardId::new(), CardId::new()];
        let restored_origin = decoder.restore_replacement_origin(wire_origin, |face| restore(face, fresh_faces)).unwrap();
        let restored_key = decoder.restore_replacement_key(wire_key, |face| restore(face, fresh_faces)).unwrap();
        let restored_decline = decoder.restore_replacement_key(wire_decline, |face| restore(face, fresh_faces)).unwrap();
        let restored = descriptor(restored_origin);
        let applied = std::collections::HashSet::from([restored_key.clone()]);
        assert!(applied.contains(&restored.application_key()));
        assert!(applied.contains(&restored.optional_decline_effect().unwrap().application_key()));
        assert_eq!(restored_decline, restored_key);
        assert_ne!(restored_key, original.application_key());
        let mut peer = StaticAbilityOccurrenceEncoder::default();
        assert_eq!(peer.retain(decoder.ability(first).unwrap().clone()).unwrap(), first);
        assert_eq!(peer.retain(decoder.ability(second).unwrap().clone()).unwrap(), second);
        assert_eq!(peer.encode_replacement_key(restored_key, |face| bind(face, fresh_faces)).unwrap(), key);
    }

    #[test]
    fn retained_replacement_identity_codec_rolls_back_partial_graph_binding_and_rejects_bad_refs() {
        let (origin, abilities, faces) = fixture();
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        assert!(matches!(encoder.encode_replacement_origin(origin.clone(), |face| bind(face, faces)),
            Err(OccurrenceBindingError::UnboundNativeOccurrence)));
        encoder.retain(abilities[0].clone()).unwrap();
        encoder.retain(abilities[1].clone()).unwrap();
        let before = serde_json::to_value(encoder.clone().into_table()).unwrap();
        let mut visits = 0;
        let failed = encoder.encode_replacement_origin(origin.clone(), |face| {
            visits += 1;
            if face == faces[1] { Ok("aborted-level".to_string()) }
            else { Err(OccurrenceBindingError::InvalidModel { detail: "unbound later face".into() }) }
        });
        assert!(failed.is_err()); assert!(visits >= 2, "the first graph mutation occurred");
        assert_eq!(serde_json::to_value(encoder.clone().into_table()).unwrap(), before);
        assert!(encoder.encode_replacement_origin(origin.clone(), |_| Ok("collapsed".to_string())).is_err());
        assert_eq!(serde_json::to_value(encoder.clone().into_table()).unwrap(), before);
        // A different binding for the first face proves the aborted private
        // graph prefix was rolled back, not merely omitted from into_table().
        let wire = encoder.encode_replacement_origin(origin.clone(), |face| bind(face, faces)).unwrap();
        let key = encoder.encode_replacement_key(descriptor(origin).application_key(), |face| bind(face, faces)).unwrap();
        let decoder = StaticAbilityOccurrenceDecoder::restore(encoder.into_table()).unwrap();
        assert!(decoder.restore_replacement_origin(wire, |_| Err(OccurrenceBindingError::InvalidModel {
            detail: "missing receiver face".into() })).is_err());
        let mut json = serde_json::to_value(key).unwrap();
        json["Ability"]["origin"]["ability"]["Borrowed"]["effect"]["static_ability"] = serde_json::json!(9999);
        let bad: RetainedOccurrenceReplacementKey<String> = serde_json::from_value(json).unwrap();
        assert!(matches!(decoder.restore_replacement_key(bad, |face| restore(face, faces)),
            Err(OccurrenceBindingError::UnknownReference { .. })));
        let registered = RetainedOccurrenceReplacementKey::<String>::Registered(ironsmith::replacement::ReplacementEffectId(19));
        assert_eq!(decoder.restore_replacement_key(registered, |_| panic!("registered key has no face")).unwrap(),
            ReplacementEffectKey::Registered(ironsmith::replacement::ReplacementEffectId(19)));
        let bad_legacy = RetainedOccurrenceReplacementKey::<String>::Regenerated {
            source: ObjectId::from_raw(71), controller: PlayerId::from_index(1),
            static_ability_instance: Some(StaticAbilityOccurrenceRef(9999)), matcher: None, replacement: "identity only".into(),
        };
        assert!(matches!(decoder.restore_replacement_key(bad_legacy, |_| panic!("no face")),
            Err(OccurrenceBindingError::UnknownReference { .. })));
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
        type Action = ironsmith::replacement::ReplacementAction<u8, u8, u8, u8>;
        let action = Action::EnterWithIntrinsicStartingCounters(rule);
        let mapped: ironsmith::replacement::ReplacementAction<u16, u16, u16, u16> = action.try_map_payloads(
            |_| Err::<u16, _>("unexpected effect payload"), |_| Err::<u16, _>("unexpected ability payload"),
            |_| Err::<u16, _>("unexpected program payload"), |_| Err::<u16, _>("unexpected key payload"),
        ).unwrap();
        let json = serde_json::to_value(&mapped).unwrap();
        let decoded: ironsmith::replacement::ReplacementAction<u16, u16, u16, u16> = serde_json::from_value(json).unwrap();
        assert_eq!(decoded, mapped);
        assert!(serde_json::from_value::<Action>(serde_json::json!({"EnterWithIntrinsicStartingCounters": "Poison"})).is_err());
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
mod keyword_grant_materialization_tests;
