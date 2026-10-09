//! Fresh native continuous-instruction encoding for the canonical wire model.
//!
//! Included as a child of artifact_materializer. Conversion is typed and keeps
//! instruction order; it never reconstructs an executor from its label. Native
//! state which the compiled schema cannot represent is an explicit error.

use super::{RuntimePayloadEncodingError as Error, encode_runtime_ability, encode_runtime_static_ability, wire};
use crate::continuous::{EffectTarget, Modification, PtSublayer};
use crate::effect::Effect;
use crate::effects::ApplyContinuousEffect;
use crate::effects::continuous::RuntimeModification;

type WireApply = ironsmith_core::ApplyContinuousEffect<
    wire::WireContinuousTarget,
    wire::WireContinuousModification,
    wire::WireRuntimeModification,
    ironsmith_core::Condition,
>;

fn unsupported(detail: &'static str) -> Error {
    Error::InvalidEffectModel { detail: detail.into() }
}

pub(super) fn encode_continuous_native_effect(effect: &Effect)
    -> Result<Option<wire::WireEffect>, Error>
{
    let Some(model) = effect.downcast_ref::<ApplyContinuousEffect>() else { return Ok(None); };
    let model = encode_apply_continuous(model.clone())?;
    serde_json::to_value(model).map(|payload| Some(wire::WireEffect::new("ApplyContinuousEffect", payload)))
        .map_err(|error| Error::InvalidEffectModel { detail: error.to_string() })
}

fn encode_apply_continuous(model: ApplyContinuousEffect) -> Result<WireApply, Error> {
    let ApplyContinuousEffect {
        target, target_spec, modification, additional_modifications, runtime_modifications,
        until, condition, source_type, source_reference_surface, set_quantifier_surface,
        type_retention_surface, animation_pt_surface, animation_duration_surface,
        lock_filter_at_resolution, resolve_set_pt_values_at_resolution, require_creature_target,
    } = model;
    if source_type.is_some() {
        // The canonical decoder currently instantiates SourceType = () and
        // materializes None. Dropping an explicit source/controller acquisition
        // descriptor would change target locking and copy/static semantics.
        return Err(unsupported("ApplyContinuousEffect source_type needs an exact acquisition-metadata wire owner"));
    }
    if modification.is_none() && additional_modifications.is_empty() && runtime_modifications.is_empty() {
        return Err(unsupported("ApplyContinuousEffect requires at least one modification"));
    }
    let target = match target {
        EffectTarget::Source => wire::WireContinuousTarget::Source,
        EffectTarget::Filter(filter) => wire::WireContinuousTarget::Filter(filter),
        // Native with_spec constructors use this inert fallback. resolve_target
        // always uses target_spec when present, and both AllPermanents and Source
        // have no fallback decision-related object spec. Canonicalizing only
        // that placeholder preserves the complete selection/target contract.
        EffectTarget::AllPermanents if target_spec.is_some() => wire::WireContinuousTarget::Source,
        EffectTarget::AllPermanents | EffectTarget::AllCreatures
        | EffectTarget::Specific(_) | EffectTarget::AttachedTo(_) => {
            return Err(unsupported("ApplyContinuousEffect native target needs an exact continuous-target wire owner"));
        }
    };
    Ok(WireApply {
        target, target_spec,
        modification: modification.map(encode_modification).transpose()?,
        additional_modifications: additional_modifications.into_iter()
            .map(encode_modification).collect::<Result<_, _>>()?,
        runtime_modifications: runtime_modifications.into_iter()
            .map(encode_runtime_modification).collect::<Result<_, _>>()?,
        until, condition, source_type: None, source_reference_surface, set_quantifier_surface,
        type_retention_surface, animation_pt_surface, animation_duration_surface,
        lock_filter_at_resolution, resolve_set_pt_values_at_resolution, require_creature_target,
    })
}

fn encode_sublayer(sublayer: PtSublayer) -> Result<ironsmith_core::CompiledPtSublayer, Error> {
    match sublayer {
        PtSublayer::Setting => Ok(ironsmith_core::CompiledPtSublayer::Setting),
        PtSublayer::CharacteristicDefining | PtSublayer::Modifying | PtSublayer::Switching => {
            Err(unsupported("continuous P/T instruction uses a sublayer absent from the compiled schema"))
        }
    }
}

fn encode_modification(modification: Modification) -> Result<wire::WireContinuousModification, Error> {
    use wire::WireContinuousModification as W;
    Ok(match modification {
        Modification::AddAbility(ability) => W::AddAbility(encode_runtime_static_ability(ability)?),
        Modification::AddAbilityGeneric(ability) => W::AddAbilityGeneric(encode_runtime_ability(ability)?),
        Modification::RemoveAbilityGeneric { ability, mode } => match mode {
            ironsmith_core::AbilityLossMode::Lose => W::RemoveAbility(encode_runtime_ability(ability)?),
            ironsmith_core::AbilityLossMode::LoseAndCantGain
            | ironsmith_core::AbilityLossMode::LoseAndCantHaveOrGain => {
                return Err(unsupported("continuous ability prohibition mode is absent from the compiled schema"));
            }
        },
        Modification::RemoveStaticAbilityFamily(id) => W::RemoveStaticAbilityFamily(id),
        Modification::AddCardTypes(types) => W::AddCardTypes(types),
        Modification::RemoveCardTypes(types) => W::RemoveCardTypes(types),
        Modification::SetCardTypes(types) => W::SetCardTypes(types),
        Modification::AddSubtypes(types) => W::AddSubtypes(types),
        Modification::RemoveSubtypes(types) => W::RemoveSubtypes(types),
        Modification::AddAllSubtypesOfFamily(family) => W::AddAllSubtypesOfFamily(family),
        Modification::RemoveAllSubtypesOfFamily(family) => W::RemoveAllSubtypesOfFamily(family),
        Modification::AddSupertypes(types) => W::AddSupertypes(types),
        Modification::RemoveSupertypes(types) => W::RemoveSupertypes(types),
        Modification::SetName(name) => W::SetName(name),
        Modification::AddColors(colors) => W::AddColors(colors),
        Modification::SetColors(colors) => W::SetColors(colors),
        Modification::MakeColorless => W::MakeColorless,
        Modification::SetPower { value, sublayer } => W::SetPower {
            power: value, sublayer: encode_sublayer(sublayer)?,
        },
        Modification::SetToughness { value, sublayer } => W::SetToughness {
            toughness: value, sublayer: encode_sublayer(sublayer)?,
        },
        Modification::SetPowerToughness { power, toughness, sublayer } => W::SetPowerToughness {
            power, toughness, sublayer: encode_sublayer(sublayer)?,
        },
        Modification::SwitchPowerToughness => W::SwitchPowerToughness,
        Modification::RemoveAllAbilities => W::RemoveAllAbilities,
        Modification::RewriteText(change) => W::RewriteText(change),
        Modification::CopyOf { .. } => {
            // WireContinuousModification has no captured-copy envelope. In
            // particular it cannot retain executable abilities, spell-program
            // Present/Absent/Unavailable evidence, attachment data and frozen
            // characteristics. Runtime CopyOf would reread a donor and is not
            // equivalent. Public CopiableValues serialization drops programs.
            return Err(unsupported("frozen continuous CopyOf requires a full RetainedCopiableValues envelope in the compiled schema"));
        }
        Modification::SetTextBox(_) => {
            return Err(unsupported("captured text-box overlay and its executable abilities are absent from the compiled schema"));
        }
        Modification::SetAuraAttachmentFilter(_) | Modification::Restriction(_) => {
            return Err(unsupported("continuous attachment/restriction occurrence metadata is absent from the compiled schema"));
        }
        Modification::ChangeText { .. } => {
            return Err(unsupported("legacy string text modification has no typed canonical instruction"));
        }
        Modification::ChangeController(_) | Modification::ChangeControllerToEffectController
        | Modification::InsertNameWords { .. } | Modification::SetSubtypes(_)
        | Modification::RemoveAllCreatureTypes | Modification::RemoveColors(_)
        | Modification::SetAbilities(_) | Modification::CopyActivatedAbilities { .. }
        | Modification::CopyStaticAbilityVariants { .. } | Modification::CopyTriggeredAbilities { .. }
        | Modification::AddCombatDamageDrawAbility | Modification::RemoveAbility(_)
        | Modification::RemoveAllAbilitiesExceptMana | Modification::ModifyPower(_)
        | Modification::ModifyToughness(_) | Modification::ModifyPowerToughness { .. }
        | Modification::ModifyPowerToughnessValue { .. }
        | Modification::ModifyPowerToughnessByColorCount { .. }
        | Modification::RemoveLandRulesTextAbilities => {
            // Moving one of these into runtime_modifications would change
            // registration order in compound instructions; do not invent it.
            return Err(unsupported("native continuous modification has no exact compiled modification variant"));
        }
    })
}

fn encode_runtime_modification(modification: RuntimeModification)
    -> Result<wire::WireRuntimeModification, Error>
{
    use wire::WireRuntimeModification as W;
    Ok(match modification {
        RuntimeModification::ChangeControllerToEffectController => W::ChangeControllerToEffectController,
        RuntimeModification::ChangeControllerToPlayer(player) => W::ChangeControllerToPlayer(player),
        RuntimeModification::CopyOf {
            source, preserve_source_abilities, name_override, name_override_surface,
            add_supertypes, copy_exception_surface,
        } => W::CopyOf {
            source, preserve_source_abilities, name_override, name_override_surface,
            add_supertypes, copy_exception_surface,
        },
        RuntimeModification::CopyOfWithAbilities {
            source, preserve_source_abilities, name_override, name_override_surface,
            add_supertypes, copy_exception_surface, abilities,
        } => W::CopyOfWithAbilities {
            source, preserve_source_abilities, name_override, name_override_surface,
            add_supertypes, copy_exception_surface,
            abilities: abilities.into_iter().map(encode_runtime_ability).collect::<Result<_, _>>()?,
        },
        RuntimeModification::ModifyPowerToughness { power, toughness } => W::ModifyPowerToughness { power, toughness },
        RuntimeModification::RemoveAllAbilities => W::RemoveAllAbilities,
        RuntimeModification::RemoveThisAbility => W::RemoveThisAbility,
        RuntimeModification::RetainSourceColors => W::RetainSourceColors,
        RuntimeModification::SetAuraAttachmentFilter(filter) => W::SetAuraAttachmentFilter(filter),
        RuntimeModification::ModifyPower { .. } | RuntimeModification::ModifyToughness { .. } => {
            return Err(unsupported("single-axis runtime P/T modification has no exact compiled variant"));
        }
    })
}

#[cfg(test)]
mod tests {
    // Source-only scenarios. All are UNRUN under the campaign execution hold.
    use super::*;
    use crate::ability::{Ability, AbilityKind};
    use crate::artifact_materializer::{encode_runtime_effect, materialize_effect};
    use crate::cost::TotalCost;
    use crate::costs::Cost;
    use crate::effects::DrawCardsEffect;
    use crate::resolution::ResolutionProgram;
    use ironsmith_core::{ChooseSpec, Color, ColorSet, Condition, ObjectFilter, PlayerFilter, TextChange, Until, Value};

    fn filter(colors: ColorSet) -> ObjectFilter {
        ObjectFilter { colors: Some(colors), ..ObjectFilter::default() }
    }
    fn grant(colors: ColorSet) -> Ability {
        let program = ResolutionProgram::from_effects(vec![Effect::new(DrawCardsEffect::you(Value::Count(filter(colors))))])
            .with_linked_exile_pair(ironsmith_core::LinkedExilePair {
                definition: ironsmith_core::LinkedExileDefinition([23; 32]), pair: 9,
            });
        Ability::activated(TotalCost::from_cost(Cost::sacrifice(filter(colors))), program)
    }
    fn canonical(effect: Effect) -> Effect {
        materialize_effect(encode_runtime_effect(effect).unwrap()).unwrap()
    }
    fn assert_grant_words(ability: &Ability, colors: ColorSet) {
        let AbilityKind::Activated(ability) = &ability.kind else { unreachable!() };
        assert_eq!(ability.mana_cost.costs()[0].compiled_model(), Some(&ironsmith_core::Cost::Sacrifice(filter(colors))));
        assert_eq!(ability.effects[0].downcast_ref::<DrawCardsEffect>().unwrap().count, Value::Count(filter(colors)));
        assert_eq!(ability.effects.linked_exile_pair.unwrap().pair, 9);
    }

    #[test]
    fn fresh_native_grant_round_trips_then_rewrites_and_round_trips_again() {
        let mut native = ApplyContinuousEffect::new(EffectTarget::Filter(filter(ColorSet::BLACK)),
            Modification::AddAbilityGeneric(grant(ColorSet::BLACK)),
            Until::TurnsPass(Value::Count(filter(ColorSet::BLACK))),
        ).with_condition(Condition::YouControl(filter(ColorSet::BLACK)));
        native.additional_modifications.push(Modification::AddColors(ColorSet::BLACK));
        native.runtime_modifications.push(RuntimeModification::ChangeControllerToPlayer(
            PlayerFilter::ControlsMost { filter: Box::new(filter(ColorSet::BLACK)) },
        ));
        native.source_reference_surface = Some(ironsmith_core::SourceReferenceSurface::FullName("Black Knight".into()));
        native.set_quantifier_surface = Some(ironsmith_core::SetQuantifierSurface::Each);
        native.type_retention_surface = Some(ironsmith_core::TypeRetentionSurface::StillALand);
        native.animation_pt_surface = Some(ironsmith_core::AnimationPtSurface::LeadingPowerToughness);
        native.animation_duration_surface = Some(ironsmith_core::AnimationDurationSurface::Leading);
        native.lock_filter_at_resolution = true;
        native.resolve_set_pt_values_at_resolution = true;
        native.require_creature_target = true;
        let fresh = Effect::new(native.clone());
        assert!(fresh.serialized_model().is_none());
        let first = canonical(fresh);
        let changed = first.with_text_change(TextChange::color(Color::Black, Color::Blue).unwrap()).unwrap();
        let round_trip = canonical(changed);
        let changed_again = round_trip.with_text_change(TextChange::color(Color::Blue, Color::Green).unwrap()).unwrap();
        let model = changed_again.downcast_ref::<ApplyContinuousEffect>().unwrap();
        assert_eq!(model.target, EffectTarget::Filter(filter(ColorSet::GREEN)));
        assert_eq!(model.condition, Some(Condition::YouControl(filter(ColorSet::GREEN))));
        assert_eq!(model.until, Until::TurnsPass(Value::Count(filter(ColorSet::GREEN))));
        let Some(Modification::AddAbilityGeneric(ability)) = &model.modification else { unreachable!() };
        assert_grant_words(ability, ColorSet::GREEN);
        assert!(matches!(model.additional_modifications[0], Modification::AddColors(colors) if colors == ColorSet::GREEN));
        assert!(matches!(&model.runtime_modifications[0], RuntimeModification::ChangeControllerToPlayer(PlayerFilter::ControlsMost { filter: value }) if value.as_ref() == &filter(ColorSet::GREEN)));
        assert_eq!(model.source_reference_surface, native.source_reference_surface);
        assert_eq!(model.set_quantifier_surface, native.set_quantifier_surface);
        assert_eq!(model.type_retention_surface, native.type_retention_surface);
        assert_eq!(model.animation_pt_surface, native.animation_pt_surface);
        assert_eq!(model.animation_duration_surface, native.animation_duration_surface);
        assert!(model.lock_filter_at_resolution && model.resolve_set_pt_values_at_resolution && model.require_creature_target);
        let first = first.downcast_ref::<ApplyContinuousEffect>().unwrap();
        let Some(Modification::AddAbilityGeneric(earlier)) = &first.modification else { unreachable!() };
        assert_grant_words(earlier, ColorSet::BLACK);
    }

    #[test]
    fn native_spec_gain_control_keeps_its_announced_target_contract() {
        let spec = ChooseSpec::target(ChooseSpec::Object(filter(ColorSet::BLACK)));
        let native = ApplyContinuousEffect::with_spec_runtime(spec.clone(),
            RuntimeModification::ChangeControllerToEffectController, Until::Forever).require_creature_target();
        assert_eq!(native.target, EffectTarget::AllPermanents);
        let restored = canonical(Effect::new(native));
        let changed = restored.with_text_change(TextChange::color(Color::Black, Color::Blue).unwrap()).unwrap();
        let model = changed.downcast_ref::<ApplyContinuousEffect>().unwrap();
        assert_eq!(model.target_spec, Some(ChooseSpec::target(ChooseSpec::Object(filter(ColorSet::BLUE)))));
        assert!(model.require_creature_target);
        assert!(matches!(model.runtime_modifications[0], RuntimeModification::ChangeControllerToEffectController));
        let captured = restored.downcast_ref::<ApplyContinuousEffect>().unwrap();
        assert_eq!(captured.target_spec, Some(spec));
        assert_eq!(captured.target, EffectTarget::Source, "only the unused fallback is canonicalized");
    }

    #[test]
    fn native_copy_exceptions_keep_quoted_words_separate_from_future_donor_values() {
        let native = ApplyContinuousEffect::new_runtime(EffectTarget::Source,
            RuntimeModification::CopyOfWithAbilities {
                source: ChooseSpec::Object(filter(ColorSet::BLACK)), preserve_source_abilities: true,
                name_override: Some("Black Knight".into()),
                name_override_surface: Some(ironsmith_core::SourceReferenceSurface::FullName("Black Knight".into())),
                add_supertypes: vec![ironsmith_core::Supertype::Legendary],
                copy_exception_surface: Some("black presentation".into()), abilities: vec![grant(ColorSet::BLACK)],
            }, Until::EndOfTurn);
        let restored = canonical(Effect::new(native));
        let changed = restored.with_text_change(TextChange::color(Color::Black, Color::Blue).unwrap()).unwrap();
        let changed = canonical(changed);
        let model = changed.downcast_ref::<ApplyContinuousEffect>().unwrap();
        let RuntimeModification::CopyOfWithAbilities {
            source, abilities, preserve_source_abilities, name_override, name_override_surface,
            add_supertypes, copy_exception_surface,
        } = &model.runtime_modifications[0] else { unreachable!() };
        assert_eq!(source, &ChooseSpec::Object(filter(ColorSet::BLUE)));
        assert_grant_words(&abilities[0], ColorSet::BLUE);
        assert!(*preserve_source_abilities);
        assert_eq!(name_override.as_deref(), Some("Black Knight"));
        assert_eq!(name_override_surface, &Some(ironsmith_core::SourceReferenceSurface::FullName("Black Knight".into())));
        assert_eq!(add_supertypes, &vec![ironsmith_core::Supertype::Legendary]);
        assert_eq!(copy_exception_surface.as_deref(), Some("black presentation"));
    }

    #[test]
    fn quoted_cumulative_upkeep_retains_trigger_definition_and_literal_mana_payment() {
        let mana = ironsmith_core::ManaCost::from_pips(vec![vec![ironsmith_core::ManaSymbol::Generic(1)]]);
        let definition = ironsmith_core::LinkedExileDefinition([24; 32]);
        let program = ResolutionProgram::from_effects(vec![
            Effect::put_counters_on_source(ironsmith_core::CounterType::Age, 1),
            Effect::cumulative_upkeep(vec![Effect::new(crate::effects::PayManaEffect::new(
                mana.clone(), ChooseSpec::SourceController,
            ))], PlayerFilter::You, vec![Effect::sacrifice_source()]),
        ]).with_trigger_definition(definition);
        let trigger = crate::triggers::Trigger::from_model(
            ironsmith_core::trigger_model::Trigger::beginning_of_upkeep(PlayerFilter::You),
        ).unwrap();
        let mut quoted = Ability::triggered(trigger, program);
        let AbilityKind::Triggered(ability) = &mut quoted.kind else { unreachable!() };
        ability.presentation_label = Some(ironsmith_core::PresentationLabel::AbilityWord("Black label is unchanged".into()));
        ability.intervening_if = Some(Condition::YouControl(filter(ColorSet::BLACK)));
        ability.choices = vec![ChooseSpec::target(ChooseSpec::Object(filter(ColorSet::BLACK)))];
        let native = ApplyContinuousEffect::with_spec(
            ChooseSpec::target(ChooseSpec::Object(filter(ColorSet::BLACK))),
            Modification::AddAbilityGeneric(quoted), Until::Forever,
        );
        let captured = canonical(Effect::new(native));
        let changed = captured.with_text_change(TextChange::color(Color::Black, Color::Blue).unwrap()).unwrap();
        let changed = canonical(changed);
        let model = changed.downcast_ref::<ApplyContinuousEffect>().unwrap();
        let Some(Modification::AddAbilityGeneric(quoted)) = &model.modification else { unreachable!() };
        let AbilityKind::Triggered(ability) = &quoted.kind else { unreachable!() };
        assert_eq!(ability.effects.retained_trigger_definition(), Some(definition));
        assert_eq!(ability.presentation_label, Some(ironsmith_core::PresentationLabel::AbilityWord("Black label is unchanged".into())));
        assert_eq!(ability.intervening_if, Some(Condition::YouControl(filter(ColorSet::BLUE))));
        assert_eq!(ability.choices, vec![ChooseSpec::target(ChooseSpec::Object(filter(ColorSet::BLUE)))]);
        let upkeep = ability.effects[1].downcast_ref::<crate::effects::CumulativeUpkeepEffect>().unwrap();
        assert_eq!(upkeep.payment[0].downcast_ref::<crate::effects::PayManaEffect>().unwrap().cost, mana);
        assert_eq!(upkeep.player, PlayerFilter::You);
        assert_eq!(upkeep.failure.len(), 1);
        let original = captured.downcast_ref::<ApplyContinuousEffect>().unwrap();
        let Some(Modification::AddAbilityGeneric(quoted)) = &original.modification else { unreachable!() };
        let AbilityKind::Triggered(earlier) = &quoted.kind else { unreachable!() };
        assert_eq!(earlier.intervening_if, Some(Condition::YouControl(filter(ColorSet::BLACK))));
    }

    #[test]
    fn native_frozen_copy_and_explicit_acquisition_state_are_reported_as_holds() {
        let frozen = ApplyContinuousEffect::new(EffectTarget::Source, Modification::CopyOf {
            target_id: crate::ids::ObjectId::from_raw(55),
            copiable_values: Box::new(crate::snapshot::CopiableValues::default()),
            preserve_source_abilities: true, name_override: None, name_override_surface: None,
            add_supertypes: vec![],
        }, Until::Forever);
        let error = encode_runtime_effect(Effect::new(frozen)).unwrap_err();
        assert!(matches!(error, Error::InvalidEffectModel { detail } if detail.contains("RetainedCopiableValues")));
        let source_type = ApplyContinuousEffect::new_runtime(EffectTarget::Source,
            RuntimeModification::ChangeControllerToEffectController, Until::Forever)
            .with_source_type(crate::continuous::EffectSourceType::Copy);
        assert!(encode_runtime_effect(Effect::new(source_type)).is_err());
        let specific = ApplyContinuousEffect::new_runtime(EffectTarget::Specific(crate::ids::ObjectId::from_raw(56)),
            RuntimeModification::ChangeControllerToEffectController, Until::Forever);
        assert!(encode_runtime_effect(Effect::new(specific)).is_err());
        let single_axis = ApplyContinuousEffect::new_runtime(EffectTarget::Source,
            RuntimeModification::ModifyPower { value: Value::Fixed(1) }, Until::Forever);
        assert!(encode_runtime_effect(Effect::new(single_axis)).is_err());
    }
}
