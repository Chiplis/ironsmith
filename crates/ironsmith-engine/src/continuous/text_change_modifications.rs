//! Authored words in continuous-effect instructions, before resolution.
//!
//! A modification embedded in an authored instruction differs from one already
//! registered in the layer store: its predicates, explicit color/type words and
//! quoted grants can change. Captured copy values and exchanged text boxes are
//! foreign definitions, and resolved replacement choices are values. None is
//! rewritten merely because it is stored inside this instruction.

use super::text_change_predicates::{
    rewrite_choose_spec_words, rewrite_condition_words, rewrite_filter_words,
    rewrite_player_filter_words, rewrite_value_words,
};
use super::text_change_programs::rewrite_ability_words;
use super::text_changes::{TextChangeDomainError as Error, rewrite_attachment_words};
use super::{EffectTarget, Modification};
use crate::effects::ApplyContinuousEffect;
use crate::effects::continuous::RuntimeModification;
use ironsmith_core::{Color, TextChange, Until};

/// Returns a complete new model. The enclosing immutable Effect owner caches
/// it; do not use model PartialEq here, since nested abilities/costs can compare
/// their display text instead of their executable definitions.
pub(crate) fn rewrite_apply_continuous_words(
    model: &ApplyContinuousEffect,
    change: TextChange,
) -> Result<ApplyContinuousEffect, Error> {
    let mut rewritten = model.clone();
    rewritten.target = rewrite_continuous_target_words(&model.target, change)?;
    rewritten.target_spec = model.target_spec.as_ref()
        .map(|spec| rewrite_choose_spec_words(spec, change)).transpose()?;
    rewritten.condition = model.condition.as_ref()
        .map(|condition| rewrite_condition_words(condition, change)).transpose()?;
    rewritten.modification = model.modification.as_ref()
        .map(|modification| rewrite_modification_words(modification, change)).transpose()?;
    rewritten.additional_modifications = model.additional_modifications.iter()
        .map(|modification| rewrite_modification_words(modification, change)).collect::<Result<_, _>>()?;
    rewritten.runtime_modifications = model.runtime_modifications.iter()
        .map(|modification| rewrite_runtime_modification_words(modification, change)).collect::<Result<_, _>>()?;
    rewritten.until = rewrite_until_words(&model.until, change)?;
    // Resolution locking, source/controller identity, all presentation fields
    // and validation flags survive via clone, including a preexisting explicit
    // source_type. Canonical transport has separate, narrower checked bounds.
    Ok(rewritten)
}

pub(crate) fn rewrite_continuous_target_words(target: &EffectTarget, change: TextChange)
    -> Result<EffectTarget, Error>
{
    Ok(match target {
        EffectTarget::Filter(filter) => EffectTarget::Filter(rewrite_filter_words(filter, change)?),
        EffectTarget::Specific(_) | EffectTarget::Source | EffectTarget::AllPermanents
        | EffectTarget::AllCreatures | EffectTarget::AttachedTo(_) => target.clone(),
    })
}

/// Duration references/counters are identities, not words. Exhaustively visit
/// the vocabulary so a future word-bearing duration cannot be silently skipped.
pub(crate) fn rewrite_until_words(until: &Until, change: TextChange) -> Result<Until, Error> {
    Ok(match until {
        Until::TurnsPass(value) => Until::TurnsPass(rewrite_value_words(value, change)?),
        Until::PlayersNextUntapStep { player } => Until::PlayersNextUntapStep {
            player: rewrite_player_filter_words(player, change)?,
        },
        Until::ForAsLongAs(predicate) => {
            fn check(predicate: &ironsmith_core::ContinuousDurationPredicate) {
                use ironsmith_core::ContinuousDurationPredicate as P;
                match predicate {
                    P::All(children) => children.iter().for_each(check),
                    P::ObjectOnBattlefield(_) | P::ObjectInZone { .. } | P::ObjectTapped(_)
                    | P::ObjectControlledBy { .. } | P::ObjectHasCounter { .. }
                    | P::ObjectAttachedTo { .. } | P::ObjectIsEnchanted(_)
                    | P::PlayerIsMonarch(_) | P::ObjectPowerAtMostObject { .. } => {}
                }
            }
            check(predicate);
            until.clone()
        }
        Until::Forever | Until::EndOfTurn | Until::EndOfTurnOrAnyPlayerRolls { .. }
        | Until::YourNextTurn | Until::YourNextTurnEnd | Until::YourNextUpkeep
        | Until::ControllersNextUntapStep | Until::NextEndStep | Until::EndOfCombat
        | Until::ThisLeavesTheBattlefield | Until::SourceUntaps | Until::YouStopControllingThis
        | Until::ObjectIsCast { .. } | Until::YourNextUntapStep
        | Until::UntilControllersNextUntapStep { .. } => until.clone(),
    })
}

pub(crate) fn rewrite_modification_words(modification: &Modification, change: TextChange)
    -> Result<Modification, Error>
{
    let mut rewritten = modification.clone();
    match &mut rewritten {
        Modification::AddSubtypes(words) | Modification::RemoveSubtypes(words)
        | Modification::SetSubtypes(words) => change.replace_subtype_words(words),
        Modification::AddColors(words) | Modification::RemoveColors(words)
        | Modification::SetColors(words) => {
            // The historical all-colors bitset also represents "all colors",
            // which has no individual color words. Its origin is ambiguous.
            let all_colors: ironsmith_core::ColorSet = Color::ALL.into_iter().collect();
            if *words == all_colors { return Err(Error::Effect); }
            change.replace_color_words(words);
        }
        Modification::AddAbility(ability) | Modification::RemoveAbility(ability) => {
            *ability = ability.with_text_change(change)?;
        }
        Modification::AddAbilityGeneric(ability)
        | Modification::RemoveAbilityGeneric { ability, .. } => {
            *ability = rewrite_ability_words(ability, change)?;
        }
        Modification::SetAbilities(abilities) => {
            *abilities = abilities.iter().map(|ability| rewrite_ability_words(ability, change))
                .collect::<Result<_, _>>()?;
        }
        Modification::CopyActivatedAbilities { filter, .. }
        | Modification::CopyStaticAbilityVariants { filter, .. }
        | Modification::CopyTriggeredAbilities { filter, .. } => {
            // Only the authored donor predicate belongs to this instruction.
            // The abilities ultimately acquired from donors are not quoted here.
            *filter = rewrite_filter_words(filter, change)?;
        }
        Modification::SetAuraAttachmentFilter(metadata) => {
            let mut retained = crate::object::RetainedAuraAttachmentMetadata::from(metadata.clone());
            retained.filter = rewrite_attachment_words(&retained.filter, change)?;
            retained.enchant_ability = retained.enchant_ability.with_text_change(change)?;
            // This conversion preserves the owned enchant occurrence; using
            // AuraAttachmentFilter::into() would allocate a different one.
            *metadata = retained.try_into().map_err(|_| Error::Attachment)?;
        }
        Modification::SetPower { value, .. } | Modification::SetToughness { value, .. } => {
            *value = rewrite_value_words(value, change)?;
        }
        Modification::SetPowerToughness { power, toughness, .. }
        | Modification::ModifyPowerToughnessValue { power, toughness } => {
            *power = rewrite_value_words(power, change)?;
            *toughness = rewrite_value_words(toughness, change)?;
        }
        // This legacy string payload cannot establish typed word provenance.
        Modification::ChangeText { .. } => return Err(Error::Effect),
        // Both payloads are immutable captured foreign definitions, not the
        // source instruction's authored text. Layer-3 application to a receiver
        // still rewrites that receiver's acquired definition in its own owner.
        Modification::CopyOf { .. } | Modification::SetTextBox(_) => {}
        // RewriteText is a completed runtime selection. Rewriting its values
        // could make a valid replacement an invalid identity substitution.
        Modification::RewriteText(_) => {}
        // RegisteredRestriction owns a canonical wordless unit ability and its
        // occurrence; cloning keeps that acquisition identity exactly.
        Modification::Restriction(_) => {}
        Modification::ChangeController(_) | Modification::ChangeControllerToEffectController
        | Modification::SetName(_) | Modification::InsertNameWords { .. }
        | Modification::AddCardTypes(_) | Modification::RemoveCardTypes(_)
        | Modification::SetCardTypes(_) | Modification::AddAllSubtypesOfFamily(_)
        | Modification::RemoveAllSubtypesOfFamily(_) | Modification::AddSupertypes(_)
        | Modification::RemoveSupertypes(_) | Modification::RemoveAllCreatureTypes
        | Modification::MakeColorless | Modification::AddCombatDamageDrawAbility
        | Modification::RemoveStaticAbilityFamily(_) | Modification::RemoveAllAbilities
        | Modification::RemoveAllAbilitiesExceptMana | Modification::ModifyPower(_)
        | Modification::ModifyToughness(_) | Modification::ModifyPowerToughness { .. }
        | Modification::ModifyPowerToughnessByColorCount { .. }
        | Modification::SwitchPowerToughness | Modification::RemoveLandRulesTextAbilities => {}
    }
    Ok(rewritten)
}

pub(crate) fn rewrite_runtime_modification_words(
    modification: &RuntimeModification,
    change: TextChange,
) -> Result<RuntimeModification, Error> {
    let mut rewritten = modification.clone();
    match &mut rewritten {
        RuntimeModification::ChangeControllerToPlayer(player) => {
            *player = rewrite_player_filter_words(player, change)?;
        }
        RuntimeModification::CopyOf { source, .. } => {
            *source = rewrite_choose_spec_words(source, change)?;
        }
        RuntimeModification::CopyOfWithAbilities { source, abilities, .. } => {
            *source = rewrite_choose_spec_words(source, change)?;
            // Copiable exceptions are the source's authored quoted definitions.
            // The donor's eventual frozen characteristics are not present yet.
            *abilities = abilities.iter().map(|ability| rewrite_ability_words(ability, change))
                .collect::<Result<_, _>>()?;
        }
        RuntimeModification::ModifyPowerToughness { power, toughness } => {
            *power = rewrite_value_words(power, change)?;
            *toughness = rewrite_value_words(toughness, change)?;
        }
        RuntimeModification::ModifyPower { value } | RuntimeModification::ModifyToughness { value } => {
            *value = rewrite_value_words(value, change)?;
        }
        RuntimeModification::SetAuraAttachmentFilter(filter) => {
            *filter = rewrite_attachment_words(filter, change)?;
        }
        RuntimeModification::ChangeControllerToEffectController
        | RuntimeModification::RemoveAllAbilities | RuntimeModification::RemoveThisAbility => {}
    }
    // Name overrides, display-only copy-exception text, preserve flags, source
    // references and supertype exceptions remain verbatim through cloning.
    Ok(rewritten)
}

#[cfg(test)]
mod tests {
    // Authored regression scenarios only. No compiler/test/engine run occurred.
    use super::*;
    use crate::ability::{Ability, AbilityKind};
    use crate::cost::TotalCost;
    use crate::costs::Cost;
    use crate::effect::Effect;
    use crate::effects::DrawCardsEffect;
    use crate::resolution::ResolutionProgram;
    use crate::static_abilities::StaticAbility;
    use ironsmith_core::{ChooseSpec, ColorSet, Condition, ObjectFilter, PlayerFilter, Subtype, Value};
    use std::sync::Arc;

    fn change() -> TextChange { TextChange::color(Color::Black, Color::Blue).unwrap() }
    fn black() -> ObjectFilter { ObjectFilter { colors: Some(ColorSet::BLACK), ..ObjectFilter::default() } }
    fn blue() -> ObjectFilter { ObjectFilter { colors: Some(ColorSet::BLUE), ..ObjectFilter::default() } }
    fn grant() -> Ability {
        let pair = ironsmith_core::LinkedExilePair {
            definition: ironsmith_core::LinkedExileDefinition([21; 32]), pair: 4,
        };
        let program = ResolutionProgram::from_effects(vec![Effect::new(DrawCardsEffect::you(Value::Count(black())))])
            .with_linked_exile_pair(pair);
        Ability::activated(TotalCost::from_cost(Cost::sacrifice(black())), program)
    }

    #[test]
    fn authored_target_condition_duration_and_quoted_grant_are_all_rewritten() {
        let mut original = ApplyContinuousEffect::new(
            EffectTarget::Filter(black()), Modification::AddAbilityGeneric(grant()),
            Until::TurnsPass(Value::Count(black())),
        ).with_condition(Condition::YouControl(black()));
        original.target_spec = Some(ChooseSpec::target(ChooseSpec::Object(black())));
        original.additional_modifications.push(Modification::SetPower {
            value: Value::Count(black()), sublayer: super::super::PtSublayer::Setting,
        });
        original.runtime_modifications.push(RuntimeModification::ChangeControllerToPlayer(
            PlayerFilter::ControlsMost { filter: Box::new(black()) },
        ));
        let captured = original.clone();
        let changed = rewrite_apply_continuous_words(&original, change()).unwrap();
        assert!(matches!(&changed.target, EffectTarget::Filter(filter) if filter == &blue()));
        assert_eq!(changed.target_spec, Some(ChooseSpec::target(ChooseSpec::Object(blue()))));
        assert_eq!(changed.condition, Some(Condition::YouControl(blue())));
        assert_eq!(changed.until, Until::TurnsPass(Value::Count(blue())));
        let Some(Modification::AddAbilityGeneric(ability)) = &changed.modification else { unreachable!() };
        let AbilityKind::Activated(ability) = &ability.kind else { unreachable!() };
        assert_eq!(ability.mana_cost.costs()[0].compiled_model(), Some(&ironsmith_core::Cost::Sacrifice(blue())));
        assert_eq!(ability.effects.linked_exile_pair.unwrap().pair, 4);
        assert_eq!(ability.effects[0].downcast_ref::<DrawCardsEffect>().unwrap().count, Value::Count(blue()));
        assert!(matches!(&changed.additional_modifications[0], Modification::SetPower { value, .. } if value == &Value::Count(blue())));
        assert!(matches!(&changed.runtime_modifications[0], RuntimeModification::ChangeControllerToPlayer(PlayerFilter::ControlsMost { filter }) if filter.as_ref() == &blue()));
        let Some(Modification::AddAbilityGeneric(earlier)) = &captured.modification else { unreachable!() };
        let AbilityKind::Activated(earlier) = &earlier.kind else { unreachable!() };
        assert_eq!(earlier.effects[0].downcast_ref::<DrawCardsEffect>().unwrap().count, Value::Count(black()));
    }

    #[test]
    fn frozen_copy_program_abilities_actual_colors_and_names_remain_captured_values() {
        let values = crate::snapshot::CopiableValues {
            name: "Black Elf".into(), compiled_card_text: "Black Elf".into(),
            subtypes: vec![Subtype::Elf], colors: ColorSet::BLACK,
            abilities: Arc::new(vec![grant()]),
            spell_effect: crate::snapshot::SpellProgramState::Present(ResolutionProgram::from_effects(vec![
                Effect::new(DrawCardsEffect::you(Value::Count(black()))),
            ])),
            ..Default::default()
        };
        let original = Modification::CopyOf {
            target_id: crate::ids::ObjectId::from_raw(21), copiable_values: Box::new(values.clone()),
            preserve_source_abilities: true, name_override: Some("Black Knight".into()),
            name_override_surface: None, add_supertypes: vec![ironsmith_core::Supertype::Legendary],
        };
        let changed = rewrite_modification_words(&original, change()).unwrap();
        let Modification::CopyOf { copiable_values, name_override, .. } = changed else { unreachable!() };
        assert!(Arc::ptr_eq(&copiable_values.abilities, &values.abilities));
        assert_eq!(copiable_values.colors, ColorSet::BLACK);
        assert_eq!(copiable_values.subtypes, vec![Subtype::Elf]);
        assert_eq!(copiable_values.name, "Black Elf");
        assert_eq!(name_override.as_deref(), Some("Black Knight"));
        let crate::snapshot::SpellProgramState::Present(program) = &copiable_values.spell_effect else { unreachable!() };
        assert_eq!(program[0].downcast_ref::<DrawCardsEffect>().unwrap().count, Value::Count(black()));
        let unavailable = Modification::CopyOf {
            target_id: crate::ids::ObjectId::from_raw(22),
            copiable_values: Box::new(crate::snapshot::CopiableValues::default()),
            preserve_source_abilities: false, name_override: None, name_override_surface: None,
            add_supertypes: vec![],
        };
        let Modification::CopyOf { copiable_values, .. } = rewrite_modification_words(&unavailable, change()).unwrap() else { unreachable!() };
        assert!(copiable_values.spell_effect.is_unavailable());
    }

    #[test]
    fn copy_exception_grants_change_but_identity_and_presentation_do_not() {
        let original = RuntimeModification::CopyOfWithAbilities {
            source: ChooseSpec::Object(black()), preserve_source_abilities: true,
            name_override: Some("Black Knight".into()),
            name_override_surface: Some(ironsmith_core::SourceReferenceSurface::FullName("Black Knight".into())),
            add_supertypes: vec![ironsmith_core::Supertype::Legendary],
            copy_exception_surface: Some("black is display only".into()), abilities: vec![grant()],
        };
        let RuntimeModification::CopyOfWithAbilities {
            source, abilities, name_override, name_override_surface, add_supertypes,
            copy_exception_surface, preserve_source_abilities,
        } = rewrite_runtime_modification_words(&original, change()).unwrap() else { unreachable!() };
        assert_eq!(source, ChooseSpec::Object(blue()));
        let AbilityKind::Activated(ability) = &abilities[0].kind else { unreachable!() };
        assert_eq!(ability.effects[0].downcast_ref::<DrawCardsEffect>().unwrap().count, Value::Count(blue()));
        assert_eq!(name_override.as_deref(), Some("Black Knight"));
        assert_eq!(name_override_surface, Some(ironsmith_core::SourceReferenceSurface::FullName("Black Knight".into())));
        assert_eq!(copy_exception_surface.as_deref(), Some("black is display only"));
        assert_eq!(add_supertypes, vec![ironsmith_core::Supertype::Legendary]);
        assert!(preserve_source_abilities);
    }

    #[test]
    fn attachment_occurrence_and_source_acquisition_metadata_survive() {
        let metadata: crate::object::AuraAttachmentMetadata =
            ironsmith_core::AuraAttachmentFilter::Object(black()).into();
        let occurrence = metadata.enchant_ability().instance_id();
        let mut original = ApplyContinuousEffect::new(
            EffectTarget::Specific(crate::ids::ObjectId::from_raw(31)),
            Modification::SetAuraAttachmentFilter(metadata), Until::YourNextUntapStep,
        ).with_source_type(super::super::EffectSourceType::Resolution {
            locked_targets: vec![crate::ids::ObjectId::from_raw(32)],
        });
        original.source_reference_surface = Some(ironsmith_core::SourceReferenceSurface::FullName("Black Knight".into()));
        original.set_quantifier_surface = Some(ironsmith_core::SetQuantifierSurface::Those);
        original.type_retention_surface = Some(ironsmith_core::TypeRetentionSurface::StillALand);
        original.animation_pt_surface = Some(ironsmith_core::AnimationPtSurface::LeadingPowerToughness);
        original.animation_duration_surface = Some(ironsmith_core::AnimationDurationSurface::Leading);
        original.lock_filter_at_resolution = true;
        original.resolve_set_pt_values_at_resolution = true;
        original.require_creature_target = true;
        let changed = rewrite_apply_continuous_words(&original, change()).unwrap();
        let Some(Modification::SetAuraAttachmentFilter(metadata)) = &changed.modification else { unreachable!() };
        assert_eq!(metadata.to_owned_value(), ironsmith_core::AuraAttachmentFilter::Object(blue()));
        assert_eq!(metadata.enchant_ability().instance_id(), occurrence);
        assert_eq!(changed.target, original.target);
        assert_eq!(changed.source_type, original.source_type);
        assert_eq!(changed.source_reference_surface, original.source_reference_surface);
        assert_eq!(changed.set_quantifier_surface, original.set_quantifier_surface);
        assert_eq!(changed.type_retention_surface, original.type_retention_surface);
        assert_eq!(changed.animation_pt_surface, original.animation_pt_surface);
        assert_eq!(changed.animation_duration_surface, original.animation_duration_surface);
        assert!(changed.lock_filter_at_resolution && changed.resolve_set_pt_values_at_resolution && changed.require_creature_target);
    }

    #[test]
    fn absent_words_runtime_choices_names_and_wordless_keywords_are_noops() {
        let mut filter = ObjectFilter::default();
        filter.chosen_color = true;
        filter.chosen_creature_type = true;
        let selected = TextChange::color(Color::Black, Color::Blue).unwrap();
        let original = ApplyContinuousEffect::new(EffectTarget::Filter(filter.clone()),
            Modification::RewriteText(selected), Until::EndOfTurn);
        let changed = rewrite_apply_continuous_words(&original, change()).unwrap();
        assert_eq!(changed.target, EffectTarget::Filter(filter));
        assert!(matches!(changed.modification, Some(Modification::RewriteText(value)) if value == selected));
        let name = rewrite_modification_words(&Modification::SetName("Black Elf".into()), change()).unwrap();
        assert!(matches!(name, Modification::SetName(value) if value == "Black Elf"));
        let protection = StaticAbility::protection(ironsmith_core::ProtectionFrom::ChosenColor);
        let changed = rewrite_modification_words(&Modification::AddAbility(protection), change()).unwrap();
        let Modification::AddAbility(ability) = changed else { unreachable!() };
        assert_eq!(ability.protection_from(), Some(&ironsmith_core::ProtectionFrom::ChosenColor));
        let predicate = ironsmith_core::ContinuousDurationPredicate::All(vec![
            ironsmith_core::ContinuousDurationPredicate::ObjectOnBattlefield(
                ironsmith_core::ContinuousDurationObject::Tagged("black-identity".into())),
        ]);
        assert_eq!(rewrite_until_words(&Until::ForAsLongAs(predicate.clone()), change()).unwrap(), Until::ForAsLongAs(predicate));
    }

    #[test]
    fn ambiguous_or_opaque_children_hold_the_complete_instruction_atomically() {
        let mut original = ApplyContinuousEffect::new(EffectTarget::Filter(black()),
            Modification::AddAbilityGeneric(grant()), Until::Forever);
        original.additional_modifications.push(Modification::ChangeText { from: "black".into(), to: "blue".into() });
        assert!(matches!(rewrite_apply_continuous_words(&original, change()), Err(Error::Effect)));
        assert_eq!(original.target, EffectTarget::Filter(black()));
        assert!(matches!(rewrite_modification_words(&Modification::SetColors(Color::ALL.into_iter().collect()), change()), Err(Error::Effect)));
        let mut opaque = black();
        opaque.ability_markers.push("black is opaque".into());
        let original = RuntimeModification::CopyOf {
            source: ChooseSpec::Object(opaque), preserve_source_abilities: false,
            name_override: None, name_override_surface: None, add_supertypes: vec![], copy_exception_surface: None,
        };
        assert!(matches!(rewrite_runtime_modification_words(&original, change()), Err(Error::ObjectFilter)));
    }
}
