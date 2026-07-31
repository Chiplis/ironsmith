use super::*;

struct SyntheticTargetIdentity<'a> {
    target: &'a ChooseSpec,
    tag: Option<&'a TagKey>,
}

fn object_ref_references_identity(
    object_ref: &crate::target::ObjectRef,
    identity: &SyntheticTargetIdentity<'_>,
) -> bool {
    matches!(
        (object_ref, identity.tag),
        (crate::target::ObjectRef::Tagged(found), Some(expected)) if found == expected
    )
}

fn player_filter_references_identity(
    player: &PlayerFilter,
    identity: &SyntheticTargetIdentity<'_>,
) -> bool {
    if let Some(target_player) = choose_spec_player_filter(identity.target) {
        if target_player == *player {
            return true;
        }
        if let (PlayerFilter::Target(target_base), PlayerFilter::AliasedTarget(candidate_base)) =
            (&target_player, player)
            && target_base == candidate_base
        {
            return true;
        }
    }

    match player {
        PlayerFilter::TaggedPlayer(found) => identity.tag.is_some_and(|tag| found == tag),
        PlayerFilter::Target(inner)
        | PlayerFilter::AliasedTarget(inner)
        | PlayerFilter::CardsInHandAtLeastMoreThanYou { base: inner, .. }
        | PlayerFilter::HasMoreLifeThanYou { base: inner }
        | PlayerFilter::MaxSpeed { base: inner, .. } => {
            player_filter_references_identity(inner, identity)
        }
        PlayerFilter::OpponentWithMoreControlledObjectsThan { player, filter } => {
            player_filter_references_identity(player, identity)
                || object_filter_references_identity(filter, identity)
        }
        PlayerFilter::Excluding { base, excluded } => {
            player_filter_references_identity(base, identity)
                || player_filter_references_identity(excluded, identity)
        }
        PlayerFilter::ControllerOf(object_ref)
        | PlayerFilter::OwnerOf(object_ref)
        | PlayerFilter::AliasedOwnerOf(object_ref)
        | PlayerFilter::AliasedControllerOf(object_ref) => {
            object_ref_references_identity(object_ref, identity)
        }
        _ => false,
    }
}

fn object_filter_references_identity(
    filter: &ObjectFilter,
    identity: &SyntheticTargetIdentity<'_>,
) -> bool {
    identity.tag.is_some_and(|tag| {
        filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag == *tag)
    }) || filter
        .controller
        .as_ref()
        .is_some_and(|player| player_filter_references_identity(player, identity))
        || filter
            .owner
            .as_ref()
            .is_some_and(|player| player_filter_references_identity(player, identity))
        || filter
            .targets_player
            .as_ref()
            .is_some_and(|player| player_filter_references_identity(player, identity))
        || filter
            .targets_only_player
            .as_ref()
            .is_some_and(|player| player_filter_references_identity(player, identity))
        || filter
            .attached_to_player
            .as_ref()
            .is_some_and(|player| player_filter_references_identity(player, identity))
        || filter
            .entered_battlefield_controller
            .as_ref()
            .is_some_and(|player| player_filter_references_identity(player, identity))
        || filter
            .attached_to_object
            .as_deref()
            .is_some_and(|object| object_filter_references_identity(object, identity))
        || filter
            .any_of
            .iter()
            .any(|candidate| object_filter_references_identity(candidate, identity))
}

fn choose_spec_references_identity(
    spec: &ChooseSpec,
    identity: &SyntheticTargetIdentity<'_>,
) -> bool {
    if target_specs_select_same_objects(spec, identity.target) {
        return true;
    }

    match spec {
        ChooseSpec::SurfaceHinted { spec, .. }
        | ChooseSpec::Target(spec)
        | ChooseSpec::WithCount(spec, _) => choose_spec_references_identity(spec, identity),
        ChooseSpec::WithCountValue(spec, _, count) => {
            choose_spec_references_identity(spec, identity)
                || value_references_identity(count, identity)
        }
        ChooseSpec::Tagged(found) => identity.tag.is_some_and(|tag| found == tag),
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            object_filter_references_identity(filter, identity)
        }
        ChooseSpec::ObjectOrPlayer(filter, player) => {
            object_filter_references_identity(filter, identity)
                || player_filter_references_identity(player, identity)
        }
        ChooseSpec::Player(player)
        | ChooseSpec::EachPlayer(player)
        | ChooseSpec::PlayerOrPlaneswalker(player) => {
            player_filter_references_identity(player, identity)
        }
        _ => false,
    }
}

fn value_references_identity(value: &Value, identity: &SyntheticTargetIdentity<'_>) -> bool {
    match value.unhinted() {
        Value::Add(left, right) | Value::Min(left, right) => {
            value_references_identity(left, identity) || value_references_identity(right, identity)
        }
        Value::Scaled(value, _)
        | Value::DividedRoundedDown(value, _)
        | Value::HalfRoundedDown(value) => value_references_identity(value, identity),
        Value::PowerOf(spec)
        | Value::ToughnessOf(spec)
        | Value::ManaValueOf(spec)
        | Value::CountersOn(spec, _) => choose_spec_references_identity(spec, identity),
        Value::ManaSymbolsInManaCostOf { spec, .. } => {
            choose_spec_references_identity(spec, identity)
        }
        Value::Count(filter)
        | Value::CountScaled(filter, _)
        | Value::GreatestCount(filter)
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
        | Value::DistinctNames(filter)
        | Value::DistinctPowers(filter) => object_filter_references_identity(filter, identity),
        Value::StaticAbilitiesAmong { filter, .. } => {
            object_filter_references_identity(filter, identity)
        }
        Value::CountPlayers(player)
        | Value::CountPlayersWithCardsInHandAtLeast(player, _)
        | Value::PartySize(player)
        | Value::LifeTotal(player)
        | Value::LifeTotalAsTurnBegan(player)
        | Value::LifeTotalDifference(player)
        | Value::UnspentMana(player)
        | Value::Speed(player)
        | Value::StartingLifeTotal(player)
        | Value::HalfLifeTotalRoundedUp(player)
        | Value::HalfLifeTotalRoundedDown(player)
        | Value::HalfStartingLifeTotalRoundedUp(player)
        | Value::HalfStartingLifeTotalRoundedDown(player)
        | Value::CardsInHand(player)
        | Value::CardsInLibrary(player)
        | Value::DevotionToChosenColor(player)
        | Value::LifeGainedThisTurn(player)
        | Value::LifeLostThisTurn(player)
        | Value::CardsDiscardedThisTurn(player)
        | Value::DamageDealtToPlayersThisTurn(player)
        | Value::NoncombatDamageDealtToPlayersThisTurn(player)
        | Value::MaxCardsDrawnThisTurn(player)
        | Value::MaxDiceRolledThisTurn(player)
        | Value::LandsEnteredBattlefieldThisTurn(player)
        | Value::MaxCardsInHand(player)
        | Value::CardsInGraveyard(player)
        | Value::SpellsCastThisTurn(player)
        | Value::SpellsCastBeforeThisTurn(player)
        | Value::CommanderCastCount(player)
        | Value::CardTypesInGraveyard(player) => {
            player_filter_references_identity(player, identity)
        }
        Value::NoncombatDamageDealtBySourcesControlledThisTurn { player, .. }
        | Value::Devotion { player, .. } => player_filter_references_identity(player, identity),
        Value::SpellsCastThisTurnMatching { player, filter, .. } => {
            player_filter_references_identity(player, identity)
                || object_filter_references_identity(filter, identity)
        }
        _ => false,
    }
}

fn restriction_references_identity(
    restriction: &crate::effect::Restriction,
    identity: &SyntheticTargetIdentity<'_>,
) -> bool {
    use crate::effect::Restriction;

    match restriction {
        Restriction::AdditionalLandPlays(player, _)
        | Restriction::NoMaximumHandSize(player)
        | Restriction::GainLife(player)
        | Restriction::SearchLibraries(player)
        | Restriction::CastSpellsOnlyAsSorcery(player)
        | Restriction::ActivateNonManaAbilities(player)
        | Restriction::DrawCards(player)
        | Restriction::DrawExtraCards(player)
        | Restriction::PoisonCounters(player)
        | Restriction::LoseLife(player)
        | Restriction::DamageCauseLifeLoss(player)
        | Restriction::ChangeLifeTotal(player)
        | Restriction::LoseGame(player)
        | Restriction::WinGame(player)
        | Restriction::BecomeMonarch(player)
        | Restriction::LoseUnspentMana(player, _)
        | Restriction::BeTargetedPlayer(player) => {
            player_filter_references_identity(player, identity)
        }
        Restriction::CastSpellsMatching(player, filter)
        | Restriction::CastMoreThanOneSpellEachTurn(player, filter) => {
            player_filter_references_identity(player, identity)
                || object_filter_references_identity(filter, identity)
        }
        Restriction::AttackPlayerOrPlaneswalkersControlledBy { attackers, player }
        | Restriction::AttackPlayer { attackers, player } => {
            object_filter_references_identity(attackers, identity)
                || player_filter_references_identity(player, identity)
        }
        Restriction::BeTargetedPlayerFrom(player, source) => {
            player_filter_references_identity(player, identity)
                || object_filter_references_identity(source, identity)
        }
        Restriction::BlockSpecificAttacker { blockers, attacker }
        | Restriction::MustBlockSpecificAttacker { blockers, attacker }
        | Restriction::BeTargetedFrom(blockers, attacker) => {
            object_filter_references_identity(blockers, identity)
                || object_filter_references_identity(attacker, identity)
        }
        Restriction::ActivateAbilitiesOf(filter)
        | Restriction::ActivateTapAbilitiesOf(filter)
        | Restriction::ActivateNonManaAbilitiesOf(filter)
        | Restriction::Attack(filter)
        | Restriction::AttackAlone(filter)
        | Restriction::Block(filter)
        | Restriction::MustBeBlocked(filter)
        | Restriction::BlockAlone(filter)
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
        | Restriction::AttackOrBlockAlone(filter) => {
            object_filter_references_identity(filter, identity)
        }
        Restriction::PreventDamage | Restriction::AttackYouUnlessControllerPaysPerAttacker(..) => {
            false
        }
    }
}

fn effect_references_identity(effect: &Effect, identity: &SyntheticTargetIdentity<'_>) -> bool {
    let effect = structural_unwrap_render_wrappers(effect);

    if rendered_action_target(effect)
        .is_some_and(|spec| choose_spec_references_identity(spec, identity))
    {
        return true;
    }
    if let Some(gain) = effect.downcast_ref::<crate::effects::GainLifeEffect>() {
        return value_references_identity(&gain.amount, identity)
            || choose_spec_references_identity(&gain.player, identity);
    }
    if let Some(draw) = effect.downcast_ref::<crate::effects::DrawCardsEffect>() {
        return value_references_identity(&draw.count, identity)
            || player_filter_references_identity(&draw.player, identity);
    }
    if let Some(damage) = effect.downcast_ref::<crate::effects::DealDamageEffect>() {
        return value_references_identity(&damage.amount, identity)
            || choose_spec_references_identity(&damage.target, identity);
    }
    if let Some(move_to_zone) = effect.downcast_ref::<crate::effects::MoveToZoneEffect>() {
        return choose_spec_references_identity(&move_to_zone.target, identity);
    }
    if let Some(attach) = effect.downcast_ref::<crate::effects::AttachObjectsEffect>() {
        return choose_spec_references_identity(&attach.objects, identity)
            || choose_spec_references_identity(&attach.target, identity);
    }
    if let Some(fight) = effect.downcast_ref::<crate::effects::FightEffect>() {
        return choose_spec_references_identity(&fight.creature1, identity)
            || choose_spec_references_identity(&fight.creature2, identity);
    }
    if let Some(execute) = effect.downcast_ref::<crate::effects::ExecuteWithSourceEffect>() {
        return choose_spec_references_identity(&execute.source, identity)
            || effect_references_identity(&execute.effect, identity);
    }
    if let Some(cast) = effect.downcast_ref::<crate::effects::CastTaggedEffect>() {
        return identity.tag.is_some_and(|tag| cast.tag == *tag)
            || player_filter_references_identity(&cast.player, identity);
    }
    if let Some(exile_top) = effect.downcast_ref::<crate::effects::ExileTopOfLibraryEffect>() {
        return value_references_identity(&exile_top.count, identity)
            || player_filter_references_identity(&exile_top.player, identity);
    }
    if let Some(grant) = effect.downcast_ref::<crate::effects::GrantPlayTaggedEffect>() {
        return identity.tag.is_some_and(|tag| grant.tag == *tag)
            || player_filter_references_identity(&grant.player, identity);
    }
    if let Some(choose) = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>() {
        return object_filter_references_identity(&choose.filter, identity)
            || player_filter_references_identity(&choose.chooser, identity);
    }
    if let Some(prevent) = effect.downcast_ref::<crate::effects::PreventNextTimeDamageEffect>() {
        let source_references = matches!(
            &prevent.source,
            crate::effects::PreventNextTimeDamageSource::Target(spec)
                if choose_spec_references_identity(spec, identity)
        ) || matches!(
            &prevent.source,
            crate::effects::PreventNextTimeDamageSource::ChoiceMatching(filter)
                | crate::effects::PreventNextTimeDamageSource::Filter(filter)
                if object_filter_references_identity(filter, identity)
        );
        let target_references = matches!(
            &prevent.target,
            crate::effects::PreventNextTimeDamageTarget::Target(spec)
                if choose_spec_references_identity(spec, identity)
        );
        if source_references || target_references {
            return true;
        }
    }
    if let Some(cant) = effect.downcast_ref::<crate::effects::CantEffect>() {
        let start_references = matches!(
            &cant.start,
            crate::effect::RestrictionStart::NextTurn(player)
                if player_filter_references_identity(player, identity)
        );
        if start_references || restriction_references_identity(&cant.restriction, identity) {
            return true;
        }
    }

    let mut child_references = false;
    effect.visit_child_effects(&mut |child| {
        child_references |= effect_references_identity(child, identity);
    });
    child_references
}

fn synthetic_target_identity<'a>(effect: &'a Effect) -> Option<SyntheticTargetIdentity<'a>> {
    let target_only = structural_unwrap_render_wrappers(effect)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let count = target_only.target.count();
    if target_only.explicit_declaration
        || target_only.chooser.is_some()
        || !target_only.target.is_target()
        || count.max != Some(1)
        || count.dynamic_x
        || count.random
    {
        return None;
    }
    Some(SyntheticTargetIdentity {
        target: &target_only.target,
        tag: wrapped_effect_tag(effect),
    })
}

fn single_synthetic_target_consumer(
    effects: &[Effect],
) -> Option<(usize, SyntheticTargetIdentity<'_>, usize)> {
    let target_entries = effects
        .iter()
        .enumerate()
        .filter_map(|(index, effect)| {
            structural_unwrap_render_wrappers(effect)
                .downcast_ref::<crate::effects::TargetOnlyEffect>()
                .map(|target| (index, effect, target))
        })
        .collect::<Vec<_>>();
    let [(target_index, target_effect, _)] = target_entries.as_slice() else {
        return None;
    };
    let identity = synthetic_target_identity(target_effect)?;
    let consumers = effects
        .iter()
        .enumerate()
        .filter(|(index, effect)| {
            *index != *target_index && effect_references_identity(effect, &identity)
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    let [consumer_index] = consumers.as_slice() else {
        return None;
    };
    (*target_index < *consumer_index).then_some((*target_index, identity, *consumer_index))
}

fn sole_synthetic_target_entry(
    effects: &[Effect],
) -> Option<(usize, &Effect, SyntheticTargetIdentity<'_>)> {
    let entries = effects
        .iter()
        .enumerate()
        .filter_map(|(index, effect)| {
            structural_unwrap_render_wrappers(effect)
                .downcast_ref::<crate::effects::TargetOnlyEffect>()
                .map(|_| (index, effect))
        })
        .collect::<Vec<_>>();
    let [(target_index, target_effect)] = entries.as_slice() else {
        return None;
    };
    Some((
        *target_index,
        *target_effect,
        synthetic_target_identity(target_effect)?,
    ))
}

pub(super) fn synthetic_target_has_single_consumer(
    effects: &[Effect],
    target_effect: &Effect,
) -> bool {
    let Some(identity) = synthetic_target_identity(target_effect) else {
        return false;
    };
    effects
        .iter()
        .filter(|candidate| {
            !std::ptr::eq(*candidate, target_effect)
                && effect_references_identity(candidate, &identity)
        })
        .take(2)
        .count()
        == 1
}

pub(super) fn synthetic_target_has_multiple_consumers(
    effects: &[Effect],
    target_effect: &Effect,
) -> bool {
    let Some(identity) = synthetic_target_identity(target_effect) else {
        return false;
    };
    effects
        .iter()
        .filter(|candidate| {
            !std::ptr::eq(*candidate, target_effect)
                && effect_references_identity(candidate, &identity)
        })
        .take(2)
        .count()
        > 1
}

pub(super) fn target_only_pair_can_fold(effects: &[Effect], target_effect: &Effect) -> bool {
    structural_unwrap_render_wrappers(target_effect)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()
        .is_some_and(|target| {
            target.explicit_declaration
                || synthetic_target_has_single_consumer(effects, target_effect)
        })
}

/// Keep a lowering-only declaration visible when multiple top-level effect
/// trees share its target identity.
///
/// A tagged `TargetOnlyEffect` renders as bookkeeping in the generic list
/// loop, so merely retaining it in the filtered list is insufficient. Render
/// its unwrapped declaration explicitly, then let the ordinary list renderer
/// compact the consumers without the producer.
pub(super) fn describe_multi_consumer_synthetic_target_declaration(
    effects: &[Effect],
) -> Option<String> {
    let (target_index, target_effect, identity) = sole_synthetic_target_entry(effects)?;
    let consumers = effects
        .iter()
        .enumerate()
        .filter(|(index, effect)| {
            *index != target_index && effect_references_identity(effect, &identity)
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    if consumers.len() < 2
        || consumers.iter().any(|consumer| *consumer < target_index)
        || (target_index > 0
            && !describe_effect_list(&effects[..target_index])
                .trim()
                .is_empty())
    {
        return None;
    }

    let declaration = describe_effect(structural_unwrap_render_wrappers(target_effect));
    if declaration.trim().is_empty() {
        return None;
    }
    let without_target = effects
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != target_index)
        .map(|(_, effect)| effect.clone())
        .collect::<Vec<_>>();
    let rendered_consumers = describe_effect_list(&without_target);
    if rendered_consumers.trim().is_empty() {
        return None;
    }

    Some(format!(
        "{}. {}",
        declaration.trim().trim_end_matches('.'),
        capitalize_first(rendered_consumers.trim().trim_end_matches('.'))
    ))
}

fn describe_target_destroy_attached(
    identity: &SyntheticTargetIdentity<'_>,
    consumer: &Effect,
) -> Option<String> {
    let tag = identity.tag?;
    let destroy = structural_unwrap_render_wrappers(consumer)
        .downcast_ref::<crate::effects::DestroyEffect>()?;
    let ChooseSpec::All(filter) = destroy.spec.base() else {
        return None;
    };
    let matching = filter
        .tagged_constraints
        .iter()
        .filter(|constraint| {
            constraint.tag == *tag
                && constraint.relation
                    == crate::filter::TaggedOpbjectRelation::AttachedToTaggedObject
        })
        .count();
    if matching != 1
        || filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag != *tag
                || constraint.relation
                    != crate::filter::TaggedOpbjectRelation::AttachedToTaggedObject
        })
    {
        return None;
    }

    let mut attachment = filter.clone();
    attachment.tagged_constraints.clear();
    attachment.zone = None;
    let described = attachment.description();
    let noun = strip_indefinite_article(&described).trim();
    if noun.is_empty() || noun == "permanent" {
        return None;
    }
    Some(format!(
        "Destroy all {} attached to {}",
        pluralize_noun_phrase(noun),
        describe_choose_spec(identity.target)
    ))
}

fn describe_target_controller_characteristic_damage(
    identity: &SyntheticTargetIdentity<'_>,
    consumer: &Effect,
) -> Option<String> {
    let tag = identity.tag?;
    let damage = structural_unwrap_render_wrappers(consumer)
        .downcast_ref::<crate::effects::DealDamageEffect>()?;
    if damage.source_is_combat || damage.unpreventable {
        return None;
    }
    let recipient_ref = match damage.target.unhinted() {
        ChooseSpec::Player(
            PlayerFilter::ControllerOf(reference) | PlayerFilter::AliasedControllerOf(reference),
        ) => reference,
        _ => return None,
    };
    if !matches!(
        recipient_ref,
        crate::target::ObjectRef::Tagged(found) if found == tag
    ) {
        return None;
    }

    let (spec, characteristic) = match damage.amount.unhinted() {
        Value::PowerOf(spec) => (spec.as_ref(), "power"),
        Value::ToughnessOf(spec) => (spec.as_ref(), "toughness"),
        Value::ManaValueOf(spec) => (spec.as_ref(), "mana value"),
        _ => return None,
    };
    if !damage
        .amount
        .has_surface_hint(ironsmith_core::ValueSurfaceHint::EqualTo)
        || !choose_spec_references_identity(spec, identity)
    {
        return None;
    }

    let target = describe_choose_spec(identity.target);
    let reference = tagged_reference_noun_from_target(identity.target)?;
    Some(format!(
        "Deal damage to {target}'s controller equal to {reference}'s {characteristic}"
    ))
}

fn effect_tree_has_same_name_reference(effect: &Effect, tag: &TagKey) -> bool {
    let effect = structural_unwrap_render_wrappers(effect);
    if effect
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()
        .is_some_and(|choose| {
            choose.filter.tagged_constraints.iter().any(|constraint| {
                constraint.tag == *tag
                    && constraint.relation == crate::filter::TaggedOpbjectRelation::SameNameAsTagged
            })
        })
    {
        return true;
    }
    let mut found = false;
    effect.visit_child_effects(&mut |child| {
        found |= effect_tree_has_same_name_reference(child, tag);
    });
    found
}

fn effect_tree_casts_tag(effect: &Effect, tag: &TagKey) -> bool {
    let effect = structural_unwrap_render_wrappers(effect);
    if effect
        .downcast_ref::<crate::effects::CastTaggedEffect>()
        .is_some_and(|cast| cast.tag == *tag)
    {
        return true;
    }
    let mut found = false;
    effect.visit_child_effects(&mut |child| {
        found |= effect_tree_casts_tag(child, tag);
    });
    found
}

fn effect_tree_executes_with_tagged_source(effect: &Effect, tag: &TagKey) -> bool {
    let effect = structural_unwrap_render_wrappers(effect);
    if effect
        .downcast_ref::<crate::effects::ExecuteWithSourceEffect>()
        .is_some_and(
            |execute| matches!(execute.source.base(), ChooseSpec::Tagged(found) if found == tag),
        )
    {
        return true;
    }
    let mut found = false;
    effect.visit_child_effects(&mut |child| {
        found |= effect_tree_executes_with_tagged_source(child, tag);
    });
    found
}

fn replace_tagged_source_subject(
    rendered: &str,
    identity: &SyntheticTargetIdentity<'_>,
) -> Option<String> {
    let reference = tagged_reference_noun_from_target(identity.target)?;
    let target = describe_choose_spec(identity.target);
    let capitalized_reference = capitalize_first(reference);
    if rendered.contains(&capitalized_reference) {
        return Some(rendered.replacen(&capitalized_reference, &capitalize_first(&target), 1));
    }
    rendered
        .contains(reference)
        .then(|| rendered.replacen(reference, &target, 1))
}

/// Fold a lowering-only target declaration into its sole consumer.
///
/// The runtime target producer remains untouched. Rendering elides it only
/// when one later top-level effect tree consumes the exact target identity.
/// Multiple consumers retain the declaration so their shared tag remains
/// visible and unambiguous.
pub(super) fn describe_single_consumer_synthetic_target_fold(effects: &[Effect]) -> Option<String> {
    let (target_index, identity, consumer_index) = single_synthetic_target_consumer(effects)?;
    let consumer = &effects[consumer_index];

    if effects.len() == 2
        && let Some(tagged) = effects[target_index].downcast_ref::<crate::effects::TaggedEffect>()
        && let Some(cant) = consumer.downcast_ref::<crate::effects::CantEffect>()
        && let Some(rendered) = describe_tagged_target_then_cant_restriction(tagged, cant)
    {
        // Preserve authored "can't be blocked ... except by ..." surfaces
        // before the generic synthetic-target fold inverts the blocker
        // filter into a "can't block" subject.
        return Some(rendered);
    }

    if effects.len() == 2
        && let Some(rendered) = describe_target_destroy_attached(&identity, consumer)
    {
        return Some(rendered);
    }

    if effects.len() == 2
        && let Some(rendered) =
            describe_target_controller_characteristic_damage(&identity, consumer)
    {
        return Some(rendered);
    }

    let without_target = effects
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != target_index)
        .map(|(_, effect)| effect.clone())
        .collect::<Vec<_>>();
    let rendered = capitalize_first(&describe_effect_list(&without_target));
    let target_text = describe_choose_spec(identity.target);
    let rendered_consumer = describe_effect(consumer);
    if rendered_consumer
        .to_ascii_lowercase()
        .contains(&target_text.to_ascii_lowercase())
    {
        return Some(rendered);
    }

    if let Some(tag) = identity.tag
        && effect_tree_has_same_name_reference(consumer, tag)
        && rendered.contains("with that name")
    {
        return Some(rendered.replacen(
            "with that name",
            &format!("with the same name as {target_text}"),
            1,
        ));
    }
    if let Some(tag) = identity.tag
        && effect_tree_casts_tag(consumer, tag)
        && rendered.contains("cast it")
    {
        return Some(rendered.replacen("cast it", &format!("cast {target_text}"), 1));
    }
    if let Some(tag) = identity.tag
        && effect_tree_executes_with_tagged_source(consumer, tag)
    {
        return replace_tagged_source_subject(&rendered, &identity);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter::{TaggedObjectConstraint, TaggedOpbjectRelation};

    #[test]
    fn characteristic_except_by_restriction_folds_to_target_subject() {
        let targeted = TagKey::from("targeted_0");
        let tagged = Effect::new(crate::effects::TargetOnlyEffect::new(
            ChooseSpec::target_creature(),
        ))
        .tag(targeted.clone());
        let mut attacker = ObjectFilter::creature();
        attacker.tagged_constraints.push(TaggedObjectConstraint {
            tag: targeted,
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        let mut blockers = ObjectFilter::creature()
            .without_type(CardType::Artifact)
            .without_colors(crate::color::ColorSet::RED);
        blockers.set_union_connective(crate::filter::ObjectFilterUnionConnective::AndOr);
        let cant = Effect::new(crate::effects::CantEffect::until_end_of_turn(
            crate::effect::Restriction::BlockSpecificAttacker { blockers, attacker },
        ));

        assert_eq!(
            describe_effect_list(&[tagged, cant]),
            "Target creature can't be blocked this turn except by artifact creatures and/or red creatures"
        );
    }
}
