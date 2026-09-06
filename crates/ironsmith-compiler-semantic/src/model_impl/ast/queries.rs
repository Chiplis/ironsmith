//! Queries and edits over the effect AST.
//!
//! These read or adjust an already-recognized program. They belong beside the
//! AST rather than in a recognition module: nothing here inspects text, tokens,
//! or words, so any phase holding an `EffectAst` may use them — including
//! lowering, which must not import recognition.

use crate::effect::Until;
use crate::filter::{TaggedObjectConstraint, TaggedOpbjectRelation};
use crate::model_impl::visit::{for_each_nested_effects, for_each_nested_effects_mut};
use crate::tag::CompilerReferenceTag;
use crate::target::{
    ChooseSpec, ChooseSpecSurfaceHint, ObjectFilter, PlayerFilter, SourceReferenceSurface,
};
use crate::zone::Zone;

use super::super::super::TargetAst;
use super::{EffectAst, SubjectVerbActionAst, CounterActionAst, DamagePreventionActionAst, GrantActionAst, LibraryActionAst, CharacteristicActionAst, KeywordActionAst, ZoneMoveActionAst, PermanentStateActionAst, RevealLookActionAst, DamageActionAst, StatChangeActionAst, StackActionAst, ControlActionAst, ObjectChoiceEffectAst};

/// The object a source reference denotes, carrying the authored surface.
pub fn source_choose_spec_for_surface(surface: SourceReferenceSurface) -> ChooseSpec {
    ChooseSpec::Source.with_surface_hint(ChooseSpecSurfaceHint::SourceReference(surface))
}

/// The filter a declared target selects, when the target denotes objects.
pub fn target_ast_to_object_filter(target: TargetAst) -> Option<ObjectFilter> {
    match target {
        TargetAst::Source(_) => Some(ObjectFilter::source()),
        TargetAst::Object(filter, _, _) => Some(filter),
        TargetAst::Spell(_) => Some(ObjectFilter::spell()),
        TargetAst::Tagged(tag, _) => Some(ObjectFilter::tagged(tag)),
        TargetAst::AnyOtherTarget(_) => {
            Some(ObjectFilter::default().not_tagged(CompilerReferenceTag::It.bind()))
        }
        TargetAst::WithCount(inner, _) => target_ast_to_object_filter(*inner),
        _ => None,
    }
}

pub fn primary_damage_target_from_effect(effect: &EffectAst) -> Option<TargetAst> {
    match effect {
        EffectAst::SubjectVerb(subject_verb) => match &subject_verb.action {
            SubjectVerbActionAst::Damage(DamageActionAst::DealDamage { target, .. })
            | SubjectVerbActionAst::Damage(DamageActionAst::DealDistributedDamage { target, .. })
            | SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEqualToPower { target, .. }) => Some(target.clone()),
            _ => None,
        },
        _ => {
            let mut found = None;
            for_each_nested_effects(effect, false, |nested| {
                if found.is_none() {
                    found = nested.iter().find_map(primary_damage_target_from_effect);
                }
            });
            found
        }
    }
}

pub fn primary_target_from_effect(effect: &EffectAst) -> Option<TargetAst> {
    match effect {
        EffectAst::SubjectVerb(subject_verb) => match &subject_verb.action {
            SubjectVerbActionAst::Damage(DamageActionAst::DealDamage { target, .. })
            | SubjectVerbActionAst::Damage(DamageActionAst::DealDistributedDamage { target, .. })
            | SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEqualToPower { target, .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Tap { target })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Untap { target })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Destroy { target, .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Exile { target, .. })
            | SubjectVerbActionAst::RevealLook(RevealLookActionAst::LookAtHand { target })
            | SubjectVerbActionAst::Stack(StackActionAst::Counter { target })
            | SubjectVerbActionAst::Stack(StackActionAst::CounterUnlessPays { target, .. })
            | SubjectVerbActionAst::Counters(CounterActionAst::PutCounters { target, .. })
            | SubjectVerbActionAst::Counters(CounterActionAst::PutCounterChoice { target, .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToHand { target, .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Detain { target })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Goad { target, .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Suspect { target })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::RemoveFromCombat { target })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Flip { target })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Regenerate { target, .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::TapOrUntap { target })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::PhaseOut { target, .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::PhaseIn { target })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Transform { target })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Convert { target })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Explore { target })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Endure { target, .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Connive { target, .. })
            | SubjectVerbActionAst::Library(LibraryActionAst::MoveToLibraryNthFromTop { target, .. })
            | SubjectVerbActionAst::Library(LibraryActionAst::MoveToLibraryTopOrBottomChoice { target })
            | SubjectVerbActionAst::Counters(CounterActionAst::RemoveUpToAnyCounters { target, .. })
            | SubjectVerbActionAst::Counters(CounterActionAst::ForEachCounterKindPutOrRemove { target, .. })
            | SubjectVerbActionAst::Counters(CounterActionAst::PutCounterOfChosenKind { target })
            | SubjectVerbActionAst::PutSticker { target, .. }
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::SwitchPowerToughness { target, .. })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantProtectionChoice { target, .. })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::AssignNoCombatDamage { source: target, .. })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventAllCombatDamageFromSource { source: target, .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileWhenSourceLeaves { target })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::SacrificeSourceWhenLeaves { target })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::RedirectNextTimeDamageToSource { target, .. })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::RedirectAllDamageThisTurnBySourceToSourceController {
                source: target,
            })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventDamage { target, .. })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventAllDamageToTarget { target, .. })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventDamageToTargetPutCounters { target, .. })
            | SubjectVerbActionAst::Counters(CounterActionAst::PutOrRemoveCounters { target, .. })
            | SubjectVerbActionAst::Counters(CounterActionAst::DoubleCountersOnTarget { target, .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileUntilSourceLeaves { target, .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToBattlefield { target, .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone { target, .. })
            | SubjectVerbActionAst::TargetOnly { target, .. }
            | SubjectVerbActionAst::StatChanges(StatChangeActionAst::Pump { target, .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetBasePowerToughness { target, .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeBasePtCreature { target, .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetBasePower { target, .. })
            | SubjectVerbActionAst::StatChanges(StatChangeActionAst::PumpForEach { target, .. })
            | SubjectVerbActionAst::StatChanges(StatChangeActionAst::PumpByLastEffect { target, .. })
            | SubjectVerbActionAst::Control(ControlActionAst::GainControl { target, .. })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesToTarget { target, .. })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantToTarget { target, .. })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesChoiceToTarget { target, .. }) => {
                Some(target.clone())
            }
            SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::RedirectNextDamageFromSourceToTarget {
                protected_target,
                destination_target,
                ..
            }) => protected_target
                .as_ref()
                .or(destination_target.as_ref())
                .cloned(),
            _ => None,
        },
        _ => {
            let mut found = None;
            for_each_nested_effects(effect, false, |nested| {
                if found.is_none() {
                    found = nested.iter().find_map(primary_target_from_effect);
                }
            });
            found
        }
    }
}

pub fn apply_cant_be_regenerated_to_last_target_effect(effects: &mut Vec<EffectAst>) -> bool {
    let Some(previous_target) = effects.last().and_then(primary_target_from_effect) else {
        return false;
    };
    let Some(mut filter) = target_ast_to_object_filter(previous_target) else {
        return false;
    };
    if !filter
        .tagged_constraints
        .iter()
        .any(|constraint| constraint.tag.as_str() == CompilerReferenceTag::It.as_str())
    {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: (CompilerReferenceTag::It.bind()).into(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    }

    effects.push(EffectAst::subject_verb_cant(
        crate::effect::Restriction::be_regenerated(filter),
        Until::EndOfTurn,
        None,
    ));
    true
}

pub fn apply_cant_be_regenerated_to_effect(effect: &mut EffectAst) -> bool {
    match effect {
        EffectAst::SubjectVerb(subject_verb) => match &mut subject_verb.action {
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Destroy {
                no_regeneration, ..
            })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DestroyAll {
                no_regeneration, ..
            })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DestroyAllOfChosenColor {
                no_regeneration, ..
            }) => {
                *no_regeneration = true;
                true
            }
            _ => false,
        },
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseOneOf { modes }) | EffectAst::ObjectChoices(ObjectChoiceEffectAst::VillainousChoice { modes, .. }) => {
            let mut applied = false;
            for mode in modes {
                applied |= apply_cant_be_regenerated_to_effects_tail(&mut mode.effects);
            }
            applied
        }
        _ => {
            let mut applied = false;
            for_each_nested_effects_mut(effect, true, |nested| {
                if !applied {
                    applied = apply_cant_be_regenerated_to_effects_tail(nested);
                }
            });
            applied
        }
    }
}

pub fn apply_cant_be_regenerated_to_effects_tail(effects: &mut [EffectAst]) -> bool {
    for effect in effects.iter_mut().rev() {
        if apply_cant_be_regenerated_to_effect(effect) {
            return true;
        }
    }
    false
}

/// The choice a declared target denotes.
///
/// A target phrase names what the effect will act on; this is the same fact in
/// the vocabulary resolution and lowering both consume. It reads only the AST.
pub fn choose_spec_for_target(target: &TargetAst) -> ChooseSpec {
    match target {
        TargetAst::Source(_) => ChooseSpec::Source,
        TargetAst::AnyTarget(_) => ChooseSpec::AnyTarget,
        TargetAst::AnyOtherTarget(_) => ChooseSpec::AnyOtherTarget,
        TargetAst::ObjectOrPlayer(object_filter, player_filter, explicit_target_span) => {
            let spec = ChooseSpec::ObjectOrPlayer(object_filter.clone(), player_filter.clone());
            if explicit_target_span.is_some() {
                ChooseSpec::target(spec)
            } else {
                spec
            }
        }
        TargetAst::PlayerOrPlaneswalker(filter, _) => {
            ChooseSpec::PlayerOrPlaneswalker(filter.clone())
        }
        TargetAst::AttackedPlayerOrPlaneswalker(_) => ChooseSpec::AttackedPlayerOrPlaneswalker,
        TargetAst::Spell(_) => ChooseSpec::target_spell(),
        TargetAst::Player(filter, explicit_target_span) => {
            if *filter == PlayerFilter::You {
                ChooseSpec::SourceController
            } else if *filter == PlayerFilter::IteratedPlayer {
                ChooseSpec::Player(filter.clone())
            } else if explicit_target_span.is_some() {
                ChooseSpec::target(ChooseSpec::Player(filter.clone()))
            } else {
                ChooseSpec::Player(filter.clone())
            }
        }
        TargetAst::Object(filter, explicit_target_span, reference_span) => {
            let spec = if filter.source && filter.zone != Some(Zone::Exile) {
                source_reference_hinted_spec(ChooseSpec::Source, filter.source_surface.clone())
            } else if explicit_target_span.is_some() {
                ChooseSpec::target(ChooseSpec::Object(filter.clone()))
            } else {
                ChooseSpec::Object(filter.clone())
            };
            let _ = reference_span;
            source_reference_hinted_spec(spec, filter.source_surface.clone())
        }
        TargetAst::Tagged(tag, _) => ChooseSpec::Tagged(tag.clone().into()),
        TargetAst::WithCount(inner, count) => choose_spec_for_target(inner).with_count(*count),
        TargetAst::WithCountValue(inner, count, value) => {
            choose_spec_for_target(inner).with_count_value(*count, value.clone())
        }
    }
}

pub fn source_reference_hinted_spec(
    spec: ChooseSpec,
    surface: Option<SourceReferenceSurface>,
) -> ChooseSpec {
    match surface {
        Some(surface) => spec.with_surface_hint(ChooseSpecSurfaceHint::SourceReference(surface)),
        None => spec,
    }
}
