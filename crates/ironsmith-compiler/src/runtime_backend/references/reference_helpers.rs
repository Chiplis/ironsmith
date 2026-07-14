use crate::cards::builders::{
    CardTextError, IT_TAG, PlayerAst, THIS_WAY_SACRIFICED_TAG, TagKey, TargetAst,
};
use crate::effect::{EventValueSpec, Restriction, Value};
use crate::filter::{Comparison, ObjectFilter, ObjectRef, PlayerFilter, TaggedOpbjectRelation};
use crate::target::{ChooseSpec, ChooseSpecSurfaceHint, SourceReferenceSurface};
use crate::zone::Zone;

use super::reference_model::ReferenceEnv;

pub(crate) fn is_sacrificed_object_reference_tag(tag: &str) -> bool {
    tag == "sacrificed"
        || tag.starts_with("sacrificed_")
        || tag.starts_with("sacrifice_cost_")
        || tag.starts_with("__sentence_helper_sacrificed")
}

pub(crate) fn is_you_player_filter(filter: &PlayerFilter) -> bool {
    match filter {
        PlayerFilter::You => true,
        PlayerFilter::Target(inner) | PlayerFilter::AliasedTarget(inner) => {
            is_you_player_filter(inner)
        }
        _ => false,
    }
}

/// Preserve that an object-relative player was introduced as the discourse
/// antecedent for a later "that player"/"they" reference. The aliased forms
/// resolve identically at runtime but keep that player surface distinct from
/// an explicit later "its controller" or "its owner" reference.
pub(crate) fn as_followup_player_alias(filter: PlayerFilter) -> PlayerFilter {
    match filter {
        PlayerFilter::Target(inner) => PlayerFilter::AliasedTarget(inner),
        PlayerFilter::ControllerOf(reference) => PlayerFilter::AliasedControllerOf(reference),
        PlayerFilter::OwnerOf(reference) => PlayerFilter::AliasedOwnerOf(reference),
        other => other,
    }
}

pub(crate) fn resolve_unless_player_filter(
    player: PlayerAst,
    refs: &ReferenceEnv,
    previous_last_player_filter: Option<PlayerFilter>,
) -> Result<PlayerFilter, CardTextError> {
    if matches!(player, PlayerAst::That)
        && !refs.iterated_player
        && refs
            .known_last_player_filter()
            .is_some_and(is_you_player_filter)
        && previous_last_player_filter
            .as_ref()
            .is_some_and(|filter| !is_you_player_filter(filter))
    {
        return previous_last_player_filter.ok_or_else(|| {
            CardTextError::InvariantViolation(
                "expected previous non-you player filter for unless-player resolution".to_string(),
            )
        });
    }
    resolve_non_target_player_filter(player, refs)
}

pub(crate) fn resolve_non_target_player_filter(
    player: PlayerAst,
    refs: &ReferenceEnv,
) -> Result<PlayerFilter, CardTextError> {
    match player {
        PlayerAst::You => Ok(PlayerFilter::You),
        PlayerAst::Any => Ok(PlayerFilter::Any),
        PlayerAst::Chosen => Ok(PlayerFilter::ChosenPlayer),
        PlayerAst::Defending => Ok(PlayerFilter::Defending),
        PlayerAst::Attacking => Ok(PlayerFilter::Attacking),
        PlayerAst::MostCardsInHand => Ok(PlayerFilter::MostCardsInHand),
        PlayerAst::MostLifeTied => Ok(PlayerFilter::MostLifeTied),
        PlayerAst::LowestLifeTied => Ok(PlayerFilter::LowestLifeTied),
        PlayerAst::Target | PlayerAst::TargetOpponent => Err(CardTextError::ParseError(
            "target player requires explicit targeting".to_string(),
        )),
        PlayerAst::Opponent => Ok(PlayerFilter::Opponent),
        PlayerAst::NotYou => {
            if let Some(excluded) = refs.known_last_player_filter()
                && !is_you_player_filter(excluded)
                && !matches!(excluded, PlayerFilter::Any | PlayerFilter::NotYou)
                && !excluded.mentions_iterated_player()
            {
                Ok(PlayerFilter::excluding(PlayerFilter::Any, excluded.clone()))
            } else {
                Ok(PlayerFilter::NotYou)
            }
        }
        PlayerAst::That => {
            let filter = if refs.iterated_player {
                PlayerFilter::IteratedPlayer
            } else if let Some(filter) = refs.known_last_player_filter()
                && !filter.mentions_iterated_player()
            {
                filter.clone()
            } else if let Some(filter) = refs.known_last_player_filter() {
                filter.clone()
            } else {
                PlayerFilter::IteratedPlayer
            };
            Ok(as_followup_player_alias(filter))
        }
        PlayerAst::ThatPlayerOrTargetController => {
            Ok(PlayerFilter::TargetPlayerOrControllerOfTarget)
        }
        PlayerAst::ItsController => {
            if let Some(tag) = refs.known_last_object_tag() {
                Ok(PlayerFilter::ControllerOf(ObjectRef::tagged(tag.clone())))
            } else {
                Ok(PlayerFilter::ControllerOf(ObjectRef::Target))
            }
        }
        PlayerAst::ItsOwner => {
            if let Some(tag) = refs.known_last_object_tag() {
                Ok(PlayerFilter::OwnerOf(ObjectRef::tagged(tag.clone())))
            } else {
                Ok(PlayerFilter::OwnerOf(ObjectRef::Target))
            }
        }
        PlayerAst::Implicit => {
            if refs.iterated_player
                && refs
                    .known_last_object_tag()
                    .is_some_and(|tag| tag.as_str() == IT_TAG)
            {
                Ok(PlayerFilter::You)
            } else if refs.iterated_player {
                Ok(PlayerFilter::IteratedPlayer)
            } else {
                Ok(PlayerFilter::You)
            }
        }
    }
}

pub(crate) fn infer_player_filter_from_object_filter(
    filter: &ObjectFilter,
) -> Option<PlayerFilter> {
    if let Some(owner) = &filter.owner {
        return Some(owner.clone());
    }
    if let Some(controller) = &filter.controller {
        return Some(controller.clone());
    }
    for constraint in &filter.tagged_constraints {
        if matches!(
            constraint.relation,
            TaggedOpbjectRelation::SameControllerAsTagged
        ) {
            return Some(PlayerFilter::AliasedControllerOf(ObjectRef::tagged(
                constraint.tag.clone(),
            )));
        }
    }
    filter
        .any_of
        .iter()
        .find_map(infer_player_filter_from_object_filter)
}

fn push_target_player_filter_choices(filter: &PlayerFilter, choices: &mut Vec<ChooseSpec>) {
    match filter {
        PlayerFilter::Target(inner) => {
            let choice = ChooseSpec::target(ChooseSpec::Player((**inner).clone()));
            if !choices.contains(&choice) {
                choices.push(choice);
            }
        }
        PlayerFilter::CardsInHandAtLeastMoreThanYou { base, .. }
        | PlayerFilter::HasMoreLifeThanYou { base }
        | PlayerFilter::MaxSpeed { base, .. } => {
            push_target_player_filter_choices(base, choices);
        }
        PlayerFilter::Excluding { base, excluded } => {
            push_target_player_filter_choices(base, choices);
            push_target_player_filter_choices(excluded, choices);
        }
        PlayerFilter::Any
        | PlayerFilter::You
        | PlayerFilter::NotYou
        | PlayerFilter::Opponent
        | PlayerFilter::Teammate
        | PlayerFilter::Active
        | PlayerFilter::Defending
        | PlayerFilter::Attacking
        | PlayerFilter::DamagedPlayer
        | PlayerFilter::EffectController
        | PlayerFilter::Specific(_)
        | PlayerFilter::MostLifeTied
        | PlayerFilter::LowestLifeTied
        | PlayerFilter::MostCardsInHand
        | PlayerFilter::CastCardTypeThisTurn(_)
        | PlayerFilter::ChosenPlayer
        | PlayerFilter::TaggedPlayer(_)
        | PlayerFilter::IteratedPlayer
        | PlayerFilter::TargetPlayerOrControllerOfTarget
        | PlayerFilter::ControllerOf(_)
        | PlayerFilter::OwnerOf(_)
        | PlayerFilter::AliasedTarget(_)
        | PlayerFilter::AliasedOwnerOf(_)
        | PlayerFilter::AliasedControllerOf(_) => {}
    }
}

fn append_object_filter_target_player_choices(
    filter: &ObjectFilter,
    choices: &mut Vec<ChooseSpec>,
) {
    if let Some(owner) = &filter.owner {
        push_target_player_filter_choices(owner, choices);
    }
    if let Some(controller) = &filter.controller {
        push_target_player_filter_choices(controller, choices);
    }
    if let Some(attached_to_player) = &filter.attached_to_player {
        push_target_player_filter_choices(attached_to_player, choices);
    }
    if let Some(attached_to) = filter.attached_to_object.as_deref() {
        append_object_filter_target_player_choices(attached_to, choices);
    }
    for branch in &filter.any_of {
        append_object_filter_target_player_choices(branch, choices);
    }
}

fn resolve_object_ref(reference: &ObjectRef, refs: &ReferenceEnv) -> ObjectRef {
    match reference {
        ObjectRef::Tagged(tag) if tag.as_str() == IT_TAG => refs
            .known_last_object_tag()
            .cloned()
            .map(ObjectRef::tagged)
            .unwrap_or(ObjectRef::Target),
        ObjectRef::Tagged(tag) => refs
            .snapshot_tag_aliases
            .iter()
            .find(|(alias, _)| alias == tag.as_str())
            .map(|(_, concrete)| ObjectRef::tagged(concrete.as_str()))
            .unwrap_or_else(|| reference.clone()),
        _ => reference.clone(),
    }
}

fn resolve_contextual_player_filter(
    filter: &PlayerFilter,
    refs: &ReferenceEnv,
) -> Result<PlayerFilter, CardTextError> {
    Ok(match filter {
        PlayerFilter::IteratedPlayer => {
            if refs.iterated_player {
                PlayerFilter::IteratedPlayer
            } else {
                refs.known_last_player_filter()
                    .filter(|filter| !filter.mentions_iterated_player())
                    .cloned()
                    .unwrap_or(PlayerFilter::IteratedPlayer)
            }
        }
        PlayerFilter::Target(inner) => {
            PlayerFilter::Target(Box::new(resolve_contextual_player_filter(inner, refs)?))
        }
        PlayerFilter::AliasedTarget(inner) => {
            PlayerFilter::AliasedTarget(Box::new(resolve_contextual_player_filter(inner, refs)?))
        }
        PlayerFilter::Excluding { base, excluded } => PlayerFilter::Excluding {
            base: Box::new(resolve_contextual_player_filter(base, refs)?),
            excluded: Box::new(resolve_contextual_player_filter(excluded, refs)?),
        },
        PlayerFilter::ControllerOf(reference) => {
            PlayerFilter::ControllerOf(resolve_object_ref(reference, refs))
        }
        PlayerFilter::OwnerOf(reference) => {
            PlayerFilter::OwnerOf(resolve_object_ref(reference, refs))
        }
        PlayerFilter::AliasedOwnerOf(reference) => {
            PlayerFilter::AliasedOwnerOf(resolve_object_ref(reference, refs))
        }
        PlayerFilter::AliasedControllerOf(reference) => {
            PlayerFilter::AliasedControllerOf(resolve_object_ref(reference, refs))
        }
        _ => filter.clone(),
    })
}

fn resolve_object_filter_comparison(
    comparison: &Comparison,
    refs: &ReferenceEnv,
) -> Result<Comparison, CardTextError> {
    Ok(match comparison {
        Comparison::EqualExpr(value) => {
            Comparison::EqualExpr(Box::new(resolve_value_it_tag(value, refs)?))
        }
        Comparison::NotEqualExpr(value) => {
            Comparison::NotEqualExpr(Box::new(resolve_value_it_tag(value, refs)?))
        }
        Comparison::LessThanExpr(value) => {
            Comparison::LessThanExpr(Box::new(resolve_value_it_tag(value, refs)?))
        }
        Comparison::LessThanOrEqualExpr(value) => {
            Comparison::LessThanOrEqualExpr(Box::new(resolve_value_it_tag(value, refs)?))
        }
        Comparison::GreaterThanExpr(value) => {
            Comparison::GreaterThanExpr(Box::new(resolve_value_it_tag(value, refs)?))
        }
        Comparison::GreaterThanOrEqualExpr(value) => {
            Comparison::GreaterThanOrEqualExpr(Box::new(resolve_value_it_tag(value, refs)?))
        }
        _ => comparison.clone(),
    })
}

fn resolve_object_filter_player_refs(
    filter: &ObjectFilter,
    refs: &ReferenceEnv,
) -> Result<ObjectFilter, CardTextError> {
    let mut resolved = filter.clone();
    if let Some(controller) = resolved.controller.as_mut() {
        *controller = resolve_contextual_player_filter(controller, refs)?;
    }
    if let Some(cast_by) = resolved.cast_by.as_mut() {
        *cast_by = resolve_contextual_player_filter(cast_by, refs)?;
    }
    if let Some(owner) = resolved.owner.as_mut() {
        *owner = resolve_contextual_player_filter(owner, refs)?;
    }
    if let Some(power) = resolved.power.as_mut() {
        *power = resolve_object_filter_comparison(power, refs)?;
    }
    if let Some(toughness) = resolved.toughness.as_mut() {
        *toughness = resolve_object_filter_comparison(toughness, refs)?;
    }
    if let Some(mana_value) = resolved.mana_value.as_mut() {
        *mana_value = resolve_object_filter_comparison(mana_value, refs)?;
    }
    if let Some(color_count) = resolved.color_count.as_mut() {
        *color_count = resolve_object_filter_comparison(color_count, refs)?;
    }
    if let Some(targets_player) = resolved.targets_player.as_mut() {
        *targets_player = resolve_contextual_player_filter(targets_player, refs)?;
    }
    if let Some(targets_object) = resolved.targets_object.as_mut() {
        **targets_object = resolve_object_filter_player_refs(targets_object, refs)?;
    }
    if let Some(targets_only_player) = resolved.targets_only_player.as_mut() {
        *targets_only_player = resolve_contextual_player_filter(targets_only_player, refs)?;
    }
    if let Some(targets_only_object) = resolved.targets_only_object.as_mut() {
        **targets_only_object = resolve_object_filter_player_refs(targets_only_object, refs)?;
    }
    if let Some(targetability) = resolved.could_be_targeted_by.as_mut() {
        targetability.stack_object = resolve_object_ref(&targetability.stack_object, refs);
    }
    if let Some(attacking_player) = resolved
        .attacking_player_or_planeswalker_controlled_by
        .as_mut()
    {
        *attacking_player = resolve_contextual_player_filter(attacking_player, refs)?;
    }
    if let Some(attached_to_player) = resolved.attached_to_player.as_mut() {
        *attached_to_player = resolve_contextual_player_filter(attached_to_player, refs)?;
    }
    if let Some(attached_to_object) = resolved.attached_to_object.as_mut() {
        **attached_to_object = resolve_object_filter_player_refs(attached_to_object, refs)?;
    }
    if let Some(entered_controller) = resolved.entered_battlefield_controller.as_mut() {
        *entered_controller = resolve_contextual_player_filter(entered_controller, refs)?;
    }
    for nested in &mut resolved.any_of {
        *nested = resolve_object_filter_player_refs(nested, refs)?;
    }
    Ok(resolved)
}

pub(crate) fn resolve_it_tag(
    filter: &ObjectFilter,
    refs: &ReferenceEnv,
) -> Result<ObjectFilter, CardTextError> {
    let mut resolved = resolve_object_filter_player_refs(filter, refs)?;
    if let Some(attached_to_object) = resolved.attached_to_object.as_mut() {
        **attached_to_object = resolve_it_tag(attached_to_object, refs)?;
    }
    for nested in &mut resolved.any_of {
        *nested = resolve_it_tag(nested, refs)?;
    }
    if !refs.snapshot_tag_aliases.is_empty() {
        for constraint in &mut resolved.tagged_constraints {
            if let Some((_, concrete)) = refs
                .snapshot_tag_aliases
                .iter()
                .find(|(alias, _)| alias == constraint.tag.as_str())
            {
                constraint.tag = TagKey::from(concrete.as_str());
            }
        }
    }
    if let Some(tag) = refs.known_last_object_tag()
        && tag.as_str() != crate::tag::SOURCE_EXILED_TAG
        && tag.as_str() != "triggering"
    {
        for constraint in &mut resolved.tagged_constraints {
            if constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG {
                constraint.tag = tag.clone();
            }
            if constraint.tag.as_str() == "__public_revealed"
                && (tag.as_str().starts_with("revealed_")
                    || tag.as_str().starts_with("__sentence_helper_revealed"))
            {
                constraint.tag = tag.clone();
            }
        }
    }
    if !filter
        .tagged_constraints
        .iter()
        .any(|constraint| constraint.tag.as_str() == IT_TAG)
    {
        return Ok(resolved);
    }

    let Some(tag) = refs.known_last_object_tag() else {
        let mut saw_it_constraint = false;
        let mut preserved_runtime_it_constraint = false;
        resolved.tagged_constraints.retain(|constraint| {
            if constraint.tag.as_str() != IT_TAG {
                return true;
            }
            // Only relations with an explicit runtime interpretation may keep
            // an unresolved `__it__`: delayed triggers bind lesser mana value
            // to their triggering object, while same-name predicates use an
            // existential comparison set. Immediate relations such as
            // "Equipment attached to that creature" must not leak an unbound
            // tag into a merged target filter.
            if matches!(
                constraint.relation,
                TaggedOpbjectRelation::ManaValueLtTagged | TaggedOpbjectRelation::SameNameAsTagged
            ) {
                preserved_runtime_it_constraint = true;
                return true;
            }
            saw_it_constraint = true;
            false
        });

        if saw_it_constraint
            && refs.has_source_object_antecedent()
            && resolved == ObjectFilter::default()
        {
            resolved.source = true;
            return Ok(resolved);
        }
        if saw_it_constraint
            && matches!(
                resolved.zone,
                Some(Zone::Hand | Zone::Library | Zone::Graveyard | Zone::Exile)
            )
            && let Some(player_filter) = refs.known_last_player_filter().cloned()
        {
            if resolved.owner.is_none() {
                resolved.owner = Some(as_followup_player_alias(player_filter));
            }
            return Ok(resolved);
        }
        if saw_it_constraint
            && resolved == ObjectFilter::default()
            && let Some(player_filter) = refs.known_last_player_filter().cloned()
        {
            resolved.zone = Some(Zone::Hand);
            resolved.owner = Some(as_followup_player_alias(player_filter));
            return Ok(resolved);
        }
        if saw_it_constraint && resolved == ObjectFilter::default() {
            resolved.source = true;
            return Ok(resolved);
        }
        if saw_it_constraint {
            return Ok(resolved);
        }

        if preserved_runtime_it_constraint {
            return Ok(resolved);
        }

        return Err(CardTextError::ParseError(
            "unable to resolve 'it' without prior reference".to_string(),
        ));
    };

    for constraint in &mut resolved.tagged_constraints {
        if constraint.tag.as_str() == IT_TAG {
            constraint.tag = tag.clone();
        }
    }
    Ok(resolved)
}

pub(crate) fn resolve_it_tag_key(
    tag: &TagKey,
    refs: &ReferenceEnv,
) -> Result<TagKey, CardTextError> {
    if let Some((_, concrete)) = refs
        .snapshot_tag_aliases
        .iter()
        .find(|(alias, _)| alias == tag.as_str())
    {
        return Ok(TagKey::from(concrete.as_str()));
    }
    if tag.as_str() == THIS_WAY_SACRIFICED_TAG {
        let resolved = refs.known_last_object_tag().ok_or_else(|| {
            CardTextError::ParseError(
                "unable to resolve 'sacrificed this way' without a prior sacrifice".to_string(),
            )
        })?;
        if !is_sacrificed_object_reference_tag(resolved.as_str()) {
            return Err(CardTextError::ParseError(
                "'sacrificed this way' does not refer to the prior object".to_string(),
            ));
        }
        return Ok(resolved.clone());
    }
    if tag.as_str() != IT_TAG {
        return Ok(tag.clone());
    }
    let resolved = refs.known_last_object_tag().ok_or_else(|| {
        CardTextError::ParseError("unable to resolve 'it' without prior reference".to_string())
    })?;
    Ok(TagKey::from(resolved.as_str()))
}

pub(crate) fn object_filter_as_tagged_reference(filter: &ObjectFilter) -> Option<TagKey> {
    if filter.tagged_constraints.len() != 1 {
        return None;
    }
    let constraint = &filter.tagged_constraints[0];
    if !matches!(constraint.relation, TaggedOpbjectRelation::IsTaggedObject) {
        return None;
    }

    let mut bare = filter.clone();
    bare.tagged_constraints.clear();
    bare.zone = None;
    bare.token = false;
    if bare == ObjectFilter::default() {
        Some(constraint.tag.clone())
    } else {
        None
    }
}

pub(crate) fn watch_tag_from_filter(filter: &ObjectFilter) -> Option<TagKey> {
    let mut tag: Option<TagKey> = None;
    for constraint in &filter.tagged_constraints {
        if !matches!(constraint.relation, TaggedOpbjectRelation::IsTaggedObject) {
            continue;
        }
        match &tag {
            Some(existing) if existing.as_str() != constraint.tag.as_str() => return None,
            Some(_) => {}
            None => tag = Some(constraint.tag.clone()),
        }
    }
    tag
}

pub(crate) fn resolve_restriction_it_tag(
    restriction: &Restriction,
    refs: &ReferenceEnv,
) -> Result<Restriction, CardTextError> {
    let resolved = match restriction {
        Restriction::AdditionalLandPlays(player, count) => Restriction::additional_land_plays(
            resolve_contextual_player_filter(player, refs)?,
            *count,
        ),
        Restriction::GainLife(player) => {
            Restriction::gain_life(resolve_contextual_player_filter(player, refs)?)
        }
        Restriction::SearchLibraries(player) => {
            Restriction::search_libraries(resolve_contextual_player_filter(player, refs)?)
        }
        Restriction::CastSpellsMatching(player, filter) => Restriction::cast_spells_matching(
            resolve_contextual_player_filter(player, refs)?,
            resolve_it_tag(filter, refs)?,
        ),
        Restriction::ActivateNonManaAbilities(player) => Restriction::activate_non_mana_abilities(
            resolve_contextual_player_filter(player, refs)?,
        ),
        Restriction::CastMoreThanOneSpellEachTurn(player, filter) => {
            Restriction::CastMoreThanOneSpellEachTurn(
                resolve_contextual_player_filter(player, refs)?,
                resolve_it_tag(filter, refs)?,
            )
        }
        Restriction::DrawCards(player) => {
            Restriction::DrawCards(resolve_contextual_player_filter(player, refs)?)
        }
        Restriction::DrawExtraCards(player) => {
            Restriction::DrawExtraCards(resolve_contextual_player_filter(player, refs)?)
        }
        Restriction::LoseLife(player) => {
            Restriction::LoseLife(resolve_contextual_player_filter(player, refs)?)
        }
        Restriction::ChangeLifeTotal(player) => {
            Restriction::ChangeLifeTotal(resolve_contextual_player_filter(player, refs)?)
        }
        Restriction::LoseGame(player) => {
            Restriction::LoseGame(resolve_contextual_player_filter(player, refs)?)
        }
        Restriction::WinGame(player) => {
            Restriction::WinGame(resolve_contextual_player_filter(player, refs)?)
        }
        Restriction::BecomeMonarch(player) => {
            Restriction::BecomeMonarch(resolve_contextual_player_filter(player, refs)?)
        }
        Restriction::Attack(filter) => Restriction::attack(resolve_it_tag(filter, refs)?),
        Restriction::AttackPlayerOrPlaneswalkersControlledBy { attackers, player } => {
            Restriction::attack_player_or_planeswalkers_controlled_by(
                resolve_it_tag(attackers, refs)?,
                resolve_contextual_player_filter(player, refs)?,
            )
        }
        Restriction::Block(filter) => Restriction::block(resolve_it_tag(filter, refs)?),
        Restriction::BlockSpecificAttacker { blockers, attacker } => {
            Restriction::block_specific_attacker(
                resolve_it_tag(blockers, refs)?,
                resolve_it_tag(attacker, refs)?,
            )
        }
        Restriction::MustBlockSpecificAttacker { blockers, attacker } => {
            Restriction::must_block_specific_attacker(
                resolve_it_tag(blockers, refs)?,
                resolve_it_tag(attacker, refs)?,
            )
        }
        Restriction::MustBeBlocked(filter) => {
            Restriction::must_be_blocked(resolve_it_tag(filter, refs)?)
        }
        Restriction::Untap(filter) => Restriction::untap(resolve_it_tag(filter, refs)?),
        Restriction::BeBlocked(filter) => Restriction::be_blocked(resolve_it_tag(filter, refs)?),
        Restriction::BeDestroyed(filter) => {
            Restriction::be_destroyed(resolve_it_tag(filter, refs)?)
        }
        Restriction::BeRegenerated(filter) => {
            Restriction::be_regenerated(resolve_it_tag(filter, refs)?)
        }
        Restriction::BeSacrificed(filter) => {
            Restriction::be_sacrificed(resolve_it_tag(filter, refs)?)
        }
        Restriction::HaveCountersPlaced(filter) => {
            Restriction::have_counters_placed(resolve_it_tag(filter, refs)?)
        }
        Restriction::BeTargeted(filter) => Restriction::be_targeted(resolve_it_tag(filter, refs)?),
        Restriction::BeTargetedFrom(filter, source_filter) => Restriction::be_targeted_from(
            resolve_it_tag(filter, refs)?,
            resolve_it_tag(source_filter, refs)?,
        ),
        Restriction::BeTargetedPlayer(player) => {
            Restriction::BeTargetedPlayer(resolve_contextual_player_filter(player, refs)?)
        }
        Restriction::BeTargetedPlayerFrom(player, source_filter) => {
            Restriction::be_targeted_player_from(
                resolve_contextual_player_filter(player, refs)?,
                resolve_it_tag(source_filter, refs)?,
            )
        }
        Restriction::BeCountered(filter) => {
            Restriction::be_countered(resolve_it_tag(filter, refs)?)
        }
        Restriction::Transform(filter) => Restriction::transform(resolve_it_tag(filter, refs)?),
        Restriction::PhaseOut(filter) => Restriction::phase_out(resolve_it_tag(filter, refs)?),
        Restriction::AttackOrBlock(filter) => {
            Restriction::attack_or_block(resolve_it_tag(filter, refs)?)
        }
        Restriction::ActivateAbilitiesOf(filter) => {
            Restriction::activate_abilities_of(resolve_it_tag(filter, refs)?)
        }
        Restriction::ActivateTapAbilitiesOf(filter) => {
            Restriction::activate_tap_abilities_of(resolve_it_tag(filter, refs)?)
        }
        Restriction::ActivateNonManaAbilitiesOf(filter) => {
            Restriction::activate_non_mana_abilities_of(resolve_it_tag(filter, refs)?)
        }
        _ => restriction.clone(),
    };
    Ok(resolved)
}

pub(crate) fn resolve_choose_spec_it_tag(
    spec: &ChooseSpec,
    refs: &ReferenceEnv,
) -> Result<ChooseSpec, CardTextError> {
    match spec {
        ChooseSpec::SurfaceHinted { spec, hints } => Ok(ChooseSpec::SurfaceHinted {
            spec: Box::new(resolve_choose_spec_it_tag(spec, refs)?),
            hints: hints.clone(),
        }),
        ChooseSpec::Tagged(tag) if tag.as_str() == IT_TAG => {
            if refs
                .known_last_object_tag()
                .is_some_and(|tag| tag.as_str() == IT_TAG)
            {
                return Ok(if refs.iterated_player || refs.iterated_object {
                    ChooseSpec::Iterated
                } else {
                    ChooseSpec::Tagged(TagKey::from(IT_TAG))
                });
            }
            if let Some(resolved) = refs.known_last_object_tag() {
                return Ok(ChooseSpec::Tagged(TagKey::from(resolved.as_str())));
            }
            if refs.has_source_object_antecedent() {
                return Ok(ChooseSpec::Source);
            }
            if let Some(player_filter) = refs.known_last_player_filter().cloned() {
                let filter = ObjectFilter {
                    zone: Some(Zone::Hand),
                    owner: Some(as_followup_player_alias(player_filter)),
                    ..Default::default()
                };
                return Ok(ChooseSpec::Object(filter));
            }
            Ok(ChooseSpec::Source)
        }
        ChooseSpec::Tagged(tag) => Ok(ChooseSpec::Tagged(tag.clone())),
        ChooseSpec::Object(filter) => {
            let resolved = resolve_it_tag(filter, refs)?;
            if resolved.source && resolved.zone != Some(Zone::Exile) {
                Ok(source_reference_hinted_spec(
                    ChooseSpec::Source,
                    resolved.source_surface.clone(),
                ))
            } else if let Some(tag) = object_filter_as_tagged_reference(&resolved) {
                Ok(ChooseSpec::Tagged(tag))
            } else {
                Ok(ChooseSpec::Object(resolved))
            }
        }
        ChooseSpec::Target(inner) => {
            let resolved = resolve_choose_spec_it_tag(inner, refs)?;
            if matches!(resolved.base(), ChooseSpec::Source) {
                Ok(resolved)
            } else {
                Ok(ChooseSpec::Target(Box::new(resolved)))
            }
        }
        ChooseSpec::WithCount(inner, count) => Ok(ChooseSpec::WithCount(
            Box::new(resolve_choose_spec_it_tag(inner, refs)?),
            *count,
        )),
        ChooseSpec::WithCountValue(inner, count, value) => Ok(ChooseSpec::WithCountValue(
            Box::new(resolve_choose_spec_it_tag(inner, refs)?),
            *count,
            resolve_value_it_tag(value, refs)?,
        )),
        ChooseSpec::All(filter) => Ok(ChooseSpec::All(resolve_it_tag(filter, refs)?)),
        ChooseSpec::Player(filter) => Ok(ChooseSpec::Player(resolve_contextual_player_filter(
            filter, refs,
        )?)),
        ChooseSpec::PlayerOrPlaneswalker(filter) => Ok(ChooseSpec::PlayerOrPlaneswalker(
            resolve_contextual_player_filter(filter, refs)?,
        )),
        ChooseSpec::AttackedPlayerOrPlaneswalker => Ok(ChooseSpec::AttackedPlayerOrPlaneswalker),
        ChooseSpec::SpecificObject(id) => Ok(ChooseSpec::SpecificObject(*id)),
        ChooseSpec::SpecificPlayer(id) => Ok(ChooseSpec::SpecificPlayer(*id)),
        ChooseSpec::AnyTarget => Ok(ChooseSpec::AnyTarget),
        ChooseSpec::AnyOtherTarget => Ok(ChooseSpec::AnyOtherTarget),
        ChooseSpec::Source => Ok(ChooseSpec::Source),
        ChooseSpec::SourceController => Ok(ChooseSpec::SourceController),
        ChooseSpec::SourceOwner => Ok(ChooseSpec::SourceOwner),
        ChooseSpec::EachPlayer(filter) => Ok(ChooseSpec::EachPlayer(
            resolve_contextual_player_filter(filter, refs)?,
        )),
        ChooseSpec::Iterated => Ok(ChooseSpec::Iterated),
    }
}

pub(crate) fn resolve_value_it_tag(
    value: &Value,
    refs: &ReferenceEnv,
) -> Result<Value, CardTextError> {
    match value {
        Value::X if refs.bind_unbound_x_to_last_effect => {
            if let Some(id) = refs.known_last_effect_id() {
                Ok(Value::EffectValue(id))
            } else {
                Ok(Value::X)
            }
        }
        Value::Add(left, right) => Ok(Value::Add(
            Box::new(resolve_value_it_tag(left, refs)?),
            Box::new(resolve_value_it_tag(right, refs)?),
        )),
        Value::Scaled(value, multiplier) => Ok(Value::Scaled(
            Box::new(resolve_value_it_tag(value, refs)?),
            *multiplier,
        )),
        Value::SurfaceHinted { value, hints } => Ok(Value::SurfaceHinted {
            value: Box::new(resolve_value_it_tag(value, refs)?),
            hints: hints.clone(),
        }),
        Value::Count(filter) => Ok(Value::Count(resolve_it_tag(filter, refs)?)),
        Value::CountScaled(filter, multiplier) => Ok(Value::CountScaled(
            resolve_it_tag(filter, refs)?,
            *multiplier,
        )),
        Value::TotalPower(filter) => Ok(Value::TotalPower(resolve_it_tag(filter, refs)?)),
        Value::TotalToughness(filter) => Ok(Value::TotalToughness(resolve_it_tag(filter, refs)?)),
        Value::TotalManaValue(filter) => Ok(Value::TotalManaValue(resolve_it_tag(filter, refs)?)),
        Value::GreatestPower(filter) => Ok(Value::GreatestPower(resolve_it_tag(filter, refs)?)),
        Value::GreatestToughness(filter) => {
            Ok(Value::GreatestToughness(resolve_it_tag(filter, refs)?))
        }
        Value::GreatestManaValue(filter) => {
            Ok(Value::GreatestManaValue(resolve_it_tag(filter, refs)?))
        }
        Value::BasicLandTypesAmong(filter) => {
            Ok(Value::BasicLandTypesAmong(resolve_it_tag(filter, refs)?))
        }
        Value::CreatureTypesAmong(filter) => {
            Ok(Value::CreatureTypesAmong(resolve_it_tag(filter, refs)?))
        }
        Value::CardTypesAmong(filter) => Ok(Value::CardTypesAmong(resolve_it_tag(filter, refs)?)),
        Value::StaticAbilitiesAmong { filter, abilities } => Ok(Value::StaticAbilitiesAmong {
            filter: resolve_it_tag(filter, refs)?,
            abilities: abilities.clone(),
        }),
        Value::ColorsAmong(filter) => Ok(Value::ColorsAmong(resolve_it_tag(filter, refs)?)),
        Value::DistinctNames(filter) => Ok(Value::DistinctNames(resolve_it_tag(filter, refs)?)),
        Value::DistinctPowers(filter) => Ok(Value::DistinctPowers(resolve_it_tag(filter, refs)?)),
        Value::TurnHistoryCount(query) => {
            use ironsmith_core::TurnHistoryCount;

            let query = match query {
                TurnHistoryCount::Died(filter) => {
                    TurnHistoryCount::Died(resolve_it_tag(filter, refs)?)
                }
                TurnHistoryCount::EnteredBattlefield(filter) => {
                    TurnHistoryCount::EnteredBattlefield(resolve_it_tag(filter, refs)?)
                }
                TurnHistoryCount::TokensCreated(player) => {
                    TurnHistoryCount::TokensCreated(resolve_contextual_player_filter(player, refs)?)
                }
                TurnHistoryCount::PutIntoGraveyard { owner, from } => {
                    TurnHistoryCount::PutIntoGraveyard {
                        owner: resolve_contextual_player_filter(owner, refs)?,
                        from: from.clone(),
                    }
                }
                TurnHistoryCount::MovedZones { filter, from, to } => TurnHistoryCount::MovedZones {
                    filter: resolve_it_tag(filter, refs)?,
                    from: *from,
                    to: *to,
                },
                TurnHistoryCount::Sacrificed { player, filter } => TurnHistoryCount::Sacrificed {
                    player: resolve_contextual_player_filter(player, refs)?,
                    filter: resolve_it_tag(filter, refs)?,
                },
                TurnHistoryCount::CountersPutOn {
                    counter_type,
                    filter,
                } => TurnHistoryCount::CountersPutOn {
                    counter_type: *counter_type,
                    filter: resolve_it_tag(filter, refs)?,
                },
                TurnHistoryCount::CreaturesAttackedWith { player, filter } => {
                    TurnHistoryCount::CreaturesAttackedWith {
                        player: resolve_contextual_player_filter(player, refs)?,
                        filter: resolve_it_tag(filter, refs)?,
                    }
                }
                TurnHistoryCount::OpponentsAttacked(player) => TurnHistoryCount::OpponentsAttacked(
                    resolve_contextual_player_filter(player, refs)?,
                ),
                TurnHistoryCount::PlayersDiscarded(player) => TurnHistoryCount::PlayersDiscarded(
                    resolve_contextual_player_filter(player, refs)?,
                ),
                TurnHistoryCount::PlayersDealtDamage(player) => {
                    TurnHistoryCount::PlayersDealtDamage(resolve_contextual_player_filter(
                        player, refs,
                    )?)
                }
                TurnHistoryCount::PlayersDealtCombatDamageBy { players, sources } => {
                    TurnHistoryCount::PlayersDealtCombatDamageBy {
                        players: resolve_contextual_player_filter(players, refs)?,
                        sources: resolve_it_tag(sources, refs)?,
                    }
                }
                TurnHistoryCount::DiscardedOrCycled(player) => TurnHistoryCount::DiscardedOrCycled(
                    resolve_contextual_player_filter(player, refs)?,
                ),
                TurnHistoryCount::Cycled(player) => {
                    TurnHistoryCount::Cycled(resolve_contextual_player_filter(player, refs)?)
                }
                TurnHistoryCount::PlayersLostLife(player) => TurnHistoryCount::PlayersLostLife(
                    resolve_contextual_player_filter(player, refs)?,
                ),
                TurnHistoryCount::SpellsCast {
                    player,
                    filter,
                    from_zone,
                    from_outside_hand,
                    exclude_source,
                    before_triggering_spell,
                } => TurnHistoryCount::SpellsCast {
                    player: resolve_contextual_player_filter(player, refs)?,
                    filter: resolve_it_tag(filter, refs)?,
                    from_zone: *from_zone,
                    from_outside_hand: *from_outside_hand,
                    exclude_source: *exclude_source,
                    before_triggering_spell: *before_triggering_spell,
                },
                TurnHistoryCount::ColorsAmongPermanentsAndSpellsCast(player) => {
                    TurnHistoryCount::ColorsAmongPermanentsAndSpellsCast(
                        resolve_contextual_player_filter(player, refs)?,
                    )
                }
            };
            Ok(Value::TurnHistoryCount(query))
        }
        Value::Devotion { player, color } => Ok(Value::Devotion {
            player: resolve_contextual_player_filter(player, refs)?,
            color: *color,
        }),
        Value::DevotionToChosenColor(player) => Ok(Value::DevotionToChosenColor(
            resolve_contextual_player_filter(player, refs)?,
        )),
        Value::PowerOf(spec) => Ok(Value::PowerOf(Box::new(resolve_choose_spec_it_tag(
            spec, refs,
        )?))),
        Value::ToughnessOf(spec) => Ok(Value::ToughnessOf(Box::new(resolve_choose_spec_it_tag(
            spec, refs,
        )?))),
        Value::ManaValueOf(spec) => Ok(Value::ManaValueOf(Box::new(resolve_choose_spec_it_tag(
            spec, refs,
        )?))),
        Value::ManaSymbolsInManaCostOf { spec, color } => Ok(Value::ManaSymbolsInManaCostOf {
            spec: Box::new(resolve_choose_spec_it_tag(spec, refs)?),
            color: *color,
        }),
        Value::EventValue(EventValueSpec::Amount)
        | Value::EventValue(EventValueSpec::LifeAmount) => {
            if !refs.allow_life_event_value {
                if let Some(id) = refs.known_last_effect_id() {
                    return Ok(Value::EffectValue(id));
                }
                return Err(CardTextError::ParseError(
                    "event-derived amount requires a compatible trigger".to_string(),
                ));
            }
            Ok(value.clone())
        }
        Value::EventValueOffset(EventValueSpec::Amount, offset)
        | Value::EventValueOffset(EventValueSpec::LifeAmount, offset) => {
            if !refs.allow_life_event_value {
                if let Some(id) = refs.known_last_effect_id() {
                    return Ok(Value::EffectValueOffset(id, *offset));
                }
                return Err(CardTextError::ParseError(
                    "event-derived amount requires a compatible trigger".to_string(),
                ));
            }
            Ok(value.clone())
        }
        Value::PendingEffectMetric { source, metric } => {
            let id = refs.known_last_effect_id().ok_or_else(|| {
                CardTextError::ParseError(
                    "pending effect metric requires a prior memory-producing effect".to_string(),
                )
            })?;
            Ok(Value::EffectMetric {
                effect_id: id,
                source: *source,
                metric: *metric,
            })
        }
        Value::PendingEffectMetricOffset {
            source,
            metric,
            offset,
        } => {
            let id = refs.known_last_effect_id().ok_or_else(|| {
                CardTextError::ParseError(
                    "pending effect metric requires a prior memory-producing effect".to_string(),
                )
            })?;
            Ok(Value::EffectMetricOffset {
                effect_id: id,
                source: *source,
                metric: *metric,
                offset: *offset,
            })
        }
        _ => Ok(value.clone()),
    }
}

pub(crate) fn resolve_total_cost_it_tags(
    cost: &crate::cost::TotalCost,
    refs: &ReferenceEnv,
) -> Result<crate::cost::TotalCost, CardTextError> {
    fn resolve_component(
        component: &crate::costs::Cost,
        refs: &ReferenceEnv,
    ) -> Result<crate::costs::Cost, CardTextError> {
        let mut resolved = component.clone();
        match &mut resolved {
            crate::costs::Cost::DynamicMana(dynamic) => {
                if let Some(value) = dynamic.x_value.as_mut() {
                    *value = resolve_value_it_tag(value, refs)?;
                }
                if let Some(value) = dynamic.additional_generic.as_mut() {
                    *value = resolve_value_it_tag(value, refs)?;
                }
                if let Some(value) = dynamic.multiplier.as_mut() {
                    *value = resolve_value_it_tag(value, refs)?;
                }
            }
            crate::costs::Cost::Energy(value)
            | crate::costs::Cost::Mill(value)
            | crate::costs::Cost::Life(value) => {
                *value = resolve_value_it_tag(value, refs)?;
            }
            _ => {}
        }
        Ok(resolved)
    }

    match cost.kind() {
        ironsmith_core::TotalCostKind::All(components) => Ok(crate::cost::TotalCost::from_costs(
            components
                .iter()
                .map(|component| resolve_component(component, refs))
                .collect::<Result<Vec<_>, _>>()?,
        )),
        ironsmith_core::TotalCostKind::OneOf(branches) => Ok(crate::cost::TotalCost::one_of(
            branches
                .iter()
                .map(|branch| resolve_total_cost_it_tags(branch, refs))
                .collect::<Result<Vec<_>, _>>()?,
        )),
    }
}

pub(crate) fn choose_spec_targets_object(spec: &ChooseSpec) -> bool {
    matches!(
        spec.base(),
        ChooseSpec::Object(_)
            | ChooseSpec::Tagged(_)
            | ChooseSpec::SpecificObject(_)
            | ChooseSpec::Source
    )
}

pub(crate) fn with_target_reference_surface_hint(
    spec: ChooseSpec,
    target: &TargetAst,
) -> ChooseSpec {
    let span = match target {
        TargetAst::Source(span) | TargetAst::Tagged(_, span) | TargetAst::Object(_, _, span) => {
            *span
        }
        TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, _, _) => {
            return with_target_reference_surface_hint(spec, inner);
        }
        _ => None,
    };
    target_reference_hinted_spec(spec, span)
}

fn source_reference_hinted_spec(
    spec: ChooseSpec,
    surface: Option<SourceReferenceSurface>,
) -> ChooseSpec {
    match surface {
        Some(surface) => spec.with_surface_hint(ChooseSpecSurfaceHint::SourceReference(surface)),
        None => spec,
    }
}

fn source_reference_surface_for_target_span(
    span: Option<crate::cards::TextSpan>,
) -> Option<SourceReferenceSurface> {
    crate::runtime_backend::util::source_reference_surface_for_span(span)
}

fn target_reference_hinted_spec(
    spec: ChooseSpec,
    span: Option<crate::cards::TextSpan>,
) -> ChooseSpec {
    let spec = source_reference_hinted_spec(spec, source_reference_surface_for_target_span(span));
    match crate::runtime_backend::util::sacrificed_object_kind_for_span(span) {
        Some(kind) => spec.with_surface_hint(ChooseSpecSurfaceHint::SacrificedObject(kind)),
        None => spec,
    }
}

fn implicit_it_reference_resolves_to_source(refs: &ReferenceEnv) -> bool {
    refs.known_last_object_tag().is_none()
        && (refs.has_source_object_antecedent() || refs.known_last_player_filter().is_none())
}

fn implicit_source_pronoun_surface(
    span: Option<crate::cards::TextSpan>,
) -> Option<SourceReferenceSurface> {
    span.map(|_| SourceReferenceSurface::ThisPermanentType("it".to_string()))
}

pub(crate) fn choose_spec_for_target(target: &TargetAst) -> ChooseSpec {
    match target {
        TargetAst::Source(span) => target_reference_hinted_spec(ChooseSpec::Source, *span),
        TargetAst::AnyTarget(_) => ChooseSpec::AnyTarget,
        TargetAst::AnyOtherTarget(_) => ChooseSpec::AnyOtherTarget,
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
            target_reference_hinted_spec(spec, *reference_span)
        }
        TargetAst::Tagged(tag, span) => {
            let spec = ChooseSpec::Tagged(tag.clone());
            target_reference_hinted_spec(spec, *span)
        }
        TargetAst::WithCount(inner, count) => choose_spec_for_target(inner).with_count(*count),
        TargetAst::WithCountValue(inner, count, value) => {
            choose_spec_for_target(inner).with_count_value(*count, value.clone())
        }
    }
}

pub(crate) fn resolve_target_spec_with_choices(
    target: &TargetAst,
    refs: &ReferenceEnv,
) -> Result<(ChooseSpec, Vec<ChooseSpec>), CardTextError> {
    let mut spec = match target {
        TargetAst::Tagged(tag, span)
            if tag.as_str() == IT_TAG
                && crate::runtime_backend::util::sacrificed_object_kind_for_span(*span)
                    .is_none()
                && implicit_it_reference_resolves_to_source(refs) =>
        {
            source_reference_hinted_spec(
                ChooseSpec::Source,
                source_reference_surface_for_target_span(*span)
                    .or_else(|| implicit_source_pronoun_surface(*span)),
            )
        }
        _ => choose_spec_for_target(target),
    };
    if let TargetAst::Player(filter, explicit_target_span) = target
        && explicit_target_span.is_none()
        && matches!(filter, PlayerFilter::Target(_))
    {
        if let Some(last_filter) = refs.known_last_player_filter() {
            spec = ChooseSpec::Player(as_followup_player_alias(last_filter.clone()));
        } else if refs.iterated_player {
            spec = ChooseSpec::Player(PlayerFilter::IteratedPlayer);
        }
    }
    let spec = resolve_choose_spec_it_tag(&spec, refs)?;
    let mut choices = if spec.is_target() {
        vec![spec.clone()]
    } else {
        Vec::new()
    };
    if let TargetAst::Object(filter, _, _) = target {
        append_object_filter_target_player_choices(filter, &mut choices);
    }
    Ok((spec, choices))
}

pub(crate) fn resolve_attach_object_spec(
    object: &TargetAst,
    refs: &ReferenceEnv,
) -> Result<(ChooseSpec, Vec<ChooseSpec>), CardTextError> {
    match object {
        TargetAst::Source(_) => Ok((choose_spec_for_target(object), Vec::new())),
        TargetAst::Tagged(tag, _) => {
            let resolved_tag = if tag.as_str() == IT_TAG {
                refs.known_last_object_tag()
                    .map(|tag| tag.as_str().to_string())
                    .ok_or_else(|| {
                        CardTextError::ParseError(
                            "cannot resolve 'it/them' in attach object clause without prior tagged object"
                                .to_string(),
                        )
                    })?
            } else {
                tag.as_str().to_string()
            };
            Ok((
                ChooseSpec::All(ObjectFilter::tagged(TagKey::from(resolved_tag.as_str()))),
                Vec::new(),
            ))
        }
        TargetAst::Object(filter, explicit_target_span, _) => {
            let resolved = resolve_it_tag(filter, refs)?;
            if explicit_target_span.is_some() {
                let spec = ChooseSpec::target(ChooseSpec::Object(resolved));
                Ok((spec.clone(), vec![spec]))
            } else {
                Ok((ChooseSpec::All(resolved), Vec::new()))
            }
        }
        TargetAst::WithCount(inner, count) => {
            let (base, _) = resolve_attach_object_spec(inner, refs)?;
            let spec = base.with_count(*count);
            let choices = if spec.is_target() {
                vec![spec.clone()]
            } else {
                Vec::new()
            };
            Ok((spec, choices))
        }
        _ => Err(CardTextError::ParseError(
            "unsupported attach object reference".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::builders::TagKey;
    use crate::runtime_backend::references::reference_model::RefState;

    #[test]
    fn target_wrapped_implicit_it_value_resolves_to_source() {
        let mut refs = ReferenceEnv::default();
        refs.source_object_antecedent = true;
        refs.last_player_filter = RefState::Known(PlayerFilter::You);

        let value = Value::PowerOf(Box::new(ChooseSpec::target(ChooseSpec::Object(
            ObjectFilter::tagged(TagKey::from(IT_TAG)),
        ))));

        let resolved = resolve_value_it_tag(&value, &refs).expect("resolve implicit it value");

        assert_eq!(
            resolved,
            Value::PowerOf(Box::new(ChooseSpec::Source)),
            "source-bound implicit it should not remain target-wrapped"
        );
    }

    #[test]
    fn public_revealed_count_binds_to_current_reveal_result_tag() {
        let refs = ReferenceEnv {
            last_object_tag: RefState::Known(TagKey::from("__sentence_helper_revealed_l0_s0_e7")),
            ..ReferenceEnv::default()
        };
        let value = Value::Count(ObjectFilter::tagged(TagKey::from("__public_revealed")));
        let resolved = resolve_value_it_tag(&value, &refs).expect("resolve reveal count");
        let Value::Count(filter) = resolved else {
            panic!("expected count value");
        };
        assert_eq!(
            filter.tagged_constraints[0].tag.as_str(),
            "__sentence_helper_revealed_l0_s0_e7"
        );
    }

    #[test]
    fn unresolved_it_relational_constraint_survives_for_runtime_trigger_binding() {
        let filter = ObjectFilter::default().match_tagged(
            TagKey::from(IT_TAG),
            TaggedOpbjectRelation::ManaValueLtTagged,
        );

        let resolved =
            resolve_it_tag(&filter, &ReferenceEnv::default()).expect("preserve runtime relation");

        assert_eq!(resolved.tagged_constraints, filter.tagged_constraints);
    }

    #[test]
    fn unresolved_immediate_attachment_relation_does_not_leak_into_target_filter() {
        let filter = ObjectFilter::creature().match_tagged(
            TagKey::from(IT_TAG),
            TaggedOpbjectRelation::AttachedToTaggedObject,
        );

        let resolved = resolve_it_tag(&filter, &ReferenceEnv::default())
            .expect("consume unbound immediate attachment relation");

        assert!(resolved.tagged_constraints.is_empty());
        assert_eq!(resolved.card_types, filter.card_types);
    }
}
