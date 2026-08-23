use crate::cards::builders::{
    EffectAst, GrantedAbilityAst, IT_TAG, PredicateAst, SubjectVerbActionAst, TargetAst,
};
use crate::effect::Value;
use crate::filter::{ObjectFilter, TaggedOpbjectRelation};
use crate::object::CounterType;

use super::effect_ast_traversal::for_each_nested_effects_mut;

/// A resolution-time object choice that is grammatically nested inside the
/// following action (for example, "gains control of one of those lands of
/// their choice").  Reference tracking deliberately does not export this tag
/// until the consuming action runs, so an earlier subject such as "that
/// creature's controller" still resolves against the trigger object.
pub const CONDITION_COLLECTION_CHOICE_TAG: &str = "__condition_collection_choice";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionAntecedentBinding {
    TaggedItOnly,
    IncludeRandomWithCountObjects,
    RandomWithCountObjectsOnly,
}

pub fn predicate_object_filter_antecedent(predicate: &PredicateAst) -> Option<ObjectFilter> {
    match predicate {
        // "if enchanted creature is untapped, tap it": the tagged condition
        // subject is the antecedent for "it" in the body effects.
        PredicateAst::TaggedMatches(tag, _) => Some(ObjectFilter::tagged(tag.clone())),
        PredicateAst::And(left, right) => match (
            predicate_object_filter_antecedent(left),
            predicate_object_filter_antecedent(right),
        ) {
            (Some(left), Some(right)) if left == right => Some(left),
            (Some(antecedent), None) | (None, Some(antecedent)) => Some(antecedent),
            _ => None,
        },
        // Either branch of an `or` can make the condition true, so it only
        // establishes an object antecedent when both branches explicitly name
        // the same tagged object. Existential/count predicates and negations
        // describe game state; their filters are not discourse referents.
        PredicateAst::Or(left, right) => match (
            predicate_object_filter_antecedent(left),
            predicate_object_filter_antecedent(right),
        ) {
            (Some(left), Some(right)) if left == right => Some(left),
            _ => None,
        },
        _ => None,
    }
}

fn predicate_random_count_object_filter_antecedent(
    predicate: &PredicateAst,
) -> Option<ObjectFilter> {
    match predicate {
        PredicateAst::ValueComparison { left, right, .. } => {
            match (left.unhinted(), right.unhinted()) {
                (Value::Count(left), Value::Count(right)) if left == right => Some(left.clone()),
                (Value::Count(filter), value) | (value, Value::Count(filter))
                    if !matches!(value, Value::Count(_)) =>
                {
                    Some(filter.clone())
                }
                _ => None,
            }
        }
        PredicateAst::PlayerHasAtLeast { filter, .. }
        | PredicateAst::PlayerControlsExactly { filter, .. }
        | PredicateAst::PlayerHasAtLeastWithDifferentPowers { filter, .. } => Some(filter.clone()),
        PredicateAst::And(left, right) => match (
            predicate_random_count_object_filter_antecedent(left),
            predicate_random_count_object_filter_antecedent(right),
        ) {
            (Some(left), Some(right)) if left == right => Some(left),
            (Some(antecedent), None) | (None, Some(antecedent)) => Some(antecedent),
            _ => None,
        },
        PredicateAst::Or(left, right) => match (
            predicate_random_count_object_filter_antecedent(left),
            predicate_random_count_object_filter_antecedent(right),
        ) {
            (Some(left), Some(right)) if left == right => Some(left),
            _ => None,
        },
        _ => None,
    }
}

pub fn predicate_source_counter_antecedent(predicate: &PredicateAst) -> Option<CounterType> {
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

pub fn bind_condition_filter_antecedent(filter: &mut ObjectFilter, antecedent: &ObjectFilter) {
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
        TargetAst::Object(filter, _, _)
            if !matches!(mode, ConditionAntecedentBinding::RandomWithCountObjectsOnly) =>
        {
            bind_condition_filter_antecedent(filter, antecedent);
        }
        // "if enchanted creature is untapped, tap it": a bare `it` target
        // binds to the condition subject.
        TargetAst::Tagged(tag, span)
            if tag.as_str() == IT_TAG
                && !matches!(mode, ConditionAntecedentBinding::RandomWithCountObjectsOnly) =>
        {
            *target = TargetAst::Object(antecedent.clone(), *span, None);
        }
        TargetAst::WithCount(inner, count) => {
            if matches!(
                mode,
                ConditionAntecedentBinding::IncludeRandomWithCountObjects
                    | ConditionAntecedentBinding::RandomWithCountObjectsOnly
            ) && count.random
            {
                if let TargetAst::Object(filter, _, _) = inner.as_mut() {
                    let references_it = filter.tagged_constraints.iter().any(|constraint| {
                        constraint.tag.as_str() == IT_TAG
                            && matches!(constraint.relation, TaggedOpbjectRelation::IsTaggedObject)
                    });
                    if references_it {
                        bind_condition_filter_antecedent(filter, antecedent);
                    } else if filter.tagged_constraints.is_empty() && filter.with_counter.is_none()
                    {
                        bind_random_those_filter(filter, antecedent);
                    }
                } else {
                    bind_condition_antecedent_in_target(
                        inner,
                        antecedent,
                        ConditionAntecedentBinding::TaggedItOnly,
                    );
                }
            } else if !matches!(mode, ConditionAntecedentBinding::RandomWithCountObjectsOnly) {
                bind_condition_antecedent_in_target(inner, antecedent, mode);
            }
        }
        _ => {}
    }
}

fn target_establishes_body_object_antecedent(target: &TargetAst) -> bool {
    match target {
        TargetAst::Source(_)
        | TargetAst::AnyTarget(_)
        | TargetAst::AnyOtherTarget(_)
        | TargetAst::ObjectOrPlayer(_, _, _)
        | TargetAst::Object(_, _, _) => true,
        TargetAst::Tagged(tag, _) => tag.as_str() != IT_TAG,
        TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, _, _) => {
            target_establishes_body_object_antecedent(inner)
        }
        TargetAst::PlayerOrPlaneswalker(_, _)
        | TargetAst::AttackedPlayerOrPlaneswalker(_)
        | TargetAst::Spell(_)
        | TargetAst::Player(_, _) => false,
    }
}

fn effect_establishes_body_object_antecedent(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::SubjectVerb(subject_verb) => match &subject_verb.action {
            SubjectVerbActionAst::Tap { target }
            | SubjectVerbActionAst::Untap { target }
            | SubjectVerbActionAst::TapOrUntap { target }
            | SubjectVerbActionAst::Destroy { target, .. }
            | SubjectVerbActionAst::Exile { target, .. }
            | SubjectVerbActionAst::DealDamage { target, .. }
            | SubjectVerbActionAst::DealDamageEqualToPower { target, .. }
            | SubjectVerbActionAst::GainControl { target, .. }
            | SubjectVerbActionAst::PutCounters { target, .. }
            | SubjectVerbActionAst::PutCounterChoice { target, .. }
            | SubjectVerbActionAst::Pump { target, .. }
            | SubjectVerbActionAst::PumpForEach { target, .. }
            | SubjectVerbActionAst::GrantAbilitiesToTarget { target, .. }
            | SubjectVerbActionAst::GrantToTarget { target, .. }
            | SubjectVerbActionAst::RemoveAbilitiesFromTarget { target, .. }
            | SubjectVerbActionAst::GrantAbilitiesChoiceToTarget { target, .. }
            | SubjectVerbActionAst::TargetOnly { target, .. } => {
                target_establishes_body_object_antecedent(target)
            }
            // These actions export a generated or selected object set when a
            // later body clause references `it`/`those`. Once one occurs, the
            // condition subject is no longer the body's newest antecedent.
            SubjectVerbActionAst::Mill { .. }
            | SubjectVerbActionAst::Discover { .. }
            | SubjectVerbActionAst::ManifestTopCardOfLibrary
            | SubjectVerbActionAst::CloakTopCardOfLibrary
            | SubjectVerbActionAst::ManifestCardFromHand
            | SubjectVerbActionAst::Amass { .. }
            | SubjectVerbActionAst::Populate { .. }
            | SubjectVerbActionAst::CreateTokenCopy { .. }
            | SubjectVerbActionAst::CreateTokenCopyFromSource { .. }
            | SubjectVerbActionAst::CreateTokenWithMods { .. } => true,
            _ => false,
        },
        EffectAst::ChooseObjects { .. }
        | EffectAst::ChooseObjectsWithAggregateConstraint { .. }
        | EffectAst::ChooseObjectsAcrossZones { .. } => true,
        _ => false,
    }
}

fn bind_condition_antecedent_in_effect(
    effect: &mut EffectAst,
    antecedent: &ObjectFilter,
    mode: ConditionAntecedentBinding,
) -> bool {
    // Only an object reference that was already explicit in the body shadows
    // the condition antecedent. An `it` target resolved below still belongs to
    // the condition and must not prevent later `it` targets from resolving to
    // that same antecedent.
    let establishes_body_antecedent = effect_establishes_body_object_antecedent(effect);
    match effect {
        EffectAst::SubjectVerb(subject_verb) => match &mut subject_verb.action {
            SubjectVerbActionAst::Tap { target }
            | SubjectVerbActionAst::Untap { target }
            | SubjectVerbActionAst::TapOrUntap { target }
            | SubjectVerbActionAst::Destroy { target, .. }
            | SubjectVerbActionAst::Exile { target, .. }
            | SubjectVerbActionAst::DealDamage { target, .. }
            | SubjectVerbActionAst::DealDamageEqualToPower { target, .. }
            | SubjectVerbActionAst::GainControl { target, .. }
            | SubjectVerbActionAst::PutCounters { target, .. }
            | SubjectVerbActionAst::PutCounterChoice { target, .. }
            | SubjectVerbActionAst::Pump { target, .. }
            | SubjectVerbActionAst::PumpForEach { target, .. }
            | SubjectVerbActionAst::GrantAbilitiesToTarget { target, .. }
            | SubjectVerbActionAst::GrantToTarget { target, .. }
            | SubjectVerbActionAst::RemoveAbilitiesFromTarget { target, .. }
            | SubjectVerbActionAst::GrantAbilitiesChoiceToTarget { target, .. }
            | SubjectVerbActionAst::TargetOnly { target, .. } => {
                bind_condition_antecedent_in_target(target, antecedent, mode);
            }
            _ => {}
        },
        EffectAst::ChooseObjects { filter, .. }
        | EffectAst::ChooseObjectsWithAggregateConstraint { filter, .. }
        | EffectAst::ChooseObjectsAcrossZones { filter, .. } => {
            bind_condition_filter_antecedent(filter, antecedent);
        }
        _ => {}
    }

    if establishes_body_antecedent {
        return true;
    }

    let mut saw_nested = false;
    let mut every_nested_branch_establishes = true;
    for_each_nested_effects_mut(effect, true, |nested| {
        saw_nested = true;
        every_nested_branch_establishes &=
            bind_condition_antecedent_in_effects_internal(nested, antecedent, mode);
    });
    saw_nested && every_nested_branch_establishes
}

fn bind_condition_antecedent_in_effects_internal(
    effects: &mut [EffectAst],
    antecedent: &ObjectFilter,
    mode: ConditionAntecedentBinding,
) -> bool {
    for effect in effects {
        if bind_condition_antecedent_in_effect(effect, antecedent, mode) {
            return true;
        }
    }
    false
}

pub fn bind_condition_antecedent_in_effects(
    effects: &mut [EffectAst],
    antecedent: &ObjectFilter,
    mode: ConditionAntecedentBinding,
) {
    let _ = bind_condition_antecedent_in_effects_internal(effects, antecedent, mode);
}

/// Bind an explicit collection choice such as "choose one of those creatures"
/// to the positive existential set established by an intervening condition.
/// This is intentionally narrower than the ordinary object-antecedent binder:
/// an existential condition alone does not make a bare `it` unambiguous, but
/// the parser's tagged collection constraint records an authored `those`.
pub fn bind_condition_collection_antecedent_in_effects(
    effects: &mut [EffectAst],
    predicate: &PredicateAst,
) {
    fn is_source_exiled_collection(filter: &ObjectFilter) -> bool {
        filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG
                && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
        })
    }

    fn collection_filter(predicate: &PredicateAst) -> Option<ObjectFilter> {
        match predicate {
            PredicateAst::PlayerControls { filter, .. } => Some(filter.clone()),
            PredicateAst::ValueComparison { left, right, .. } => {
                match (left.unhinted(), right.unhinted()) {
                    (Value::Count(filter), Value::Fixed(_))
                    | (Value::Fixed(_), Value::Count(filter))
                        if is_source_exiled_collection(filter) =>
                    {
                        Some(filter.clone())
                    }
                    _ => None,
                }
            }
            PredicateAst::And(left, right) => {
                match (collection_filter(left), collection_filter(right)) {
                    (Some(left), Some(right)) if left == right => Some(left),
                    (Some(filter), None) | (None, Some(filter)) => Some(filter),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn rewrite_inline_collection_choice(effect: &mut EffectAst, antecedent: &ObjectFilter) -> bool {
        let EffectAst::SubjectVerb(subject_verb) = effect else {
            return false;
        };
        let SubjectVerbActionAst::GainControl { target, .. } = &mut subject_verb.action else {
            return false;
        };
        let TargetAst::WithCount(inner, count) = target else {
            return false;
        };
        if !count.is_single() {
            return false;
        }
        let TargetAst::Object(filter, _, _) = inner.as_ref() else {
            return false;
        };
        let references_condition_collection = filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == IT_TAG
                && matches!(constraint.relation, TaggedOpbjectRelation::IsTaggedObject)
        });
        if !references_condition_collection {
            return false;
        }

        let mut choice_filter = antecedent.clone();
        // The condition's subject is plural ("one or more lands"), while the
        // nested choice selects exactly one member. Keep the authored plural
        // counter noun but render the selected permanent itself as singular.
        let (one_or_more, plural_noun, _) =
            choice_filter.union_surface.counter_requirement_surface();
        choice_filter.union_surface = choice_filter
            .union_surface
            .with_counter_requirement_surface(one_or_more, plural_noun, false);
        // `PlayerControls` keeps its actor separate from the object filter.
        // Make the copied filter chooser-relative so the player denoted by
        // "their choice" can only choose among permanents they control.
        choice_filter
            .controller
            .get_or_insert(crate::filter::PlayerFilter::IteratedPlayer);
        let tag = crate::tag::TagKey::from(CONDITION_COLLECTION_CHOICE_TAG);
        let choice = EffectAst::ChooseObjects {
            filter: choice_filter,
            count: *count,
            count_value: None,
            player: crate::cards::builders::PlayerAst::That,
            tag: tag.clone(),
        };
        *target = TargetAst::Tagged(tag, None);
        let gain_control = std::mem::replace(
            effect,
            EffectAst::Sequence {
                effects: Vec::new(),
            },
        );
        *effect = EffectAst::Sequence {
            effects: vec![choice, gain_control],
        };
        true
    }

    fn rewrite_plural_collection_move(effect: &mut EffectAst, antecedent: &ObjectFilter) -> bool {
        let EffectAst::SubjectVerb(subject_verb) = effect else {
            return false;
        };
        let SubjectVerbActionAst::MoveToZone {
            target,
            target_plural_surface,
            all,
            ..
        } = &mut subject_verb.action
        else {
            return false;
        };
        if !*target_plural_surface || !target_references_tag(target, |tag| tag == IT_TAG) {
            return false;
        }
        bind_condition_antecedent_in_target(
            target,
            antecedent,
            ConditionAntecedentBinding::TaggedItOnly,
        );
        *all = true;
        true
    }

    fn bind(effect: &mut EffectAst, antecedent: &ObjectFilter) {
        if rewrite_inline_collection_choice(effect, antecedent) {
            return;
        }
        if rewrite_plural_collection_move(effect, antecedent) {
            return;
        }
        match effect {
            EffectAst::ChooseObjects { filter, .. }
            | EffectAst::ChooseObjectsWithAggregateConstraint { filter, .. }
            | EffectAst::ChooseObjectsAcrossZones { filter, .. } => {
                bind_condition_filter_antecedent(filter, antecedent);
            }
            _ => {}
        }
        for_each_nested_effects_mut(effect, true, |nested| {
            for nested_effect in nested {
                bind(nested_effect, antecedent);
            }
        });
    }

    let Some(antecedent) = collection_filter(predicate) else {
        return;
    };
    for effect in effects {
        bind(effect, &antecedent);
    }
}

pub fn bind_random_count_condition_antecedent_in_effects(
    effects: &mut [EffectAst],
    predicate: &PredicateAst,
) {
    let Some(antecedent) = predicate_random_count_object_filter_antecedent(predicate) else {
        return;
    };
    bind_condition_antecedent_in_effects(
        effects,
        &antecedent,
        ConditionAntecedentBinding::RandomWithCountObjectsOnly,
    );
}

#[derive(Debug, Clone, Copy, Default)]
struct ObservationAntecedentState {
    saw_top_library_observation: bool,
    observed_object_was_moved: bool,
}

fn target_references_tag(target: &TargetAst, expected: impl Fn(&str) -> bool + Copy) -> bool {
    match target {
        TargetAst::Tagged(tag, _) => expected(tag.as_str()),
        TargetAst::Object(filter, _, _) => filter.tagged_constraints.iter().any(|constraint| {
            matches!(constraint.relation, TaggedOpbjectRelation::IsTaggedObject)
                && expected(constraint.tag.as_str())
        }),
        TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, _, _) => {
            target_references_tag(inner, expected)
        }
        _ => false,
    }
}

fn target_references_observed_object(target: &TargetAst) -> bool {
    target_references_tag(target, |tag| {
        tag == IT_TAG || tag == "__public_revealed" || tag.starts_with("__sentence_helper_revealed")
    })
}

fn retarget_unresolved_it(target: &mut TargetAst, antecedent_tag: &crate::tag::TagKey) {
    match target {
        TargetAst::Tagged(tag, _) if tag.as_str() == IT_TAG => {
            *tag = antecedent_tag.clone();
        }
        TargetAst::Object(filter, explicit_target_span, _) if explicit_target_span.is_none() => {
            for constraint in &mut filter.tagged_constraints {
                if constraint.tag.as_str() == IT_TAG
                    && matches!(constraint.relation, TaggedOpbjectRelation::IsTaggedObject)
                {
                    constraint.tag = antecedent_tag.clone();
                }
            }
        }
        TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, _, _) => {
            retarget_unresolved_it(inner, antecedent_tag);
        }
        _ => {}
    }
}

fn is_top_library_observation(action: &SubjectVerbActionAst) -> bool {
    matches!(
        action,
        SubjectVerbActionAst::RevealTop | SubjectVerbActionAst::LookAtTopCards { .. }
    )
}

fn persistent_battlefield_subject(action: &mut SubjectVerbActionAst) -> Option<&mut TargetAst> {
    match action {
        // These actions require a battlefield object. Immediately after a
        // top-library observation, a still-unmoved card cannot be their
        // subject, so an unresolved pronoun continues to denote the trigger's
        // persistent object instead.
        SubjectVerbActionAst::Destroy { target, .. }
        | SubjectVerbActionAst::Pump { target, .. }
        | SubjectVerbActionAst::RemoveFromCombat { target } => Some(target),
        _ => None,
    }
}

fn moves_observed_object(action: &SubjectVerbActionAst) -> bool {
    let target = match action {
        SubjectVerbActionAst::MoveToZone { target, .. }
        | SubjectVerbActionAst::MayMoveToZone { target, .. }
        | SubjectVerbActionAst::PutOntoBattlefield { target, .. }
        | SubjectVerbActionAst::MoveToLibraryTopOrBottomChoice { target }
        | SubjectVerbActionAst::ReturnToBattlefield { target, .. }
        | SubjectVerbActionAst::Exile { target, .. } => target,
        _ => return false,
    };
    target_references_observed_object(target)
}

fn bind_trigger_antecedent_after_observation_in_effects(
    effects: &mut [EffectAst],
    antecedent_tag: &crate::tag::TagKey,
    mut state: ObservationAntecedentState,
) -> ObservationAntecedentState {
    for effect in effects {
        if let EffectAst::SubjectVerb(subject_verb) = effect {
            if is_top_library_observation(&subject_verb.action) {
                state.saw_top_library_observation = true;
                state.observed_object_was_moved = false;
            } else {
                if state.saw_top_library_observation
                    && !state.observed_object_was_moved
                    && let Some(target) = persistent_battlefield_subject(&mut subject_verb.action)
                {
                    retarget_unresolved_it(target, antecedent_tag);
                }
                if state.saw_top_library_observation && moves_observed_object(&subject_verb.action)
                {
                    state.observed_object_was_moved = true;
                }
            }
        }

        if state.saw_top_library_observation {
            let nested_initial = state;
            let mut nested_outcomes = Vec::new();
            for_each_nested_effects_mut(effect, true, |nested| {
                if !nested.is_empty() {
                    nested_outcomes.push(bind_trigger_antecedent_after_observation_in_effects(
                        nested,
                        antecedent_tag,
                        nested_initial,
                    ));
                }
            });
            // A moved object is a safe subsequent antecedent only when every
            // represented branch moves it. A single nested sequence (including
            // a `may` wrapper) carries its result forward within that sequence.
            if !nested_outcomes.is_empty()
                && nested_outcomes
                    .iter()
                    .all(|outcome| outcome.observed_object_was_moved)
            {
                state.observed_object_was_moved = true;
            }
        }
    }
    state
}

/// Preserve the trigger's object antecedent across a top-library observation.
///
/// Revealing or looking at a card makes that card the ordinary `it` referent.
/// A battlefield-only action cannot apply to that card until a zone move makes
/// it a permanent, however, so such an action still refers to the persistent
/// object supplied by the trigger. Once the observed card moves, subsequent
/// references follow the moved result instead.
pub fn bind_trigger_antecedent_after_top_library_observation(
    effects: &mut [EffectAst],
    antecedent_tag: &crate::tag::TagKey,
) {
    let _ = bind_trigger_antecedent_after_observation_in_effects(
        effects,
        antecedent_tag,
        ObservationAntecedentState::default(),
    );
}

fn source_deals_damage_to_player(effect: &EffectAst) -> bool {
    matches!(
        effect,
        EffectAst::SubjectVerb(subject_verb)
            if matches!(
                &subject_verb.action,
                SubjectVerbActionAst::DealDamage {
                    target: TargetAst::Player(_, _),
                    ..
                }
            )
    )
}

fn retarget_implicit_must_attack_to_source(effect: &mut EffectAst) {
    let EffectAst::SubjectVerb(subject_verb) = effect else {
        return;
    };
    let SubjectVerbActionAst::GrantAbilitiesToTarget {
        target, abilities, ..
    } = &mut subject_verb.action
    else {
        return;
    };
    if !abilities.contains(&GrantedAbilityAst::MustAttack) {
        return;
    }
    if let TargetAst::Tagged(tag, span) = target
        && tag.as_str() == IT_TAG
    {
        *target = TargetAst::Source(*span);
    }
}

fn retarget_source_damage_attack_followups_to_source_internal(effects: &mut [EffectAst]) {
    for index in 1..effects.len() {
        let (before, after) = effects.split_at_mut(index);
        if source_deals_damage_to_player(&before[index - 1]) {
            retarget_implicit_must_attack_to_source(&mut after[0]);
        }
    }

    for effect in effects {
        for_each_nested_effects_mut(effect, true, |nested| {
            retarget_source_damage_attack_followups_to_source_internal(nested);
        });
    }
}

pub fn retarget_source_damage_attack_followups_to_source(effects: &mut [EffectAst]) {
    retarget_source_damage_attack_followups_to_source_internal(effects);
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

pub fn bind_condition_counter_antecedent_in_effects(
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

fn retarget_it_animation_to_source(effect: &mut EffectAst) -> bool {
    // As with condition-filter binding, an implicit `it` that is retargeted to
    // the source is not a new local antecedent. Keep walking so a coordinated
    // sequence of source-bound grants/animations is retargeted consistently.
    let establishes_body_antecedent = effect_establishes_body_object_antecedent(effect);
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

    if establishes_body_antecedent {
        return true;
    }

    let mut saw_nested = false;
    let mut every_nested_branch_establishes = true;
    for_each_nested_effects_mut(effect, true, |nested| {
        saw_nested = true;
        every_nested_branch_establishes &= retarget_it_animations_to_source_internal(nested);
    });
    saw_nested && every_nested_branch_establishes
}

fn retarget_it_animations_to_source_internal(effects: &mut [EffectAst]) -> bool {
    for effect in effects {
        if retarget_it_animation_to_source(effect) {
            return true;
        }
    }
    false
}

pub fn retarget_it_animations_to_source(effects: &mut [EffectAst]) {
    let _ = retarget_it_animations_to_source_internal(effects);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::builders::{
        PlayerAst, SubjectVerbEffectAst, SubjectVerbRoleAst, SubjectVerbSubjectAst,
    };
    use crate::effect::Until;

    fn effect(action: SubjectVerbActionAst) -> EffectAst {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject: SubjectVerbSubjectAst {
                role: SubjectVerbRoleAst::Actor,
                player: PlayerAst::You,
            },
            action,
        })
    }

    fn it_target() -> TargetAst {
        TargetAst::Tagged(IT_TAG.into(), None)
    }

    #[test]
    fn direct_tagged_condition_establishes_object_antecedent() {
        let tag: crate::tag::TagKey = "enchanted".into();
        let predicate = PredicateAst::TaggedMatches(tag.clone(), ObjectFilter::creature());

        assert_eq!(
            predicate_object_filter_antecedent(&predicate),
            Some(ObjectFilter::tagged(tag))
        );
    }

    #[test]
    fn existential_condition_does_not_establish_object_antecedent() {
        let predicate = PredicateAst::PlayerControls {
            player: PlayerAst::You,
            filter: ObjectFilter::creature(),
        };

        assert_eq!(predicate_object_filter_antecedent(&predicate), None);
    }

    #[test]
    fn existential_collection_choice_materializes_one_of_those_objects() {
        let mut contested_lands =
            ObjectFilter::land().with_counter_type(CounterType::Named("contested".into()));
        contested_lands.union_surface = contested_lands
            .union_surface
            .with_counter_requirement_surface(false, true, true);
        let predicate = PredicateAst::PlayerControls {
            player: PlayerAst::That,
            filter: contested_lands,
        };
        let one_of_those = TargetAst::WithCount(
            Box::new(TargetAst::Object(ObjectFilter::tagged(IT_TAG), None, None)),
            crate::effect::ChoiceCount::exactly(1),
        );
        let mut effects = vec![
            effect(SubjectVerbActionAst::GainControl {
                target: one_of_those,
                duration: Until::Forever,
                condition: None,
                controller_reference: None,
                source_reference_surface: None,
            }),
            effect(SubjectVerbActionAst::Untap {
                target: it_target(),
            }),
        ];

        bind_condition_collection_antecedent_in_effects(&mut effects, &predicate);

        let [
            EffectAst::Sequence {
                effects: collection_effects,
            },
            EffectAst::SubjectVerb(untap),
        ] = effects.as_slice()
        else {
            panic!("expected an explicit collection choice followed by untap: {effects:#?}");
        };
        let [
            EffectAst::ChooseObjects {
                filter,
                count,
                player,
                tag,
                ..
            },
            EffectAst::SubjectVerb(gain),
        ] = collection_effects.as_slice()
        else {
            panic!("expected choose-then-gain sequence: {collection_effects:#?}");
        };
        assert!(count.is_single());
        assert_eq!(*player, PlayerAst::That);
        assert_eq!(tag.as_str(), CONDITION_COLLECTION_CHOICE_TAG);
        assert_eq!(filter.card_types, [crate::types::CardType::Land]);
        assert_eq!(
            filter.controller,
            Some(crate::target::PlayerFilter::IteratedPlayer)
        );
        assert!(filter.with_counter.is_some());
        assert!(matches!(
            &gain.action,
            SubjectVerbActionAst::GainControl {
                target: TargetAst::Tagged(gain_tag, _),
                ..
            } if gain_tag == tag
        ));
        assert!(matches!(
            &untap.action,
            SubjectVerbActionAst::Untap {
                target: TargetAst::Tagged(untap_tag, _),
            } if untap_tag.as_str() == IT_TAG
        ));
    }

    #[test]
    fn negated_tagged_condition_does_not_establish_object_antecedent() {
        let predicate = PredicateAst::Not(Box::new(PredicateAst::TaggedMatches(
            "enchanted".into(),
            ObjectFilter::creature(),
        )));

        assert_eq!(predicate_object_filter_antecedent(&predicate), None);
    }

    #[test]
    fn ambiguous_or_condition_does_not_establish_object_antecedent() {
        let predicate = PredicateAst::Or(
            Box::new(PredicateAst::TaggedMatches(
                "that_creature".into(),
                ObjectFilter::creature(),
            )),
            Box::new(PredicateAst::PlayerControls {
                player: PlayerAst::You,
                filter: ObjectFilter::creature().with_subtype(crate::types::Subtype::Lizard),
            }),
        );

        assert_eq!(predicate_object_filter_antecedent(&predicate), None);
    }

    #[test]
    fn or_condition_with_same_tagged_subject_establishes_unique_antecedent() {
        let tag: crate::tag::TagKey = "that_creature".into();
        let predicate = PredicateAst::Or(
            Box::new(PredicateAst::TaggedMatches(
                tag.clone(),
                ObjectFilter::creature(),
            )),
            Box::new(PredicateAst::TaggedMatches(
                tag.clone(),
                ObjectFilter::creature().you_control(),
            )),
        );

        assert_eq!(
            predicate_object_filter_antecedent(&predicate),
            Some(ObjectFilter::tagged(tag))
        );
    }

    #[test]
    fn counted_condition_binds_only_explicit_random_those_target() {
        let mut counted = ObjectFilter::creature().with_counter_type(CounterType::Aim);
        counted.controller = Some(crate::target::PlayerFilter::NotYou);
        let predicate = PredicateAst::ValueComparison {
            left: Value::Count(counted.clone()),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(2),
        };
        let random_those = TargetAst::WithCount(
            Box::new(TargetAst::Object(ObjectFilter::tagged(IT_TAG), None, None)),
            crate::effect::ChoiceCount::exactly(1).at_random(),
        );
        let mut effects = vec![EffectAst::subject_verb_destroy(random_those)];

        bind_random_count_condition_antecedent_in_effects(&mut effects, &predicate);

        let EffectAst::SubjectVerb(destroy) = &effects[0] else {
            panic!("expected destroy effect");
        };
        assert!(matches!(
            &destroy.action,
            SubjectVerbActionAst::Destroy {
                target: TargetAst::WithCount(inner, count),
                ..
            } if count.random
                && matches!(inner.as_ref(), TargetAst::Object(filter, _, _) if filter == &counted)
        ));
    }

    #[test]
    fn counted_condition_leaves_nonrandom_it_target_unbound() {
        let predicate = PredicateAst::ValueComparison {
            left: Value::Count(ObjectFilter::creature()),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(2),
        };
        let mut effects = vec![effect(SubjectVerbActionAst::GrantAbilitiesToTarget {
            target: it_target(),
            abilities: Vec::new(),
            duration: Until::EndOfTurn,
            condition: None,
            set_quantifier_surface: None,
        })];

        bind_random_count_condition_antecedent_in_effects(&mut effects, &predicate);

        let EffectAst::SubjectVerb(grant) = &effects[0] else {
            panic!("expected grant effect");
        };
        assert!(matches!(
            &grant.action,
            SubjectVerbActionAst::GrantAbilitiesToTarget {
                target: TargetAst::Tagged(tag, _),
                ..
            } if tag.as_str() == IT_TAG
        ));
    }

    #[test]
    fn source_exiled_count_condition_binds_plural_move_to_the_whole_collection() {
        let mut source_exiled =
            ObjectFilter::tagged(crate::tag::SOURCE_EXILED_TAG).in_zone(crate::zone::Zone::Exile);
        source_exiled.source_surface =
            Some(crate::target::SourceReferenceSurface::ThisPermanentType(
                "this enchantment".to_string(),
            ));
        let predicate = PredicateAst::ValueComparison {
            left: Value::Count(source_exiled.clone()),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(1),
        };
        let mut effects = vec![
            EffectAst::subject_verb_move_to_zone(
                it_target(),
                crate::zone::Zone::Graveyard,
                false,
                crate::cards::builders::ReturnControllerAst::Preserve,
                false,
                None,
            )
            .with_move_to_zone_plural_surface(),
        ];

        bind_condition_collection_antecedent_in_effects(&mut effects, &predicate);

        let EffectAst::SubjectVerb(move_effect) = &effects[0] else {
            panic!("expected move effect");
        };
        assert!(matches!(
            &move_effect.action,
            SubjectVerbActionAst::MoveToZone {
                target: TargetAst::Object(filter, _, _),
                target_plural_surface: true,
                all: true,
                ..
            } if filter == &source_exiled
        ));
    }

    #[test]
    fn source_damage_to_player_retargets_implicit_must_attack_subject() {
        let mut effects = vec![
            EffectAst::subject_verb_damage(
                Value::Fixed(3),
                TargetAst::Player(crate::target::PlayerFilter::You, None),
            ),
            EffectAst::subject_verb_grant_abilities_to_target(
                it_target(),
                vec![GrantedAbilityAst::MustAttack],
                Until::EndOfTurn,
            ),
        ];

        retarget_source_damage_attack_followups_to_source(&mut effects);

        let EffectAst::SubjectVerb(grant) = &effects[1] else {
            panic!("expected grant effect");
        };
        assert!(matches!(
            &grant.action,
            SubjectVerbActionAst::GrantAbilitiesToTarget {
                target: TargetAst::Source(_),
                ..
            }
        ));
    }

    #[test]
    fn body_local_target_supersedes_condition_antecedent() {
        let controlled = TargetAst::Object(ObjectFilter::creature(), None, None);
        let mut effects = vec![
            effect(SubjectVerbActionAst::GainControl {
                target: controlled,
                duration: Until::EndOfTurn,
                condition: None,
                controller_reference: None,
                source_reference_surface: None,
            }),
            effect(SubjectVerbActionAst::Untap {
                target: it_target(),
            }),
        ];

        bind_condition_antecedent_in_effects(
            &mut effects,
            &ObjectFilter::creature().you_control(),
            ConditionAntecedentBinding::TaggedItOnly,
        );

        let EffectAst::SubjectVerb(untap) = &effects[1] else {
            panic!("expected untap effect");
        };
        assert!(matches!(
            &untap.action,
            SubjectVerbActionAst::Untap {
                target: TargetAst::Tagged(tag, _)
            } if tag.as_str() == IT_TAG
        ));
    }

    #[test]
    fn created_tokens_supersede_condition_antecedent() {
        let create = effect(SubjectVerbActionAst::CreateTokenWithMods {
            name: "Goblin Rogue".to_string(),
            definition: crate::grammar::token_definitions::parse_token_definition_shape_text(
                "1/1 black Goblin Rogue creature token",
            )
            .expect("test token definition should parse"),
            count: Value::Fixed(2),
            dynamic_power_toughness: None,
            player: PlayerAst::That,
            actor_surface_explicit: false,
            attached_to: None,
            tapped: false,
            attacking: false,
            attack_target_player: None,
            exile_at_end_of_combat: false,
            sacrifice_at_end_of_combat: false,
            sacrifice_at_next_end_step: false,
            exile_at_next_end_step: false,
            next_end_step_player: crate::target::PlayerFilter::Any,
            granted_abilities: Vec::new(),
            ability_presentation: None,
        });
        let mut effects = vec![
            create,
            effect(SubjectVerbActionAst::GrantAbilitiesToTarget {
                target: it_target(),
                abilities: vec![GrantedAbilityAst::KeywordAction(
                    crate::cards::builders::KeywordAction::Haste,
                )],
                duration: Until::EndOfTurn,
                condition: None,
                set_quantifier_surface: None,
            }),
        ];

        bind_condition_antecedent_in_effects(
            &mut effects,
            &ObjectFilter::tagged("sacrificed"),
            ConditionAntecedentBinding::TaggedItOnly,
        );

        let EffectAst::SubjectVerb(grant) = &effects[1] else {
            panic!("expected token ability grant");
        };
        assert!(matches!(
            &grant.action,
            SubjectVerbActionAst::GrantAbilitiesToTarget {
                target: TargetAst::Tagged(tag, _),
                ..
            } if tag.as_str() == IT_TAG
        ));
    }

    #[test]
    fn condition_antecedent_binds_coordinated_object_actions() {
        let mut effects = vec![
            effect(SubjectVerbActionAst::GainLife {
                amount: Value::Fixed(1),
            }),
            effect(SubjectVerbActionAst::Tap {
                target: it_target(),
            }),
            effect(SubjectVerbActionAst::Untap {
                target: it_target(),
            }),
        ];
        let antecedent = ObjectFilter::creature().you_control();

        bind_condition_antecedent_in_effects(
            &mut effects,
            &antecedent,
            ConditionAntecedentBinding::TaggedItOnly,
        );

        let EffectAst::SubjectVerb(tap) = &effects[1] else {
            panic!("expected tap effect");
        };
        assert!(matches!(
            &tap.action,
            SubjectVerbActionAst::Tap {
                target: TargetAst::Object(filter, _, _)
            } if filter == &antecedent
        ));
        let EffectAst::SubjectVerb(untap) = &effects[2] else {
            panic!("expected untap effect");
        };
        assert!(matches!(
            &untap.action,
            SubjectVerbActionAst::Untap {
                target: TargetAst::Object(filter, _, _)
            } if filter == &antecedent
        ));
    }

    #[test]
    fn source_condition_animation_retarget_yields_to_body_local_target() {
        let mut effects = vec![
            effect(SubjectVerbActionAst::GainControl {
                target: TargetAst::Object(ObjectFilter::creature(), None, None),
                duration: Until::EndOfTurn,
                condition: None,
                controller_reference: None,
                source_reference_surface: None,
            }),
            effect(SubjectVerbActionAst::GrantAbilitiesToTarget {
                target: it_target(),
                abilities: Vec::new(),
                duration: Until::EndOfTurn,
                condition: None,
                set_quantifier_surface: None,
            }),
        ];

        retarget_it_animations_to_source(&mut effects);

        let EffectAst::SubjectVerb(grant) = &effects[1] else {
            panic!("expected grant effect");
        };
        assert!(matches!(
            &grant.action,
            SubjectVerbActionAst::GrantAbilitiesToTarget {
                target: TargetAst::Tagged(tag, _),
                ..
            } if tag.as_str() == IT_TAG
        ));
    }

    #[test]
    fn source_condition_animation_retargets_coordinated_unshadowed_it() {
        let grant = || {
            effect(SubjectVerbActionAst::GrantAbilitiesToTarget {
                target: it_target(),
                abilities: Vec::new(),
                duration: Until::EndOfTurn,
                condition: None,
                set_quantifier_surface: None,
            })
        };
        let mut effects = vec![grant(), grant()];

        retarget_it_animations_to_source(&mut effects);

        for grant in &effects {
            let EffectAst::SubjectVerb(grant) = grant else {
                panic!("expected grant effect");
            };
            assert!(matches!(
                &grant.action,
                SubjectVerbActionAst::GrantAbilitiesToTarget {
                    target: TargetAst::Source(_),
                    ..
                }
            ));
        }
    }

    #[test]
    fn top_library_observation_keeps_persistent_trigger_subjects_distinct() {
        let mut effects = vec![
            EffectAst::subject_verb_reveal_top(PlayerAst::You),
            EffectAst::Conditional {
                predicate: PredicateAst::ItIsLandCard,
                if_true: vec![EffectAst::subject_verb_destroy(it_target())],
                if_false: vec![EffectAst::subject_verb_pump(
                    Value::Fixed(3),
                    Value::Fixed(3),
                    it_target(),
                    Until::EndOfTurn,
                    None,
                )],
            },
            EffectAst::subject_verb_remove_from_combat(it_target()),
            EffectAst::subject_verb_move_to_zone(
                it_target(),
                crate::zone::Zone::Library,
                false,
                crate::cards::builders::ReturnControllerAst::Preserve,
                false,
                None,
            ),
        ];

        bind_trigger_antecedent_after_top_library_observation(
            &mut effects,
            &crate::tag::TagKey::from("triggering"),
        );

        let EffectAst::Conditional {
            if_true, if_false, ..
        } = &effects[1]
        else {
            panic!("expected conditional");
        };
        assert!(matches!(
            &if_true[0],
            EffectAst::SubjectVerb(subject_verb)
                if matches!(
                    &subject_verb.action,
                    SubjectVerbActionAst::Destroy {
                        target: TargetAst::Tagged(tag, _),
                        ..
                    } if tag.as_str() == "triggering"
                )
        ));
        assert!(matches!(
            &if_false[0],
            EffectAst::SubjectVerb(subject_verb)
                if matches!(
                    &subject_verb.action,
                    SubjectVerbActionAst::Pump {
                        target: TargetAst::Tagged(tag, _),
                        ..
                    } if tag.as_str() == "triggering"
                )
        ));
        assert!(matches!(
            &effects[2],
            EffectAst::SubjectVerb(subject_verb)
                if matches!(
                    &subject_verb.action,
                    SubjectVerbActionAst::RemoveFromCombat {
                        target: TargetAst::Tagged(tag, _),
                    } if tag.as_str() == "triggering"
                )
        ));
        assert!(matches!(
            &effects[3],
            EffectAst::SubjectVerb(subject_verb)
                if matches!(
                    &subject_verb.action,
                    SubjectVerbActionAst::MoveToZone {
                        target: TargetAst::Tagged(tag, _),
                        ..
                    } if tag.as_str() == IT_TAG
                )
        ));
    }

    #[test]
    fn moved_observed_card_supersedes_trigger_antecedent_within_its_branch() {
        let mut effects = vec![
            EffectAst::subject_verb_reveal_top(PlayerAst::You),
            EffectAst::Conditional {
                predicate: PredicateAst::ItIsLandCard,
                if_true: vec![
                    EffectAst::subject_verb_move_to_zone(
                        it_target(),
                        crate::zone::Zone::Battlefield,
                        false,
                        crate::cards::builders::ReturnControllerAst::Preserve,
                        false,
                        None,
                    ),
                    EffectAst::subject_verb_pump(
                        Value::Fixed(1),
                        Value::Fixed(1),
                        it_target(),
                        Until::Forever,
                        None,
                    ),
                ],
                if_false: Vec::new(),
            },
        ];

        bind_trigger_antecedent_after_top_library_observation(
            &mut effects,
            &crate::tag::TagKey::from("triggering"),
        );

        let EffectAst::Conditional { if_true, .. } = &effects[1] else {
            panic!("expected conditional");
        };
        for effect in if_true {
            let EffectAst::SubjectVerb(subject_verb) = effect else {
                panic!("expected subject-verb effect");
            };
            let target = match &subject_verb.action {
                SubjectVerbActionAst::MoveToZone { target, .. }
                | SubjectVerbActionAst::Pump { target, .. } => target,
                other => panic!("unexpected action {other:?}"),
            };
            assert!(matches!(
                target,
                TargetAst::Tagged(tag, _) if tag.as_str() == IT_TAG
            ));
        }
    }
}
