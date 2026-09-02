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
use super::{EffectAst, SubjectVerbActionAst};

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
            Some(ObjectFilter::default().not_tagged(CompilerReferenceTag::It.key()))
        }
        TargetAst::WithCount(inner, _) => target_ast_to_object_filter(*inner),
        _ => None,
    }
}

pub fn primary_damage_target_from_effect(effect: &EffectAst) -> Option<TargetAst> {
    match effect {
        EffectAst::SubjectVerb(subject_verb) => match &subject_verb.action {
            SubjectVerbActionAst::DealDamage { target, .. }
            | SubjectVerbActionAst::DealDistributedDamage { target, .. }
            | SubjectVerbActionAst::DealDamageEqualToPower { target, .. } => Some(target.clone()),
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
            SubjectVerbActionAst::DealDamage { target, .. }
            | SubjectVerbActionAst::DealDistributedDamage { target, .. }
            | SubjectVerbActionAst::DealDamageEqualToPower { target, .. }
            | SubjectVerbActionAst::Tap { target }
            | SubjectVerbActionAst::Untap { target }
            | SubjectVerbActionAst::Destroy { target, .. }
            | SubjectVerbActionAst::Exile { target, .. }
            | SubjectVerbActionAst::LookAtHand { target }
            | SubjectVerbActionAst::Counter { target }
            | SubjectVerbActionAst::CounterUnlessPays { target, .. }
            | SubjectVerbActionAst::PutCounters { target, .. }
            | SubjectVerbActionAst::PutCounterChoice { target, .. }
            | SubjectVerbActionAst::ReturnToHand { target, .. }
            | SubjectVerbActionAst::Detain { target }
            | SubjectVerbActionAst::Goad { target, .. }
            | SubjectVerbActionAst::Suspect { target }
            | SubjectVerbActionAst::RemoveFromCombat { target }
            | SubjectVerbActionAst::Flip { target }
            | SubjectVerbActionAst::Regenerate { target, .. }
            | SubjectVerbActionAst::TapOrUntap { target }
            | SubjectVerbActionAst::PhaseOut { target, .. }
            | SubjectVerbActionAst::PhaseIn { target }
            | SubjectVerbActionAst::Transform { target }
            | SubjectVerbActionAst::Convert { target }
            | SubjectVerbActionAst::Explore { target }
            | SubjectVerbActionAst::Endure { target, .. }
            | SubjectVerbActionAst::Connive { target, .. }
            | SubjectVerbActionAst::MoveToLibraryNthFromTop { target, .. }
            | SubjectVerbActionAst::MoveToLibraryTopOrBottomChoice { target }
            | SubjectVerbActionAst::RemoveUpToAnyCounters { target, .. }
            | SubjectVerbActionAst::ForEachCounterKindPutOrRemove { target, .. }
            | SubjectVerbActionAst::PutCounterOfChosenKind { target }
            | SubjectVerbActionAst::PutSticker { target, .. }
            | SubjectVerbActionAst::SwitchPowerToughness { target, .. }
            | SubjectVerbActionAst::GrantProtectionChoice { target, .. }
            | SubjectVerbActionAst::AssignNoCombatDamage { source: target, .. }
            | SubjectVerbActionAst::PreventAllCombatDamageFromSource { source: target, .. }
            | SubjectVerbActionAst::ExileWhenSourceLeaves { target }
            | SubjectVerbActionAst::SacrificeSourceWhenLeaves { target }
            | SubjectVerbActionAst::RedirectNextTimeDamageToSource { target, .. }
            | SubjectVerbActionAst::RedirectAllDamageThisTurnBySourceToSourceController {
                source: target,
            }
            | SubjectVerbActionAst::PreventDamage { target, .. }
            | SubjectVerbActionAst::PreventAllDamageToTarget { target, .. }
            | SubjectVerbActionAst::PreventDamageToTargetPutCounters { target, .. }
            | SubjectVerbActionAst::PutOrRemoveCounters { target, .. }
            | SubjectVerbActionAst::DoubleCountersOnTarget { target, .. }
            | SubjectVerbActionAst::ExileUntilSourceLeaves { target, .. }
            | SubjectVerbActionAst::ReturnToBattlefield { target, .. }
            | SubjectVerbActionAst::MoveToZone { target, .. }
            | SubjectVerbActionAst::TargetOnly { target, .. }
            | SubjectVerbActionAst::Pump { target, .. }
            | SubjectVerbActionAst::SetBasePowerToughness { target, .. }
            | SubjectVerbActionAst::BecomeBasePtCreature { target, .. }
            | SubjectVerbActionAst::SetBasePower { target, .. }
            | SubjectVerbActionAst::PumpForEach { target, .. }
            | SubjectVerbActionAst::PumpByLastEffect { target, .. }
            | SubjectVerbActionAst::GainControl { target, .. }
            | SubjectVerbActionAst::GrantAbilitiesToTarget { target, .. }
            | SubjectVerbActionAst::GrantToTarget { target, .. }
            | SubjectVerbActionAst::GrantAbilitiesChoiceToTarget { target, .. } => {
                Some(target.clone())
            }
            SubjectVerbActionAst::RedirectNextDamageFromSourceToTarget {
                protected_target,
                destination_target,
                ..
            } => protected_target
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
            tag: CompilerReferenceTag::It.key(),
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
            SubjectVerbActionAst::Destroy {
                no_regeneration, ..
            }
            | SubjectVerbActionAst::DestroyAll {
                no_regeneration, ..
            }
            | SubjectVerbActionAst::DestroyAllOfChosenColor {
                no_regeneration, ..
            } => {
                *no_regeneration = true;
                true
            }
            _ => false,
        },
        EffectAst::ChooseOneOf { modes } | EffectAst::VillainousChoice { modes, .. } => {
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
        TargetAst::Tagged(tag, _) => ChooseSpec::Tagged(tag.clone()),
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
