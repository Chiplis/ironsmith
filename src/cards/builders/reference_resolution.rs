use super::effect_ast_traversal::for_each_nested_effects_mut;
use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReferenceBindings {
    pub(crate) seed_tag: TagKey,
    pub(crate) unresolved_it_before: usize,
    pub(crate) unresolved_it_after: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct BoundEffectsAst {
    pub(crate) effects: Vec<EffectAst>,
    pub(crate) bindings: ReferenceBindings,
}

pub(crate) fn bind_unresolved_it_references_with_bindings(
    effects: &[EffectAst],
    seed_last_object_tag: Option<&str>,
) -> BoundEffectsAst {
    let seed_tag = seed_last_object_tag
        .map(TagKey::from)
        .unwrap_or_else(|| TagKey::from(IT_TAG));
    let unresolved_it_before = count_unresolved_it_occurrences(effects);
    let mut resolved = effects.to_vec();
    for effect in &mut resolved {
        let _ = bind_unresolved_it_in_effect(effect, &seed_tag);
    }
    let unresolved_it_after = count_unresolved_it_occurrences(&resolved);
    BoundEffectsAst {
        effects: resolved,
        bindings: ReferenceBindings {
            seed_tag,
            unresolved_it_before,
            unresolved_it_after,
        },
    }
}

fn count_unresolved_it_occurrences(effects: &[EffectAst]) -> usize {
    let mut cloned = effects.to_vec();
    let sentinel = TagKey::from("__count_unresolved_it__");
    cloned
        .iter_mut()
        .map(|effect| bind_unresolved_it_in_effect(effect, &sentinel))
        .sum()
}

fn bind_unresolved_it_in_effect(effect: &mut EffectAst, seed_tag: &TagKey) -> usize {
    let mut replacements = bind_unresolved_it_in_effect_fields(effect, seed_tag);
    for_each_nested_effects_mut(effect, true, |nested| {
        for inner in nested {
            replacements += bind_unresolved_it_in_effect(inner, seed_tag);
        }
    });
    replacements
}

fn bind_unresolved_it_in_effect_fields(effect: &mut EffectAst, seed_tag: &TagKey) -> usize {
    match effect {
        EffectAst::DealDamage { amount, target } => {
            bind_unresolved_it_in_value(amount, seed_tag)
                + bind_unresolved_it_in_target(target, seed_tag)
        }
        EffectAst::DealDamageEqualToPower { source, target } => {
            bind_unresolved_it_in_target(source, seed_tag)
                + bind_unresolved_it_in_target(target, seed_tag)
        }
        EffectAst::Fight {
            creature1,
            creature2,
        } => {
            bind_unresolved_it_in_target(creature1, seed_tag)
                + bind_unresolved_it_in_target(creature2, seed_tag)
        }
        EffectAst::FightIterated { creature2 } => bind_unresolved_it_in_target(creature2, seed_tag),
        EffectAst::DealDamageEach { amount, filter } => {
            bind_unresolved_it_in_value(amount, seed_tag)
                + bind_unresolved_it_in_filter(filter, seed_tag)
        }
        EffectAst::Draw { count, .. } => bind_unresolved_it_in_value(count, seed_tag),
        EffectAst::Counter { target } => bind_unresolved_it_in_target(target, seed_tag),
        EffectAst::CounterUnlessPays { target, .. } => {
            bind_unresolved_it_in_target(target, seed_tag)
        }
        EffectAst::PutCounters { count, target, .. } => {
            bind_unresolved_it_in_value(count, seed_tag)
                + bind_unresolved_it_in_target(target, seed_tag)
        }
        EffectAst::PutOrRemoveCounters {
            put_count,
            remove_count,
            target,
            ..
        } => {
            bind_unresolved_it_in_value(put_count, seed_tag)
                + bind_unresolved_it_in_value(remove_count, seed_tag)
                + bind_unresolved_it_in_target(target, seed_tag)
        }
        EffectAst::PutCountersAll { count, filter, .. } => {
            bind_unresolved_it_in_value(count, seed_tag)
                + bind_unresolved_it_in_filter(filter, seed_tag)
        }
        EffectAst::RemoveCountersAll { amount, filter, .. } => {
            bind_unresolved_it_in_value(amount, seed_tag)
                + bind_unresolved_it_in_filter(filter, seed_tag)
        }
        EffectAst::DoubleCountersOnEach { filter, .. } => {
            bind_unresolved_it_in_filter(filter, seed_tag)
        }
        EffectAst::Tap { target }
        | EffectAst::Untap { target }
        | EffectAst::RemoveFromCombat { target }
        | EffectAst::TapOrUntap { target }
        | EffectAst::Connive { target }
        | EffectAst::Goad { target }
        | EffectAst::Transform { target }
        | EffectAst::Flip { target }
        | EffectAst::Regenerate { target }
        | EffectAst::ReturnToHand { target, .. }
        | EffectAst::ReturnToBattlefield { target, .. }
        | EffectAst::MoveToLibrarySecondFromTop { target }
        | EffectAst::LookAtHand { target }
        | EffectAst::TargetOnly { target }
        | EffectAst::Destroy { target }
        | EffectAst::DestroyNoRegeneration { target }
        | EffectAst::Exile { target, .. }
        | EffectAst::ExileWhenSourceLeaves { target }
        | EffectAst::SacrificeSourceWhenLeaves { target }
        | EffectAst::ExileUntilSourceLeaves { target, .. }
        | EffectAst::GainControl { target, .. }
        | EffectAst::GrantAbilitiesToTarget { target, .. }
        | EffectAst::RemoveAbilitiesFromTarget { target, .. }
        | EffectAst::GrantAbilitiesChoiceToTarget { target, .. }
        | EffectAst::SwitchPowerToughness { target, .. }
        | EffectAst::AddCardTypes { target, .. }
        | EffectAst::AddSubtypes { target, .. }
        | EffectAst::SetColors { target, .. }
        | EffectAst::MakeColorless { target, .. }
        | EffectAst::PumpByLastEffect { target, .. } => {
            bind_unresolved_it_in_target(target, seed_tag)
        }
        EffectAst::TapAll { filter }
        | EffectAst::UntapAll { filter }
        | EffectAst::DestroyAll { filter }
        | EffectAst::DestroyAllNoRegeneration { filter }
        | EffectAst::DestroyAllOfChosenColor { filter }
        | EffectAst::DestroyAllOfChosenColorNoRegeneration { filter }
        | EffectAst::ExileAll { filter, .. }
        | EffectAst::RegenerateAll { filter }
        | EffectAst::ReturnAllToHand { filter }
        | EffectAst::ReturnAllToHandOfChosenColor { filter }
        | EffectAst::ReturnAllToBattlefield { filter, .. }
        | EffectAst::Enchant { filter }
        | EffectAst::PumpAll { filter, .. }
        | EffectAst::GrantAbilitiesAll { filter, .. }
        | EffectAst::RemoveAbilitiesAll { filter, .. }
        | EffectAst::GrantAbilitiesChoiceAll { filter, .. }
        | EffectAst::ForEachObject { filter, .. } => bind_unresolved_it_in_filter(filter, seed_tag),
        EffectAst::LoseLife { amount, .. }
        | EffectAst::GainLife { amount, .. }
        | EffectAst::SetLifeTotal { amount, .. }
        | EffectAst::PoisonCounters { count: amount, .. }
        | EffectAst::EnergyCounters { count: amount, .. }
        | EffectAst::Monstrosity { amount } => bind_unresolved_it_in_value(amount, seed_tag),
        EffectAst::PreventDamage { amount, target, .. } => {
            bind_unresolved_it_in_value(amount, seed_tag)
                + bind_unresolved_it_in_target(target, seed_tag)
        }
        EffectAst::PreventNextTimeDamage { source, .. } => {
            bind_unresolved_it_in_prevent_next_source(source, seed_tag)
        }
        EffectAst::RedirectNextDamageFromSourceToTarget { amount, target } => {
            bind_unresolved_it_in_value(amount, seed_tag)
                + bind_unresolved_it_in_target(target, seed_tag)
        }
        EffectAst::RedirectNextTimeDamageToSource { source, target } => {
            bind_unresolved_it_in_prevent_next_source(source, seed_tag)
                + bind_unresolved_it_in_target(target, seed_tag)
        }
        EffectAst::PreventAllDamageToTarget { target, .. } => {
            bind_unresolved_it_in_target(target, seed_tag)
        }
        EffectAst::PreventDamageEach { amount, filter, .. } => {
            bind_unresolved_it_in_value(amount, seed_tag)
                + bind_unresolved_it_in_filter(filter, seed_tag)
        }
        EffectAst::AddManaScaled { amount, .. }
        | EffectAst::AddManaAnyColor { amount, .. }
        | EffectAst::AddManaAnyOneColor { amount, .. }
        | EffectAst::AddManaChosenColor { amount, .. }
        | EffectAst::AddManaCommanderIdentity { amount, .. }
        | EffectAst::Scry { count: amount, .. }
        | EffectAst::Discover { count: amount, .. }
        | EffectAst::Surveil { count: amount, .. }
        | EffectAst::PayEnergy { amount, .. } => bind_unresolved_it_in_value(amount, seed_tag),
        EffectAst::AddManaFromLandCouldProduce {
            amount,
            land_filter,
            ..
        } => {
            bind_unresolved_it_in_value(amount, seed_tag)
                + bind_unresolved_it_in_filter(land_filter, seed_tag)
        }
        EffectAst::Cant { restriction, .. } => {
            bind_unresolved_it_in_restriction(restriction, seed_tag)
        }
        EffectAst::GrantPlayTaggedUntilEndOfTurn { tag, .. }
        | EffectAst::GrantTaggedSpellAlternativeCostPayLifeByManaValueUntilEndOfTurn {
            tag, ..
        }
        | EffectAst::GrantPlayTaggedUntilYourNextTurn { tag, .. }
        | EffectAst::CastTagged { tag, .. }
        | EffectAst::RevealTagged { tag }
        | EffectAst::ReorderTopOfLibrary { tag }
        | EffectAst::MayByTaggedController { tag, .. }
        | EffectAst::ForEachTagged { tag, .. }
        | EffectAst::ForEachTaggedPlayer { tag, .. } => bind_unresolved_it_in_tag(tag, seed_tag),
        EffectAst::ControlPlayer { player, .. }
        | EffectAst::ForEachPlayersFiltered { filter: player, .. } => {
            bind_unresolved_it_in_player_filter(player, seed_tag)
        }
        EffectAst::DelayedWhenLastObjectDiesThisTurn { filter, .. } => {
            if let Some(filter) = filter.as_mut() {
                bind_unresolved_it_in_filter(filter, seed_tag)
            } else {
                0
            }
        }
        EffectAst::LookAtTopCards { count, tag, .. } => {
            bind_unresolved_it_in_value(count, seed_tag) + bind_unresolved_it_in_tag(tag, seed_tag)
        }
        EffectAst::PutIntoHand { object, .. } => {
            bind_unresolved_it_in_object_ref_ast(object, seed_tag)
        }
        EffectAst::CopySpell { target, count, .. } => {
            bind_unresolved_it_in_target(target, seed_tag)
                + bind_unresolved_it_in_value(count, seed_tag)
        }
        EffectAst::RetargetStackObject {
            target,
            mode,
            new_target_restriction,
            ..
        } => {
            let mut replacements = bind_unresolved_it_in_target(target, seed_tag);
            if let RetargetModeAst::OneToFixed { target } = mode {
                replacements += bind_unresolved_it_in_target(target, seed_tag);
            }
            if let Some(NewTargetRestrictionAst::Object(filter)) = new_target_restriction.as_mut() {
                replacements += bind_unresolved_it_in_filter(filter, seed_tag);
            }
            replacements
        }
        EffectAst::Conditional { predicate, .. } => {
            bind_unresolved_it_in_predicate(predicate, seed_tag)
        }
        EffectAst::ChooseObjects { filter, tag, .. } => {
            bind_unresolved_it_in_filter(filter, seed_tag)
                + bind_unresolved_it_in_tag(tag, seed_tag)
        }
        EffectAst::Sacrifice { filter, .. }
        | EffectAst::SacrificeAll { filter, .. }
        | EffectAst::ExchangeControl { filter, .. }
        | EffectAst::DestroyAllAttachedTo { filter, .. }
        | EffectAst::SearchLibrary { filter, .. } => bind_unresolved_it_in_filter(filter, seed_tag),
        EffectAst::Discard { count, filter, .. } => {
            let mut replacements = bind_unresolved_it_in_value(count, seed_tag);
            if let Some(filter) = filter.as_mut() {
                replacements += bind_unresolved_it_in_filter(filter, seed_tag);
            }
            replacements
        }
        EffectAst::MoveToZone {
            target,
            attached_to,
            ..
        } => {
            let mut replacements = bind_unresolved_it_in_target(target, seed_tag);
            if let Some(attach) = attached_to.as_mut() {
                replacements += bind_unresolved_it_in_target(attach, seed_tag);
            }
            replacements
        }
        EffectAst::CreateToken { count, .. } | EffectAst::Investigate { count } => {
            bind_unresolved_it_in_value(count, seed_tag)
        }
        EffectAst::CreateTokenCopy { object, count, .. } => {
            bind_unresolved_it_in_object_ref_ast(object, seed_tag)
                + bind_unresolved_it_in_value(count, seed_tag)
        }
        EffectAst::CreateTokenCopyFromSource { source, count, .. } => {
            bind_unresolved_it_in_target(source, seed_tag)
                + bind_unresolved_it_in_value(count, seed_tag)
        }
        EffectAst::CreateTokenWithMods {
            count, attached_to, ..
        } => {
            let mut replacements = bind_unresolved_it_in_value(count, seed_tag);
            if let Some(target) = attached_to.as_mut() {
                replacements += bind_unresolved_it_in_target(target, seed_tag);
            }
            replacements
        }
        EffectAst::RemoveUpToAnyCounters { amount, target, .. } => {
            bind_unresolved_it_in_value(amount, seed_tag)
                + bind_unresolved_it_in_target(target, seed_tag)
        }
        EffectAst::MoveAllCounters { from, to } => {
            bind_unresolved_it_in_target(from, seed_tag)
                + bind_unresolved_it_in_target(to, seed_tag)
        }
        EffectAst::Pump {
            power,
            toughness,
            target,
            ..
        }
        | EffectAst::SetBasePowerToughness {
            power,
            toughness,
            target,
            ..
        }
        | EffectAst::BecomeBasePtCreature {
            power,
            toughness,
            target,
            ..
        } => {
            bind_unresolved_it_in_value(power, seed_tag)
                + bind_unresolved_it_in_value(toughness, seed_tag)
                + bind_unresolved_it_in_target(target, seed_tag)
        }
        EffectAst::SetBasePower { power, target, .. } => {
            bind_unresolved_it_in_value(power, seed_tag)
                + bind_unresolved_it_in_target(target, seed_tag)
        }
        EffectAst::PumpForEach { target, count, .. } => {
            bind_unresolved_it_in_target(target, seed_tag)
                + bind_unresolved_it_in_value(count, seed_tag)
        }
        EffectAst::ForEachOpponentDid {
            predicate: Some(predicate),
            ..
        }
        | EffectAst::ForEachPlayerDid {
            predicate: Some(predicate),
            ..
        } => bind_unresolved_it_in_predicate(predicate, seed_tag),
        EffectAst::Attach { object, target } => {
            bind_unresolved_it_in_target(object, seed_tag)
                + bind_unresolved_it_in_target(target, seed_tag)
        }
        _ => 0,
    }
}

fn bind_unresolved_it_in_object_ref_ast(reference: &mut ObjectRefAst, seed_tag: &TagKey) -> usize {
    let ObjectRefAst::Tagged(tag) = reference;
    bind_unresolved_it_in_tag(tag, seed_tag)
}

fn bind_unresolved_it_in_tag(tag: &mut TagKey, seed_tag: &TagKey) -> usize {
    if tag.as_str() == IT_TAG {
        *tag = seed_tag.clone();
        1
    } else {
        0
    }
}

fn bind_unresolved_it_in_runtime_object_ref(
    reference: &mut crate::filter::ObjectRef,
    seed_tag: &TagKey,
) -> usize {
    if let crate::filter::ObjectRef::Tagged(tag) = reference {
        bind_unresolved_it_in_tag(tag, seed_tag)
    } else {
        0
    }
}

fn bind_unresolved_it_in_player_filter(filter: &mut PlayerFilter, seed_tag: &TagKey) -> usize {
    match filter {
        PlayerFilter::Target(inner) => bind_unresolved_it_in_player_filter(inner, seed_tag),
        PlayerFilter::Excluding { base, excluded } => {
            bind_unresolved_it_in_player_filter(base, seed_tag)
                + bind_unresolved_it_in_player_filter(excluded, seed_tag)
        }
        PlayerFilter::ControllerOf(reference) | PlayerFilter::OwnerOf(reference) => {
            bind_unresolved_it_in_runtime_object_ref(reference, seed_tag)
        }
        _ => 0,
    }
}

fn bind_unresolved_it_in_filter(filter: &mut ObjectFilter, seed_tag: &TagKey) -> usize {
    let mut replacements = 0;
    for constraint in &mut filter.tagged_constraints {
        replacements += bind_unresolved_it_in_tag(&mut constraint.tag, seed_tag);
    }
    if let Some(owner) = filter.owner.as_mut() {
        replacements += bind_unresolved_it_in_player_filter(owner, seed_tag);
    }
    if let Some(controller) = filter.controller.as_mut() {
        replacements += bind_unresolved_it_in_player_filter(controller, seed_tag);
    }
    replacements
}

fn bind_unresolved_it_in_target(target: &mut TargetAst, seed_tag: &TagKey) -> usize {
    match target {
        TargetAst::Tagged(tag, _) => bind_unresolved_it_in_tag(tag, seed_tag),
        TargetAst::Object(filter, _, _) => bind_unresolved_it_in_filter(filter, seed_tag),
        TargetAst::Player(filter, _) | TargetAst::PlayerOrPlaneswalker(filter, _) => {
            bind_unresolved_it_in_player_filter(filter, seed_tag)
        }
        TargetAst::WithCount(inner, _) => bind_unresolved_it_in_target(inner, seed_tag),
        _ => 0,
    }
}

fn bind_unresolved_it_in_prevent_next_source(
    source: &mut PreventNextTimeDamageSourceAst,
    seed_tag: &TagKey,
) -> usize {
    if let PreventNextTimeDamageSourceAst::Filter(filter) = source {
        bind_unresolved_it_in_filter(filter, seed_tag)
    } else {
        0
    }
}

fn bind_unresolved_it_in_choose_spec(spec: &mut ChooseSpec, seed_tag: &TagKey) -> usize {
    match spec {
        ChooseSpec::Tagged(tag) => bind_unresolved_it_in_tag(tag, seed_tag),
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            bind_unresolved_it_in_filter(filter, seed_tag)
        }
        ChooseSpec::Target(inner) | ChooseSpec::WithCount(inner, _) => {
            bind_unresolved_it_in_choose_spec(inner, seed_tag)
        }
        ChooseSpec::Player(filter) | ChooseSpec::PlayerOrPlaneswalker(filter) => {
            bind_unresolved_it_in_player_filter(filter, seed_tag)
        }
        ChooseSpec::EachPlayer(filter) => bind_unresolved_it_in_player_filter(filter, seed_tag),
        _ => 0,
    }
}

fn bind_unresolved_it_in_value(value: &mut Value, seed_tag: &TagKey) -> usize {
    match value {
        Value::Add(left, right) => {
            bind_unresolved_it_in_value(left, seed_tag)
                + bind_unresolved_it_in_value(right, seed_tag)
        }
        Value::Count(filter)
        | Value::CountScaled(filter, _)
        | Value::TotalPower(filter)
        | Value::TotalToughness(filter)
        | Value::TotalManaValue(filter)
        | Value::GreatestPower(filter)
        | Value::GreatestManaValue(filter)
        | Value::BasicLandTypesAmong(filter)
        | Value::ColorsAmong(filter) => bind_unresolved_it_in_filter(filter, seed_tag),
        Value::PowerOf(spec)
        | Value::ToughnessOf(spec)
        | Value::ManaValueOf(spec)
        | Value::CountersOn(spec, _) => bind_unresolved_it_in_choose_spec(spec, seed_tag),
        _ => 0,
    }
}

fn bind_unresolved_it_in_predicate(predicate: &mut PredicateAst, seed_tag: &TagKey) -> usize {
    match predicate {
        PredicateAst::ItMatches(filter) | PredicateAst::TaggedMatches(_, filter) => {
            let mut replacements = bind_unresolved_it_in_filter(filter, seed_tag);
            if let PredicateAst::TaggedMatches(tag, _) = predicate {
                replacements += bind_unresolved_it_in_tag(tag, seed_tag);
            }
            replacements
        }
        PredicateAst::PlayerTaggedObjectMatches { tag, filter, .. } => {
            bind_unresolved_it_in_tag(tag, seed_tag)
                + bind_unresolved_it_in_filter(filter, seed_tag)
        }
        PredicateAst::PlayerControls { filter, .. }
        | PredicateAst::PlayerControlsAtLeast { filter, .. }
        | PredicateAst::PlayerControlsExactly { filter, .. }
        | PredicateAst::PlayerControlsAtLeastWithDifferentPowers { filter, .. }
        | PredicateAst::PlayerControlsNo { filter, .. }
        | PredicateAst::PlayerControlsMost { filter, .. } => {
            bind_unresolved_it_in_filter(filter, seed_tag)
        }
        PredicateAst::PlayerControlsOrHasCardInGraveyard {
            control_filter,
            graveyard_filter,
            ..
        } => {
            bind_unresolved_it_in_filter(control_filter, seed_tag)
                + bind_unresolved_it_in_filter(graveyard_filter, seed_tag)
        }
        PredicateAst::And(left, right) => {
            bind_unresolved_it_in_predicate(left, seed_tag)
                + bind_unresolved_it_in_predicate(right, seed_tag)
        }
        _ => 0,
    }
}

fn bind_unresolved_it_in_restriction(
    restriction: &mut crate::effect::Restriction,
    seed_tag: &TagKey,
) -> usize {
    use crate::effect::Restriction;

    match restriction {
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
        | Restriction::ActivateNonManaAbilitiesOf(filter) => {
            bind_unresolved_it_in_filter(filter, seed_tag)
        }
        Restriction::BlockSpecificAttacker { blockers, attacker }
        | Restriction::MustBlockSpecificAttacker { blockers, attacker } => {
            bind_unresolved_it_in_filter(blockers, seed_tag)
                + bind_unresolved_it_in_filter(attacker, seed_tag)
        }
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binding_reports_typed_unresolved_it_counts() {
        let mut filter = ObjectFilter::default();
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from(IT_TAG),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });

        let effects = vec![EffectAst::DealDamage {
            amount: Value::Count(filter),
            target: TargetAst::Tagged(TagKey::from(IT_TAG), None),
        }];

        let bound = bind_unresolved_it_references_with_bindings(&effects, Some("bound_target"));
        assert_eq!(bound.bindings.unresolved_it_before, 2);
        assert_eq!(bound.bindings.unresolved_it_after, 0);
        assert!(format!("{:?}", bound.effects).contains("bound_target"));
    }
}
