use crate::cards::builders::{
    EffectAst, IT_TAG, ObjectRefAst, ParseAnnotations, PlayerAst, PredicateAst, RetargetModeAst,
    SubjectVerbActionAst, SubjectVerbEffectAst, TagKey, TargetAst,
};
use crate::effect::{EventValueSpec, Value};
use crate::filter::{ObjectFilter, ObjectRef, PlayerFilter, TaggedOpbjectRelation};
use crate::target::ChooseSpec;

use super::{NormalizedLine, assert_effect_ast_variant_coverage, for_each_nested_effects};

const REVEALED_COLLECTION_TAG_PREFIX: &str = "revealed";
const SEARCHED_COLLECTION_TAG_PREFIX: &str = "searched";
const EXILE_COST_TAG_PREFIX: &str = "exile_cost_";
const EXILED_COLLECTION_TAG_PREFIX: &str = "exiled_";
const SENTENCE_HELPER_EXILED_TAG_PREFIX: &str = "__sentence_helper_exiled";

fn tag_str_has_prefix(tag: &str, prefix: &str) -> bool {
    tag.strip_prefix(prefix).is_some()
}

pub(crate) fn is_revealed_collection_tag(tag: &str) -> bool {
    tag_str_has_prefix(tag, REVEALED_COLLECTION_TAG_PREFIX)
}

pub(crate) fn is_searched_collection_tag(tag: &str) -> bool {
    tag_str_has_prefix(tag, SEARCHED_COLLECTION_TAG_PREFIX)
}

pub(crate) fn is_exile_cost_collection_tag(tag: &str) -> bool {
    tag_str_has_prefix(tag, EXILE_COST_TAG_PREFIX)
}

pub(crate) fn is_sentence_helper_exiled_collection_tag(tag: &str) -> bool {
    tag_str_has_prefix(tag, SENTENCE_HELPER_EXILED_TAG_PREFIX)
}

pub(crate) fn is_exiled_collection_tag(tag: &str) -> bool {
    tag_str_has_prefix(tag, EXILED_COLLECTION_TAG_PREFIX)
        || is_sentence_helper_exiled_collection_tag(tag)
        || tag == crate::tag::SOURCE_EXILED_TAG
}

fn total_cost_values_any(
    cost: &crate::cost::TotalCost,
    predicate: impl Fn(&Value) -> bool + Copy,
) -> bool {
    match cost.kind() {
        ironsmith_core::TotalCostKind::All(components) => components
            .iter()
            .any(|component| cost_component_values_any(component, predicate)),
        ironsmith_core::TotalCostKind::OneOf(branches) => branches
            .iter()
            .any(|branch| total_cost_values_any(branch, predicate)),
    }
}

fn cost_component_values_any(
    component: &crate::costs::Cost,
    predicate: impl Fn(&Value) -> bool + Copy,
) -> bool {
    match component {
        crate::costs::Cost::DynamicMana(dynamic) => {
            dynamic.x_value.as_ref().is_some_and(predicate)
                || dynamic.additional_generic.as_ref().is_some_and(predicate)
                || dynamic.multiplier.as_ref().is_some_and(predicate)
        }
        crate::costs::Cost::Energy(value)
        | crate::costs::Cost::Mill(value)
        | crate::costs::Cost::Life(value) => predicate(value),
        _ => false,
    }
}

pub(crate) fn effects_reference_tag(effects: &[EffectAst], tag: &str) -> bool {
    effects
        .iter()
        .any(|effect| effect_references_tag(effect, tag))
}

pub(crate) fn effects_reference_tag_in_object_position(effects: &[EffectAst], tag: &str) -> bool {
    effects
        .iter()
        .any(|effect| effect_references_tag_in_object_position(effect, tag))
}

fn with_direct_effect_targets(effect: &EffectAst, mut visit: impl FnMut(&TargetAst)) {
    assert_effect_ast_variant_coverage(effect);
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
            | SubjectVerbActionAst::LookAtTarget { target }
            | SubjectVerbActionAst::Counter { target }
            | SubjectVerbActionAst::CounterUnlessPays { target, .. }
            | SubjectVerbActionAst::PutCounters { target, .. }
            | SubjectVerbActionAst::PutCounterChoice { target, .. }
            | SubjectVerbActionAst::PutOrRemoveCounters { target, .. }
            | SubjectVerbActionAst::CopySpell { target, .. }
            | SubjectVerbActionAst::CopySpellForEachTarget { target, .. }
            | SubjectVerbActionAst::ExileUntilSourceLeaves { target, .. }
            | SubjectVerbActionAst::ReturnToBattlefield { target, .. }
            | SubjectVerbActionAst::FightIterated { creature2: target }
            | SubjectVerbActionAst::Detain { target }
            | SubjectVerbActionAst::Goad { target }
            | SubjectVerbActionAst::Suspect { target }
            | SubjectVerbActionAst::RemoveFromCombat { target }
            | SubjectVerbActionAst::Flip { target }
            | SubjectVerbActionAst::Regenerate { target, .. }
            | SubjectVerbActionAst::TapOrUntap { target }
            | SubjectVerbActionAst::PhaseOut { target }
            | SubjectVerbActionAst::PhaseIn { target }
            | SubjectVerbActionAst::Transform { target }
            | SubjectVerbActionAst::Convert { target }
            | SubjectVerbActionAst::Explore { target }
            | SubjectVerbActionAst::Endure { target, .. }
            | SubjectVerbActionAst::Connive { target, .. }
            | SubjectVerbActionAst::ExchangeControlHeterogeneous {
                permanent1: target, ..
            }
            | SubjectVerbActionAst::DestroyAllAttachedTo { target, .. }
            | SubjectVerbActionAst::ExileAllAttachedTo { target, .. }
            | SubjectVerbActionAst::ExileWhenSourceLeaves { target }
            | SubjectVerbActionAst::SacrificeSourceWhenLeaves { target }
            | SubjectVerbActionAst::MayMoveToZone { target, .. }
            | SubjectVerbActionAst::RegisterZoneReplacement { target, .. }
            | SubjectVerbActionAst::ShuffleObjectsIntoLibrary { target }
            | SubjectVerbActionAst::PreventAllCombatDamageFromSource { source: target, .. }
            | SubjectVerbActionAst::RedirectNextTimeDamageToSource { target, .. }
            | SubjectVerbActionAst::RedirectAllDamageThisTurnBySourceToSourceController {
                source: target,
            }
            | SubjectVerbActionAst::RedirectAllDamageThisTurnToTarget { target, .. }
            | SubjectVerbActionAst::CreateTokenCopyFromSource { source: target, .. }
            | SubjectVerbActionAst::PreventDamage { target, .. }
            | SubjectVerbActionAst::PreventAllDamageToTarget { target, .. }
            | SubjectVerbActionAst::PreventDamageToTargetPutCounters { target, .. }
            | SubjectVerbActionAst::MoveToLibraryNthFromTop { target, .. }
            | SubjectVerbActionAst::MoveToLibraryTopOrBottomChoice { target }
            | SubjectVerbActionAst::RemoveUpToAnyCounters { target, .. }
            | SubjectVerbActionAst::DoubleCountersOnTarget { target, .. }
            | SubjectVerbActionAst::ForEachCounterKindPutOrRemove { target, .. }
            | SubjectVerbActionAst::PutCounterOfChosenKind { target }
            | SubjectVerbActionAst::PutSticker { target, .. }
            | SubjectVerbActionAst::SwitchPowerToughness { target, .. }
            | SubjectVerbActionAst::GrantProtectionChoice { target, .. }
            | SubjectVerbActionAst::ReturnToHand { target, .. } => visit(target),
            SubjectVerbActionAst::RedirectNextDamageFromSourceToTarget {
                protected_target,
                destination_target,
                ..
            } => {
                if let Some(target) = protected_target {
                    visit(target);
                }
                if let Some(target) = destination_target {
                    visit(target);
                }
            }
            SubjectVerbActionAst::RetargetStackObject { target, mode, .. } => {
                visit(target);
                if let RetargetModeAst::OneToFixed { target: fixed } = mode {
                    visit(fixed);
                }
            }
            SubjectVerbActionAst::MoveAllCounters { from, to }
            | SubjectVerbActionAst::MoveOneCounter { from, to } => {
                visit(from);
                visit(to);
            }
            SubjectVerbActionAst::Attach { object, target } => {
                visit(object);
                visit(target);
            }
            SubjectVerbActionAst::Unattach { object } => visit(object),
            SubjectVerbActionAst::Fight {
                creature1,
                creature2,
            } => {
                visit(creature1);
                visit(creature2);
            }
            SubjectVerbActionAst::Sacrifice {
                target: Some(target),
                ..
            }
            | SubjectVerbActionAst::GainControl { target, .. } => visit(target),
            SubjectVerbActionAst::MoveToZone {
                target,
                attached_to,
                ..
            } => {
                visit(target);
                if let Some(attach_target) = attached_to {
                    visit(attach_target);
                }
            }
            SubjectVerbActionAst::TargetOnly { target } => visit(target),
            SubjectVerbActionAst::Pump { target, .. }
            | SubjectVerbActionAst::SetBasePowerToughness { target, .. }
            | SubjectVerbActionAst::BecomeBasePtCreature { target, .. }
            | SubjectVerbActionAst::SetBasePower { target, .. }
            | SubjectVerbActionAst::PumpForEach { target, .. }
            | SubjectVerbActionAst::PumpByLastEffect { target, .. }
            | SubjectVerbActionAst::AddCardTypes { target, .. }
            | SubjectVerbActionAst::RemoveCardTypes { target, .. }
            | SubjectVerbActionAst::AddSubtypes { target, .. }
            | SubjectVerbActionAst::SetCreatureSubtypes { target, .. }
            | SubjectVerbActionAst::BecomeSaddledUntilEndOfTurn { target }
            | SubjectVerbActionAst::AddColors { target, .. }
            | SubjectVerbActionAst::AddAllSubtypesOfFamily { target, .. }
            | SubjectVerbActionAst::RemoveAllSubtypesOfFamily { target, .. }
            | SubjectVerbActionAst::BecomeBasicLandType { target, .. }
            | SubjectVerbActionAst::SetColors { target, .. }
            | SubjectVerbActionAst::MakeColorless { target, .. }
            | SubjectVerbActionAst::BecomeBasicLandTypeChoice { target, .. }
            | SubjectVerbActionAst::BecomeCreatureTypeChoice { target, .. }
            | SubjectVerbActionAst::BecomeColorChoice { target, .. }
            | SubjectVerbActionAst::GrantAbilitiesToTarget { target, .. }
            | SubjectVerbActionAst::GrantToTarget { target, .. }
            | SubjectVerbActionAst::RemoveAbilitiesFromTarget { target, .. }
            | SubjectVerbActionAst::GrantAbilitiesChoiceToTarget { target, .. } => visit(target),
            SubjectVerbActionAst::BecomeCopy { target, source, .. } => {
                visit(target);
                visit(source);
            }
            _ => {}
        },
        _ => {}
    }
}

fn direct_effect_targets_reference_tag(effect: &EffectAst, tag: &str) -> bool {
    let mut references = false;
    with_direct_effect_targets(effect, |target| {
        if !references {
            references = target_references_tag(target, tag);
        }
    });
    references
}

fn effect_references_tag_in_object_position(effect: &EffectAst, tag: &str) -> bool {
    assert_effect_ast_variant_coverage(effect);
    if direct_effect_targets_reference_tag(effect, tag) {
        return true;
    }
    if let Some(filter) = effect_tagged_filter(effect)
        && filter_references_tag(filter, tag)
    {
        return true;
    }

    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::DealDamageEach { filter, .. },
            ..
        }) => filter_references_tag(filter, tag),
        EffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        }
        | EffectAst::SelfReplacement {
            predicate,
            if_true,
            if_false,
        } => {
            predicate_references_tag(predicate, tag)
                || effects_reference_tag_in_object_position(if_true, tag)
                || effects_reference_tag_in_object_position(if_false, tag)
        }
        EffectAst::ForEachObject { filter, effects } => {
            filter_references_tag(filter, tag)
                || effects_reference_tag_in_object_position(effects, tag)
        }
        EffectAst::ForEachTagged {
            tag: found,
            effects,
        } => found.as_str() == tag || effects_reference_tag_in_object_position(effects, tag),
        _ => {
            let mut references = false;
            for_each_nested_effects(effect, true, |nested| {
                if !references {
                    references = nested.iter().any(|nested_effect| {
                        effect_references_tag_in_object_position(nested_effect, tag)
                    });
                }
            });
            references
        }
    }
}

pub(crate) fn filter_references_tag(filter: &ObjectFilter, tag: &str) -> bool {
    filter
        .tagged_constraints
        .iter()
        .any(|constraint| constraint.tag.as_str() == tag)
        || filter
            .could_be_targeted_by
            .as_ref()
        .is_some_and(|constraint| {
            matches!(&constraint.stack_object, crate::filter::ObjectRef::Tagged(object_tag) if object_tag.as_str() == tag)
        })
        || matches!(&filter.blocked_by, Some(crate::filter::ObjectRef::Tagged(object_tag)) if object_tag.as_str() == tag)
        || filter
            .targets_object
            .as_deref()
            .is_some_and(|targets| filter_references_tag(targets, tag))
        || filter
            .targets_only_object
            .as_deref()
            .is_some_and(|targets| filter_references_tag(targets, tag))
        || filter
            .any_of
            .iter()
            .any(|branch| filter_references_tag(branch, tag))
}

fn effect_tagged_filter(effect: &EffectAst) -> Option<&ObjectFilter> {
    match effect {
        EffectAst::SubjectVerb(subject_verb) => match &subject_verb.action {
            SubjectVerbActionAst::DestroyAll { filter, .. }
            | SubjectVerbActionAst::DestroyAllOfChosenColor { filter, .. }
            | SubjectVerbActionAst::ExileAll { filter, .. }
            | SubjectVerbActionAst::ReturnAllToHand { filter }
            | SubjectVerbActionAst::ReturnAllToHandOfChosenColor { filter }
            | SubjectVerbActionAst::PutCountersAll { filter, .. }
            | SubjectVerbActionAst::DoubleCountersOnEach { filter, .. }
            | SubjectVerbActionAst::RemoveCountersAll { filter, .. }
            | SubjectVerbActionAst::ScalePowerToughnessAll { filter, .. }
            | SubjectVerbActionAst::Sacrifice { filter, .. }
            | SubjectVerbActionAst::ExchangeControl { filter, .. }
            | SubjectVerbActionAst::DestroyAllAttachedTo { filter, .. }
            | SubjectVerbActionAst::ExileAllAttachedTo { filter, .. }
            | SubjectVerbActionAst::ChooseSpellCastHistory { filter, .. }
            | SubjectVerbActionAst::PreventDamageEach { filter, .. }
            | SubjectVerbActionAst::ReturnAllToBattlefield { filter, .. }
            | SubjectVerbActionAst::PumpAll { filter, .. }
            | SubjectVerbActionAst::GrantAbilitiesAll { filter, .. }
            | SubjectVerbActionAst::RemoveAbilitiesAll { filter, .. }
            | SubjectVerbActionAst::GrantAbilitiesChoiceAll { filter, .. }
            | SubjectVerbActionAst::GrantBySpec {
                spec: crate::grant::GrantSpec { filter, .. },
                ..
            }
            | SubjectVerbActionAst::ConsultTopOfLibrary { filter, .. }
            | SubjectVerbActionAst::SearchLibrary { filter, .. }
            | SubjectVerbActionAst::SacrificeAll { filter } => Some(filter),
            SubjectVerbActionAst::Enchant {
                filter: crate::object::AuraAttachmentFilter::Object(filter),
            } => Some(filter),
            _ => None,
        },
        EffectAst::ChooseObjects { filter, .. }
        | EffectAst::ChooseObjectsAcrossZones { filter, .. }
        | EffectAst::MayCastMatchingSpellWithoutPayingManaCost { filter, .. } => Some(filter),
        _ => None,
    }
}

pub(crate) fn effect_references_tag(effect: &EffectAst, tag: &str) -> bool {
    assert_effect_ast_variant_coverage(effect);
    if direct_effect_targets_reference_tag(effect, tag) {
        return true;
    }
    if let Some(filter) = effect_tagged_filter(effect) {
        return filter_references_tag(filter, tag);
    }

    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::PutIntoHand { object },
            ..
        }) => match object {
            ObjectRefAst::Tagged(found) => found.as_str() == tag,
        },
        EffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        }
        | EffectAst::SelfReplacement {
            predicate,
            if_true,
            if_false,
        } => {
            predicate_references_tag(predicate, tag)
                || effects_reference_tag(if_true, tag)
                || effects_reference_tag(if_false, tag)
        }
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::CopySpellForEachTarget { object_filter, .. },
            ..
        }) => object_filter
            .as_ref()
            .is_some_and(|filter| filter_references_tag(filter, tag)),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::DealDamageEqualToPower {
                    source,
                    amount,
                    target,
                },
            ..
        }) => {
            target_references_tag(source, tag)
                || value_references_tag(amount, tag)
                || target_references_tag(target, tag)
        }
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::CreateTokenCopy { object, .. },
            ..
        }) => match object {
            ObjectRefAst::Tagged(found) => found.as_str() == tag,
        },
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::CreateTokenWithMods { count, .. },
            ..
        }) => value_references_tag(count, tag),
        EffectAst::ForEachObject { filter, effects } => {
            filter
                .tagged_constraints
                .iter()
                .any(|constraint| constraint.tag.as_str() == tag)
                || effects_reference_tag(effects, tag)
        }
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Cant { restriction, .. },
            ..
        }) => restriction_references_tag(restriction, tag),
        _ => {
            let mut references = false;
            for_each_nested_effects(effect, true, |nested| {
                if !references {
                    references = nested
                        .iter()
                        .any(|nested_effect| effect_references_tag(nested_effect, tag));
                }
            });
            references
        }
    }
}

pub(crate) fn value_references_tag(value: &Value, tag: &str) -> bool {
    match value {
        Value::SurfaceHinted { value, .. } => value_references_tag(value, tag),
        Value::Add(left, right) | Value::Min(left, right) => {
            value_references_tag(left, tag) || value_references_tag(right, tag)
        }
        Value::Scaled(value, _)
        | Value::DividedRoundedDown(value, _)
        | Value::HalfRoundedDown(value) => value_references_tag(value, tag),
        Value::Count(filter) | Value::CountScaled(filter, _) => filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == tag),
        Value::TotalPower(filter)
        | Value::TotalToughness(filter)
        | Value::TotalManaValue(filter)
        | Value::GreatestPower(filter)
        | Value::GreatestToughness(filter)
        | Value::GreatestManaValue(filter)
        | Value::BasicLandTypesAmong(filter)
        | Value::CreatureTypesAmong(filter)
        | Value::CardTypesAmong(filter)
        | Value::ColorsAmong(filter)
        | Value::DistinctNames(filter)
        | Value::DistinctPowers(filter) => filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == tag),
        Value::StaticAbilitiesAmong { filter, .. } => filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == tag),
        Value::PowerOf(spec) | Value::ToughnessOf(spec) => choose_spec_references_tag(spec, tag),
        Value::ManaValueOf(spec) => choose_spec_references_tag(spec, tag),
        Value::CountersOn(spec, _) => choose_spec_references_tag(spec, tag),
        Value::DamageDealtThisTurnByTaggedSpellCast(t) => t.as_str() == tag,
        _ => false,
    }
}

pub(crate) fn predicate_references_tag(predicate: &PredicateAst, tag: &str) -> bool {
    match predicate {
        PredicateAst::ItMatches(filter)
        | PredicateAst::TargetMatches(filter)
        | PredicateAst::SourceMatches(filter)
        | PredicateAst::NoVoteObjectsMatched { filter }
        | PredicateAst::ObjectEnteredBattlefieldThisTurn(filter)
        | PredicateAst::ObjectEnteredBattlefieldLastTurn(filter)
        | PredicateAst::ObjectPutIntoGraveyardFromBattlefieldThisTurn(filter) => {
            filter_references_tag(filter, tag)
        }
        PredicateAst::TaggedMatches(found, filter) => {
            found.as_str() == tag || filter_references_tag(filter, tag)
        }
        PredicateAst::TaggedWasCast(found)
        | PredicateAst::PlayerTaggedObjectEnteredBattlefieldThisTurn { tag: found, .. } => {
            found.as_str() == tag
        }
        PredicateAst::PlayerTaggedObjectMatches {
            tag: found, filter, ..
        } => found.as_str() == tag || filter_references_tag(filter, tag),
        PredicateAst::PlayerControls { filter, .. }
        | PredicateAst::PlayerHasAtLeast { filter, .. }
        | PredicateAst::PlayerControlsExactly { filter, .. }
        | PredicateAst::PlayerHasAtLeastWithDifferentPowers { filter, .. }
        | PredicateAst::PlayerControlsNo { filter, .. }
        | PredicateAst::PlayerControlsMost { filter, .. }
        | PredicateAst::PlayerControlsMoreThanEachOtherPlayer { filter, .. }
        | PredicateAst::AnOpponentControlsMoreThanPlayer { filter, .. }
        | PredicateAst::AnOpponentHasFewerThanPlayer { filter, .. }
        | PredicateAst::PlayerControlsMoreThanYou { filter, .. }
        | PredicateAst::SourceHasAttachmentsMatching { filter, .. } => {
            filter_references_tag(filter, tag)
        }
        PredicateAst::PlayerControlsOrHasCardInGraveyard {
            control_filter,
            graveyard_filter,
            ..
        } => {
            filter_references_tag(control_filter, tag)
                || filter_references_tag(graveyard_filter, tag)
        }
        PredicateAst::ValueComparison { left, right, .. } => {
            value_references_tag(left, tag) || value_references_tag(right, tag)
        }
        PredicateAst::Not(inner) => predicate_references_tag(inner, tag),
        PredicateAst::And(left, right) | PredicateAst::Or(left, right) => {
            predicate_references_tag(left, tag) || predicate_references_tag(right, tag)
        }
        _ => false,
    }
}

pub(crate) fn choose_spec_references_tag(spec: &ChooseSpec, tag: &str) -> bool {
    match spec {
        ChooseSpec::Tagged(t) => t.as_str() == tag,
        ChooseSpec::Target(inner) | ChooseSpec::WithCount(inner, _) => {
            choose_spec_references_tag(inner, tag)
        }
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == tag),
        _ => false,
    }
}

pub(crate) fn choose_spec_references_exiled_tag(spec: &ChooseSpec) -> bool {
    fn is_exiled_tag(tag: &TagKey) -> bool {
        is_exiled_collection_tag(tag.as_str())
    }

    match spec {
        ChooseSpec::Tagged(tag) => is_exiled_tag(tag),
        ChooseSpec::Target(inner) | ChooseSpec::WithCount(inner, _) => {
            choose_spec_references_exiled_tag(inner)
        }
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            filter.tagged_constraints.iter().any(|constraint| {
                matches!(constraint.relation, TaggedOpbjectRelation::IsTaggedObject)
                    && is_exiled_tag(&constraint.tag)
            })
        }
        _ => false,
    }
}

pub(crate) fn object_ref_references_tag(reference: &ObjectRef, tag: &str) -> bool {
    matches!(reference, ObjectRef::Tagged(found) if found.as_str() == tag)
}

pub(crate) fn player_filter_references_tag(filter: &PlayerFilter, tag: &str) -> bool {
    match filter {
        PlayerFilter::Target(inner) => player_filter_references_tag(inner, tag),
        PlayerFilter::ControllerOf(reference)
        | PlayerFilter::OwnerOf(reference)
        | PlayerFilter::AliasedOwnerOf(reference)
        | PlayerFilter::AliasedControllerOf(reference) => object_ref_references_tag(reference, tag),
        _ => false,
    }
}

pub(crate) fn target_references_tag(target: &TargetAst, tag: &str) -> bool {
    match target {
        TargetAst::Tagged(found, _) => found.as_str() == tag,
        TargetAst::Object(filter, _, _) => filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == tag),
        TargetAst::Player(filter, _) | TargetAst::PlayerOrPlaneswalker(filter, _) => {
            player_filter_references_tag(filter, tag)
        }
        TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, _, _) => {
            target_references_tag(inner, tag)
        }
        TargetAst::AttackedPlayerOrPlaneswalker(_) => false,
        TargetAst::Source(_)
        | TargetAst::AnyTarget(_)
        | TargetAst::AnyOtherTarget(_)
        | TargetAst::Spell(_) => false,
    }
}

pub(crate) fn effects_reference_it_tag(effects: &[EffectAst]) -> bool {
    effects.iter().any(effect_references_it_tag)
}

pub(crate) fn effects_reference_its_controller(effects: &[EffectAst]) -> bool {
    effects.iter().any(effect_references_its_controller)
}

pub(crate) fn value_references_event_derived_amount(value: &Value) -> bool {
    match value {
        Value::EventValue(EventValueSpec::Amount)
        | Value::EventValue(EventValueSpec::LifeAmount)
        | Value::EventValueOffset(EventValueSpec::Amount, _)
        | Value::EventValueOffset(EventValueSpec::LifeAmount, _) => true,
        Value::PendingEffectMetric { .. } | Value::PendingEffectMetricOffset { .. } => true,
        Value::SurfaceHinted { value, .. } => value_references_event_derived_amount(value),
        Value::Add(left, right) | Value::Min(left, right) => {
            value_references_event_derived_amount(left)
                || value_references_event_derived_amount(right)
        }
        Value::Scaled(value, _)
        | Value::DividedRoundedDown(value, _)
        | Value::HalfRoundedDown(value) => value_references_event_derived_amount(value),
        _ => false,
    }
}

fn comparison_references_event_derived_amount(comparison: &crate::filter::Comparison) -> bool {
    match comparison {
        crate::filter::Comparison::EqualExpr(value)
        | crate::filter::Comparison::NotEqualExpr(value)
        | crate::filter::Comparison::LessThanExpr(value)
        | crate::filter::Comparison::LessThanOrEqualExpr(value)
        | crate::filter::Comparison::GreaterThanExpr(value)
        | crate::filter::Comparison::GreaterThanOrEqualExpr(value) => {
            value_references_event_derived_amount(value)
        }
        _ => false,
    }
}

fn filter_references_event_derived_amount(filter: &ObjectFilter) -> bool {
    filter
        .power
        .as_ref()
        .is_some_and(comparison_references_event_derived_amount)
        || filter
            .toughness
            .as_ref()
            .is_some_and(comparison_references_event_derived_amount)
        || filter
            .mana_value
            .as_ref()
            .is_some_and(comparison_references_event_derived_amount)
        || filter
            .color_count
            .as_ref()
            .is_some_and(comparison_references_event_derived_amount)
        || filter
            .any_of
            .iter()
            .any(filter_references_event_derived_amount)
}

fn subject_verb_action_value(action: &SubjectVerbActionAst) -> Option<&Value> {
    match action {
        SubjectVerbActionAst::Draw { count }
        | SubjectVerbActionAst::Mill { count }
        | SubjectVerbActionAst::ExileTopOfLibrary { count, .. }
        | SubjectVerbActionAst::Scry { count }
        | SubjectVerbActionAst::Surveil { count }
        | SubjectVerbActionAst::Proliferate { count }
        | SubjectVerbActionAst::Investigate { count }
        | SubjectVerbActionAst::Discover { count }
        | SubjectVerbActionAst::Fateseal { count }
        | SubjectVerbActionAst::Populate { count, .. }
        | SubjectVerbActionAst::Connive { count, .. }
        | SubjectVerbActionAst::CreateTokenCopy { count, .. }
        | SubjectVerbActionAst::CreateTokenCopyFromSource { count, .. }
        | SubjectVerbActionAst::CreateTokenWithMods { count, .. } => Some(count),
        SubjectVerbActionAst::Incubate { amount, .. } => Some(amount),
        SubjectVerbActionAst::Monstrosity { amount } => Some(amount),
        SubjectVerbActionAst::LoseLife { amount }
        | SubjectVerbActionAst::GainLife { amount }
        | SubjectVerbActionAst::DealDamage { amount, .. }
        | SubjectVerbActionAst::DealDistributedDamage { amount, .. }
        | SubjectVerbActionAst::DealDamageEach { amount, .. }
        | SubjectVerbActionAst::PreventDamage { amount, .. }
        | SubjectVerbActionAst::PreventDamageEach { amount, .. }
        | SubjectVerbActionAst::CopySpell { count: amount, .. }
        | SubjectVerbActionAst::PutCounters { count: amount, .. }
        | SubjectVerbActionAst::PutCounterChoice { count: amount, .. }
        | SubjectVerbActionAst::PutCountersAll { count: amount, .. }
        | SubjectVerbActionAst::RemoveUpToAnyCounters { amount, .. }
        | SubjectVerbActionAst::RemoveCountersAll { amount, .. }
        | SubjectVerbActionAst::Discard { count: amount, .. }
        | SubjectVerbActionAst::PoisonCounters { count: amount }
        | SubjectVerbActionAst::EnergyCounters { count: amount }
        | SubjectVerbActionAst::ExperienceCounters { count: amount }
        | SubjectVerbActionAst::TicketCounters { count: amount }
        | SubjectVerbActionAst::PayEnergy { amount }
        | SubjectVerbActionAst::SetLifeTotal { amount }
        | SubjectVerbActionAst::AddManaScaled { amount, .. }
        | SubjectVerbActionAst::AddManaAnyColor { amount, .. }
        | SubjectVerbActionAst::AddManaAnyOneColor { amount }
        | SubjectVerbActionAst::AddManaChosenColor { amount, .. }
        | SubjectVerbActionAst::AddManaFromLandCouldProduce { amount, .. }
        | SubjectVerbActionAst::AddManaCommanderIdentity { amount }
        | SubjectVerbActionAst::RedirectNextDamageFromSourceToTarget { amount, .. }
        | SubjectVerbActionAst::LookAtTopCards { count: amount, .. }
        | SubjectVerbActionAst::MoveToLibraryNthFromTop {
            position: amount, ..
        }
        | SubjectVerbActionAst::AdditionalLandPlays { count: amount, .. } => Some(amount),
        SubjectVerbActionAst::DealDamageEqualToPower { .. }
        | SubjectVerbActionAst::DrawForEachTaggedMatching { .. }
        | SubjectVerbActionAst::RevealHand
        | SubjectVerbActionAst::RevealTop
        | SubjectVerbActionAst::RevealTagged { .. }
        | SubjectVerbActionAst::RevealCardsFromHand { .. }
        | SubjectVerbActionAst::LookAtObjects { .. }
        | SubjectVerbActionAst::LookAtTarget { .. }
        | SubjectVerbActionAst::EmitKeywordAction { .. }
        | SubjectVerbActionAst::Amass { .. }
        | SubjectVerbActionAst::Bolster { .. }
        | SubjectVerbActionAst::Support { .. }
        | SubjectVerbActionAst::Adapt { .. }
        | SubjectVerbActionAst::Explore { .. }
        | SubjectVerbActionAst::Endure { .. }
        | SubjectVerbActionAst::Exploit
        | SubjectVerbActionAst::ConniveIterated
        | SubjectVerbActionAst::OpenAttraction
        | SubjectVerbActionAst::ManifestTopCardOfLibrary
        | SubjectVerbActionAst::ManifestCardFromHand
        | SubjectVerbActionAst::ManifestDread
        | SubjectVerbActionAst::Earthbend { .. }
        | SubjectVerbActionAst::Behold { .. }
        | SubjectVerbActionAst::Fight { .. }
        | SubjectVerbActionAst::FightIterated { .. }
        | SubjectVerbActionAst::Clash { .. }
        | SubjectVerbActionAst::FlipCoin
        | SubjectVerbActionAst::RollDie { .. }
        | SubjectVerbActionAst::RollDiceChooseResult { .. }
        | SubjectVerbActionAst::ShuffleHandAndGraveyardIntoLibrary
        | SubjectVerbActionAst::ShuffleGraveyardIntoLibrary
        | SubjectVerbActionAst::ReorderGraveyard
        | SubjectVerbActionAst::ChooseColor
        | SubjectVerbActionAst::ChooseCardType { .. }
        | SubjectVerbActionAst::ChooseNamedOption { .. }
        | SubjectVerbActionAst::ChooseCreatureType { .. }
        | SubjectVerbActionAst::ChooseCardName { .. }
        | SubjectVerbActionAst::ChoosePlayer { .. }
        | SubjectVerbActionAst::NoteLifeTotal
        | SubjectVerbActionAst::AddMana { .. }
        | SubjectVerbActionAst::ExchangeLifeTotals { .. }
        | SubjectVerbActionAst::ExchangeTextBoxes { .. }
        | SubjectVerbActionAst::ExchangeZones { .. }
        | SubjectVerbActionAst::PutRestOnBottomOfLibrary
        | SubjectVerbActionAst::DontLoseThisManaAsStepsAndPhasesEndThisTurn
        | SubjectVerbActionAst::ExchangeValues { .. }
        | SubjectVerbActionAst::ExileInsteadOfGraveyardThisTurn
        | SubjectVerbActionAst::ControlCombatChoicesThisTurn { .. }
        | SubjectVerbActionAst::GainControl { .. }
        | SubjectVerbActionAst::PutSticker { .. }
        | SubjectVerbActionAst::SwitchPowerToughness { .. }
        | SubjectVerbActionAst::ScalePowerToughnessAll { .. }
        | SubjectVerbActionAst::ScaleXValue { .. }
        | SubjectVerbActionAst::AddManaColorsAmong { .. }
        | SubjectVerbActionAst::AddManaImprintedColors
        | SubjectVerbActionAst::DoubleManaPool
        | SubjectVerbActionAst::EmptyManaPool
        | SubjectVerbActionAst::EndTurn
        | SubjectVerbActionAst::SkipTurn
        | SubjectVerbActionAst::SkipCombatPhases
        | SubjectVerbActionAst::SkipNextCombatPhaseThisTurn
        | SubjectVerbActionAst::SkipMainPhasesThisTurn
        | SubjectVerbActionAst::SkipCombatPhasesThisTurn
        | SubjectVerbActionAst::SkipDrawStep
        | SubjectVerbActionAst::PlayFromGraveyardUntilEot
        | SubjectVerbActionAst::ControlPlayer { .. }
        | SubjectVerbActionAst::ReduceNextSpellCostThisTurn { .. }
        | SubjectVerbActionAst::ReduceMatchingSpellCostThisTurn { .. }
        | SubjectVerbActionAst::GrantNextSpellAbilityThisTurn { .. }
        | SubjectVerbActionAst::RingTemptsYou
        | SubjectVerbActionAst::VentureIntoDungeon { .. }
        | SubjectVerbActionAst::BecomeMonarch
        | SubjectVerbActionAst::TakeInitiative
        | SubjectVerbActionAst::CreateEmblem { .. }
        | SubjectVerbActionAst::LoseGame
        | SubjectVerbActionAst::WinGame
        | SubjectVerbActionAst::PayAnyEnergy { .. }
        | SubjectVerbActionAst::PayAnyLife { .. }
        | SubjectVerbActionAst::PayMana { .. }
        | SubjectVerbActionAst::DiscardHand
        | SubjectVerbActionAst::Detain { .. }
        | SubjectVerbActionAst::Goad { .. }
        | SubjectVerbActionAst::Suspect { .. }
        | SubjectVerbActionAst::ClearSuspected { .. }
        | SubjectVerbActionAst::RemoveFromCombat { .. }
        | SubjectVerbActionAst::Flip { .. }
        | SubjectVerbActionAst::Regenerate { .. }
        | SubjectVerbActionAst::RegenerateAll { .. }
        | SubjectVerbActionAst::TapAll { .. }
        | SubjectVerbActionAst::UntapAll { .. }
        | SubjectVerbActionAst::TapOrUntap { .. }
        | SubjectVerbActionAst::TapOrUntapAll { .. }
        | SubjectVerbActionAst::PhaseOut { .. }
        | SubjectVerbActionAst::PhaseOutAll { .. }
        | SubjectVerbActionAst::PhaseIn { .. }
        | SubjectVerbActionAst::PhaseInAll { .. }
        | SubjectVerbActionAst::Transform { .. }
        | SubjectVerbActionAst::Convert { .. }
        | SubjectVerbActionAst::Tap { .. }
        | SubjectVerbActionAst::Untap { .. }
        | SubjectVerbActionAst::Destroy { .. }
        | SubjectVerbActionAst::DestroyAll { .. }
        | SubjectVerbActionAst::DestroyAllOfChosenColor { .. }
        | SubjectVerbActionAst::Exile { .. }
        | SubjectVerbActionAst::ExileAll { .. }
        | SubjectVerbActionAst::LookAtHand { .. }
        | SubjectVerbActionAst::Counter { .. }
        | SubjectVerbActionAst::CounterUnlessPays { .. }
        | SubjectVerbActionAst::MoveAllCounters { .. }
        | SubjectVerbActionAst::MoveOneCounter { .. }
        | SubjectVerbActionAst::ForEachCounterKindPutOrRemove { .. }
        | SubjectVerbActionAst::PutCounterOfChosenKind { .. }
        | SubjectVerbActionAst::DoubleCountersOnTarget { .. }
        | SubjectVerbActionAst::ReturnToHand { .. }
        | SubjectVerbActionAst::ReturnAllToHand { .. }
        | SubjectVerbActionAst::ReturnAllToHandOfChosenColor { .. }
        | SubjectVerbActionAst::DoubleCountersOnEach { .. }
        | SubjectVerbActionAst::Sacrifice { .. }
        | SubjectVerbActionAst::SacrificeAll { .. }
        | SubjectVerbActionAst::PutIntoHand { .. }
        | SubjectVerbActionAst::ExtraTurnAfterTurn { .. }
        | SubjectVerbActionAst::RearrangeLookedCardsInLibrary { .. }
        | SubjectVerbActionAst::ReorderTopOfLibrary { .. }
        | SubjectVerbActionAst::ShuffleObjectsIntoLibrary { .. }
        | SubjectVerbActionAst::GrantProtectionChoice { .. }
        | SubjectVerbActionAst::PreventAllCombatDamage { .. }
        | SubjectVerbActionAst::PreventAllCombatDamageFromSource { .. }
        | SubjectVerbActionAst::PreventAllCombatDamageFromSourceFilter { .. }
        | SubjectVerbActionAst::PreventAllCombatDamageToPlayers { .. }
        | SubjectVerbActionAst::PreventAllCombatDamageToYou { .. }
        | SubjectVerbActionAst::PreventNextTimeDamage { .. }
        | SubjectVerbActionAst::RedirectNextTimeDamageToSource { .. }
        | SubjectVerbActionAst::RedirectAllDamageThisTurnBySourceToSourceController { .. }
        | SubjectVerbActionAst::RedirectAllDamageThisTurnToTarget { .. }
        | SubjectVerbActionAst::PreventAllDamageToTarget { .. }
        | SubjectVerbActionAst::PreventAllDamageToTargetFromSourceFilter { .. }
        | SubjectVerbActionAst::PreventAllDamageFromSourceFilter { .. }
        | SubjectVerbActionAst::PreventDamageToTargetPutCounters { .. }
        | SubjectVerbActionAst::PutOrRemoveCounters { .. }
        | SubjectVerbActionAst::CopySpellForEachTarget { .. }
        | SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary { .. }
        | SubjectVerbActionAst::PutTaggedRemainderInZone { .. }
        | SubjectVerbActionAst::CastTagged { .. }
        | SubjectVerbActionAst::GrantPlayTaggedUntilEndOfTurn { .. }
        | SubjectVerbActionAst::GrantTaggedSpellAlternativeCostPayLifeByManaValueUntilEndOfTurn {
            ..
        }
        | SubjectVerbActionAst::GrantPlayTaggedUntilYourNextTurn { .. }
        | SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled { .. }
        | SubjectVerbActionAst::GrantPlayTaggedForAsLongAsYouControlSource { .. }
        | SubjectVerbActionAst::ReturnToBattlefield { .. }
        | SubjectVerbActionAst::ReturnAllToBattlefield { .. }
        | SubjectVerbActionAst::ExileUntilSourceLeaves { .. }
        | SubjectVerbActionAst::MoveToZone { .. }
        | SubjectVerbActionAst::PutOntoBattlefield { .. }
        | SubjectVerbActionAst::MoveToLibraryTopOrBottomChoice { .. }
        | SubjectVerbActionAst::TargetOnly { .. }
        | SubjectVerbActionAst::TagMatchingObjects { .. }
        | SubjectVerbActionAst::Pump { .. }
        | SubjectVerbActionAst::SetBasePowerToughness { .. }
        | SubjectVerbActionAst::BecomeBasePtCreature { .. }
        | SubjectVerbActionAst::SetBasePower { .. }
        | SubjectVerbActionAst::PumpForEach { .. }
        | SubjectVerbActionAst::PumpAll { .. }
        | SubjectVerbActionAst::PumpByLastEffect { .. }
        | SubjectVerbActionAst::AddCardTypes { .. }
        | SubjectVerbActionAst::RemoveCardTypes { .. }
        | SubjectVerbActionAst::AddSubtypes { .. }
        | SubjectVerbActionAst::SetCreatureSubtypes { .. }
        | SubjectVerbActionAst::BecomeSaddledUntilEndOfTurn { .. }
        | SubjectVerbActionAst::AddColors { .. }
        | SubjectVerbActionAst::AddAllSubtypesOfFamily { .. }
        | SubjectVerbActionAst::RemoveAllSubtypesOfFamily { .. }
        | SubjectVerbActionAst::BecomeAuraEnchantment { .. }
        | SubjectVerbActionAst::BecomeBasicLandType { .. }
        | SubjectVerbActionAst::SetColors { .. }
        | SubjectVerbActionAst::MakeColorless { .. }
        | SubjectVerbActionAst::BecomeBasicLandTypeChoice { .. }
        | SubjectVerbActionAst::BecomeCreatureTypeChoice { .. }
        | SubjectVerbActionAst::BecomeColorChoice { .. }
        | SubjectVerbActionAst::BecomeCopy { .. }
        | SubjectVerbActionAst::GrantAbilitiesAll { .. }
        | SubjectVerbActionAst::RemoveAbilitiesAll { .. }
        | SubjectVerbActionAst::GrantAbilitiesChoiceAll { .. }
        | SubjectVerbActionAst::GrantAbilitiesToTarget { .. }
        | SubjectVerbActionAst::GrantToTarget { .. }
        | SubjectVerbActionAst::GrantBySpec { .. }
        | SubjectVerbActionAst::RemoveAbilitiesFromTarget { .. }
        | SubjectVerbActionAst::GrantAbilitiesChoiceToTarget { .. }
        | SubjectVerbActionAst::ConsultTopOfLibrary { .. }
        | SubjectVerbActionAst::SearchLibrary { .. }
        | SubjectVerbActionAst::Cant { .. }
        | SubjectVerbActionAst::Meld { .. }
        | SubjectVerbActionAst::SearchLibrarySlotsToHand { .. }
        | SubjectVerbActionAst::RetargetStackObject { .. }
        | SubjectVerbActionAst::GrantAbilityToSource { .. }
        | SubjectVerbActionAst::ExchangeControl { .. }
        | SubjectVerbActionAst::ExchangeControlHeterogeneous { .. }
        | SubjectVerbActionAst::DestroyAllAttachedTo { .. }
        | SubjectVerbActionAst::ExileAllAttachedTo { .. }
        | SubjectVerbActionAst::Attach { .. }
        | SubjectVerbActionAst::Unattach { .. }
        | SubjectVerbActionAst::ExileWhenSourceLeaves { .. }
        | SubjectVerbActionAst::SacrificeSourceWhenLeaves { .. }
        | SubjectVerbActionAst::MayMoveToZone { .. }
        | SubjectVerbActionAst::RegisterZoneReplacement { .. }
        | SubjectVerbActionAst::RegisterFutureZoneReplacement { .. }
        | SubjectVerbActionAst::RegisterDrawReplacement { .. }
        | SubjectVerbActionAst::RegisterManaReplacement { .. }
        | SubjectVerbActionAst::RegisterDamagedBySourceZoneReplacement { .. }
        | SubjectVerbActionAst::RegisterEnterUnderControlReplacement { .. }
        | SubjectVerbActionAst::Enchant { .. }
        | SubjectVerbActionAst::ChooseSpellCastHistory { .. }
        | SubjectVerbActionAst::AdditionalPhases { .. }
        | SubjectVerbActionAst::Learn
        | SubjectVerbActionAst::TurnFaceUp { .. }
        | SubjectVerbActionAst::ShuffleLibrary => None,
    }
}

pub(crate) fn effect_references_event_derived_amount(effect: &EffectAst) -> bool {
    assert_effect_ast_variant_coverage(effect);
    match effect {
        EffectAst::SubjectVerb(subject_verb) => {
            subject_verb_action_value(&subject_verb.action)
                .is_some_and(value_references_event_derived_amount)
                || match &subject_verb.action {
                    SubjectVerbActionAst::CounterUnlessPays { cost, .. } => {
                        total_cost_values_any(cost, value_references_event_derived_amount)
                    }
                    SubjectVerbActionAst::PreventDamageToTargetPutCounters {
                        amount: Some(amount),
                        ..
                    } => value_references_event_derived_amount(amount),
                    SubjectVerbActionAst::PutOrRemoveCounters {
                        put_count,
                        remove_count,
                        ..
                    } => {
                        value_references_event_derived_amount(put_count)
                            || value_references_event_derived_amount(remove_count)
                    }
                    SubjectVerbActionAst::Pump {
                        power, toughness, ..
                    }
                    | SubjectVerbActionAst::SetBasePowerToughness {
                        power, toughness, ..
                    }
                    | SubjectVerbActionAst::BecomeBasePtCreature {
                        power, toughness, ..
                    }
                    | SubjectVerbActionAst::PumpAll {
                        power, toughness, ..
                    } => {
                        value_references_event_derived_amount(power)
                            || value_references_event_derived_amount(toughness)
                    }
                    SubjectVerbActionAst::SetBasePower { power, .. } => {
                        value_references_event_derived_amount(power)
                    }
                    SubjectVerbActionAst::PumpForEach { count, .. } => {
                        value_references_event_derived_amount(count)
                    }
                    SubjectVerbActionAst::ReturnToBattlefield {
                        count_value: Some(count_value),
                        ..
                    } => value_references_event_derived_amount(count_value),
                    SubjectVerbActionAst::DestroyAll { filter, .. }
                    | SubjectVerbActionAst::DestroyAllOfChosenColor { filter, .. }
                    | SubjectVerbActionAst::ExileAll { filter, .. }
                    | SubjectVerbActionAst::ReturnAllToHand { filter }
                    | SubjectVerbActionAst::ReturnAllToHandOfChosenColor { filter }
                    | SubjectVerbActionAst::TapAll { filter }
                    | SubjectVerbActionAst::UntapAll { filter }
                    | SubjectVerbActionAst::PhaseOutAll { filter }
                    | SubjectVerbActionAst::PhaseInAll { filter }
                    | SubjectVerbActionAst::ScalePowerToughnessAll { filter, .. }
                    | SubjectVerbActionAst::SacrificeAll { filter }
                    | SubjectVerbActionAst::RegenerateAll { filter }
                    | SubjectVerbActionAst::ReturnAllToBattlefield { filter, .. }
                    | SubjectVerbActionAst::TagMatchingObjects { filter, .. }
                    | SubjectVerbActionAst::GrantAbilitiesAll { filter, .. }
                    | SubjectVerbActionAst::RemoveAbilitiesAll { filter, .. } => {
                        filter_references_event_derived_amount(filter)
                    }
                    SubjectVerbActionAst::CreateTokenWithMods {
                        dynamic_power_toughness: Some((power, toughness)),
                        ..
                    } => {
                        value_references_event_derived_amount(power)
                            || value_references_event_derived_amount(toughness)
                    }
                    SubjectVerbActionAst::ConsultTopOfLibrary { stop_rule, .. } => matches!(
                        stop_rule,
                        crate::cards::builders::LibraryConsultStopRuleAst::MatchCount(value)
                            if value_references_event_derived_amount(value)
                    ),
                    _ => false,
                }
        }
        _ => {
            let mut references = false;
            for_each_nested_effects(effect, true, |nested| {
                if !references {
                    references = nested.iter().any(effect_references_event_derived_amount);
                }
            });
            references
        }
    }
}

pub(crate) fn effect_references_its_controller(effect: &EffectAst) -> bool {
    assert_effect_ast_variant_coverage(effect);
    match effect {
        EffectAst::SubjectVerb(subject_verb) => {
            matches!(
                subject_verb.subject.player,
                PlayerAst::ItsController | PlayerAst::ItsOwner
            ) || matches!(
                &subject_verb.action,
                SubjectVerbActionAst::ExchangeLifeTotals {
                    player2: PlayerAst::ItsController | PlayerAst::ItsOwner
                } | SubjectVerbActionAst::CreateTokenCopy {
                    player: PlayerAst::ItsController | PlayerAst::ItsOwner,
                    ..
                } | SubjectVerbActionAst::CreateTokenCopyFromSource {
                    player: PlayerAst::ItsController | PlayerAst::ItsOwner,
                    ..
                } | SubjectVerbActionAst::CreateTokenWithMods {
                    player: PlayerAst::ItsController | PlayerAst::ItsOwner,
                    ..
                }
            )
        }
        EffectAst::ChooseObjects { player, .. }
        | EffectAst::ChooseObjectsAcrossZones { player, .. } => {
            matches!(player, PlayerAst::ItsController | PlayerAst::ItsOwner)
        }
        EffectAst::MayCastMatchingSpellWithoutPayingManaCost {
            player, zone_owner, ..
        } => {
            matches!(player, PlayerAst::ItsController | PlayerAst::ItsOwner)
                || matches!(zone_owner, PlayerAst::ItsController | PlayerAst::ItsOwner)
        }
        EffectAst::MayByPlayer { player, effects } => {
            matches!(player, PlayerAst::ItsController | PlayerAst::ItsOwner)
                || effects_reference_its_controller(effects)
        }
        EffectAst::UnlessPays {
            effects, player, ..
        } => {
            matches!(player, PlayerAst::ItsController | PlayerAst::ItsOwner)
                || effects_reference_its_controller(effects)
        }
        EffectAst::UnlessAction {
            effects,
            alternative,
            player,
            ..
        } => {
            matches!(player, PlayerAst::ItsController | PlayerAst::ItsOwner)
                || effects_reference_its_controller(effects)
                || effects_reference_its_controller(alternative)
        }
        _ => {
            let mut references = false;
            for_each_nested_effects(effect, true, |nested| {
                if !references {
                    references = nested.iter().any(effect_references_its_controller);
                }
            });
            references
        }
    }
}

pub(crate) fn effect_references_it_tag(effect: &EffectAst) -> bool {
    assert_effect_ast_variant_coverage(effect);
    if direct_effect_targets_reference_tag(effect, IT_TAG) {
        return true;
    }

    match effect {
        EffectAst::SubjectVerb(subject_verb) => match &subject_verb.action {
            SubjectVerbActionAst::DealDamageEach { amount, filter } => {
                value_references_tag(amount, IT_TAG) || filter_references_tag(filter, IT_TAG)
            }
            SubjectVerbActionAst::CounterUnlessPays { cost, .. } => {
                total_cost_values_any(cost, |value| value_references_tag(value, IT_TAG))
            }
            SubjectVerbActionAst::Discard { count, filter, .. } => {
                value_references_tag(count, IT_TAG)
                    || filter
                        .as_ref()
                        .is_some_and(|filter| filter_references_tag(filter, IT_TAG))
            }
            SubjectVerbActionAst::Sacrifice { filter, target, .. } => {
                filter_references_tag(filter, IT_TAG)
                    || target
                        .as_ref()
                        .is_some_and(|target| target_references_tag(target, IT_TAG))
            }
            SubjectVerbActionAst::SacrificeAll { filter } => filter_references_tag(filter, IT_TAG),
            SubjectVerbActionAst::PutSticker { target, .. }
            | SubjectVerbActionAst::MoveToLibraryNthFromTop { target, .. }
            | SubjectVerbActionAst::MoveToLibraryTopOrBottomChoice { target }
            | SubjectVerbActionAst::SwitchPowerToughness { target, .. } => {
                target_references_tag(target, IT_TAG)
            }
            SubjectVerbActionAst::Regenerate {
                follow_up_effects, ..
            } => effects_reference_it_tag(follow_up_effects),
            SubjectVerbActionAst::DestroyAll { filter, .. }
            | SubjectVerbActionAst::DestroyAllOfChosenColor { filter, .. }
            | SubjectVerbActionAst::ExileAll { filter, .. }
            | SubjectVerbActionAst::ReturnAllToHand { filter }
            | SubjectVerbActionAst::ReturnAllToHandOfChosenColor { filter }
            | SubjectVerbActionAst::TapAll { filter }
            | SubjectVerbActionAst::UntapAll { filter }
            | SubjectVerbActionAst::PhaseOutAll { filter }
            | SubjectVerbActionAst::PhaseInAll { filter }
            | SubjectVerbActionAst::ScalePowerToughnessAll { filter, .. }
            | SubjectVerbActionAst::RegenerateAll { filter } => {
                filter_references_tag(filter, IT_TAG)
            }
            SubjectVerbActionAst::TapOrUntapAll {
                tap_filter,
                untap_filter,
            } => {
                filter_references_tag(tap_filter, IT_TAG)
                    || filter_references_tag(untap_filter, IT_TAG)
            }
            SubjectVerbActionAst::PutIntoHand { object } => {
                matches!(object, ObjectRefAst::Tagged(tag) if tag.as_str() == IT_TAG)
            }
            SubjectVerbActionAst::ExileTopOfLibrary {
                count,
                tags,
                accumulated_tags,
            } => {
                value_references_tag(count, IT_TAG)
                    || tags.iter().any(|tag| tag.as_str() == IT_TAG)
                    || accumulated_tags.iter().any(|tag| tag.as_str() == IT_TAG)
            }
            SubjectVerbActionAst::DrawForEachTaggedMatching { tag, filter } => {
                tag.as_str() == IT_TAG || filter_references_tag(filter, IT_TAG)
            }
            SubjectVerbActionAst::PutCountersAll { count, filter, .. }
            | SubjectVerbActionAst::RemoveCountersAll {
                amount: count,
                filter,
                ..
            } => value_references_tag(count, IT_TAG) || filter_references_tag(filter, IT_TAG),
            SubjectVerbActionAst::RearrangeLookedCardsInLibrary { tag, .. }
            | SubjectVerbActionAst::ReorderTopOfLibrary { tag } => tag.as_str() == IT_TAG,
            SubjectVerbActionAst::ReduceNextSpellCostThisTurn { filter, .. }
            | SubjectVerbActionAst::ReduceMatchingSpellCostThisTurn { filter, .. } => {
                filter_references_tag(filter, IT_TAG)
            }
            SubjectVerbActionAst::PreventDamageToTargetPutCounters {
                amount: Some(amount),
                ..
            } => value_references_tag(amount, IT_TAG),
            SubjectVerbActionAst::PreventDamageEach { amount, filter, .. } => {
                value_references_tag(amount, IT_TAG) || filter_references_tag(filter, IT_TAG)
            }
            SubjectVerbActionAst::PutOrRemoveCounters {
                put_count,
                remove_count,
                ..
            } => value_references_tag(put_count, IT_TAG) || value_references_tag(remove_count, IT_TAG),
            SubjectVerbActionAst::PutCounterChoice { count, .. } => {
                value_references_tag(count, IT_TAG)
            }
            SubjectVerbActionAst::CopySpellForEachTarget { object_filter, .. } => object_filter
                .as_ref()
                .is_some_and(|filter| filter_references_tag(filter, IT_TAG)),
            SubjectVerbActionAst::DealDamageEqualToPower {
                source,
                amount,
                target,
            } => {
                target_references_tag(source, IT_TAG)
                    || value_references_tag(amount, IT_TAG)
                    || target_references_tag(target, IT_TAG)
            }
            SubjectVerbActionAst::CastTagged { tag, .. } => tag.as_str() == IT_TAG,
            SubjectVerbActionAst::GrantPlayTaggedUntilEndOfTurn { tag, .. }
            | SubjectVerbActionAst::GrantTaggedSpellAlternativeCostPayLifeByManaValueUntilEndOfTurn {
                tag,
                ..
            }
            | SubjectVerbActionAst::GrantPlayTaggedUntilYourNextTurn { tag, .. }
            | SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled { tag, .. }
            | SubjectVerbActionAst::GrantPlayTaggedForAsLongAsYouControlSource { tag, .. } => {
                tag.as_str() == IT_TAG
            }
            SubjectVerbActionAst::PutRestOnBottomOfLibrary => true,
            SubjectVerbActionAst::Cant { restriction, .. } => {
                restriction_references_tag(restriction, IT_TAG)
            }
            SubjectVerbActionAst::CreateTokenCopy { object, .. } => {
                matches!(object, ObjectRefAst::Tagged(tag) if tag.as_str() == IT_TAG)
            }
            action => subject_verb_action_value(action)
                .is_some_and(|value| value_references_tag(value, IT_TAG)),
        },
        EffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        }
        | EffectAst::SelfReplacement {
            predicate,
            if_true,
            if_false,
        } => {
            predicate_uses_implicit_it_reference(predicate)
                || predicate_references_tag(predicate, IT_TAG)
                || effects_reference_it_tag(if_true)
                || effects_reference_it_tag(if_false)
        }
        EffectAst::ForEachTagged { tag, effects } => {
            tag.as_str() == IT_TAG || effects_reference_it_tag(effects)
        }
        EffectAst::DelayedWhenLastObjectDiesThisTurn { .. } => true,
        EffectAst::ForEachObject { filter, effects } => {
            filter_references_tag(filter, IT_TAG) || effects_reference_it_tag(effects)
        }
        _ => {
            if let Some(filter) = effect_tagged_filter(effect) {
                return filter_references_tag(filter, IT_TAG);
            }
            let mut references = false;
            for_each_nested_effects(effect, true, |nested| {
                if !references {
                    references = nested.iter().any(effect_references_it_tag);
                }
            });
            references
        }
    }
}

fn predicate_uses_implicit_it_reference(predicate: &PredicateAst) -> bool {
    match predicate {
        PredicateAst::ItIsLandCard
        | PredicateAst::ItIsSoulbondPaired
        | PredicateAst::ItMatches(_)
        | PredicateAst::TargetMatches(_) => true,
        PredicateAst::Not(inner) => predicate_uses_implicit_it_reference(inner),
        PredicateAst::And(left, right) | PredicateAst::Or(left, right) => {
            predicate_uses_implicit_it_reference(left)
                || predicate_uses_implicit_it_reference(right)
        }
        _ => false,
    }
}

pub(crate) fn restriction_references_tag(
    restriction: &crate::effect::Restriction,
    tag: &str,
) -> bool {
    use crate::effect::Restriction;

    let maybe_filter = match restriction {
        Restriction::Attack(filter)
        | Restriction::Block(filter)
        | Restriction::MustBeBlocked(filter)
        | Restriction::Untap(filter)
        | Restriction::BeBlocked(filter)
        | Restriction::BeDestroyed(filter)
        | Restriction::BeRegenerated(filter)
        | Restriction::BeSacrificed(filter)
        | Restriction::HaveCountersPlaced(filter)
        | Restriction::BeTargeted(filter)
        | Restriction::BeCountered(filter)
        | Restriction::Transform(filter)
        | Restriction::PhaseOut(filter)
        | Restriction::AttackOrBlock(filter)
        | Restriction::ActivateAbilitiesOf(filter)
        | Restriction::ActivateTapAbilitiesOf(filter)
        | Restriction::ActivateNonManaAbilitiesOf(filter) => Some(filter),
        _ => None,
    };
    if let Some(filter) = maybe_filter {
        return filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == tag);
    }

    if let Restriction::BlockSpecificAttacker { blockers, attacker } = restriction {
        let blockers_reference = blockers
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == tag);
        let attacker_reference = attacker
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == tag);
        return blockers_reference || attacker_reference;
    }
    if let Restriction::MustBlockSpecificAttacker { blockers, attacker } = restriction {
        let blockers_reference = blockers
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == tag);
        let attacker_reference = attacker
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == tag);
        return blockers_reference || attacker_reference;
    }

    if let Restriction::AttackPlayerOrPlaneswalkersControlledBy { attackers, .. } = restriction {
        return attackers
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == tag);
    }

    false
}

pub(crate) fn collect_tag_spans_from_effects_with_context(
    effects: &[EffectAst],
    annotations: &mut ParseAnnotations,
    ctx: &NormalizedLine,
) {
    for effect in effects {
        collect_tag_spans_from_effect(effect, annotations, ctx);
    }
}

fn collect_direct_effect_target_spans(
    effect: &EffectAst,
    annotations: &mut ParseAnnotations,
    ctx: &NormalizedLine,
) -> bool {
    let mut collected = false;
    with_direct_effect_targets(effect, |target| {
        collect_tag_spans_from_target(target, annotations, ctx);
        collected = true;
    });
    collected
}

pub(crate) fn collect_tag_spans_from_effect(
    effect: &EffectAst,
    annotations: &mut ParseAnnotations,
    ctx: &NormalizedLine,
) {
    assert_effect_ast_variant_coverage(effect);
    if collect_direct_effect_target_spans(effect, annotations, ctx) {
        return;
    }

    for_each_nested_effects(effect, true, |nested| {
        collect_tag_spans_from_effects_with_context(nested, annotations, ctx);
    });
}

pub(crate) fn collect_tag_spans_from_target(
    target: &TargetAst,
    annotations: &mut ParseAnnotations,
    ctx: &NormalizedLine,
) {
    if let TargetAst::WithCount(inner, _) = target {
        collect_tag_spans_from_target(inner, annotations, ctx);
        return;
    }
    if let TargetAst::Tagged(tag, Some(span)) = target {
        let mapped =
            super::map_span_to_original(*span, &ctx.normalized, &ctx.original, &ctx.char_map);
        #[cfg(not(feature = "serialization"))]
        annotations.record_tag_span(tag.as_str(), mapped);
        #[cfg(feature = "serialization")]
        annotations.record_tag_span(tag, mapped);
    }
    if let TargetAst::Object(filter, _, Some(it_span)) = target
        && filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == IT_TAG)
    {
        let mapped =
            super::map_span_to_original(*it_span, &ctx.normalized, &ctx.original, &ctx.char_map);
        #[cfg(not(feature = "serialization"))]
        annotations.record_tag_span(IT_TAG, mapped);
        #[cfg(feature = "serialization")]
        {
            let it_tag = TagKey::from(IT_TAG);
            annotations.record_tag_span(&it_tag, mapped);
        }
    }
}
