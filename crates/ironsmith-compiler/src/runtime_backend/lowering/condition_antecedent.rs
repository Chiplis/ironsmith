use crate::cards::builders::{EffectAst, IT_TAG, PredicateAst, SubjectVerbActionAst, TargetAst};
use crate::effect::Value;
use crate::filter::{ObjectFilter, TaggedOpbjectRelation};
use crate::object::CounterType;

use super::effect_ast_traversal::for_each_nested_effects_mut;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConditionAntecedentBinding {
    TaggedItOnly,
    IncludeRandomWithCountObjects,
}

pub(crate) fn predicate_object_filter_antecedent(predicate: &PredicateAst) -> Option<ObjectFilter> {
    match predicate {
        // "if enchanted creature is untapped, tap it": the tagged condition
        // subject is the antecedent for "it" in the body effects.
        PredicateAst::TaggedMatches(tag, _) => Some(ObjectFilter::tagged(tag.clone())),
        PredicateAst::PlayerControls { filter, .. }
        | PredicateAst::PlayerHasAtLeast { filter, .. }
        | PredicateAst::PlayerControlsExactly { filter, .. }
        | PredicateAst::PlayerHasAtLeastWithDifferentPowers { filter, .. }
        | PredicateAst::PlayerControlsNo { filter, .. }
        | PredicateAst::PlayerControlsMost { filter, .. }
        | PredicateAst::PlayerControlsMoreThanEachOtherPlayer { filter, .. }
        | PredicateAst::AnOpponentHasFewerThanPlayer { filter, .. } => Some(filter.clone()),
        PredicateAst::ValueComparison {
            left: crate::effect::Value::Count(filter),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            ..
        } => Some(filter.clone()),
        PredicateAst::And(left, right) | PredicateAst::Or(left, right) => {
            predicate_object_filter_antecedent(left)
                .or_else(|| predicate_object_filter_antecedent(right))
        }
        PredicateAst::Not(inner) => predicate_object_filter_antecedent(inner),
        _ => None,
    }
}

pub(crate) fn predicate_source_counter_antecedent(predicate: &PredicateAst) -> Option<CounterType> {
    match predicate {
        PredicateAst::SourceHasCounterAtLeast { counter_type, .. } => Some(*counter_type),
        PredicateAst::And(left, right) => match (
            predicate_source_counter_antecedent(left),
            predicate_source_counter_antecedent(right),
        ) {
            (Some(left), Some(right)) if left == right => Some(left),
            (Some(counter_type), None) | (None, Some(counter_type)) => Some(counter_type),
            _ => None,
        },
        _ => None,
    }
}

fn merge_filter_overlay(base: &mut ObjectFilter, overlay: ObjectFilter) {
    if let Some(zone) = overlay.zone {
        base.zone.get_or_insert(zone);
    }
    if base.controller.is_none() {
        base.controller = overlay.controller;
    }
    if base.owner.is_none() {
        base.owner = overlay.owner;
    }
    base.other |= overlay.other;
    for card_type in overlay.card_types {
        if !base.card_types.contains(&card_type) {
            base.card_types.push(card_type);
        }
    }
    for subtype in overlay.subtypes {
        if !base.subtypes.contains(&subtype) {
            base.subtypes.push(subtype);
        }
    }
    if let Some(colors) = overlay.colors {
        base.colors = Some(
            base.colors
                .map_or(colors, |existing| existing.intersection(colors)),
        );
    }
}

fn bind_condition_filter_antecedent(filter: &mut ObjectFilter, antecedent: &ObjectFilter) {
    let references_it = filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag.as_str() == IT_TAG
            && matches!(constraint.relation, TaggedOpbjectRelation::IsTaggedObject)
    });
    if !references_it {
        return;
    }

    let mut overlay = filter.clone();
    overlay.tagged_constraints.retain(|constraint| {
        !(constraint.tag.as_str() == IT_TAG
            && matches!(constraint.relation, TaggedOpbjectRelation::IsTaggedObject))
    });
    let mut replacement = antecedent.clone();
    merge_filter_overlay(&mut replacement, overlay);
    *filter = replacement;
}

fn bind_random_those_filter(filter: &mut ObjectFilter, antecedent: &ObjectFilter) {
    let mut replacement = antecedent.clone();
    merge_filter_overlay(&mut replacement, filter.clone());
    *filter = replacement;
}

fn bind_condition_antecedent_in_target(
    target: &mut TargetAst,
    antecedent: &ObjectFilter,
    mode: ConditionAntecedentBinding,
) {
    match target {
        TargetAst::Object(filter, _, _) => bind_condition_filter_antecedent(filter, antecedent),
        // "if enchanted creature is untapped, tap it": a bare `it` target
        // binds to the condition subject.
        TargetAst::Tagged(tag, span) if tag.as_str() == IT_TAG => {
            *target = TargetAst::Object(antecedent.clone(), *span, None);
        }
        TargetAst::WithCount(inner, count) => {
            if matches!(
                mode,
                ConditionAntecedentBinding::IncludeRandomWithCountObjects
            ) && count.random
                && let TargetAst::Object(filter, _, _) = inner.as_mut()
                && filter.tagged_constraints.is_empty()
                && filter.with_counter.is_none()
            {
                bind_random_those_filter(filter, antecedent);
            } else {
                bind_condition_antecedent_in_target(inner, antecedent, mode);
            }
        }
        _ => {}
    }
}

fn bind_condition_antecedent_in_effect(
    effect: &mut EffectAst,
    antecedent: &ObjectFilter,
    mode: ConditionAntecedentBinding,
) {
    match effect {
        EffectAst::SubjectVerb(subject_verb) => match &mut subject_verb.action {
            SubjectVerbActionAst::Tap { target }
            | SubjectVerbActionAst::Untap { target }
            | SubjectVerbActionAst::Destroy { target, .. }
            | SubjectVerbActionAst::Exile { target, .. }
            | SubjectVerbActionAst::DealDamage { target, .. }
            | SubjectVerbActionAst::DealDamageEqualToPower { target, .. } => {
                bind_condition_antecedent_in_target(target, antecedent, mode);
            }
            _ => {}
        },
        EffectAst::ChooseObjects { filter, .. }
        | EffectAst::ChooseObjectsAcrossZones { filter, .. } => {
            bind_condition_filter_antecedent(filter, antecedent);
        }
        _ => {}
    }

    for_each_nested_effects_mut(effect, true, |nested| {
        bind_condition_antecedent_in_effects(nested, antecedent, mode);
    });
}

pub(crate) fn bind_condition_antecedent_in_effects(
    effects: &mut [EffectAst],
    antecedent: &ObjectFilter,
    mode: ConditionAntecedentBinding,
) {
    for effect in effects {
        bind_condition_antecedent_in_effect(effect, antecedent, mode);
    }
}

fn bind_condition_counter_antecedent_in_effect(effect: &mut EffectAst, counter_type: CounterType) {
    if let EffectAst::SubjectVerb(subject_verb) = effect
        && let SubjectVerbActionAst::RemoveUpToAnyCounters {
            amount,
            target,
            counter_type: remove_counter_type,
            all_of_them,
            ..
        } = &mut subject_verb.action
        && *all_of_them
        && remove_counter_type.is_none()
        && matches!(target, TargetAst::Source(_))
    {
        *amount = Value::CountersOnSource(counter_type);
        *remove_counter_type = Some(counter_type);
        *all_of_them = false;
    }

    for_each_nested_effects_mut(effect, true, |nested| {
        bind_condition_counter_antecedent_in_effects(nested, counter_type);
    });
}

pub(crate) fn bind_condition_counter_antecedent_in_effects(
    effects: &mut [EffectAst],
    counter_type: CounterType,
) {
    for effect in effects {
        bind_condition_counter_antecedent_in_effect(effect, counter_type);
    }
}

fn retarget_it_animation_target_to_source(target: &mut TargetAst) {
    match target {
        TargetAst::Tagged(tag, span) if tag.as_str() == IT_TAG => {
            *target = TargetAst::Source(*span);
        }
        TargetAst::WithCount(inner, _) => retarget_it_animation_target_to_source(inner),
        _ => {}
    }
}

pub(crate) fn retarget_it_animation_to_source(effect: &mut EffectAst) {
    if let EffectAst::SubjectVerb(subject_verb) = effect
        && let SubjectVerbActionAst::BecomeBasePtCreature { target, .. }
        | SubjectVerbActionAst::GrantAbilitiesToTarget { target, .. }
        | SubjectVerbActionAst::GrantToTarget { target, .. }
        | SubjectVerbActionAst::RemoveAbilitiesFromTarget { target, .. }
        | SubjectVerbActionAst::GrantAbilitiesChoiceToTarget { target, .. } =
            &mut subject_verb.action
    {
        retarget_it_animation_target_to_source(target);
    }

    for_each_nested_effects_mut(effect, true, |nested| {
        retarget_it_animations_to_source(nested);
    });
}

pub(crate) fn retarget_it_animations_to_source(effects: &mut [EffectAst]) {
    for effect in effects {
        retarget_it_animation_to_source(effect);
    }
}
