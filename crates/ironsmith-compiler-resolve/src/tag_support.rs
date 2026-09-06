use ironsmith_compiler_semantic::model::ForEachEffectAst;
use ironsmith_compiler_semantic::model::DamagePreventionActionAst;
use crate::cards::builders::{
    EffectAst, ObjectRefAst, ParseAnnotations, PlayerAst, PredicateAst, RetargetModeAst,
    SubjectVerbActionAst, SubjectVerbEffectAst, TagKey, TargetAst, CounterActionAst, GrantActionAst, ManaActionAst, LibraryActionAst, ReplacementActionAst, ExchangeActionAst, TurnStructureActionAst, CharacteristicActionAst, KeywordActionAst, ZoneMoveActionAst, PermanentStateActionAst, RevealLookActionAst, RandomActionAst, LifeResourceActionAst, ChoiceActionAst, DamageActionAst, StatChangeActionAst, StackActionAst, TokenActionAst, ControlActionAst, GameActionAst, DelayedEffectAst, ObjectChoiceEffectAst, ConditionalEffectAst, PermissionEffectAst, PlayerPredicateAst, SourcePredicateAst, TurnEventPredicateAst,
};
use crate::effect::{EventValueSpec, Value};
use crate::filter::{ObjectFilter, ObjectRef, PlayerFilter, TaggedOpbjectRelation};
use crate::target::ChooseSpec;

use super::{SpanMappingContext, assert_effect_ast_variant_coverage, for_each_nested_effects};

pub fn is_revealed_collection_tag(tag: &TagKey) -> bool {
    crate::tag::CompilerTagClass::RevealedCollection.contains(tag)
}

pub fn is_searched_collection_tag(tag: &TagKey) -> bool {
    crate::tag::CompilerTagClass::SearchedCollection.contains(tag)
}

pub fn is_exile_cost_collection_tag(tag: &TagKey) -> bool {
    crate::tag::CompilerCostObjectTag::Exile.matches(tag)
}

pub fn is_sentence_helper_exiled_collection_tag(tag: &TagKey) -> bool {
    crate::tag::CompilerTagClass::SentenceHelperExiledCollection.contains(tag)
}

pub fn is_sentence_helper_consult_match_tag(tag: &TagKey) -> bool {
    crate::tag::CompilerTagClass::SentenceHelperConsultMatch.contains(tag)
}

pub fn is_exiled_collection_tag(tag: &TagKey) -> bool {
    crate::tag::CompilerTagClass::ExiledCollection.contains(tag)
}

fn total_cost_values_any(
    cost: &ironsmith_core::TotalCost<crate::model::CompilerCost>,
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
    component: &crate::model::CompilerCost,
    predicate: impl Fn(&Value) -> bool + Copy,
) -> bool {
    match component {
        crate::model::CompilerCost::DynamicMana(dynamic) => {
            dynamic.x_value.as_ref().is_some_and(predicate)
                || dynamic.additional_generic.as_ref().is_some_and(predicate)
                || dynamic.multiplier.as_ref().is_some_and(predicate)
        }
        crate::model::CompilerCost::Life(value) => predicate(value),
        crate::model::CompilerCost::Effect(effect)
        | crate::model::CompilerCost::ValidatedEffect(effect) => {
            effect_ast_values_any(effect, predicate)
        }
        _ => false,
    }
}

fn effect_ast_values_any(effect: &EffectAst, predicate: impl Fn(&Value) -> bool + Copy) -> bool {
    let mut found = match effect {
        EffectAst::SubjectVerb(subject_verb) => {
            subject_verb_action_value(&subject_verb.action).is_some_and(predicate)
        }
        EffectAst::ForEach(ForEachEffectAst::RepeatEffects { count, .. }) => predicate(count),
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
            count_value: Some(value),
            ..
        })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsBottomOfLibrary {
            count_value: Some(value),
            ..
        })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsTopOfLibrary {
            count_value: Some(value),
            ..
        })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsAcrossZones {
            count_value: Some(value),
            ..
        }) => predicate(value),
        _ => false,
    };
    for_each_nested_effects(effect, true, |nested| {
        found |= nested
            .iter()
            .any(|effect| effect_ast_values_any(effect, predicate));
    });
    found
}

pub fn effects_reference_tag(effects: &[EffectAst], tag: &str) -> bool {
    effects
        .iter()
        .any(|effect| effect_references_tag(effect, tag))
}

fn push_unique_tag(tags: &mut Vec<TagKey>, tag: &TagKey) {
    if !tags.iter().any(|known| known == tag) {
        tags.push(tag.clone());
    }
}

fn collect_effect_produced_tags(effect: &EffectAst, tags: &mut Vec<TagKey>) {
    match effect {
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects { tag, .. })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsWithAggregateConstraint { tag, .. })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsBottomOfLibrary { tag, .. })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsTopOfLibrary { tag, .. })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone { tag, .. })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsAcrossZones { tag, .. })
        | EffectAst::TagAffected { tag, .. } => push_unique_tag(tags, tag),
        EffectAst::SnapshotLastObjectTag { into } => push_unique_tag(tags, into),
        EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) => match action {
            SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseCardName { tag, .. })
            | SubjectVerbActionAst::Choices(ChoiceActionAst::ChoosePlayer { tag, .. })
            | SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseSpellCastHistory { tag, .. })
            | SubjectVerbActionAst::RevealLook(RevealLookActionAst::RevealCardsFromHand { tag, .. })
            | SubjectVerbActionAst::RevealLook(RevealLookActionAst::LookAtTopCards { tag, .. })
            | SubjectVerbActionAst::TagMatchingObjects { tag, .. }
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::SearchLibrarySlotsToHand {
                progress_tag: tag, ..
            }) => push_unique_tag(tags, tag),
            SubjectVerbActionAst::Library(LibraryActionAst::ExileTopOfLibrary {
                tags: result_tags,
                accumulated_tags,
                ..
            }) => {
                for tag in result_tags.iter().chain(accumulated_tags) {
                    push_unique_tag(tags, tag);
                }
            }
            SubjectVerbActionAst::Library(LibraryActionAst::ConsultTopOfLibrary {
                all_tag, match_tag, ..
            }) => {
                push_unique_tag(tags, all_tag);
                push_unique_tag(tags, match_tag);
            }
            SubjectVerbActionAst::Stack(StackActionAst::CopySpell { .. })
            | SubjectVerbActionAst::Stack(StackActionAst::CopySpellForEachTarget { .. }) => push_unique_tag(
                tags,
                &crate::tag::CompilerReferenceTag::CopiedStackObject.bind(),
            ),
            _ => {}
        },
        _ => {}
    }

    for_each_nested_effects(effect, true, |nested| {
        for nested_effect in nested {
            collect_effect_produced_tags(nested_effect, tags);
        }
    });
}

/// Whether a later sibling consumes an explicit tag produced by an earlier
/// sibling. Such siblings are a semantic pipeline, not independent display
/// arms, and must remain flat for reference flow and specialist lowering.
pub fn effects_have_cross_arm_tag_dependency(effects: &[EffectAst]) -> bool {
    let mut prior_tags = Vec::<TagKey>::new();
    for effect in effects {
        if prior_tags
            .iter()
            .any(|tag| effect_references_tag(effect, tag.as_str()))
        {
            return true;
        }
        collect_effect_produced_tags(effect, &mut prior_tags);
    }
    false
}

pub fn effects_reference_tag_in_object_position(effects: &[EffectAst], tag: &str) -> bool {
    effects
        .iter()
        .any(|effect| effect_references_tag_in_object_position(effect, tag))
}

fn with_direct_effect_targets(effect: &EffectAst, mut visit: impl FnMut(&TargetAst)) {
    assert_effect_ast_variant_coverage(effect);
    if let EffectAst::SubjectVerb(subject_verb) = effect {
        match &subject_verb.action {
            SubjectVerbActionAst::Damage(DamageActionAst::DealDistributedDamage { target, source, .. }) => {
                visit(target);
                visit(source);
            }
            SubjectVerbActionAst::Damage(DamageActionAst::DealDamage { target, .. })
            | SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEqualToPower { target, .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Tap { target })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Untap { target })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Destroy { target, .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Exile { target, .. })
            | SubjectVerbActionAst::RevealLook(RevealLookActionAst::LookAtHand { target })
            | SubjectVerbActionAst::RevealLook(RevealLookActionAst::LookAtTarget { target })
            | SubjectVerbActionAst::Stack(StackActionAst::Counter { target })
            | SubjectVerbActionAst::Stack(StackActionAst::CounterUnlessPays { target, .. })
            | SubjectVerbActionAst::Counters(CounterActionAst::PutCounters { target, .. })
            | SubjectVerbActionAst::Counters(CounterActionAst::PutCounterChoice { target, .. })
            | SubjectVerbActionAst::Counters(CounterActionAst::PutOrRemoveCounters { target, .. })
            | SubjectVerbActionAst::Stack(StackActionAst::CopySpell { target, .. })
            | SubjectVerbActionAst::Stack(StackActionAst::CopySpellForEachTarget { target, .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToBattlefield { target, .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::FightIterated { creature2: target })
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
            | SubjectVerbActionAst::Exchanges(ExchangeActionAst::ExchangeControlHeterogeneous {
                permanent1: target, ..
            })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DestroyAllAttachedTo { target, .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileAllAttachedTo { target, .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileWhenSourceLeaves { target })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::SacrificeSourceWhenLeaves { target })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MayMoveToZone { target, .. })
            | SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterZoneReplacement { target, .. })
            | SubjectVerbActionAst::Library(LibraryActionAst::ShuffleObjectsIntoLibrary { target, .. })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::AssignNoCombatDamage { source: target, .. })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventAllCombatDamageFromSource { source: target, .. })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::RedirectNextTimeDamageToSource { target, .. })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::RedirectAllDamageThisTurnBySourceToSourceController {
                source: target,
            })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::RedirectAllDamageThisTurnToTarget { target, .. })
            | SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenCopyFromSource { source: target, .. })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventDamage { target, .. })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventDamageToTargetPutCounters { target, .. })
            | SubjectVerbActionAst::Library(LibraryActionAst::MoveToLibraryNthFromTop { target, .. })
            | SubjectVerbActionAst::Library(LibraryActionAst::MoveToLibraryTopOrBottomChoice { target })
            | SubjectVerbActionAst::Counters(CounterActionAst::RemoveUpToAnyCounters { target, .. })
            | SubjectVerbActionAst::Counters(CounterActionAst::DoubleCountersOnTarget { target, .. })
            | SubjectVerbActionAst::Counters(CounterActionAst::PutCounterOfChosenKind { target })
            | SubjectVerbActionAst::PutSticker { target, .. }
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::SwitchPowerToughness { target, .. })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantProtectionChoice { target, .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToHand { target, .. }) => visit(target),
            SubjectVerbActionAst::Counters(CounterActionAst::ForEachCounterKindPutOrRemove {
                target,
                counter_source,
                ..
            }) => {
                visit(target);
                if let Some(counter_source) = counter_source {
                    visit(counter_source);
                }
            }
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileUntilSourceLeaves {
                target,
                leave_watcher,
                ..
            }) => {
                visit(target);
                if let Some(leave_watcher) = leave_watcher {
                    visit(leave_watcher);
                }
            }
            SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventAllDamageToTarget {
                target,
                source_target,
                ..
            }) => {
                visit(target);
                if let Some(source_target) = source_target {
                    visit(source_target);
                }
            }
            SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventAllCombatDamageFromSourceFilter {
                excluded_source_target: Some(target),
                ..
            }) => visit(target),
            SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::RedirectNextDamageFromSourceToTarget {
                protected_target,
                destination_target,
                ..
            }) => {
                if let Some(target) = protected_target {
                    visit(target);
                }
                if let Some(target) = destination_target {
                    visit(target);
                }
            }
            SubjectVerbActionAst::Stack(StackActionAst::RetargetStackObject { target, mode, .. }) => {
                visit(target);
                if let RetargetModeAst::OneToFixed { target: fixed } = mode {
                    visit(fixed);
                }
            }
            SubjectVerbActionAst::Counters(CounterActionAst::MoveAllCounters { from, to })
            | SubjectVerbActionAst::Counters(CounterActionAst::MoveOneCounter { from, to }) => {
                visit(from);
                visit(to);
            }
            SubjectVerbActionAst::Control(ControlActionAst::Attach { object, target }) => {
                visit(object);
                visit(target);
            }
            SubjectVerbActionAst::Control(ControlActionAst::Unattach { object }) => visit(object),
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::Fight {
                creature1,
                creature2,
                ..
            }) => {
                visit(creature1);
                visit(creature2);
            }
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Sacrifice {
                target: Some(target),
                ..
            })
            | SubjectVerbActionAst::Control(ControlActionAst::GainControl { target, .. }) => visit(target),
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone {
                target,
                attached_to,
                ..
            }) => {
                visit(target);
                if let Some(attach_target) = attached_to {
                    visit(attach_target);
                }
            }
            SubjectVerbActionAst::TargetOnly { target, .. } => visit(target),
            SubjectVerbActionAst::StatChanges(StatChangeActionAst::Pump { target, .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetBasePowerToughness { target, .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeBasePtCreature { target, .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetBasePower { target, .. })
            | SubjectVerbActionAst::StatChanges(StatChangeActionAst::PumpForEach { target, .. })
            | SubjectVerbActionAst::StatChanges(StatChangeActionAst::PumpByLastEffect { target, .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::AddCardTypes { target, .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetCardTypes { target, .. })
            | SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveCardTypes { target, .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::AddSubtypes { target, .. })
            | SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveSubtypes { target, .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetCreatureSubtypes { target, .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeSaddledUntilEndOfTurn { target })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::AddColors { target, .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::AddAllSubtypesOfFamily { target, .. })
            | SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveAllSubtypesOfFamily { target, .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeBasicLandType { target, .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetColors { target, .. })
            | SubjectVerbActionAst::StatChanges(StatChangeActionAst::MakeColorless { target, .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeBasicLandTypeChoice { target, .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeCreatureTypeChoice { target, .. })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeColorChoice { target, .. })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesToTarget { target, .. })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantToTarget { target, .. })
            | SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveAbilitiesFromTarget { target, .. })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesChoiceToTarget { target, .. }) => visit(target),
            SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeCopy { target, source, .. }) => {
                visit(target);
                visit(source);
            }
            _ => {}
        }
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
            action: SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEach { filter, .. }),
            ..
        }) => filter_references_tag(filter, tag),
        EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        })
        | EffectAst::SelfReplacement {
            predicate,
            if_true,
            if_false,
            ..
        } => {
            predicate_references_tag(predicate, tag)
                || effects_reference_tag_in_object_position(if_true, tag)
                || effects_reference_tag_in_object_position(if_false, tag)
        }
        EffectAst::Conditionals(ConditionalEffectAst::TrailingIf { predicate, effects })
        | EffectAst::Conditionals(ConditionalEffectAst::TrailingUnless { predicate, effects }) => {
            predicate_references_tag(predicate, tag)
                || effects_reference_tag_in_object_position(effects, tag)
        }
        EffectAst::ForEach(ForEachEffectAst::ForEachObject { filter, effects }) => {
            filter_references_tag(filter, tag)
                || effects_reference_tag_in_object_position(effects, tag)
        }
        EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
            tag: found,
            effects,
        }) => found.as_str() == tag || effects_reference_tag_in_object_position(effects, tag),
        EffectAst::ForEach(ForEachEffectAst::ForEachTaggedWithControllerAtLastBlockedBy {
            tag: found,
            blocker_tag,
            effects,
        }) => {
            found.as_str() == tag
                || blocker_tag.as_str() == tag
                || effects_reference_tag_in_object_position(effects, tag)
        }
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

pub fn filter_references_tag(filter: &ObjectFilter, tag: &str) -> bool {
    let comparison_references_tag = |comparison: &crate::filter::Comparison| match comparison {
        crate::filter::Comparison::EqualExpr(value)
        | crate::filter::Comparison::NotEqualExpr(value)
        | crate::filter::Comparison::LessThanExpr(value)
        | crate::filter::Comparison::LessThanOrEqualExpr(value)
        | crate::filter::Comparison::GreaterThanExpr(value)
        | crate::filter::Comparison::GreaterThanOrEqualExpr(value) => {
            value_references_tag(value, tag)
        }
        _ => false,
    };
    filter
        .tagged_constraints
        .iter()
        .any(|constraint| constraint.tag.as_str() == tag)
        || [
            filter.power.as_ref(),
            filter.toughness.as_ref(),
            filter.mana_value.as_ref(),
            filter.color_count.as_ref(),
        ]
        .into_iter()
        .flatten()
        .any(comparison_references_tag)
        || filter
            .controller
            .as_ref()
            .is_some_and(|player| player_filter_references_tag(player, tag))
        || filter
            .owner
            .as_ref()
            .is_some_and(|player| player_filter_references_tag(player, tag))
        || filter
            .cast_by
            .as_ref()
            .is_some_and(|player| player_filter_references_tag(player, tag))
        || filter
            .targets_player
            .as_ref()
            .is_some_and(|player| player_filter_references_tag(player, tag))
        || filter
            .targets_only_player
            .as_ref()
            .is_some_and(|player| player_filter_references_tag(player, tag))
        || filter
            .attached_to_player
            .as_ref()
            .is_some_and(|player| player_filter_references_tag(player, tag))
        || filter
            .entered_battlefield_controller
            .as_ref()
            .is_some_and(|player| player_filter_references_tag(player, tag))
        || filter
            .counters_put_on_this_turn
            .as_ref()
            .is_some_and(|constraint| {
                player_filter_references_tag(&constraint.source_controller, tag)
            })
        || filter
            .discarded_or_cycled_this_turn_by
            .as_ref()
            .is_some_and(|player| player_filter_references_tag(player, tag))
        || filter
            .dealt_damage_to_player_this_turn
            .as_ref()
            .is_some_and(|player| player_filter_references_tag(player, tag))
        || filter
            .could_be_targeted_by
            .as_ref()
        .is_some_and(|constraint| {
            matches!(&constraint.stack_object, crate::filter::ObjectRef::Tagged(object_tag) if object_tag.as_str() == tag)
        })
        || matches!(&filter.blocked_by, Some(crate::filter::ObjectRef::Tagged(object_tag)) if object_tag.as_str() == tag)
        || matches!(&filter.in_combat_with, Some(crate::filter::ObjectRef::Tagged(object_tag)) if object_tag.as_str() == tag)
        || filter
            .targets_object
            .as_deref()
            .is_some_and(|targets| filter_references_tag(targets, tag))
        || filter
            .targets_only_object
            .as_deref()
            .is_some_and(|targets| filter_references_tag(targets, tag))
        || filter
            .attached_to_object
            .as_deref()
            .is_some_and(|attached_to| filter_references_tag(attached_to, tag))
        || filter
            .blocked_or_was_blocked_by_this_turn
            .as_deref()
            .is_some_and(|combat_partner| filter_references_tag(combat_partner, tag))
        || filter
            .any_of
            .iter()
            .any(|branch| filter_references_tag(branch, tag))
}

fn effect_tagged_filter(effect: &EffectAst) -> Option<&ObjectFilter> {
    match effect {
        EffectAst::SubjectVerb(subject_verb) => match &subject_verb.action {
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DestroyAll { filter, .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DestroyAllOfChosenColor { filter, .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileAll { filter, .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnAllToHand { filter, .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnAllToHandOfChosenColor { filter })
            | SubjectVerbActionAst::Counters(CounterActionAst::PutCountersAll { filter, .. })
            | SubjectVerbActionAst::Counters(CounterActionAst::DoubleCountersOnEach { filter, .. })
            | SubjectVerbActionAst::Counters(CounterActionAst::RemoveCountersAll { filter, .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::ScalePowerToughnessAll { filter, .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Sacrifice { filter, .. })
            | SubjectVerbActionAst::Exchanges(ExchangeActionAst::ExchangeControl { filter, .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DestroyAllAttachedTo { filter, .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileAllAttachedTo { filter, .. })
            | SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseSpellCastHistory { filter, .. })
            | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventDamageEach { filter, .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnAllToBattlefield { filter, .. })
            | SubjectVerbActionAst::StatChanges(StatChangeActionAst::PumpAll { filter, .. })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesAll { filter, .. })
            | SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveAbilitiesAll { filter, .. })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesChoiceAll { filter, .. })
            | SubjectVerbActionAst::Library(LibraryActionAst::ConsultTopOfLibrary { filter, .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::SearchLibrary { filter, .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::SacrificeAll { filter }) => Some(filter),
            SubjectVerbActionAst::Grants(GrantActionAst::GrantBySpec { spec, .. }) => Some(&spec.filter),
            SubjectVerbActionAst::Control(ControlActionAst::Enchant {
                filter: crate::object::AuraAttachmentFilter::Object(filter),
            }) => Some(filter),
            _ => None,
        },
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects { filter, .. })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsWithAggregateConstraint { filter, .. })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsAcrossZones { filter, .. })
        | EffectAst::Permissions(PermissionEffectAst::MayCastMatchingSpellWithoutPayingManaCost { filter, .. }) => Some(filter),
        _ => None,
    }
}

pub fn effect_references_tag(effect: &EffectAst, tag: &str) -> bool {
    assert_effect_ast_variant_coverage(effect);
    if tag == "triggering_source"
        && matches!(
            effect,
            EffectAst::SubjectVerb(subject_verb)
                if subject_verb.subject.player == PlayerAst::TriggeringSourceController
        )
    {
        return true;
    }
    if direct_effect_targets_reference_tag(effect, tag) {
        return true;
    }
    if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::Control(ControlActionAst::GainControl {
                controller_reference: Some(ObjectRef::Tagged(controller_reference_tag)),
                ..
            }),
        ..
    }) = effect
        && controller_reference_tag.as_str() == tag
    {
        return true;
    }
    if let EffectAst::SubjectVerb(subject_verb) = effect
        && subject_verb_action_value(&subject_verb.action)
            .is_some_and(|value| value_references_tag(value, tag))
    {
        return true;
    }
    if let EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsWithAggregateConstraint { constraint, .. }) = effect
        && (value_references_tag(&constraint.maximum, tag)
            || constraint
                .minimum
                .as_ref()
                .is_some_and(|minimum| value_references_tag(minimum, tag)))
    {
        return true;
    }
    if let Some(filter) = effect_tagged_filter(effect) {
        return filter_references_tag(filter, tag);
    }

    match effect {
        EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        })
        | EffectAst::SelfReplacement {
            predicate,
            if_true,
            if_false,
            ..
        } => {
            predicate_references_tag(predicate, tag)
                || effects_reference_tag(if_true, tag)
                || effects_reference_tag(if_false, tag)
        }
        EffectAst::Conditionals(ConditionalEffectAst::TrailingIf { predicate, effects })
        | EffectAst::Conditionals(ConditionalEffectAst::TrailingUnless { predicate, effects }) => {
            predicate_references_tag(predicate, tag) || effects_reference_tag(effects, tag)
        }
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Stack(StackActionAst::CopySpellForEachTarget { object_filter, .. }),
            ..
        }) => object_filter
            .as_ref()
            .is_some_and(|filter| filter_references_tag(filter, tag)),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEqualToPower {
                    source,
                    amount,
                    target,
                    ..
                }),
            ..
        }) => {
            target_references_tag(source, tag)
                || value_references_tag(amount, tag)
                || target_references_tag(target, tag)
        }
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenCopy { object, .. }),
            ..
        }) => match object {
            ObjectRefAst::Tagged(found) => found.as_str() == tag,
        },
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods {
                    count,
                    dynamic_power_toughness,
                    attached_to,
                    ..
                }),
            ..
        }) => {
            value_references_tag(count, tag)
                || dynamic_power_toughness
                    .as_ref()
                    .is_some_and(|(power, toughness)| {
                        value_references_tag(power, tag) || value_references_tag(toughness, tag)
                    })
                || attached_to
                    .as_ref()
                    .is_some_and(|target| target_references_tag(target, tag))
        }
        EffectAst::ForEach(ForEachEffectAst::ForEachObject { filter, effects }) => {
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

pub fn value_references_tag(value: &Value, tag: &str) -> bool {
    match value {
        Value::SurfaceHinted { value, .. } => value_references_tag(value, tag),
        Value::Add(left, right) | Value::Min(left, right) => {
            value_references_tag(left, tag) || value_references_tag(right, tag)
        }
        Value::Scaled(value, _)
        | Value::DividedRoundedDown(value, _)
        | Value::HalfRoundedDown(value) => value_references_tag(value, tag),
        Value::Count(filter)
        | Value::CountScaled(filter, _)
        | Value::GreatestCount(filter)
        | Value::GreatestSharedCreatureTypeCount(filter)
        | Value::TotalPower(filter)
        | Value::TotalToughness(filter)
        | Value::TotalManaValue(filter)
        | Value::GreatestPower(filter)
        | Value::GreatestToughness(filter)
        | Value::GreatestManaValue(filter)
        | Value::LeastPower(filter)
        | Value::LeastToughness(filter)
        | Value::LeastManaValue(filter)
        | Value::BasicLandTypesAmong(filter)
        | Value::CreatureTypesAmong(filter)
        | Value::CardTypesAmong(filter)
        | Value::ColorsAmong(filter)
        | Value::ColorPairsAmong(filter)
        | Value::DistinctCounterTypesAmong(filter)
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
        Value::ManaValueOf(spec) | Value::ManaSymbolsInManaCostOf { spec, .. } => {
            choose_spec_references_tag(spec, tag)
        }
        Value::CountersOn(spec, _) => choose_spec_references_tag(spec, tag),
        Value::DamageDealtThisTurnByTaggedSpellCast(t) => t.as_str() == tag,
        Value::PriorEffectMetric { query, .. } | Value::PendingPriorEffectMetric(query) => {
            query.filter.as_ref().is_some_and(|filter| {
                filter
                    .tagged_constraints
                    .iter()
                    .any(|constraint| constraint.tag.as_str() == tag)
            }) || query
                .player
                .as_ref()
                .is_some_and(|player| player_filter_references_tag(player, tag))
        }
        _ => false,
    }
}

pub fn predicate_references_tag(predicate: &PredicateAst, tag: &str) -> bool {
    match predicate {
        PredicateAst::ItMatches(filter)
        | PredicateAst::ItMatchedLastKnown(filter)
        | PredicateAst::TargetMatches(filter)
        | PredicateAst::Source(SourcePredicateAst::SourceMatches(filter))
        | PredicateAst::AttachedToSourceMatches(filter)
        | PredicateAst::NoVoteObjectsMatched { filter }
        | PredicateAst::TurnEvents(TurnEventPredicateAst::ObjectEnteredBattlefieldThisTurn(filter))
        | PredicateAst::TurnEvents(TurnEventPredicateAst::ObjectEnteredBattlefieldLastTurn(filter))
        | PredicateAst::TurnEvents(TurnEventPredicateAst::ObjectPutIntoGraveyardFromBattlefieldThisTurn(filter)) => {
            filter_references_tag(filter, tag)
        }
        PredicateAst::TaggedMatches(found, filter) => {
            found.as_str() == tag || filter_references_tag(filter, tag)
        }
        PredicateAst::TaggedWasCast(found)
        | PredicateAst::Player(PlayerPredicateAst::PlayerTaggedObjectEnteredBattlefieldThisTurn { tag: found, .. }) => {
            found.as_str() == tag
        }
        PredicateAst::Player(PlayerPredicateAst::PlayerTaggedObjectMatches {
            tag: found, filter, ..
        }) => found.as_str() == tag || filter_references_tag(filter, tag),
        PredicateAst::Player(PlayerPredicateAst::PlayerControls { filter, .. })
        | PredicateAst::Player(PlayerPredicateAst::PlayerHasAtLeast { filter, .. })
        | PredicateAst::Player(PlayerPredicateAst::PlayerControlsExactly { filter, .. })
        | PredicateAst::Player(PlayerPredicateAst::PlayerHasAtLeastWithDifferentPowers { filter, .. })
        | PredicateAst::Player(PlayerPredicateAst::PlayerControlsNo { filter, .. })
        | PredicateAst::Player(PlayerPredicateAst::PlayerControlsMost { filter, .. })
        | PredicateAst::Player(PlayerPredicateAst::PlayerControlsMoreThanEachOtherPlayer { filter, .. })
        | PredicateAst::AnOpponentHasFewerThanPlayer { filter, .. }
        | PredicateAst::Player(PlayerPredicateAst::PlayerControlsMoreThanYou { filter, .. })
        | PredicateAst::Source(SourcePredicateAst::SourceHasAttachmentsMatching { filter, .. }) => {
            filter_references_tag(filter, tag)
        }
        PredicateAst::Player(PlayerPredicateAst::PlayerControlsOrHasCardInGraveyard {
            control_filter,
            graveyard_filter,
            ..
        }) => {
            filter_references_tag(control_filter, tag)
                || filter_references_tag(graveyard_filter, tag)
        }
        PredicateAst::ValueComparison { left, right, .. } => {
            value_references_tag(left, tag) || value_references_tag(right, tag)
        }
        PredicateAst::ValueIsPrime(value) => value_references_tag(value, tag),
        PredicateAst::Not(inner) => predicate_references_tag(inner, tag),
        PredicateAst::And(left, right) | PredicateAst::Or(left, right) => {
            predicate_references_tag(left, tag) || predicate_references_tag(right, tag)
        }
        _ => false,
    }
}

pub fn choose_spec_references_tag(spec: &ChooseSpec, tag: &str) -> bool {
    match spec {
        ChooseSpec::Tagged(t) => t.as_str() == tag,
        ChooseSpec::SurfaceHinted { spec: inner, .. }
        | ChooseSpec::Target(inner)
        | ChooseSpec::WithCount(inner, _)
        | ChooseSpec::WithCountValue(inner, _, _) => choose_spec_references_tag(inner, tag),
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == tag),
        _ => false,
    }
}

pub fn choose_spec_references_exiled_tag(spec: &ChooseSpec) -> bool {
    fn is_exiled_tag(tag: &TagKey) -> bool {
        is_exiled_collection_tag(tag)
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

pub fn object_ref_references_tag(reference: &ObjectRef, tag: &str) -> bool {
    matches!(reference, ObjectRef::Tagged(found) if found.as_str() == tag)
}

pub fn player_filter_references_tag(filter: &PlayerFilter, tag: &str) -> bool {
    match filter {
        PlayerFilter::Target(inner)
        | PlayerFilter::AliasedTarget(inner)
        | PlayerFilter::CardsInHandAtLeastMoreThanYou { base: inner, .. }
        | PlayerFilter::HasMoreLifeThanYou { base: inner }
        | PlayerFilter::MaxSpeed { base: inner, .. }
        | PlayerFilter::WasDealtDamageBySourceThisGame { base: inner }
        | PlayerFilter::LostLifeThisTurn { base: inner } => {
            player_filter_references_tag(inner, tag)
        }
        PlayerFilter::WasDealtCombatDamageByDistinctSourcesThisTurn { base, sources, .. } => {
            player_filter_references_tag(base, tag) || filter_references_tag(sources, tag)
        }
        PlayerFilter::Excluding { base, excluded } => {
            player_filter_references_tag(base, tag) || player_filter_references_tag(excluded, tag)
        }
        PlayerFilter::ControllerOf(reference)
        | PlayerFilter::OwnerOf(reference)
        | PlayerFilter::AliasedOwnerOf(reference)
        | PlayerFilter::AliasedControllerOf(reference) => object_ref_references_tag(reference, tag),
        _ => false,
    }
}

pub fn target_references_tag(target: &TargetAst, tag: &str) -> bool {
    match target {
        TargetAst::Tagged(found, _) => found.as_str() == tag,
        TargetAst::Object(filter, _, _) => filter_references_tag(filter, tag),
        TargetAst::ObjectOrPlayer(object_filter, player_filter, _) => {
            filter_references_tag(object_filter, tag)
                || player_filter_references_tag(player_filter, tag)
        }
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

pub fn effects_reference_it_tag(effects: &[EffectAst]) -> bool {
    effects.iter().any(effect_references_it_tag)
}

pub fn effects_reference_its_controller(effects: &[EffectAst]) -> bool {
    effects.iter().any(effect_references_its_controller)
}

pub fn value_references_event_derived_amount(value: &Value) -> bool {
    match value {
        Value::EventValue(EventValueSpec::Amount)
        | Value::EventValue(EventValueSpec::LifeAmount)
        | Value::EventValueOffset(EventValueSpec::Amount, _)
        | Value::EventValueOffset(EventValueSpec::LifeAmount, _) => true,
        Value::PendingEffectMetric { .. }
        | Value::PendingEffectMetricOffset { .. }
        | Value::PendingPriorEffectMetric(_) => true,
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
            .attached_to_object
            .as_deref()
            .is_some_and(filter_references_event_derived_amount)
        || filter
            .blocked_or_was_blocked_by_this_turn
            .as_deref()
            .is_some_and(filter_references_event_derived_amount)
        || filter
            .any_of
            .iter()
            .any(filter_references_event_derived_amount)
}

fn target_references_event_derived_amount(target: &TargetAst) -> bool {
    match target {
        TargetAst::Object(filter, _, _) => filter_references_event_derived_amount(filter),
        TargetAst::WithCount(inner, _) => target_references_event_derived_amount(inner),
        TargetAst::WithCountValue(inner, _, value) => {
            target_references_event_derived_amount(inner)
                || value_references_event_derived_amount(value)
        }
        _ => false,
    }
}

fn subject_verb_action_value(action: &SubjectVerbActionAst) -> Option<&Value> {
    match action {
        SubjectVerbActionAst::LifeResources(LifeResourceActionAst::Draw { count })
        | SubjectVerbActionAst::Library(LibraryActionAst::Mill { count })
        | SubjectVerbActionAst::Library(LibraryActionAst::ExileTopOfLibrary { count, .. })
        | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Scry { count })
        | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Surveil { count })
        | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Proliferate { count })
        | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Investigate { count })
        | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Discover { count })
        | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Fateseal { count })
        | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Populate { count, .. })
        | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Connive { count, .. })
        | SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenCopy { count, .. })
        | SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenCopyFromSource { count, .. })
        | SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods { count, .. }) => Some(count),
        SubjectVerbActionAst::KeywordActions(KeywordActionAst::Incubate { amount, .. }) => Some(amount),
        SubjectVerbActionAst::KeywordActions(KeywordActionAst::Monstrosity { amount }) => Some(amount),
        SubjectVerbActionAst::LifeResources(LifeResourceActionAst::LoseLife { amount })
        | SubjectVerbActionAst::LifeResources(LifeResourceActionAst::PayLife { amount })
        | SubjectVerbActionAst::LifeResources(LifeResourceActionAst::GainLife { amount })
        | SubjectVerbActionAst::Damage(DamageActionAst::DealDamage { amount, .. })
        | SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEqualToPower { amount, .. })
        | SubjectVerbActionAst::Damage(DamageActionAst::DealDistributedDamage { amount, .. })
        | SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEach { amount, .. })
        | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventDamage { amount, .. })
        | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventDamageEach { amount, .. })
        | SubjectVerbActionAst::Stack(StackActionAst::CopySpell { count: amount, .. })
        | SubjectVerbActionAst::Counters(CounterActionAst::PutCounters { count: amount, .. })
        | SubjectVerbActionAst::Counters(CounterActionAst::PutCounterChoice { count: amount, .. })
        | SubjectVerbActionAst::Counters(CounterActionAst::PutCountersAll { count: amount, .. })
        | SubjectVerbActionAst::Counters(CounterActionAst::RemoveUpToAnyCounters { amount, .. })
        | SubjectVerbActionAst::Counters(CounterActionAst::RemoveCountersAll { amount, .. })
        | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Discard { count: amount, .. })
        | SubjectVerbActionAst::Counters(CounterActionAst::PoisonCounters { count: amount })
        | SubjectVerbActionAst::Counters(CounterActionAst::EnergyCounters { count: amount })
        | SubjectVerbActionAst::Counters(CounterActionAst::ExperienceCounters { count: amount })
        | SubjectVerbActionAst::Counters(CounterActionAst::TicketCounters { count: amount })
        | SubjectVerbActionAst::LifeResources(LifeResourceActionAst::PayEnergy { amount })
        | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetLifeTotal { amount })
        | SubjectVerbActionAst::Mana(ManaActionAst::AddManaScaled { amount, .. })
        | SubjectVerbActionAst::Mana(ManaActionAst::AddManaAnyColor { amount, .. })
        | SubjectVerbActionAst::Mana(ManaActionAst::AddManaAnyOneColor { amount })
        | SubjectVerbActionAst::Mana(ManaActionAst::AddManaChosenColor { amount, .. })
        | SubjectVerbActionAst::Mana(ManaActionAst::AddManaFromLandCouldProduce { amount, .. })
        | SubjectVerbActionAst::Mana(ManaActionAst::AddManaCommanderIdentity { amount })
        | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::RedirectNextDamageFromSourceToTarget { amount, .. })
        | SubjectVerbActionAst::RevealLook(RevealLookActionAst::LookAtTopCards { count: amount, .. })
        | SubjectVerbActionAst::Library(LibraryActionAst::MoveToLibraryNthFromTop {
            position: amount, ..
        })
        | SubjectVerbActionAst::TurnStructure(TurnStructureActionAst::AdditionalLandPlays { count: amount, .. })
        | SubjectVerbActionAst::Damage(DamageActionAst::HealDamage {
            amount: Some(amount),
            ..
        }) => Some(amount),
        SubjectVerbActionAst::Mana(ManaActionAst::PayMana {
            x_value, x_maximum, ..
        }) => x_value.as_ref().or(x_maximum.as_ref()),
        SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::SearchLibrary {
            count_value: Some(value),
            ..
        }) => Some(value),
        SubjectVerbActionAst::LifeResources(LifeResourceActionAst::DrawForEachTaggedMatching { .. })
        | SubjectVerbActionAst::RevealLook(RevealLookActionAst::RevealHand)
        | SubjectVerbActionAst::RevealLook(RevealLookActionAst::RevealTop)
        | SubjectVerbActionAst::RevealLook(RevealLookActionAst::RevealTagged { .. })
        | SubjectVerbActionAst::RevealLook(RevealLookActionAst::RevealCardsFromHand { .. })
        | SubjectVerbActionAst::RevealLook(RevealLookActionAst::LookAtObjects { .. })
        | SubjectVerbActionAst::RevealLook(RevealLookActionAst::LookAtTarget { .. })
        | SubjectVerbActionAst::KeywordActions(KeywordActionAst::EmitKeywordAction { .. })
        | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Amass { .. })
        | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Bolster { .. })
        | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Support { .. })
        | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Adapt { .. })
        | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Explore { .. })
        | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Endure { .. })
        | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Exploit)
        | SubjectVerbActionAst::KeywordActions(KeywordActionAst::ConniveIterated)
        | SubjectVerbActionAst::KeywordActions(KeywordActionAst::OpenAttraction { .. })
        | SubjectVerbActionAst::Library(LibraryActionAst::ManifestTopCardOfLibrary)
        | SubjectVerbActionAst::Library(LibraryActionAst::CloakTopCardOfLibrary)
        | SubjectVerbActionAst::KeywordActions(KeywordActionAst::ManifestCardFromHand)
        | SubjectVerbActionAst::KeywordActions(KeywordActionAst::ManifestDread)
        | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Earthbend { .. })
        | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Behold { .. })
        | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Fight { .. })
        | SubjectVerbActionAst::KeywordActions(KeywordActionAst::FightIterated { .. })
        | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Clash { .. })
        | SubjectVerbActionAst::Random(RandomActionAst::FlipCoin)
        | SubjectVerbActionAst::Random(RandomActionAst::FlipCoinFaceOnly)
        | SubjectVerbActionAst::Random(RandomActionAst::RollDie { .. })
        | SubjectVerbActionAst::Random(RandomActionAst::RollDiceChooseResult { .. })
        | SubjectVerbActionAst::Library(LibraryActionAst::ShuffleHandAndGraveyardIntoLibrary)
        | SubjectVerbActionAst::Library(LibraryActionAst::ShuffleHandGraveyardAndOwnedPermanentsIntoLibrary)
        | SubjectVerbActionAst::Library(LibraryActionAst::ShuffleGraveyardIntoLibrary { .. })
        | SubjectVerbActionAst::Library(LibraryActionAst::ReorderGraveyard)
        | SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseColor)
        | SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseCardType { .. })
        | SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseNamedOption { .. })
        | SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseCreatureType { .. })
        | SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseLandType { .. })
        | SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseCardName { .. })
        | SubjectVerbActionAst::Choices(ChoiceActionAst::ChoosePlayer { .. })
        | SubjectVerbActionAst::LifeResources(LifeResourceActionAst::NoteLifeTotal)
        | SubjectVerbActionAst::Mana(ManaActionAst::AddMana { .. })
        | SubjectVerbActionAst::Exchanges(ExchangeActionAst::ExchangeLifeTotals { .. })
        | SubjectVerbActionAst::Exchanges(ExchangeActionAst::ExchangeTextBoxes { .. })
        | SubjectVerbActionAst::Exchanges(ExchangeActionAst::ExchangeZones { .. })
        | SubjectVerbActionAst::Library(LibraryActionAst::PutRestOnBottomOfLibrary)
        | SubjectVerbActionAst::Mana(ManaActionAst::DontLoseThisManaAsStepsAndPhasesEndThisTurn)
        | SubjectVerbActionAst::Exchanges(ExchangeActionAst::ExchangeValues { .. })
        | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileInsteadOfGraveyardThisTurn)
        | SubjectVerbActionAst::Control(ControlActionAst::ControlCombatChoicesThisTurn { .. })
        | SubjectVerbActionAst::Control(ControlActionAst::GainControl { .. })
        | SubjectVerbActionAst::PutSticker { .. }
        | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::SwitchPowerToughness { .. })
        | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::ScalePowerToughnessAll { .. })
        | SubjectVerbActionAst::Stack(StackActionAst::ScaleXValue { .. })
        | SubjectVerbActionAst::Mana(ManaActionAst::AddManaColorsAmong { .. })
        | SubjectVerbActionAst::Mana(ManaActionAst::AddOneManaAnyColorAmong { .. })
        | SubjectVerbActionAst::Mana(ManaActionAst::AddManaImprintedColors)
        | SubjectVerbActionAst::Mana(ManaActionAst::DoubleManaPool)
        | SubjectVerbActionAst::Mana(ManaActionAst::EmptyManaPool)
        | SubjectVerbActionAst::Game(GameActionAst::EndTurn)
        | SubjectVerbActionAst::Game(GameActionAst::EndCombatPhase)
        | SubjectVerbActionAst::TurnStructure(TurnStructureActionAst::SkipTurn)
        | SubjectVerbActionAst::TurnStructure(TurnStructureActionAst::SkipCombatPhases)
        | SubjectVerbActionAst::TurnStructure(TurnStructureActionAst::SkipNextCombatPhaseThisTurn)
        | SubjectVerbActionAst::TurnStructure(TurnStructureActionAst::SkipMainPhasesThisTurn)
        | SubjectVerbActionAst::TurnStructure(TurnStructureActionAst::SkipCombatPhasesThisTurn)
        | SubjectVerbActionAst::TurnStructure(TurnStructureActionAst::SkipDrawStep)
        | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::PlayFromGraveyardUntilEot)
        | SubjectVerbActionAst::Control(ControlActionAst::ControlPlayer { .. })
        | SubjectVerbActionAst::Stack(StackActionAst::ReduceNextSpellCostThisTurn { .. })
        | SubjectVerbActionAst::Stack(StackActionAst::ReduceMatchingSpellCostThisTurn { .. })
        | SubjectVerbActionAst::Grants(GrantActionAst::GrantNextSpellAbilityThisTurn { .. })
        | SubjectVerbActionAst::KeywordActions(KeywordActionAst::RingTemptsYou)
        | SubjectVerbActionAst::KeywordActions(KeywordActionAst::VentureIntoDungeon { .. })
        | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeMonarch)
        | SubjectVerbActionAst::KeywordActions(KeywordActionAst::TakeInitiative)
        | SubjectVerbActionAst::Tokens(TokenActionAst::CreateEmblem { .. })
        | SubjectVerbActionAst::Game(GameActionAst::LoseGame)
        | SubjectVerbActionAst::Game(GameActionAst::WinGame)
        | SubjectVerbActionAst::ReorderTopPlanarDeck { .. }
        | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnSourceTransformedFromExile)
        | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Reconfigure { .. })
        | SubjectVerbActionAst::KeywordActions(KeywordActionAst::CumulativeUpkeep { .. })
        | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Casualty { .. })
        | SubjectVerbActionAst::LifeResources(LifeResourceActionAst::PayAnyEnergy { .. })
        | SubjectVerbActionAst::LifeResources(LifeResourceActionAst::PayAnyLife { .. })
        | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DiscardHand)
        | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Detain { .. })
        | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Goad { .. })
        | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Suspect { .. })
        | SubjectVerbActionAst::KeywordActions(KeywordActionAst::ClearSuspected { .. })
        | SubjectVerbActionAst::Damage(DamageActionAst::HealDamage { amount: None, .. })
        | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::RemoveFromCombat { .. })
        | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Flip { .. })
        | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Regenerate { .. })
        | SubjectVerbActionAst::KeywordActions(KeywordActionAst::RegenerateAll { .. })
        | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::TapAll { .. })
        | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::UntapAll { .. })
        | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::TapOrUntap { .. })
        | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::TapOrUntapAll { .. })
        | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::PhaseOut { .. })
        | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::PhaseOutAll { .. })
        | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::PhaseIn { .. })
        | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::PhaseInAll { .. })
        | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Transform { .. })
        | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Convert { .. })
        | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Tap { .. })
        | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::Untap { .. })
        | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Destroy { .. })
        | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DestroyAll { .. })
        | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DestroyAllOfChosenColor { .. })
        | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Exile { .. })
        | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileAll { .. })
        | SubjectVerbActionAst::RevealLook(RevealLookActionAst::LookAtHand { .. })
        | SubjectVerbActionAst::Stack(StackActionAst::Counter { .. })
        | SubjectVerbActionAst::Stack(StackActionAst::CounterUnlessPays { .. })
        | SubjectVerbActionAst::Counters(CounterActionAst::MoveAllCounters { .. })
        | SubjectVerbActionAst::Counters(CounterActionAst::MoveOneCounter { .. })
        | SubjectVerbActionAst::Counters(CounterActionAst::ForEachCounterKindPutOrRemove { .. })
        | SubjectVerbActionAst::Counters(CounterActionAst::PutCounterOfChosenKind { .. })
        | SubjectVerbActionAst::Counters(CounterActionAst::DoubleCountersOnTarget { .. })
        | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToHand { .. })
        | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnAllToHand { .. })
        | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnAllToHandOfChosenColor { .. })
        | SubjectVerbActionAst::Counters(CounterActionAst::DoubleCountersOnEach { .. })
        | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Sacrifice { .. })
        | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::SacrificeAll { .. })
        | SubjectVerbActionAst::Game(GameActionAst::ExtraTurnAfterTurn { .. })
        | SubjectVerbActionAst::Library(LibraryActionAst::ReorderTopOfLibrary { .. })
        | SubjectVerbActionAst::Library(LibraryActionAst::ShuffleObjectsIntoLibrary { .. })
        | SubjectVerbActionAst::Grants(GrantActionAst::GrantProtectionChoice { .. })
        | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventAllCombatDamage { .. })
        | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::AssignNoCombatDamage { .. })
        | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventAllCombatDamageFromSource { .. })
        | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventAllCombatDamageFromSourceFilter { .. })
        | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventAllCombatDamageToPlayers { .. })
        | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventAllCombatDamageToYou { .. })
        | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventNextTimeDamage { .. })
        | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::ReplaceNextDamageToTarget { .. })
        | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::RedirectNextTimeDamageToSource { .. })
        | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::RedirectAllDamageThisTurnBySourceToSourceController { .. })
        | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::RedirectAllDamageThisTurnToTarget { .. })
        | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventAllDamageToTarget { .. })
        | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventAllDamageToTargetFromSourceFilter { .. })
        | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventAllDamageFromSourceFilter { .. })
        | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventDamageToTargetPutCounters { .. })
        | SubjectVerbActionAst::Counters(CounterActionAst::PutOrRemoveCounters { .. })
        | SubjectVerbActionAst::Stack(StackActionAst::CopySpellForEachTarget { .. })
        | SubjectVerbActionAst::Library(LibraryActionAst::PutTaggedRemainderOnBottomOfLibrary { .. })
        | SubjectVerbActionAst::Library(LibraryActionAst::PutTaggedRemainderInZone { .. })
        | SubjectVerbActionAst::Stack(StackActionAst::CastTagged { .. })
        | SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedUntilEndOfTurn { .. })
        | SubjectVerbActionAst::Grants(GrantActionAst::GrantTaggedSpellAlternativeCostPayLifeByManaValueUntilEndOfTurn {
            ..
        })
        | SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedUntilYourNextTurn { .. })
        | SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedForAsLongAsExiled { .. })
        | SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedForAsLongAsYouControlSource { .. })
        | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToBattlefield { .. })
        | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnAllToBattlefield { .. })
        | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileUntilSourceLeaves { .. })
        | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MoveToZone { .. })
        | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::PutOntoBattlefield { .. })
        | SubjectVerbActionAst::Library(LibraryActionAst::MoveToLibraryTopOrBottomChoice { .. })
        | SubjectVerbActionAst::TargetOnly { .. }
        | SubjectVerbActionAst::TagMatchingObjects { .. }
        | SubjectVerbActionAst::StatChanges(StatChangeActionAst::Pump { .. })
        | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetBasePowerToughness { .. })
        | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeBasePtCreature { .. })
        | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetBasePower { .. })
        | SubjectVerbActionAst::StatChanges(StatChangeActionAst::PumpForEach { .. })
        | SubjectVerbActionAst::StatChanges(StatChangeActionAst::PumpAll { .. })
        | SubjectVerbActionAst::StatChanges(StatChangeActionAst::PumpByLastEffect { .. })
        | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::AddCardTypes { .. })
        | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetCardTypes { .. })
        | SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveCardTypes { .. })
        | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::AddSubtypes { .. })
        | SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveSubtypes { .. })
        | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetCreatureSubtypes { .. })
        | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeSaddledUntilEndOfTurn { .. })
        | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::AddColors { .. })
        | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::AddAllSubtypesOfFamily { .. })
        | SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveAllSubtypesOfFamily { .. })
        | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeAuraEnchantment { .. })
        | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeBasicLandType { .. })
        | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetColors { .. })
        | SubjectVerbActionAst::StatChanges(StatChangeActionAst::MakeColorless { .. })
        | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeBasicLandTypeChoice { .. })
        | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeCreatureTypeChoice { .. })
        | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeColorChoice { .. })
        | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeCopy { .. })
        | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesAll { .. })
        | SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveAbilitiesAll { .. })
        | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesChoiceAll { .. })
        | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesToTarget { .. })
        | SubjectVerbActionAst::Grants(GrantActionAst::GrantToTarget { .. })
        | SubjectVerbActionAst::Grants(GrantActionAst::GrantBySpec { .. })
        | SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveAbilitiesFromTarget { .. })
        | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesChoiceToTarget { .. })
        | SubjectVerbActionAst::Library(LibraryActionAst::ConsultTopOfLibrary { .. })
        | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::SearchLibrary { .. })
        | SubjectVerbActionAst::Cant { .. }
        | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Meld { .. })
        | SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenChoice { .. })
        | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::SearchLibrarySlotsToHand { .. })
        | SubjectVerbActionAst::Stack(StackActionAst::RetargetStackObject { .. })
        | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilityToSource { .. })
        | SubjectVerbActionAst::Exchanges(ExchangeActionAst::ExchangeControl { .. })
        | SubjectVerbActionAst::Exchanges(ExchangeActionAst::ExchangeControlHeterogeneous { .. })
        | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DestroyAllAttachedTo { .. })
        | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileAllAttachedTo { .. })
        | SubjectVerbActionAst::Control(ControlActionAst::Attach { .. })
        | SubjectVerbActionAst::Control(ControlActionAst::Unattach { .. })
        | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileWhenSourceLeaves { .. })
        | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::SacrificeSourceWhenLeaves { .. })
        | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::MayMoveToZone { .. })
        | SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterZoneReplacement { .. })
        | SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterFutureZoneReplacement { .. })
        | SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterDrawReplacement { .. })
        | SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterManaReplacement { .. })
        | SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterDamagedBySourceZoneReplacement { .. })
        | SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterEnterUnderControlReplacement { .. })
        | SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterEnterTappedReplacement { .. })
        | SubjectVerbActionAst::Replacements(ReplacementActionAst::RegisterNextBatchEnterWithCounters { .. })
        | SubjectVerbActionAst::Control(ControlActionAst::Enchant { .. })
        | SubjectVerbActionAst::Choices(ChoiceActionAst::ChooseSpellCastHistory { .. })
        | SubjectVerbActionAst::TurnStructure(TurnStructureActionAst::AdditionalPhases { .. })
        | SubjectVerbActionAst::KeywordActions(KeywordActionAst::Learn)
        | SubjectVerbActionAst::KeywordActions(KeywordActionAst::UnlockRoomDoor)
        | SubjectVerbActionAst::Game(GameActionAst::ReverseTurnOrder)
        | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::TurnFaceUp { .. })
        | SubjectVerbActionAst::Library(LibraryActionAst::ShuffleLibrary) => None,
    }
}

pub fn effect_references_event_derived_amount(effect: &EffectAst) -> bool {
    assert_effect_ast_variant_coverage(effect);
    let mut target_references = false;
    with_direct_effect_targets(effect, |target| {
        target_references |= target_references_event_derived_amount(target);
    });
    if target_references {
        return true;
    }
    match effect {
        EffectAst::ForEach(ForEachEffectAst::RepeatEffects { count, effects }) => {
            value_references_event_derived_amount(count)
                || effects.iter().any(effect_references_event_derived_amount)
        }
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
            count_value: Some(count_value),
            ..
        })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsBottomOfLibrary {
            count_value: Some(count_value),
            ..
        })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsTopOfLibrary {
            count_value: Some(count_value),
            ..
        })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsAcrossZones {
            count_value: Some(count_value),
            ..
        }) => value_references_event_derived_amount(count_value),
        EffectAst::SubjectVerb(subject_verb) => {
            subject_verb_action_value(&subject_verb.action)
                .is_some_and(value_references_event_derived_amount)
                || match &subject_verb.action {
                    SubjectVerbActionAst::Stack(StackActionAst::CounterUnlessPays { cost, .. }) => {
                        total_cost_values_any(cost, value_references_event_derived_amount)
                    }
                    SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventDamageToTargetPutCounters {
                        amount: Some(amount),
                        ..
                    }) => value_references_event_derived_amount(amount),
                    SubjectVerbActionAst::Counters(CounterActionAst::PutOrRemoveCounters {
                        put_count,
                        remove_count,
                        ..
                    }) => {
                        value_references_event_derived_amount(put_count)
                            || value_references_event_derived_amount(remove_count)
                    }
                    SubjectVerbActionAst::StatChanges(StatChangeActionAst::Pump {
                        power, toughness, ..
                    })
                    | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetBasePowerToughness {
                        power, toughness, ..
                    })
                    | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeBasePtCreature {
                        power, toughness, ..
                    })
                    | SubjectVerbActionAst::StatChanges(StatChangeActionAst::PumpAll {
                        power, toughness, ..
                    }) => {
                        value_references_event_derived_amount(power)
                            || value_references_event_derived_amount(toughness)
                    }
                    SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetBasePower { power, .. }) => {
                        value_references_event_derived_amount(power)
                    }
                    SubjectVerbActionAst::StatChanges(StatChangeActionAst::PumpForEach { count, .. }) => {
                        value_references_event_derived_amount(count)
                    }
                    SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToBattlefield {
                        count_value: Some(count_value),
                        ..
                    }) => value_references_event_derived_amount(count_value),
                    SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DestroyAll { filter, .. })
                    | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DestroyAllOfChosenColor { filter, .. })
                    | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileAll { filter, .. })
                    | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnAllToHand { filter, .. })
                    | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnAllToHandOfChosenColor { filter })
                    | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::TapAll { filter })
                    | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::UntapAll { filter })
                    | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::PhaseOutAll { filter, .. })
                    | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::PhaseInAll { filter })
                    | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::ScalePowerToughnessAll { filter, .. })
                    | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::SacrificeAll { filter })
                    | SubjectVerbActionAst::KeywordActions(KeywordActionAst::RegenerateAll { filter })
                    | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnAllToBattlefield { filter, .. })
                    | SubjectVerbActionAst::TagMatchingObjects { filter, .. }
                    | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesAll { filter, .. })
                    | SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveAbilitiesAll { filter, .. }) => {
                        filter_references_event_derived_amount(filter)
                    }
                    SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods {
                        dynamic_power_toughness: Some((power, toughness)),
                        ..
                    }) => {
                        value_references_event_derived_amount(power)
                            || value_references_event_derived_amount(toughness)
                    }
                    SubjectVerbActionAst::Library(LibraryActionAst::ConsultTopOfLibrary {
                        stop_rule,
                        max_exposed,
                        ..
                    }) => {
                        matches!(
                            stop_rule,
                            crate::cards::builders::LibraryConsultStopRuleAst::MatchCount(value)
                                if value_references_event_derived_amount(value)
                        ) || max_exposed
                            .as_ref()
                            .is_some_and(value_references_event_derived_amount)
                    }
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

pub fn effect_references_its_controller(effect: &EffectAst) -> bool {
    assert_effect_ast_variant_coverage(effect);
    match effect {
        EffectAst::SubjectVerb(subject_verb) => {
            matches!(
                subject_verb.subject.player,
                PlayerAst::ItsController | PlayerAst::ItsOwner
            ) || matches!(
                &subject_verb.action,
                SubjectVerbActionAst::Exchanges(ExchangeActionAst::ExchangeLifeTotals {
                    player2: PlayerAst::ItsController | PlayerAst::ItsOwner
                }) | SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenCopy {
                    player: PlayerAst::ItsController | PlayerAst::ItsOwner,
                    ..
                }) | SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenCopyFromSource {
                    player: PlayerAst::ItsController | PlayerAst::ItsOwner,
                    ..
                }) | SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods {
                    player: PlayerAst::ItsController | PlayerAst::ItsOwner,
                    ..
                })
            )
        }
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects { player, .. })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsWithAggregateConstraint { player, .. })
        | EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjectsAcrossZones { player, .. }) => {
            matches!(player, PlayerAst::ItsController | PlayerAst::ItsOwner)
        }
        EffectAst::Permissions(PermissionEffectAst::MayCastMatchingSpellWithoutPayingManaCost {
            player, zone_owner, ..
        }) => {
            matches!(player, PlayerAst::ItsController | PlayerAst::ItsOwner)
                || matches!(zone_owner, PlayerAst::ItsController | PlayerAst::ItsOwner)
        }
        EffectAst::Permissions(PermissionEffectAst::MayByPlayer { player, effects }) => {
            matches!(player, PlayerAst::ItsController | PlayerAst::ItsOwner)
                || effects_reference_its_controller(effects)
        }
        EffectAst::Conditionals(ConditionalEffectAst::UnlessPays {
            effects, player, ..
        }) => {
            matches!(player, PlayerAst::ItsController | PlayerAst::ItsOwner)
                || effects_reference_its_controller(effects)
        }
        EffectAst::Conditionals(ConditionalEffectAst::UnlessAction {
            effects,
            alternative,
            player,
            ..
        }) => {
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

pub fn effect_references_it_tag(effect: &EffectAst) -> bool {
    assert_effect_ast_variant_coverage(effect);
    if direct_effect_targets_reference_tag(effect, crate::tag::CompilerReferenceTag::It.as_str()) {
        return true;
    }

    match effect {
        EffectAst::SubjectVerb(subject_verb) => match &subject_verb.action {
            SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEach { amount, filter }) => {
                value_references_tag(amount, crate::tag::CompilerReferenceTag::It.as_str()) || filter_references_tag(filter, crate::tag::CompilerReferenceTag::It.as_str())
            }
            SubjectVerbActionAst::Stack(StackActionAst::CounterUnlessPays { cost, .. }) => {
                total_cost_values_any(cost, |value| value_references_tag(value, crate::tag::CompilerReferenceTag::It.as_str()))
            }
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Discard { count, filter, .. }) => {
                value_references_tag(count, crate::tag::CompilerReferenceTag::It.as_str())
                    || filter
                        .as_ref()
                        .is_some_and(|filter| filter_references_tag(filter, crate::tag::CompilerReferenceTag::It.as_str()))
            }
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Sacrifice { filter, target, .. }) => {
                filter_references_tag(filter, crate::tag::CompilerReferenceTag::It.as_str())
                    || target
                        .as_ref()
                        .is_some_and(|target| target_references_tag(target, crate::tag::CompilerReferenceTag::It.as_str()))
            }
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::SacrificeAll { filter }) => filter_references_tag(filter, crate::tag::CompilerReferenceTag::It.as_str()),
            SubjectVerbActionAst::PutSticker { target, .. }
            | SubjectVerbActionAst::Library(LibraryActionAst::MoveToLibraryNthFromTop { target, .. })
            | SubjectVerbActionAst::Library(LibraryActionAst::MoveToLibraryTopOrBottomChoice { target })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::SwitchPowerToughness { target, .. }) => {
                target_references_tag(target, crate::tag::CompilerReferenceTag::It.as_str())
            }
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::Regenerate {
                follow_up_effects, ..
            }) => effects_reference_it_tag(follow_up_effects),
            SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DestroyAll { filter, .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::DestroyAllOfChosenColor { filter, .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileAll { filter, .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnAllToHand { filter, .. })
            | SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnAllToHandOfChosenColor { filter })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::TapAll { filter })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::UntapAll { filter })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::PhaseOutAll { filter, .. })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::PhaseInAll { filter })
            | SubjectVerbActionAst::PermanentState(PermanentStateActionAst::ScalePowerToughnessAll { filter, .. })
            | SubjectVerbActionAst::KeywordActions(KeywordActionAst::RegenerateAll { filter }) => {
                filter_references_tag(filter, crate::tag::CompilerReferenceTag::It.as_str())
            }
            SubjectVerbActionAst::PermanentState(PermanentStateActionAst::TapOrUntapAll {
                tap_filter,
                untap_filter,
            }) => {
                filter_references_tag(tap_filter, crate::tag::CompilerReferenceTag::It.as_str())
                    || filter_references_tag(untap_filter, crate::tag::CompilerReferenceTag::It.as_str())
            }
            SubjectVerbActionAst::Library(LibraryActionAst::ExileTopOfLibrary {
                count,
                tags,
                accumulated_tags,
                ..
            }) => {
                value_references_tag(count, crate::tag::CompilerReferenceTag::It.as_str())
                    || tags.iter().any(|tag| tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str())
                    || accumulated_tags.iter().any(|tag| tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str())
            }
            SubjectVerbActionAst::LifeResources(LifeResourceActionAst::DrawForEachTaggedMatching { tag, filter }) => {
                tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str() || filter_references_tag(filter, crate::tag::CompilerReferenceTag::It.as_str())
            }
            SubjectVerbActionAst::Counters(CounterActionAst::PutCountersAll { count, filter, .. })
            | SubjectVerbActionAst::Counters(CounterActionAst::RemoveCountersAll {
                amount: count,
                filter,
                ..
            }) => value_references_tag(count, crate::tag::CompilerReferenceTag::It.as_str()) || filter_references_tag(filter, crate::tag::CompilerReferenceTag::It.as_str()),
            SubjectVerbActionAst::Library(LibraryActionAst::ReorderTopOfLibrary { tag }) => tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str(),
            SubjectVerbActionAst::Stack(StackActionAst::ReduceNextSpellCostThisTurn { filter, .. })
            | SubjectVerbActionAst::Stack(StackActionAst::ReduceMatchingSpellCostThisTurn { filter, .. }) => {
                filter_references_tag(filter, crate::tag::CompilerReferenceTag::It.as_str())
            }
            SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventDamageToTargetPutCounters {
                amount: Some(amount),
                ..
            }) => value_references_tag(amount, crate::tag::CompilerReferenceTag::It.as_str()),
            SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventDamageEach { amount, filter, .. }) => {
                value_references_tag(amount, crate::tag::CompilerReferenceTag::It.as_str()) || filter_references_tag(filter, crate::tag::CompilerReferenceTag::It.as_str())
            }
            SubjectVerbActionAst::Counters(CounterActionAst::PutOrRemoveCounters {
                put_count,
                remove_count,
                ..
            }) => value_references_tag(put_count, crate::tag::CompilerReferenceTag::It.as_str()) || value_references_tag(remove_count, crate::tag::CompilerReferenceTag::It.as_str()),
            SubjectVerbActionAst::Counters(CounterActionAst::PutCounterChoice { count, .. }) => {
                value_references_tag(count, crate::tag::CompilerReferenceTag::It.as_str())
            }
            SubjectVerbActionAst::StatChanges(StatChangeActionAst::Pump {
                power, toughness, ..
            })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetBasePowerToughness {
                power, toughness, ..
            })
            | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeBasePtCreature {
                power, toughness, ..
            })
            | SubjectVerbActionAst::StatChanges(StatChangeActionAst::PumpAll {
                power, toughness, ..
            }) => {
                value_references_tag(power, crate::tag::CompilerReferenceTag::It.as_str()) || value_references_tag(toughness, crate::tag::CompilerReferenceTag::It.as_str())
            }
            SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetBasePower { power, .. }) => {
                value_references_tag(power, crate::tag::CompilerReferenceTag::It.as_str())
            }
            SubjectVerbActionAst::StatChanges(StatChangeActionAst::PumpForEach { count, .. }) => {
                value_references_tag(count, crate::tag::CompilerReferenceTag::It.as_str())
            }
            SubjectVerbActionAst::Stack(StackActionAst::CopySpellForEachTarget { object_filter, .. }) => object_filter
                .as_ref()
                .is_some_and(|filter| filter_references_tag(filter, crate::tag::CompilerReferenceTag::It.as_str())),
            SubjectVerbActionAst::Damage(DamageActionAst::DealDamageEqualToPower {
                source,
                amount,
                target,
                ..
            }) => {
                target_references_tag(source, crate::tag::CompilerReferenceTag::It.as_str())
                    || value_references_tag(amount, crate::tag::CompilerReferenceTag::It.as_str())
                    || target_references_tag(target, crate::tag::CompilerReferenceTag::It.as_str())
            }
            SubjectVerbActionAst::Stack(StackActionAst::CastTagged { tag, .. }) => tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str(),
            SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedUntilEndOfTurn { tag, .. })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantTaggedSpellAlternativeCostPayLifeByManaValueUntilEndOfTurn {
                tag,
                ..
            })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedUntilYourNextTurn { tag, .. })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedForAsLongAsExiled { tag, .. })
            | SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedForAsLongAsYouControlSource { tag, .. }) => {
                tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
            }
            SubjectVerbActionAst::Library(LibraryActionAst::PutRestOnBottomOfLibrary) => true,
            SubjectVerbActionAst::Cant { restriction, .. } => {
                restriction_references_tag(restriction, crate::tag::CompilerReferenceTag::It.as_str())
            }
            SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenCopy { object, .. }) => {
                matches!(object, ObjectRefAst::Tagged(tag) if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str())
            }
            SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenWithMods {
                count,
                dynamic_power_toughness,
                attached_to,
                ..
            }) => {
                value_references_tag(count, crate::tag::CompilerReferenceTag::It.as_str())
                    || dynamic_power_toughness.as_ref().is_some_and(|(power, toughness)| {
                        value_references_tag(power, crate::tag::CompilerReferenceTag::It.as_str())
                            || value_references_tag(toughness, crate::tag::CompilerReferenceTag::It.as_str())
                    })
                    || attached_to
                        .as_ref()
                        .is_some_and(|target| target_references_tag(target, crate::tag::CompilerReferenceTag::It.as_str()))
            }
            action => subject_verb_action_value(action)
                .is_some_and(|value| value_references_tag(value, crate::tag::CompilerReferenceTag::It.as_str())),
        },
        EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        })
        | EffectAst::SelfReplacement {
            predicate,
            if_true,
            if_false,
            ..
        } => {
            predicate_uses_implicit_it_reference(predicate)
                || predicate_references_tag(predicate, crate::tag::CompilerReferenceTag::It.as_str())
                || effects_reference_it_tag(if_true)
                || effects_reference_it_tag(if_false)
        }
        EffectAst::Conditionals(ConditionalEffectAst::TrailingIf { predicate, effects })
        | EffectAst::Conditionals(ConditionalEffectAst::TrailingUnless { predicate, effects }) => {
            predicate_uses_implicit_it_reference(predicate)
                || predicate_references_tag(predicate, crate::tag::CompilerReferenceTag::It.as_str())
                || effects_reference_it_tag(effects)
        }
        EffectAst::ForEach(ForEachEffectAst::ForEachTagged { tag, effects }) => {
            tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str() || effects_reference_it_tag(effects)
        }
        EffectAst::ForEach(ForEachEffectAst::ForEachTaggedWithControllerAtLastBlockedBy {
            tag,
            blocker_tag,
            effects,
        }) => {
            tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
                || blocker_tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
                || effects_reference_it_tag(effects)
        }
        EffectAst::Delayed(DelayedEffectAst::DelayedWhenLastObjectDiesThisTurn { .. })
        | EffectAst::Delayed(DelayedEffectAst::DelayedWhenLastObjectLeavesBattlefield { .. }) => true,
        EffectAst::ForEach(ForEachEffectAst::ForEachObject { filter, effects }) => {
            filter_references_tag(filter, crate::tag::CompilerReferenceTag::It.as_str()) || effects_reference_it_tag(effects)
        }
        EffectAst::ControlFlow(control) => {
            let condition_references_it = |condition: &crate::model::ControlConditionAst| {
                matches!(
                    &condition.predicate,
                    crate::model::ControlPredicateAst::State(predicate)
                        if predicate_uses_implicit_it_reference(predicate)
                            || predicate_references_tag(predicate, crate::tag::CompilerReferenceTag::It.as_str())
                )
            };
            let node_references_it = match &control.node {
                crate::model::ControlFlowNodeAst::Condition { condition, .. } => {
                    condition_references_it(condition)
                }
                crate::model::ControlFlowNodeAst::Replacement(replacement) => replacement
                    .condition
                    .as_ref()
                    .is_some_and(condition_references_it),
                crate::model::ControlFlowNodeAst::Prevention(prevention) => prevention
                    .condition
                    .as_ref()
                    .is_some_and(condition_references_it),
                crate::model::ControlFlowNodeAst::Permission(_)
                | crate::model::ControlFlowNodeAst::Duration { .. }
                | crate::model::ControlFlowNodeAst::Delayed { .. }
                | crate::model::ControlFlowNodeAst::NestedAbility { .. } => false,
            };
            node_references_it
                || control
                    .programs
                    .iter()
                    .any(|program| effects_reference_it_tag(&program.effects))
        }
        _ => {
            if let Some(filter) = effect_tagged_filter(effect) {
                return filter_references_tag(filter, crate::tag::CompilerReferenceTag::It.as_str());
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
        | PredicateAst::ItMatchedLastKnown(_)
        | PredicateAst::TargetMatches(_) => true,
        PredicateAst::Not(inner) => predicate_uses_implicit_it_reference(inner),
        PredicateAst::And(left, right) | PredicateAst::Or(left, right) => {
            predicate_uses_implicit_it_reference(left)
                || predicate_uses_implicit_it_reference(right)
        }
        _ => false,
    }
}

pub fn restriction_references_tag(restriction: &crate::effect::Restriction, tag: &str) -> bool {
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
        | Restriction::PhaseIn(filter)
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

    if let Restriction::AttackPlayerOrPlaneswalkersControlledBy { attackers, .. }
    | Restriction::AttackPlayer { attackers, .. } = restriction
    {
        return attackers
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == tag);
    }

    false
}

pub fn collect_tag_spans_from_effects_with_context(
    effects: &[EffectAst],
    annotations: &mut ParseAnnotations,
    ctx: &SpanMappingContext<'_>,
) {
    for effect in effects {
        collect_tag_spans_from_effect(effect, annotations, ctx);
    }
}

fn collect_direct_effect_target_spans(
    effect: &EffectAst,
    annotations: &mut ParseAnnotations,
    ctx: &SpanMappingContext<'_>,
) -> bool {
    let mut collected = false;
    with_direct_effect_targets(effect, |target| {
        collect_tag_spans_from_target(target, annotations, ctx);
        collected = true;
    });
    collected
}

pub fn collect_tag_spans_from_effect(
    effect: &EffectAst,
    annotations: &mut ParseAnnotations,
    ctx: &SpanMappingContext<'_>,
) {
    assert_effect_ast_variant_coverage(effect);
    if collect_direct_effect_target_spans(effect, annotations, ctx) {
        return;
    }

    for_each_nested_effects(effect, true, |nested| {
        collect_tag_spans_from_effects_with_context(nested, annotations, ctx);
    });
}

pub fn collect_tag_spans_from_target(
    target: &TargetAst,
    annotations: &mut ParseAnnotations,
    ctx: &SpanMappingContext<'_>,
) {
    if let TargetAst::WithCount(inner, _) = target {
        collect_tag_spans_from_target(inner, annotations, ctx);
        return;
    }
    if let TargetAst::Tagged(tag, Some(span)) = target {
        let mapped = super::map_span_to_original(*span, ctx.normalized, ctx.original, ctx.char_map);
        #[cfg(not(feature = "serialization"))]
        annotations.record_tag_span(tag.as_str(), mapped);
        #[cfg(feature = "serialization")]
        annotations.record_tag_span(tag, mapped);
    }
    if let TargetAst::Object(filter, _, Some(it_span)) = target
        && filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
        })
    {
        let mapped =
            super::map_span_to_original(*it_span, ctx.normalized, ctx.original, ctx.char_map);
        #[cfg(not(feature = "serialization"))]
        annotations.record_tag_span(crate::tag::CompilerReferenceTag::It.as_str(), mapped);
        #[cfg(feature = "serialization")]
        {
            let it_tag = crate::tag::CompilerReferenceTag::It.bind();
            annotations.record_tag_span(&it_tag, mapped);
        }
    }
}
