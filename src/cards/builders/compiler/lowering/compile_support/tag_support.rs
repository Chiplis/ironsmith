use crate::cards::builders::{
    EffectAst, IT_TAG, ObjectRefAst, ParseAnnotations, PlayerAst, PredicateAst, RetargetModeAst,
    TagKey, TargetAst,
};
use crate::effect::{EventValueSpec, Value};
use crate::filter::{ObjectFilter, ObjectRef, PlayerFilter, TaggedOpbjectRelation};
use crate::target::ChooseSpec;

use super::{NormalizedLine, assert_effect_ast_variant_coverage, for_each_nested_effects};

pub(crate) fn effects_reference_tag(effects: &[EffectAst], tag: &str) -> bool {
    effects
        .iter()
        .any(|effect| effect_references_tag(effect, tag))
}

// Keep direct target-bearing variants in one place to prevent drift across
// tag-reference checks and tag-span collection.
macro_rules! direct_target_effect_variants {
    ($target:ident) => {
        EffectAst::DealDamage {
            target: $target,
            ..
        } | EffectAst::Counter { target: $target }
            | EffectAst::CounterUnlessPays {
                target: $target,
                ..
            }
            | EffectAst::Explore { target: $target }
            | EffectAst::Connive { target: $target }
            | EffectAst::Goad { target: $target }
            | EffectAst::PutCounters {
                target: $target,
                ..
            }
            | EffectAst::PutOrRemoveCounters {
                target: $target,
                ..
            }
            | EffectAst::ForEachCounterKindPutOrRemove { target: $target }
            | EffectAst::Tap { target: $target }
            | EffectAst::Untap { target: $target }
            | EffectAst::PhaseOut { target: $target }
            | EffectAst::RemoveFromCombat { target: $target }
            | EffectAst::TapOrUntap { target: $target }
            | EffectAst::Destroy { target: $target }
            | EffectAst::DestroyNoRegeneration { target: $target }
            | EffectAst::Exile {
                target: $target,
                ..
            }
            | EffectAst::ExileWhenSourceLeaves { target: $target }
            | EffectAst::SacrificeSourceWhenLeaves { target: $target }
            | EffectAst::ExileUntilSourceLeaves {
                target: $target,
                ..
            }
            | EffectAst::LookAtHand { target: $target }
            | EffectAst::Transform { target: $target }
            | EffectAst::Convert { target: $target }
            | EffectAst::Flip { target: $target }
            | EffectAst::Regenerate { target: $target }
            | EffectAst::TargetOnly { target: $target }
            | EffectAst::RemoveUpToAnyCounters {
                target: $target,
                ..
            }
            | EffectAst::ReturnToHand {
                target: $target,
                ..
            }
            | EffectAst::ReturnToBattlefield {
                target: $target,
                ..
            }
            | EffectAst::Pump {
                target: $target,
                ..
            }
            | EffectAst::BecomeBasicLandType {
                target: $target,
                ..
            }
            | EffectAst::BecomeBasicLandTypeChoice {
                target: $target,
                ..
            }
            | EffectAst::BecomeCreatureTypeChoice {
                target: $target,
                ..
            }
            | EffectAst::BecomeColorChoice {
                target: $target,
                ..
            }
            | EffectAst::SetBasePower {
                target: $target,
                ..
            }
            | EffectAst::SetBasePowerToughness {
                target: $target,
                ..
            }
            | EffectAst::BecomeBasePtCreature {
                target: $target,
                ..
            }
            | EffectAst::AddCardTypes {
                target: $target,
                ..
            }
            | EffectAst::RemoveCardTypes {
                target: $target,
                ..
            }
            | EffectAst::AddSubtypes {
                target: $target,
                ..
            }
            | EffectAst::SetColors {
                target: $target,
                ..
            }
            | EffectAst::MakeColorless {
                target: $target,
                ..
            }
            | EffectAst::PumpForEach {
                target: $target,
                ..
            }
            | EffectAst::PumpByLastEffect {
                target: $target,
                ..
            }
            | EffectAst::GrantAbilitiesToTarget {
                target: $target,
                ..
            }
            | EffectAst::GrantToTarget {
                target: $target,
                ..
            }
            | EffectAst::RemoveAbilitiesFromTarget {
                target: $target,
                ..
            }
            | EffectAst::GrantAbilitiesChoiceToTarget {
                target: $target,
                ..
            }
            | EffectAst::GrantProtectionChoice {
                target: $target,
                ..
            }
            | EffectAst::PreventDamage {
                target: $target,
                ..
            }
            | EffectAst::PreventAllDamageToTarget {
                target: $target,
                ..
            }
            | EffectAst::PreventDamageToTargetPutCounters {
                target: $target,
                ..
            }
            | EffectAst::RedirectNextDamageFromSourceToTarget {
                target: $target,
                ..
            }
            | EffectAst::RedirectNextTimeDamageToSource {
                target: $target,
                ..
            }
            | EffectAst::GainControl {
                target: $target,
                ..
            }
            | EffectAst::CopySpell {
                target: $target,
                ..
            }
            | EffectAst::MoveToLibraryNthFromTop {
                target: $target,
                ..
            }
            | EffectAst::ShuffleObjectsIntoLibrary {
                target: $target,
                ..
            }
            | EffectAst::CreateTokenCopyFromSource {
                source: $target,
                ..
            }
            | EffectAst::PreventAllCombatDamageFromSource {
                source: $target,
                ..
            }
    };
}

fn with_direct_effect_targets(effect: &EffectAst, mut visit: impl FnMut(&TargetAst)) {
    assert_effect_ast_variant_coverage(effect);
    match effect {
        EffectAst::Fight {
            creature1,
            creature2,
        } => {
            visit(creature1);
            visit(creature2);
        }
        EffectAst::FightIterated { creature2 } => {
            visit(creature2);
        }
        EffectAst::DealDamageEqualToPower { source, target } => {
            visit(source);
            visit(target);
        }
        EffectAst::BecomeCopy { target, source, .. } => {
            visit(target);
            visit(source);
        }
        direct_target_effect_variants!(target) => {
            visit(target);
        }
        EffectAst::Sacrifice {
            target: Some(target),
            ..
        } => {
            visit(target);
        }
        EffectAst::MoveToZone {
            target,
            attached_to,
            ..
        } => {
            visit(target);
            if let Some(attach_target) = attached_to {
                visit(attach_target);
            }
        }
        EffectAst::MoveAllCounters { from, to } => {
            visit(from);
            visit(to);
        }
        EffectAst::DestroyAllAttachedTo { target, .. } => {
            visit(target);
        }
        EffectAst::Attach { object, target } => {
            visit(object);
            visit(target);
        }
        EffectAst::RetargetStackObject { target, mode, .. } => {
            visit(target);
            if let RetargetModeAst::OneToFixed { target: fixed } = mode {
                visit(fixed);
            }
        }
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

fn filter_references_tag(filter: &ObjectFilter, tag: &str) -> bool {
    filter
        .tagged_constraints
        .iter()
        .any(|constraint| constraint.tag.as_str() == tag)
}

fn effect_tagged_filter(effect: &EffectAst) -> Option<&ObjectFilter> {
    match effect {
        EffectAst::DealDamageEach { filter, .. }
        | EffectAst::PutCountersAll { filter, .. }
        | EffectAst::RemoveCountersAll { filter, .. }
        | EffectAst::DoubleCountersOnEach { filter, .. }
        | EffectAst::TapAll { filter }
        | EffectAst::ChooseObjects { filter, .. }
        | EffectAst::ChooseObjectsAcrossZones { filter, .. }
        | EffectAst::Sacrifice { filter, .. }
        | EffectAst::SacrificeAll { filter, .. }
        | EffectAst::RegenerateAll { filter }
        | EffectAst::DestroyAll { filter }
        | EffectAst::DestroyAllOfChosenColor { filter }
        | EffectAst::ExileAll { filter, .. }
        | EffectAst::PreventDamageEach { filter, .. }
        | EffectAst::ReturnAllToHand { filter }
        | EffectAst::ReturnAllToHandOfChosenColor { filter }
        | EffectAst::ReturnAllToBattlefield { filter, .. }
        | EffectAst::ExchangeControl { filter, .. }
        | EffectAst::PumpAll { filter, .. }
        | EffectAst::ScalePowerToughnessAll { filter, .. }
        | EffectAst::UntapAll { filter }
        | EffectAst::GrantAbilitiesAll { filter, .. }
        | EffectAst::RemoveAbilitiesAll { filter, .. }
        | EffectAst::GrantAbilitiesChoiceAll { filter, .. }
        | EffectAst::GrantBySpec {
            spec: crate::grant::GrantSpec { filter, .. },
            ..
        }
        | EffectAst::SearchLibrary { filter, .. }
        | EffectAst::DestroyAllAttachedTo { filter, .. } => Some(filter),
        EffectAst::Enchant {
            filter: crate::object::AuraAttachmentFilter::Object(filter),
        } => Some(filter),
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
            matches!(predicate, PredicateAst::TaggedMatches(t, _) if t.as_str() == tag)
                || matches!(predicate, PredicateAst::PlayerTaggedObjectMatches { tag: t, .. } if t.as_str() == tag)
                || effects_reference_tag(if_true, tag)
                || effects_reference_tag(if_false, tag)
        }
        EffectAst::DrawForEachTaggedMatching {
            tag: effect_tag,
            filter,
            ..
        } => effect_tag.as_str() == tag || filter_references_tag(filter, tag),
        EffectAst::RetargetStackObject { .. } => false,
        EffectAst::PutIntoHand { object, .. } => match object {
            ObjectRefAst::Tagged(found) => found.as_str() == tag,
        },
        EffectAst::CreateTokenCopy { object, .. } => match object {
            ObjectRefAst::Tagged(found) => found.as_str() == tag,
        },
        EffectAst::CreateTokenWithMods { count, .. } => value_references_tag(count, tag),
        EffectAst::ForEachObject { filter, effects } => {
            filter
                .tagged_constraints
                .iter()
                .any(|constraint| constraint.tag.as_str() == tag)
                || effects_reference_tag(effects, tag)
        }
        EffectAst::Cant { restriction, .. } => restriction_references_tag(restriction, tag),
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
        Value::Add(left, right) => {
            value_references_tag(left, tag) || value_references_tag(right, tag)
        }
        Value::Scaled(value, _) => value_references_tag(value, tag),
        Value::HalfRoundedDown(value) => value_references_tag(value, tag),
        Value::Count(filter) | Value::CountScaled(filter, _) => filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == tag),
        Value::TotalPower(filter)
        | Value::TotalToughness(filter)
        | Value::TotalManaValue(filter)
        | Value::GreatestPower(filter)
        | Value::GreatestToughness(filter)
        | Value::GreatestManaValue(filter) => filter
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
        let tag = tag.as_str();
        super::str_starts_with(tag, "exiled_")
            || super::str_starts_with(tag, "__sentence_helper_exiled")
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
        TargetAst::WithCount(inner, _) => target_references_tag(inner, tag),
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
    matches!(
        value,
        Value::EventValue(EventValueSpec::Amount)
            | Value::EventValue(EventValueSpec::LifeAmount)
            | Value::EventValueOffset(EventValueSpec::Amount, _)
            | Value::EventValueOffset(EventValueSpec::LifeAmount, _)
    )
}

pub(crate) fn effect_references_event_derived_amount(effect: &EffectAst) -> bool {
    assert_effect_ast_variant_coverage(effect);
    match effect {
        EffectAst::DealDamage { amount, .. }
        | EffectAst::DealDamageEach { amount, .. }
        | EffectAst::Draw { count: amount, .. }
        | EffectAst::LoseLife { amount, .. }
        | EffectAst::GainLife { amount, .. }
        | EffectAst::Mill { count: amount, .. }
        | EffectAst::SetLifeTotal { amount, .. }
        | EffectAst::PoisonCounters { count: amount, .. }
        | EffectAst::EnergyCounters { count: amount, .. }
        | EffectAst::PreventDamage { amount, .. }
        | EffectAst::RedirectNextDamageFromSourceToTarget { amount, .. }
        | EffectAst::PreventDamageEach { amount, .. }
        | EffectAst::AddManaScaled { amount, .. }
        | EffectAst::AddManaAnyColor { amount, .. }
        | EffectAst::AddManaAnyOneColor { amount, .. }
        | EffectAst::AddManaChosenColor { amount, .. }
        | EffectAst::AddManaFromLandCouldProduce { amount, .. }
        | EffectAst::AddManaCommanderIdentity { amount, .. }
        | EffectAst::Scry { count: amount, .. }
        | EffectAst::Fateseal { count: amount, .. }
        | EffectAst::Discover { count: amount, .. }
        | EffectAst::Surveil { count: amount, .. }
        | EffectAst::PayEnergy { amount, .. }
        | EffectAst::LookAtTopCards { count: amount, .. }
        | EffectAst::CopySpell { count: amount, .. }
        | EffectAst::Investigate { count: amount }
        | EffectAst::CreateTokenCopy { count: amount, .. }
        | EffectAst::CreateTokenCopyFromSource { count: amount, .. }
        | EffectAst::CreateTokenWithMods { count: amount, .. }
        | EffectAst::RemoveUpToAnyCounters { amount, .. } => {
            value_references_event_derived_amount(amount)
        }
        EffectAst::PreventDamageToTargetPutCounters {
            amount: Some(amount),
            ..
        } => value_references_event_derived_amount(amount),
        EffectAst::PutCounters { count, .. } | EffectAst::PutCountersAll { count, .. } => {
            value_references_event_derived_amount(count)
        }
        EffectAst::PutOrRemoveCounters {
            put_count,
            remove_count,
            ..
        } => {
            value_references_event_derived_amount(put_count)
                || value_references_event_derived_amount(remove_count)
        }
        EffectAst::RemoveCountersAll { amount, .. } => {
            value_references_event_derived_amount(amount)
        }
        EffectAst::CounterUnlessPays {
            life,
            additional_generic,
            ..
        } => {
            life.as_ref()
                .is_some_and(value_references_event_derived_amount)
                || additional_generic
                    .as_ref()
                    .is_some_and(value_references_event_derived_amount)
        }
        EffectAst::Discard { count, .. } => value_references_event_derived_amount(count),
        EffectAst::Pump {
            power, toughness, ..
        }
        | EffectAst::SetBasePowerToughness {
            power, toughness, ..
        }
        | EffectAst::BecomeBasePtCreature {
            power, toughness, ..
        }
        | EffectAst::PumpAll {
            power, toughness, ..
        } => {
            value_references_event_derived_amount(power)
                || value_references_event_derived_amount(toughness)
        }
        EffectAst::ScalePowerToughnessAll { .. } => false,
        EffectAst::SetBasePower { power, .. } => value_references_event_derived_amount(power),
        EffectAst::PumpForEach { count, .. } => value_references_event_derived_amount(count),
        EffectAst::ConsultTopOfLibrary { stop_rule, .. } => {
            matches!(
                stop_rule,
                crate::cards::builders::LibraryConsultStopRuleAst::MatchCount(value)
                    if value_references_event_derived_amount(value)
            )
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
        EffectAst::Draw { player, .. }
        | EffectAst::DrawForEachTaggedMatching { player, .. }
        | EffectAst::LoseLife { player, .. }
        | EffectAst::GainLife { player, .. }
        | EffectAst::GainControl { player, .. }
        | EffectAst::LoseGame { player }
        | EffectAst::AddMana { player, .. }
        | EffectAst::AddManaScaled { player, .. }
        | EffectAst::AddManaAnyColor { player, .. }
        | EffectAst::AddManaAnyOneColor { player, .. }
        | EffectAst::AddManaChosenColor { player, .. }
        | EffectAst::AddManaFromLandCouldProduce { player, .. }
        | EffectAst::AddManaCommanderIdentity { player, .. }
        | EffectAst::Scry { player, .. }
        | EffectAst::Fateseal { player, .. }
        | EffectAst::Surveil { player, .. }
        | EffectAst::PlayFromGraveyardUntilEot { player }
        | EffectAst::AdditionalLandPlays { player, .. }
        | EffectAst::ReduceNextSpellCostThisTurn { player, .. }
        | EffectAst::GrantNextSpellAbilityThisTurn { player, .. }
        | EffectAst::GrantPlayTaggedUntilEndOfTurn { player, .. }
        | EffectAst::GrantBySpec { player, .. }
        | EffectAst::GrantTaggedSpellAlternativeCostPayLifeByManaValueUntilEndOfTurn {
            player,
            ..
        }
        | EffectAst::ExileInsteadOfGraveyardThisTurn { player }
        | EffectAst::ExtraTurnAfterTurn { player, .. }
        | EffectAst::RevealTop { player }
        | EffectAst::RevealTopPutMatchingIntoHandRestIntoGraveyard { player, .. }
        | EffectAst::LookAtTopCards { player, .. }
        | EffectAst::RevealHand { player }
        | EffectAst::PutIntoHand { player, .. }
        | EffectAst::CopySpell { player, .. }
        | EffectAst::RetargetStackObject {
            chooser: player, ..
        }
        | EffectAst::DiscardHand { player }
        | EffectAst::Discard { player, .. }
        | EffectAst::Mill { player, .. }
        | EffectAst::DoubleManaPool { player }
        | EffectAst::SetLifeTotal { player, .. }
        | EffectAst::SkipTurn { player }
        | EffectAst::SkipCombatPhases { player }
        | EffectAst::SkipNextCombatPhaseThisTurn { player }
        | EffectAst::SkipDrawStep { player }
        | EffectAst::PoisonCounters { player, .. }
        | EffectAst::EnergyCounters { player, .. }
        | EffectAst::CreateTokenCopy { player, .. }
        | EffectAst::CreateTokenCopyFromSource { player, .. }
        | EffectAst::CreateTokenWithMods { player, .. }
        | EffectAst::SearchLibrary { player, .. }
        | EffectAst::ShuffleHandAndGraveyardIntoLibrary { player }
        | EffectAst::ShuffleGraveyardIntoLibrary { player }
        | EffectAst::ShuffleLibrary { player }
        | EffectAst::ShuffleObjectsIntoLibrary { player, .. }
        | EffectAst::Sacrifice { player, .. }
        | EffectAst::SacrificeAll { player, .. }
        | EffectAst::ChooseObjects { player, .. }
        | EffectAst::ChooseObjectsAcrossZones { player, .. }
        | EffectAst::ChooseFromLookedCardsOntoBattlefieldOrIntoHandRestOnBottomOfLibrary {
            player,
            ..
        }
        | EffectAst::ChooseFromLookedCardsOntoBattlefieldAndIntoHandRestOnBottomOfLibrary {
            player,
            ..
        } => {
            matches!(player, PlayerAst::ItsController | PlayerAst::ItsOwner)
        }
        EffectAst::ExchangeLifeTotals { player1, player2 } => {
            matches!(player1, PlayerAst::ItsController | PlayerAst::ItsOwner)
                || matches!(player2, PlayerAst::ItsController | PlayerAst::ItsOwner)
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
        EffectAst::DealDamage { amount, .. } => value_references_tag(amount, IT_TAG),
        EffectAst::DealDamageEach { amount, filter } => {
            value_references_tag(amount, IT_TAG) || filter_references_tag(filter, IT_TAG)
        }
        EffectAst::Draw { count, .. } => value_references_tag(count, IT_TAG),
        EffectAst::DrawForEachTaggedMatching { tag, filter, .. } => {
            tag.as_str() == IT_TAG || filter_references_tag(filter, IT_TAG)
        }
        EffectAst::LoseLife { amount, .. } | EffectAst::GainLife { amount, .. } => {
            value_references_tag(amount, IT_TAG)
        }
        EffectAst::PreventDamage { amount, .. } => value_references_tag(amount, IT_TAG),
        EffectAst::PreventDamageToTargetPutCounters {
            amount: Some(amount),
            ..
        } => value_references_tag(amount, IT_TAG),
        EffectAst::PreventDamageEach { amount, filter, .. } => {
            value_references_tag(amount, IT_TAG) || filter_references_tag(filter, IT_TAG)
        }
        EffectAst::PutCounters { count, .. } => value_references_tag(count, IT_TAG),
        EffectAst::PutCountersAll { count, filter, .. } => {
            value_references_tag(count, IT_TAG) || filter_references_tag(filter, IT_TAG)
        }
        EffectAst::CounterUnlessPays {
            life,
            additional_generic,
            ..
        } => {
            life.as_ref()
                .is_some_and(|value| value_references_tag(value, IT_TAG))
                || additional_generic
                    .as_ref()
                    .is_some_and(|value| value_references_tag(value, IT_TAG))
        }
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
            matches!(
                predicate,
                PredicateAst::ItIsLandCard
                    | PredicateAst::ItIsSoulbondPaired
                    | PredicateAst::ItMatches(_)
            ) || matches!(predicate, PredicateAst::TaggedMatches(t, _) if t.as_str() == IT_TAG)
                || matches!(
                    predicate,
                    PredicateAst::PlayerTaggedObjectMatches { tag: t, .. } if t.as_str() == IT_TAG
                )
                || effects_reference_it_tag(if_true)
                || effects_reference_it_tag(if_false)
        }
        EffectAst::PutIntoHand { object, .. } => {
            matches!(object, ObjectRefAst::Tagged(tag) if tag.as_str() == IT_TAG)
        }
        EffectAst::ChooseFromLookedCardsIntoHandRestIntoGraveyard { .. }
        | EffectAst::ChooseFromLookedCardsIntoHandRestOnBottomOfLibrary { .. } => true,
        EffectAst::ChooseFromLookedCardsOntoBattlefieldOrIntoHandRestOnBottomOfLibrary {
            ..
        }
        | EffectAst::ChooseFromLookedCardsOntoBattlefieldAndIntoHandRestOnBottomOfLibrary {
            ..
        } => true,
        EffectAst::PutRestOnBottomOfLibrary => true,
        EffectAst::RetargetStackObject { .. } => false,
        EffectAst::CreateTokenCopy { object, .. } => {
            matches!(object, ObjectRefAst::Tagged(tag) if tag.as_str() == IT_TAG)
        }
        EffectAst::GrantPlayTaggedUntilEndOfTurn { tag, .. }
        | EffectAst::GrantTaggedSpellAlternativeCostPayLifeByManaValueUntilEndOfTurn {
            tag, ..
        }
        | EffectAst::GrantPlayTaggedUntilYourNextTurn { tag, .. }
        | EffectAst::CastTagged { tag, .. }
        | EffectAst::ReorderTopOfLibrary { tag } => tag.as_str() == IT_TAG,
        EffectAst::CreateTokenWithMods { count, .. } => value_references_tag(count, IT_TAG),
        EffectAst::ForEachTagged { tag, effects } => {
            tag.as_str() == IT_TAG || effects_reference_it_tag(effects)
        }
        EffectAst::DelayedWhenLastObjectDiesThisTurn { .. } => true,
        EffectAst::ForEachObject { filter, effects } => {
            filter_references_tag(filter, IT_TAG) || effects_reference_it_tag(effects)
        }
        EffectAst::Cant { restriction, .. } => restriction_references_tag(restriction, IT_TAG),
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

pub(crate) fn restriction_references_tag(
    restriction: &crate::effect::Restriction,
    tag: &str,
) -> bool {
    use crate::effect::Restriction;

    let maybe_filter = match restriction {
        Restriction::Attack(filter)
        | Restriction::Block(filter)
        | Restriction::Untap(filter)
        | Restriction::BeBlocked(filter)
        | Restriction::BeDestroyed(filter)
        | Restriction::BeRegenerated(filter)
        | Restriction::BeSacrificed(filter)
        | Restriction::HaveCountersPlaced(filter)
        | Restriction::BeTargeted(filter)
        | Restriction::BeCountered(filter)
        | Restriction::Transform(filter)
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

    match effect {
        EffectAst::RemoveCountersAll { .. } => {}
        _ => for_each_nested_effects(effect, true, |nested| {
            collect_tag_spans_from_effects_with_context(nested, annotations, ctx);
        }),
    }
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
        annotations.record_tag_span(&TagKey::from(IT_TAG), mapped);
    }
}
