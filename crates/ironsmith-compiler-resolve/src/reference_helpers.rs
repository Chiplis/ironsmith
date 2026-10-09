use crate::cards::builders::{CardTextError, PlayerAst, TagKey, TargetAst};
use crate::effect::{EffectId, EventValueSpec, Restriction, Value};
use crate::filter::{Comparison, ObjectFilter, ObjectRef, PlayerFilter, TaggedOpbjectRelation};
use crate::target::{ChooseSpec, ChooseSpecSurfaceHint, SourceReferenceSurface};
use crate::zone::Zone;
use ironsmith_core::TurnHistoryCount;

use crate::model::reference_state::ReferenceEnv;

/// Parse-time alias for the object set produced by the instruction that
/// registers a delayed trigger ("... this way" inside the delayed body).
/// Lowering binds it to the registering instruction's concrete result tag,
/// which the delayed trigger captures when it is scheduled.
pub const DELAYED_REGISTRATION_RESULT_ALIAS: &str = "__delayed_registration_result__";

pub fn is_sacrificed_object_reference_tag(tag: &str) -> bool {
    tag == "sacrificed"
        || tag.starts_with("sacrificed_")
        || tag.starts_with("sacrifice_cost_")
        || tag.starts_with("__sentence_helper_sacrificed")
}

/// A result tag naming newly created tokens.
fn is_created_token_result_tag(tag: &str) -> bool {
    tag.starts_with("created_")
}

/// A result tag naming cards seen or moved outside the battlefield (revealed,
/// looked at, milled, discarded) rather than permanents.
fn is_off_battlefield_card_result_tag(tag: &str) -> bool {
    tag.starts_with("milled_")
        || tag.starts_with("discarded_")
        || tag.starts_with("__sentence_helper_revealed")
        || tag.starts_with("__sentence_helper_looked")
        || tag.starts_with("__sentence_helper_consult_match")
}

/// Object results that some definite descriptions cannot name: a token is
/// never "that card" or "that spell", and a revealed or milled card is never
/// "that creature".
pub fn is_noun_restricted_object_result_tag(tag: &TagKey) -> bool {
    is_created_token_result_tag(tag.as_str()) || is_off_battlefield_card_result_tag(tag.as_str())
}

/// Whether the head noun of a definite `it` description ("that card",
/// "that creature") excludes the kind of object `tag` names.
fn definite_noun_excludes_antecedent(filter: &ObjectFilter, tag: &TagKey) -> bool {
    use crate::types::CardType;
    // "... and sacrifices the rest. Each of those creatures can't attack"
    // (Promise of Loyalty): permanents still on the battlefield can't be the
    // ones a prior clause sacrificed.
    if is_sacrificed_object_reference_tag(tag.as_str())
        && filter.zone == Some(Zone::Battlefield)
        && !filter.card_types.is_empty()
    {
        return true;
    }
    if is_created_token_result_tag(tag.as_str()) {
        return filter.has_explicit_card_noun()
            || filter.zone == Some(Zone::Stack)
            || filter.stack_kind.is_some();
    }
    if is_off_battlefield_card_result_tag(tag.as_str()) {
        let is_permanent_type = |card_type: &CardType| {
            matches!(
                card_type,
                CardType::Creature
                    | CardType::Artifact
                    | CardType::Enchantment
                    | CardType::Land
                    | CardType::Planeswalker
                    | CardType::Battle
            )
        };
        return !filter.has_explicit_card_noun()
            && filter.zone.is_none_or(|zone| zone == Zone::Battlefield)
            && (filter.card_types.iter().any(is_permanent_type)
                || filter
                    .explicit_card_type_noun()
                    .is_some_and(|card_type| is_permanent_type(&card_type)));
    }
    false
}

/// A tag naming cards a discard in this ability produced.
pub fn is_discard_result_reference_tag(tag: &str) -> bool {
    tag.starts_with("discarded")
        || tag.starts_with("__sentence_helper_discarded")
        || tag.starts_with("discard_cost_")
}

fn is_exiled_collection_reference_tag(tag: &str) -> bool {
    tag == "exiled" || tag.starts_with("exiled_") || tag.starts_with("__sentence_helper_exiled")
}

/// The card an activation cost just exiled ("Exile the top card of your
/// library: ... the exiled card's mana value"). Unlike the source's linked
/// exile set, this names exactly the paid object.
fn is_cost_exiled_reference_tag(tag: &TagKey) -> bool {
    crate::tag::CompilerCostObjectTag::Exile.matches(tag)
        || tag.as_str() == crate::tag::CompilerReferenceTag::CostExiledTop.as_str()
        || tag.as_str() == crate::tag::CompilerReferenceTag::CostExiledFromHand.as_str()
}

pub fn is_you_player_filter(filter: &PlayerFilter) -> bool {
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
pub fn as_followup_player_alias(filter: PlayerFilter) -> PlayerFilter {
    match filter {
        PlayerFilter::Target(inner) => PlayerFilter::AliasedTarget(inner),
        PlayerFilter::ControllerOf(reference) => PlayerFilter::AliasedControllerOf(reference),
        PlayerFilter::OwnerOf(reference) => PlayerFilter::AliasedOwnerOf(reference),
        other => other,
    }
}

fn contextual_chosen_player_filter(refs: &ReferenceEnv) -> PlayerFilter {
    match refs.known_last_player_filter() {
        Some(PlayerFilter::Target(inner)) | Some(PlayerFilter::AliasedTarget(inner)) => {
            PlayerFilter::AliasedTarget(inner.clone())
        }
        _ => PlayerFilter::ChosenPlayer,
    }
}

pub fn resolve_unless_player_filter(
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
        let previous = previous_last_player_filter.ok_or_else(|| {
            CardTextError::InvariantViolation(
                "expected previous non-you player filter for unless-player resolution".to_string(),
            )
        })?;
        return resolve_contextual_player_filter(&previous, refs);
    }
    resolve_non_target_player_filter(player, refs)
}

pub fn resolve_non_target_player_filter(
    player: PlayerAst,
    refs: &ReferenceEnv,
) -> Result<PlayerFilter, CardTextError> {
    match player {
        PlayerAst::You => Ok(PlayerFilter::You),
        PlayerAst::Active => Ok(PlayerFilter::Active),
        PlayerAst::Any => Ok(PlayerFilter::Any),
        PlayerAst::Chosen => Ok(contextual_chosen_player_filter(refs)),
        PlayerAst::Defending => Ok(PlayerFilter::Defending),
        PlayerAst::Attacking => Ok(PlayerFilter::Attacking),
        PlayerAst::MostCardsInHand => Ok(PlayerFilter::MostCardsInHand),
        PlayerAst::MostLifeTied => Ok(PlayerFilter::MostLifeTied),
        PlayerAst::LowestLifeTied => Ok(PlayerFilter::LowestLifeTied),
        PlayerAst::Target => match refs.known_last_player_filter() {
            Some(PlayerFilter::Target(inner)) | Some(PlayerFilter::AliasedTarget(inner)) => {
                Ok(PlayerFilter::Target(inner.clone()))
            }
            _ => Err(CardTextError::ParseError(
                "target player requires explicit targeting".to_string(),
            )),
        },
        PlayerAst::TargetOpponent => match refs.known_last_player_filter() {
            Some(PlayerFilter::Target(inner)) | Some(PlayerFilter::AliasedTarget(inner))
                if inner.as_ref() == &PlayerFilter::Opponent =>
            {
                Ok(PlayerFilter::target_opponent())
            }
            _ => Err(CardTextError::ParseError(
                "target player requires explicit targeting".to_string(),
            )),
        },
        PlayerAst::AnotherTarget => Ok(PlayerFilter::another_target_player()),
        PlayerAst::Opponent => Ok(PlayerFilter::Opponent),
        PlayerAst::PlayerToYourLeft => Ok(PlayerFilter::PlayerToYourLeft),
        PlayerAst::PlayerToYourRight => Ok(PlayerFilter::PlayerToYourRight),
        PlayerAst::Enchanted => Ok(PlayerFilter::TaggedPlayer(
            ironsmith_compiler_semantic::tag::declared_key("enchanted").into(),
        )),
        PlayerAst::Teammate => Ok(PlayerFilter::Teammate),
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
            Ok(as_followup_player_alias(resolve_contextual_player_filter(
                &filter, refs,
            )?))
        }
        PlayerAst::ThatPlayerOrTargetController => {
            // "that player" can only fall back to a target's controller when
            // the discourse has no non-target player antecedent. A trigger's
            // event player ("deal combat damage to a player, ... that
            // player's library") is such an antecedent.
            // With no player antecedent at all, "that player" is the
            // event's player (bound per damaged/iterated player at runtime),
            // exactly as a bare "that player" would be.
            // An earlier object antecedent other than the trigger's event
            // object ("Exile target creature. Its controller manifests ...")
            // keeps the target-controller reading.
            if refs.known_last_player_filter().is_none()
                && refs.known_last_object_tag().is_none_or(|tag| {
                    tag.as_str() == crate::tag::CompilerReferenceTag::Triggering.as_str()
                })
            {
                return resolve_non_target_player_filter(PlayerAst::That, refs);
            }
            if let Some(filter) = refs.known_last_player_filter()
                && !matches!(
                    filter,
                    PlayerFilter::Target(_)
                        | PlayerFilter::AliasedTarget(_)
                        | PlayerFilter::ControllerOf(_)
                        | PlayerFilter::AliasedControllerOf(_)
                        | PlayerFilter::OwnerOf(_)
                        | PlayerFilter::AliasedOwnerOf(_)
                        | PlayerFilter::TargetPlayerOrControllerOfTarget
                        | PlayerFilter::DamagedPlayer
                        | PlayerFilter::You
                        | PlayerFilter::Any
                )
            {
                return resolve_non_target_player_filter(PlayerAst::That, refs);
            }
            Ok(PlayerFilter::TargetPlayerOrControllerOfTarget)
        }
        PlayerAst::TriggeringSourceController => Ok(PlayerFilter::ControllerOf(ObjectRef::tagged(
            "triggering_source",
        ))),
        PlayerAst::ItsController => {
            if let Some(tag) = controller_antecedent_tag(refs) {
                Ok(PlayerFilter::ControllerOf(ObjectRef::tagged(tag.clone())))
            } else {
                Ok(PlayerFilter::ControllerOf(ObjectRef::Target))
            }
        }
        PlayerAst::SourceOwner => Ok(PlayerFilter::OwnerOf(ObjectRef::tagged(
            ironsmith_core::SOURCE_OBJECT_TAG,
        ))),
        PlayerAst::ItsOwner => {
            if let Some(tag) = refs.known_last_object_tag() {
                Ok(PlayerFilter::OwnerOf(ObjectRef::tagged(tag.clone())))
            } else {
                Ok(PlayerFilter::OwnerOf(ObjectRef::Target))
            }
        }
        PlayerAst::Implicit => {
            if refs.iterated_player
                && refs.known_last_object_tag().is_some_and(|tag| {
                    tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
                })
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

pub fn player_filter_from_object_filter(filter: &ObjectFilter) -> Option<PlayerFilter> {
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
        .find_map(player_filter_from_object_filter)
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
        | PlayerFilter::OpponentOf(base)
        | PlayerFilter::MaxSpeed { base, .. } => {
            push_target_player_filter_choices(base, choices);
        }
        PlayerFilter::OpponentWithMoreControlledObjectsThan { player, filter, .. } => {
            push_target_player_filter_choices(player, choices);
            append_object_filter_target_player_choices(filter, choices);
        }
        PlayerFilter::ControlsMost { filter } | PlayerFilter::ControlsFewestTied { filter } => {
            append_object_filter_target_player_choices(filter, choices);
        }
        PlayerFilter::Excluding { base, excluded } => {
            push_target_player_filter_choices(base, choices);
            push_target_player_filter_choices(excluded, choices);
        }
        PlayerFilter::WasDealtDamageBySourceThisGame { base } => {
            push_target_player_filter_choices(base, choices);
        }
        PlayerFilter::LostLifeThisTurn { base } => {
            push_target_player_filter_choices(base, choices);
        }
        PlayerFilter::WasDealtCombatDamageByDistinctSourcesThisTurn { base, sources, .. } => {
            push_target_player_filter_choices(base, choices);
            append_object_filter_target_player_choices(sources, choices);
        }
        PlayerFilter::WasDealtCombatDamageBySourcesThisGame { base, sources } => {
            push_target_player_filter_choices(base, choices);
            append_object_filter_target_player_choices(sources, choices);
        }
        PlayerFilter::Any
        | PlayerFilter::You
        | PlayerFilter::NotYou
        | PlayerFilter::Opponent
        | PlayerFilter::Teammate
        | PlayerFilter::PlayerToYourLeft
        | PlayerFilter::PlayerToYourRight
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
        | PlayerFilter::AttackedBySourceThisTurn
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
    for player_filter in [
        filter.owner.as_ref(),
        filter.controller.as_ref(),
        filter.cast_by.as_ref(),
        filter.targets_player.as_ref(),
        filter.targets_only_player.as_ref(),
        filter
            .attacking_player_or_planeswalker_controlled_by
            .as_ref(),
        filter.protected_by.as_ref(),
        filter.attached_to_player.as_ref(),
        filter.entered_battlefield_controller.as_ref(),
        filter
            .counters_put_on_this_turn
            .as_ref()
            .map(|constraint| &constraint.source_controller),
        filter.discarded_or_cycled_this_turn_by.as_ref(),
        filter.dealt_damage_to_player_this_turn.as_ref(),
        filter.last_drawn_this_turn.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        push_target_player_filter_choices(player_filter, choices);
    }
    for nested_filter in [
        filter.attached_to_object.as_deref(),
        filter.blocked_or_was_blocked_by_this_turn.as_deref(),
        filter.targets_object.as_deref(),
        filter.targets_only_object.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        append_object_filter_target_player_choices(nested_filter, choices);
    }
    for nested_filter in &filter.no_shared_creature_types_with {
        append_object_filter_target_player_choices(nested_filter, choices);
    }
    for relation in &filter.characteristic_relations {
        append_object_filter_target_player_choices(&relation.comparison, choices);
    }
    for branch in &filter.any_of {
        append_object_filter_target_player_choices(branch, choices);
    }
}

/// The object a controller back-reference names. A card just moved out of a
/// graveyard, hand, or library has no controller (CR 108.4a), so the
/// reference keeps naming the object antecedent that card superseded.
fn controller_antecedent_tag(refs: &ReferenceEnv) -> Option<&TagKey> {
    let last = refs.known_last_object_tag()?;
    let is_off_battlefield_card = is_off_battlefield_card_result_tag(last.as_str())
        || refs.snapshot_tag_aliases.iter().any(|(alias, concrete)| {
            concrete == last
                && alias.as_str()
                    == crate::tag::CompilerReferenceTag::OffBattlefieldCardAntecedent.as_str()
        });
    if !is_off_battlefield_card {
        return Some(last);
    }
    refs.snapshot_tag_aliases
        .iter()
        .find(|(alias, _)| {
            alias.as_str() == crate::tag::CompilerReferenceTag::PriorObjectAntecedent.as_str()
        })
        .map(|(_, prior)| prior)
        .or(Some(last))
}

fn resolve_object_ref(reference: &ObjectRef, refs: &ReferenceEnv) -> ObjectRef {
    match reference {
        ObjectRef::Tagged(tag) if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str() => {
            if let Some(tag) = refs.known_last_object_tag() {
                ObjectRef::tagged(tag.clone())
            } else if refs.has_source_object_antecedent() {
                ObjectRef::tagged(crate::tag::CompilerReferenceTag::SourceObject.bind())
            } else {
                ObjectRef::Target
            }
        }
        ObjectRef::Tagged(tag) => refs
            .snapshot_tag_aliases
            .iter()
            .find(|(alias, _)| alias == tag)
            .map(|(_, concrete)| ObjectRef::tagged(concrete.clone()))
            .or_else(|| {
                (tag.as_str() == crate::tag::CompilerReferenceTag::AdditionalCostObject.as_str())
                    .then(|| refs.known_last_object_tag())
                    .flatten()
                    .map(|resolved| ObjectRef::tagged(resolved.clone()))
            })
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
        PlayerFilter::AliasedTarget(inner)
            if matches!(inner.as_ref(), PlayerFilter::IteratedPlayer) =>
        {
            // This wrapper is initially only a discourse marker for
            // "that player"/"their". Once the antecedent is known, retain a
            // target alias only when that antecedent was an announced target.
            // A persistent participant such as ChosenPlayer has no target
            // assignment to consult at runtime.
            as_followup_player_alias(resolve_contextual_player_filter(inner, refs)?)
        }
        PlayerFilter::AliasedTarget(inner) => {
            PlayerFilter::AliasedTarget(Box::new(resolve_contextual_player_filter(inner, refs)?))
        }
        PlayerFilter::ChosenPlayer => contextual_chosen_player_filter(refs),
        PlayerFilter::Excluding { base, excluded } => {
            let excluded = if matches!(excluded.as_ref(), PlayerFilter::IteratedPlayer)
                && !refs.iterated_player
            {
                refs.known_last_player_filter()
                    .filter(|filter| !filter.mentions_iterated_player())
                    .cloned()
                    .or_else(|| {
                        refs.known_last_object_tag()
                            .map(|tag| PlayerFilter::ControllerOf(ObjectRef::tagged(tag.clone())))
                    })
                    .unwrap_or(PlayerFilter::You)
            } else {
                resolve_contextual_player_filter(excluded, refs)?
            };
            PlayerFilter::Excluding {
                base: Box::new(resolve_contextual_player_filter(base, refs)?),
                excluded: Box::new(excluded),
            }
        }
        PlayerFilter::WasDealtDamageBySourceThisGame { base } => {
            PlayerFilter::WasDealtDamageBySourceThisGame {
                base: Box::new(resolve_contextual_player_filter(base, refs)?),
            }
        }
        PlayerFilter::LostLifeThisTurn { base } => PlayerFilter::LostLifeThisTurn {
            base: Box::new(resolve_contextual_player_filter(base, refs)?),
        },
        PlayerFilter::WasDealtCombatDamageByDistinctSourcesThisTurn {
            base,
            sources,
            minimum,
        } => PlayerFilter::WasDealtCombatDamageByDistinctSourcesThisTurn {
            base: Box::new(resolve_contextual_player_filter(base, refs)?),
            sources: Box::new(resolve_object_filter_player_refs(sources, refs)?),
            minimum: *minimum,
        },
        PlayerFilter::ControllerOf(ObjectRef::Tagged(tag))
            if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
                && controller_antecedent_tag(refs).is_some() =>
        {
            PlayerFilter::ControllerOf(ObjectRef::tagged(
                controller_antecedent_tag(refs)
                    .cloned()
                    .unwrap_or_else(|| tag.clone()),
            ))
        }
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

fn replace_it_tag_in_value(value: &mut Value, tag: &TagKey) {
    match value {
        Value::SurfaceHinted { value, .. }
        | Value::Scaled(value, _)
        | Value::DividedRoundedDown(value, _)
        | Value::HalfRoundedDown(value) => replace_it_tag_in_value(value, tag),
        Value::Add(left, right) | Value::Min(left, right) => {
            replace_it_tag_in_value(left, tag);
            replace_it_tag_in_value(right, tag);
        }
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
        | Value::DistinctManaValues(filter)
        | Value::UnlockedDoorsAmong(filter)
        | Value::DistinctPowers(filter) => replace_it_tag_in_filter(filter, tag),
        Value::StaticAbilitiesAmong { filter, .. } => replace_it_tag_in_filter(filter, tag),
        Value::TurnHistoryCount(
            TurnHistoryCount::DestroyedBy { filter, cause }
            | TurnHistoryCount::CastSpellsCounteredBy { filter, cause, .. },
        ) => {
            replace_it_tag_in_filter(filter, tag);
            if let Some(filter) = cause.source_filter.as_mut() {
                replace_it_tag_in_filter(filter, tag);
            }
        }
        Value::TurnHistoryCount(
            TurnHistoryCount::MaxEnteredBattlefieldByController { filter, .. }
            | TurnHistoryCount::Died { filter, .. }
            | TurnHistoryCount::EnteredBattlefield(filter)
            | TurnHistoryCount::MovedZones { filter, .. }
            | TurnHistoryCount::Sacrificed { filter, .. }
            | TurnHistoryCount::SacrificedCardTypes { filter, .. }
            | TurnHistoryCount::CountersPutOn { filter, .. }
            | TurnHistoryCount::CreaturesAttackedWith { filter, .. },
        ) => replace_it_tag_in_filter(filter, tag),
        Value::SpellsCastThisTurnMatching { filter, .. }
        | Value::TotalManaValueOfSpellsCastThisTurnMatching { filter, .. } => {
            replace_it_tag_in_filter(filter, tag)
        }
        Value::ManaFromSourceSpentToCastThisSpell { source_filter, .. } => {
            replace_it_tag_in_filter(source_filter, tag)
        }
        Value::PendingPriorEffectMetric(query) | Value::PriorEffectMetric { query, .. } => {
            if let Some(filter) = query.filter.as_mut() {
                replace_it_tag_in_filter(filter, tag);
            }
        }
        _ => {}
    }
}

fn replace_it_tag_in_filter(filter: &mut ObjectFilter, tag: &TagKey) {
    for constraint in &mut filter.tagged_constraints {
        if constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str() {
            constraint.tag = tag.clone();
        }
    }
    if let Some(ObjectRef::Tagged(reference_tag)) = &mut filter.in_combat_with
        && reference_tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
    {
        *reference_tag = tag.clone();
    }
    for comparison in [
        filter.power.as_mut(),
        filter.toughness.as_mut(),
        filter.mana_value.as_mut(),
        filter.color_count.as_mut(),
        filter.card_type_count.as_mut(),
    ]
    .into_iter()
    .flatten()
    {
        if let Comparison::EqualExpr(value)
        | Comparison::NotEqualExpr(value)
        | Comparison::LessThanExpr(value)
        | Comparison::LessThanOrEqualExpr(value)
        | Comparison::GreaterThanExpr(value)
        | Comparison::GreaterThanOrEqualExpr(value) = comparison
        {
            replace_it_tag_in_value(value, tag);
        }
    }
    for nested in [
        filter.attached_to_object.as_deref_mut(),
        filter.blocked_or_was_blocked_by_this_turn.as_deref_mut(),
        filter.targets_object.as_deref_mut(),
        filter.targets_only_object.as_deref_mut(),
    ]
    .into_iter()
    .flatten()
    {
        replace_it_tag_in_filter(nested, tag);
    }
    for nested in &mut filter.no_shared_creature_types_with {
        replace_it_tag_in_filter(nested, tag);
    }
    for relation in &mut filter.characteristic_relations {
        replace_it_tag_in_filter(&mut relation.comparison, tag);
    }
    for branch in &mut filter.any_of {
        replace_it_tag_in_filter(branch, tag);
    }
}

fn resolve_object_filter_player_refs(
    filter: &ObjectFilter,
    refs: &ReferenceEnv,
) -> Result<ObjectFilter, CardTextError> {
    let mut resolved = filter.clone();
    if let Some(constraint) = resolved.target_set_aggregate_constraint.as_mut() {
        constraint.maximum = resolve_value_it_tag(&constraint.maximum, refs)?;
    }
    if let Some(controller) = resolved.controller.as_mut() {
        *controller = resolve_contextual_player_filter(controller, refs)?;
    }
    if let Some(cast_by) = resolved.cast_by.as_mut() {
        *cast_by = resolve_contextual_player_filter(cast_by, refs)?;
    }
    if let Some(owner) = resolved.owner.as_mut() {
        *owner = resolve_contextual_player_filter(owner, refs)?;
    }
    if let Some(player) = resolved.last_drawn_this_turn.as_mut() {
        *player = resolve_contextual_player_filter(player, refs)?;
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
    if let Some(card_type_count) = resolved.card_type_count.as_mut() {
        *card_type_count = resolve_object_filter_comparison(card_type_count, refs)?;
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
    if let Some(protector) = resolved.protected_by.as_mut() {
        *protector = resolve_contextual_player_filter(protector, refs)?;
    }
    if let Some(attached_to_player) = resolved.attached_to_player.as_mut() {
        *attached_to_player = resolve_contextual_player_filter(attached_to_player, refs)?;
    }
    if let Some(attached_to_object) = resolved.attached_to_object.as_mut() {
        **attached_to_object = resolve_object_filter_player_refs(attached_to_object, refs)?;
    }
    if let Some(blocked_by) = resolved.blocked_by.as_mut() {
        *blocked_by = resolve_object_ref(blocked_by, refs);
    }
    if let Some(in_combat_with) = resolved.in_combat_with.as_mut() {
        *in_combat_with = resolve_object_ref(in_combat_with, refs);
    }
    if let Some(combat_partner) = resolved.blocked_or_was_blocked_by_this_turn.as_mut() {
        **combat_partner = resolve_object_filter_player_refs(combat_partner, refs)?;
    }
    if let Some(entered_controller) = resolved.entered_battlefield_controller.as_mut() {
        *entered_controller = resolve_contextual_player_filter(entered_controller, refs)?;
    }
    if let Some(constraint) = resolved.counters_put_on_this_turn.as_mut() {
        constraint.source_controller =
            resolve_contextual_player_filter(&constraint.source_controller, refs)?;
    }
    for nested in &mut resolved.no_shared_creature_types_with {
        *nested = resolve_object_filter_player_refs(nested, refs)?;
    }
    for relation in &mut resolved.characteristic_relations {
        relation.comparison = resolve_object_filter_player_refs(&relation.comparison, refs)?;
    }
    for nested in &mut resolved.any_of {
        *nested = resolve_object_filter_player_refs(nested, refs)?;
    }
    Ok(resolved)
}

/// "Then remove a time counter from each other card you own in exile"
/// (Alaundo), "return up to one other target creature card ... from your
/// graveyard" (Colfenor): an off-battlefield "other" filter cannot be
/// contrasting the card with the ability source; it contrasts it with the
/// clause's object antecedent, the card just moved to that zone. Exclude that
/// antecedent by identity. The source exclusion is kept: it is inert for a
/// source outside that zone, and still correct for an ability of a card in
/// that zone whose antecedent is the card itself ("Return this card and up
/// to one other target creature card from your graveyard").
fn bind_off_battlefield_other_to_antecedent(filter: &mut ObjectFilter, refs: &ReferenceEnv) {
    if !filter.other || !matches!(filter.zone, Some(Zone::Graveyard | Zone::Exile)) {
        return;
    }
    let Some(tag) = refs.known_last_object_tag() else {
        return;
    };
    if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str() {
        return;
    }
    // "Exile it, then return all other cards exiled with this artifact": the
    // antecedent was exiled with the source during this resolution. The link
    // tag is re-read as the full link set, and exile result tags carry the
    // pre-move identity, so exclude it through the runtime's record of what
    // this resolution exiled with its source.
    let names_source_exile_set = filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag.as_str() == crate::tag::CompilerReferenceTag::SourceExiled.as_str()
            && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
    });
    let exclusion: TagKey = if names_source_exile_set
        && (tag.as_str() == crate::tag::CompilerReferenceTag::SourceExiled.as_str()
            || is_exiled_collection_reference_tag(tag.as_str()))
    {
        TagKey::from(ironsmith_core::SOURCE_EXILED_THIS_RESOLUTION_TAG)
    } else {
        tag.clone()
    };
    if !filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag == exclusion
            && constraint.relation == TaggedOpbjectRelation::IsNotTaggedObject
    }) {
        filter
            .tagged_constraints
            .push(crate::filter::TaggedObjectConstraint {
                tag: exclusion,
                relation: TaggedOpbjectRelation::IsNotTaggedObject,
            });
    }
}

pub fn resolve_it_tag(
    filter: &ObjectFilter,
    refs: &ReferenceEnv,
) -> Result<ObjectFilter, CardTextError> {
    // "creatures tapped this way" not bound to a preceding "tap all" in the
    // same instruction list names the ordinary object antecedent.
    let tapped_marker = crate::tag::CompilerReferenceTag::TappedThisWay.as_str();
    let unbound_tapped_marker = filter
        .tagged_constraints
        .iter()
        .any(|constraint| constraint.tag.as_str() == tapped_marker);
    let rebound;
    let filter = if unbound_tapped_marker {
        let mut owned = filter.clone();
        for constraint in &mut owned.tagged_constraints {
            if constraint.tag.as_str() == tapped_marker {
                constraint.tag = crate::tag::CompilerReferenceTag::It.key();
            }
        }
        rebound = owned;
        &rebound
    } else {
        filter
    };
    let mut resolved = resolve_it_tag_inner(filter, refs)?;
    bind_other_to_related_it_object(filter, &mut resolved);
    exclude_this_resolution_source_exiles(&mut resolved);
    Ok(resolved)
}

/// "target creature and each other creature that shares a color with it":
/// `other` is relative to the object the relation names, not to the source.
/// Once that `it` resolves to a tagged object, exclude it by identity.
fn bind_other_to_related_it_object(original: &ObjectFilter, resolved: &mut ObjectFilter) {
    if !original.other || !resolved.other {
        return;
    }
    let relates_to_object = |relation: TaggedOpbjectRelation| {
        matches!(
            relation,
            TaggedOpbjectRelation::SharesCardType
                | TaggedOpbjectRelation::SharesPermanentType
                | TaggedOpbjectRelation::SharesSubtypeWithTagged
                | TaggedOpbjectRelation::SharesColorWithTagged
                | TaggedOpbjectRelation::SameNameAsTagged
                | TaggedOpbjectRelation::SameControllerAsTagged
                | TaggedOpbjectRelation::SameManaValueAsTagged
        )
    };
    let it = crate::tag::CompilerReferenceTag::It.as_str();
    let mut related_relations = original
        .tagged_constraints
        .iter()
        .filter(|constraint| {
            constraint.tag.as_str() == it && relates_to_object(constraint.relation)
        })
        .map(|constraint| constraint.relation);
    let (Some(relation), None) = (related_relations.next(), related_relations.next()) else {
        return;
    };
    let mut bound = resolved
        .tagged_constraints
        .iter()
        .filter(|constraint| constraint.relation == relation && constraint.tag.as_str() != it)
        .map(|constraint| constraint.tag.clone());
    let (Some(tag), None) = (bound.next(), bound.next()) else {
        return;
    };
    resolved.other = false;
    resolved
        .tagged_constraints
        .push(crate::filter::TaggedObjectConstraint {
            tag,
            relation: TaggedOpbjectRelation::IsNotTaggedObject,
        });
}

/// "Exile it, then return all other cards exiled with this artifact": an
/// `other` exclusion of an object this resolution exiled with the source
/// cannot read the source's link tag (the same full link set the filter
/// selects) or an exile result tag (the pre-move identity). It reads the
/// runtime's record of what this resolution exiled with its source.
fn exclude_this_resolution_source_exiles(filter: &mut ObjectFilter) {
    let source_exiled = crate::tag::CompilerReferenceTag::SourceExiled.as_str();
    if !filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag.as_str() == source_exiled
            && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
    }) {
        return;
    }
    for constraint in &mut filter.tagged_constraints {
        if constraint.relation == TaggedOpbjectRelation::IsNotTaggedObject
            && (constraint.tag.as_str() == source_exiled
                || is_exiled_collection_reference_tag(constraint.tag.as_str()))
        {
            constraint.tag = TagKey::from(ironsmith_core::SOURCE_EXILED_THIS_RESOLUTION_TAG);
        }
    }
}

fn resolve_it_tag_inner(
    filter: &ObjectFilter,
    refs: &ReferenceEnv,
) -> Result<ObjectFilter, CardTextError> {
    if let Some(rebound) = rebind_triggering_stack_target_filter(filter, refs) {
        return resolve_it_tag(&rebound, refs);
    }
    // "exile that creature until Hixus leaves the battlefield": a
    // demonstrative object never names the source even when the source is
    // the only live antecedent.
    // "create a token that's a copy of that Wizard": a subtype-headed pronoun
    // filter is a demonstrative too.
    let is_demonstrative = filter.demonstrative_antecedent_surface().is_some()
        || !filter.subtypes.is_empty()
        || matches!(
            &filter.source_surface,
            Some(SourceReferenceSurface::ThisPermanentType(text)) if text.starts_with("that ")
        );
    if is_demonstrative
        && filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
                && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
        })
        && let Some(previous) = demonstrative_superseded_antecedent(
            &[ChooseSpecSurfaceHint::SourceReference(
                SourceReferenceSurface::ThisPermanentType("that object".to_string()),
            )],
            refs,
        )
    {
        let mut rebound = filter.clone();
        for constraint in &mut rebound.tagged_constraints {
            if constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
                && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
            {
                constraint.tag = previous.clone();
            }
        }
        return resolve_it_tag(&rebound, refs);
    }
    // Search filters such as "a creature with exactly that many colors plus
    // one" carry the sacrificial object as an outer constraint, while the
    // aggregate value is initially parsed with the generic `it` marker. Bind
    // that nested marker to the same sacrifice-cost object before the normal
    // reference pass can discard an otherwise unbound `it` filter.
    let mut filter_with_context = filter.clone();
    bind_off_battlefield_other_to_antecedent(&mut filter_with_context, refs);
    if let Some(cost_tag) = filter.tagged_constraints.iter().find_map(|constraint| {
        (constraint.relation == TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag.as_str().starts_with("sacrifice_cost_"))
        .then(|| constraint.tag.clone())
    }) && let Some(color_count) = filter_with_context.color_count.as_mut()
        && let Comparison::EqualExpr(value)
        | Comparison::NotEqualExpr(value)
        | Comparison::LessThanExpr(value)
        | Comparison::LessThanOrEqualExpr(value)
        | Comparison::GreaterThanExpr(value)
        | Comparison::GreaterThanOrEqualExpr(value) = color_count
    {
        replace_it_tag_in_value(value, &cost_tag);
    }
    let mut resolved = resolve_object_filter_player_refs(&filter_with_context, refs)?;
    if let Some(attached_to_object) = resolved.attached_to_object.as_mut() {
        **attached_to_object = resolve_it_tag(attached_to_object, refs)?;
    }
    if let Some(host) = resolved.could_enchant_object.as_mut() {
        **host = resolve_it_tag(host, refs)?;
    }
    if let Some(combat_partner) = resolved.blocked_or_was_blocked_by_this_turn.as_mut() {
        **combat_partner = resolve_it_tag(combat_partner, refs)?;
    }
    for nested in &mut resolved.no_shared_creature_types_with {
        *nested = resolve_it_tag(nested, refs)?;
    }
    for relation in &mut resolved.characteristic_relations {
        relation.comparison = resolve_it_tag(&relation.comparison, refs)?;
    }
    for nested in &mut resolved.any_of {
        *nested = resolve_it_tag(nested, refs)?;
    }
    let plural_cost_reference = filter.has_plural_object_noun_surface()
        && filter.has_explicit_card_noun()
        && refs.known_last_object_tag().is_some_and(|tag| {
            tag.as_str() == crate::tag::CompilerReferenceTag::SourceExiled.as_str()
        })
        && refs.snapshot_tag_aliases.iter().any(|(alias, _)| {
            alias == &crate::tag::CompilerReferenceTag::AdditionalCostObject.key()
        });
    if plural_cost_reference {
        let cost_tag = refs
            .snapshot_tag_aliases
            .iter()
            .find(|(alias, _)| {
                alias == &crate::tag::CompilerReferenceTag::AdditionalCostObject.key()
            })
            .map(|(_, concrete)| concrete.clone())
            .expect("plural cost reference proved a snapshot above");
        for constraint in &mut resolved.tagged_constraints {
            if constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
                && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
            {
                constraint.tag = cost_tag.clone();
            }
        }
        // The paid objects may already have changed zones. Their result tag,
        // rather than the pre-cost battlefield zone inferred from the noun,
        // is the stable identity needed by the follow-up move.
        resolved.zone = None;
    }
    let exiled_this_way = refs
        .snapshot_tag_aliases
        .iter()
        .filter(|(alias, _)| alias == &crate::tag::CompilerReferenceTag::ExiledThisWay.key())
        .map(|(_, exiled)| exiled.clone())
        .collect::<Vec<_>>();
    if filter.prior_effect_action_surface() == Some(ironsmith_core::PriorEffectAction::Exiled)
        && let [exiled] = exiled_this_way.as_slice()
    {
        for constraint in &mut resolved.tagged_constraints {
            if constraint.tag == crate::tag::CompilerReferenceTag::It.key() {
                constraint.tag = exiled.clone();
            }
        }
    } else if filter.prior_effect_action_surface()
        == Some(ironsmith_core::PriorEffectAction::Exiled)
        && exiled_this_way.len() > 1
        && let Some(index) = resolved
            .tagged_constraints
            .iter()
            .position(|constraint| constraint.tag == crate::tag::CompilerReferenceTag::It.key())
    {
        // One instruction exiled several groups: the object is any of them.
        let relation = resolved.tagged_constraints.remove(index).relation;
        resolved.any_of = exiled_this_way
            .into_iter()
            .map(|exiled| {
                let mut member = ObjectFilter::default();
                member.tagged_constraints.push(crate::filter::TaggedObjectConstraint {
                    tag: exiled,
                    relation,
                });
                member
            })
            .collect();
    }
    let revealed_collection_tag = (filter.prior_effect_action_surface()
        == Some(ironsmith_core::PriorEffectAction::Revealed))
    .then(|| {
        refs.snapshot_tag_aliases
            .iter()
            .find(|(alias, _)| alias == &crate::tag::CompilerReferenceTag::PublicRevealed.key())
            .map(|(_, concrete)| concrete.clone())
    })
    .flatten();
    if let Some(revealed_collection_tag) = revealed_collection_tag {
        for constraint in &mut resolved.tagged_constraints {
            if constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
                && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
            {
                // A typed "cards revealed this way" filter names the complete
                // consult exposure, while ordinary `it` still names the
                // singular matching card kept in last-object memory.
                constraint.tag = revealed_collection_tag.clone();
            }
        }
    }
    if !refs.snapshot_tag_aliases.is_empty() {
        for constraint in &mut resolved.tagged_constraints {
            if let Some((_, concrete)) = refs
                .snapshot_tag_aliases
                .iter()
                .find(|(alias, _)| alias == &constraint.tag)
            {
                constraint.tag = concrete.clone();
            }
        }
    }
    if let Some(cost_tag) = refs.known_last_object_tag() {
        for constraint in &mut resolved.tagged_constraints {
            if constraint.tag.as_str()
                == crate::tag::CompilerReferenceTag::AdditionalCostObject.as_str()
            {
                constraint.tag = cost_tag.clone();
            }
        }
    }
    let source_exiled_set_excludes_current =
        filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == crate::tag::CompilerReferenceTag::SourceExiled.as_str()
                && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
        }) && (filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
                && constraint.relation == TaggedOpbjectRelation::IsNotTaggedObject
        }) || (filter.other && filter.zone == Some(Zone::Exile)));
    if let Some(tag) = refs.known_last_object_tag()
        && tag.as_str() != crate::tag::CompilerReferenceTag::SourceExiled.as_str()
        && tag.as_str() != "triggering"
        && !source_exiled_set_excludes_current
    {
        for constraint in &mut resolved.tagged_constraints {
            if constraint.tag.as_str() == crate::tag::CompilerReferenceTag::SourceExiled.as_str()
                && is_exiled_collection_reference_tag(tag.as_str())
            {
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
    if resolved.tagged_constraints.iter().any(|constraint| {
        constraint.tag.as_str() == crate::tag::CompilerReferenceTag::ThoseCardsReference.as_str()
    }) {
        let collection = refs
            .known_last_object_tag()
            .cloned()
            .unwrap_or_else(|| crate::tag::CompilerReferenceTag::Triggering.key());
        for constraint in &mut resolved.tagged_constraints {
            if constraint.tag.as_str()
                == crate::tag::CompilerReferenceTag::ThoseCardsReference.as_str()
            {
                constraint.tag = collection.clone();
            }
        }
    }
    if resolved.tagged_constraints.iter().any(|constraint| {
        constraint.tag.as_str() == crate::tag::CompilerReferenceTag::DiscardedCardReference.as_str()
    }) {
        let discarded = refs
            .known_last_object_tag()
            .filter(|tag| is_discard_result_reference_tag(tag.as_str()))
            .cloned()
            .unwrap_or_else(|| crate::tag::CompilerReferenceTag::Triggering.key());
        for constraint in &mut resolved.tagged_constraints {
            if constraint.tag.as_str()
                == crate::tag::CompilerReferenceTag::DiscardedCardReference.as_str()
            {
                constraint.tag = discarded.clone();
            }
        }
    }
    if !filter
        .tagged_constraints
        .iter()
        .any(|constraint| constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str())
    {
        clear_redundant_live_combat_role_for_event_tag(&mut resolved);
        return Ok(resolved);
    }

    if refs.has_source_object_antecedent()
        && refs
            .known_last_object_tag()
            .is_none_or(|tag| tag.as_str() == crate::tag::CompilerReferenceTag::Triggering.as_str())
        && resolved.attached_to_object.is_none()
        && resolved.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
                && constraint.relation == TaggedOpbjectRelation::AttachedToTaggedObject
        })
    {
        // In a source-triggered clause, "attached to it" relates the
        // counted objects to the source; it is not a disposable identity
        // qualifier on those objects.
        resolved.attached_to_object = Some(Box::new(ObjectFilter {
            source: true,
            ..ObjectFilter::default()
        }));
        resolved.tagged_constraints.retain(|constraint| {
            constraint.tag.as_str() != crate::tag::CompilerReferenceTag::It.as_str()
                || constraint.relation != TaggedOpbjectRelation::AttachedToTaggedObject
        });
        if !resolved.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
        }) {
            return Ok(resolved);
        }
    }
    // "Whenever you discard a card, you may sacrifice this creature. If you
    // do, return the discarded card ..." (Pitchstone Wall): with no tagged
    // antecedent left (the source was the last object), the discarded card
    // is the triggering card, never an arbitrary card in the zone.
    let Some(tag) = refs.known_last_object_tag() else {
        let mut saw_it_constraint = false;
        let mut preserved_runtime_it_constraint = false;
        resolved.tagged_constraints.retain(|constraint| {
            if constraint.tag.as_str() != crate::tag::CompilerReferenceTag::It.as_str() {
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

        let mut identity = resolved.clone();
        identity.source_surface = None;
        let identity_is_unqualified = identity == ObjectFilter::default();

        if saw_it_constraint && refs.has_source_object_antecedent() && identity_is_unqualified {
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
            if looked_at_hand_antecedent(refs).is_some() {
                // "Look at that player's hand ... from among those cards":
                // the looked-at cards are still in that player's hand.
                resolved.zone = Some(Zone::Hand);
            }
            if resolved.owner.is_none() {
                resolved.owner = Some(as_followup_player_alias(player_filter));
            }
            return Ok(resolved);
        }
        if saw_it_constraint
            && identity_is_unqualified
            && let Some(tag) = looked_at_hand_antecedent(refs)
        {
            resolved
                .tagged_constraints
                .push(crate::filter::TaggedObjectConstraint {
                    tag,
                    relation: TaggedOpbjectRelation::IsTaggedObject,
                });
            return Ok(resolved);
        }
        if saw_it_constraint && identity_is_unqualified {
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

    let tag = if definite_noun_excludes_antecedent(filter, tag)
        && let Some((_, prior)) = refs.snapshot_tag_aliases.iter().find(|(alias, _)| {
            alias == &crate::tag::CompilerReferenceTag::PriorObjectAntecedent.key()
        })
        && !definite_noun_excludes_antecedent(filter, prior)
    {
        prior
    } else {
        tag
    };
    for constraint in &mut resolved.tagged_constraints {
        if constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str() {
            constraint.tag = if source_exiled_set_excludes_current
                && constraint.relation == TaggedOpbjectRelation::IsNotTaggedObject
                && tag.as_str() == crate::tag::CompilerReferenceTag::SourceExiled.as_str()
            {
                // "Each other card exiled with ~": the full linked set minus
                // what this resolution just exiled with the source.
                ironsmith_compiler_semantic::tag::declared_key(
                    ironsmith_core::tag::SOURCE_EXILED_THIS_RESOLUTION_TAG,
                )
                .into()
            } else {
                tag.clone()
            };
        }
    }
    clear_redundant_live_combat_role_for_event_tag(&mut resolved);
    Ok(resolved)
}

fn clear_redundant_live_combat_role_for_event_tag(filter: &mut ObjectFilter) {
    for constraint in &filter.tagged_constraints {
        if constraint.relation != TaggedOpbjectRelation::IsTaggedObject {
            continue;
        }
        match constraint.tag.as_str() {
            "blocking" => filter.blocking = false,
            "blocked" => filter.attacking = false,
            _ => {}
        }
    }
}

/// Records a private look at a player's hand as the antecedent of a later
/// unqualified back-reference ("look at target opponent's hand and exile
/// those cards"). The runtime look tags exactly the looked-at cards under
/// `LOOKED_AT_HAND_TAG`.
///
/// The look deliberately leaves `last_object_tag` empty so a qualified
/// selection ("choose a nonland card from it") keeps resolving to that
/// player's hand; this identity alias is consulted only where an unqualified
/// pronoun would otherwise fall back to the source object.
pub fn remember_looked_at_hand(aliases: &mut Vec<(TagKey, TagKey)>) {
    let tag: TagKey = (crate::tag::CompilerReferenceTag::LookedAtHand.bind()).into();
    aliases.retain(|(alias, _)| alias != &tag);
    aliases.push((tag.clone(), tag));
}

/// The looked-at hand, when it is the live antecedent of an unqualified
/// pronoun (no object reference or source antecedent since the look).
fn looked_at_hand_antecedent(refs: &ReferenceEnv) -> Option<TagKey> {
    if refs.known_last_object_tag().is_some() || refs.has_source_object_antecedent() {
        return None;
    }
    let key = crate::tag::CompilerReferenceTag::LookedAtHand.as_str();
    refs.snapshot_tag_aliases
        .iter()
        .find(|(alias, _)| alias.as_str() == key)
        .map(|(_, concrete)| concrete.clone())
}

pub fn resolve_it_tag_key(tag: &TagKey, refs: &ReferenceEnv) -> Result<TagKey, CardTextError> {
    if let Some((_, concrete)) = refs
        .snapshot_tag_aliases
        .iter()
        .find(|(alias, _)| alias == tag)
    {
        return Ok(concrete.clone());
    }
    if tag.as_str() == crate::tag::PRIOR_TAPPED_OBJECT_QUANTITY_TAG {
        return Err(CardTextError::ParseError(
            "tapped-object quantity requires a prior tap instruction or imported tap cost".into(),
        ));
    }
    if tag.as_str() == crate::tag::RETURNED_THIS_WAY_QUANTITY_TAG {
        return Err(CardTextError::ParseError(
            "returned-object quantity requires a prior return instruction".into(),
        ));
    }
    if tag.as_str() == crate::tag::CompilerReferenceTag::SourceExiled.as_str() {
        // A local exile result can supply this reference; an unrelated event
        // object cannot replace the source's persistent linked exile set.
        return Ok(refs
            .known_last_object_tag()
            .filter(|known| {
                is_exiled_collection_reference_tag(known.as_str())
                    || is_cost_exiled_reference_tag(known)
            })
            .cloned()
            .unwrap_or_else(|| tag.clone()));
    }
    if tag.as_str() == crate::tag::CompilerReferenceTag::AdditionalCostObject.as_str() {
        return refs.known_last_object_tag().cloned().ok_or_else(|| {
            CardTextError::ParseError(
                "unable to resolve additional-cost object without a prior object".to_string(),
            )
        });
    }
    if tag.as_str() == crate::tag::CompilerReferenceTag::ThisWaySacrificed.as_str() {
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
    if let Some(cost_index) = tag.as_str().strip_prefix("tap_cost_")
        && let Some(resolved) = refs.known_last_object_tag()
        && resolved.as_str().strip_prefix("tapped_") == Some(cost_index)
    {
        return Ok(resolved.clone());
    }
    if tag.as_str() != crate::tag::CompilerReferenceTag::It.as_str() {
        return Ok(tag.clone());
    }
    let resolved = refs.known_last_object_tag().ok_or_else(|| {
        CardTextError::ParseError("unable to resolve 'it' without prior reference".to_string())
    })?;
    Ok(resolved.clone())
}

pub fn object_filter_as_tagged_reference(filter: &ObjectFilter) -> Option<TagKey> {
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
    bare.source_surface = None;
    if bare == ObjectFilter::default() {
        Some(constraint.tag.clone())
    } else {
        None
    }
}

pub fn watch_tag_from_filter(filter: &ObjectFilter) -> Option<TagKey> {
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

/// The bare pronoun subject of an attack/block restriction ("they can't
/// attack you this combat") whose only antecedent is a player names that
/// player's creatures: a player is not an object that can attack or block.
fn resolve_combat_actor_it_tag(
    filter: &ObjectFilter,
    refs: &ReferenceEnv,
) -> Result<ObjectFilter, CardTextError> {
    let mut identity = filter.clone();
    identity.source_surface = None;
    let bare_it = identity == ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.bind());
    if bare_it
        && refs.known_last_object_tag().is_none()
        && !refs.has_source_object_antecedent()
        && let Some(player) = refs.known_last_player_filter().cloned()
    {
        let mut creatures = ObjectFilter::creature();
        creatures.controller = Some(as_followup_player_alias(player));
        return Ok(creatures);
    }
    resolve_it_tag(filter, refs)
}

pub fn resolve_restriction_it_tag(
    restriction: &Restriction,
    refs: &ReferenceEnv,
) -> Result<Restriction, CardTextError> {
    let resolved = match restriction {
        Restriction::PreventDamageFrom {
            sources,
            combat_only,
        } => Restriction::PreventDamageFrom {
            sources: resolve_it_tag(sources, refs)?,
            combat_only: *combat_only,
        },
        Restriction::AdditionalLandPlays(player, count) => Restriction::additional_land_plays(
            resolve_contextual_player_filter(player, refs)?,
            *count,
        ),
        Restriction::NoMaximumHandSize(player) => {
            Restriction::no_maximum_hand_size(resolve_contextual_player_filter(player, refs)?)
        }
        Restriction::GainLife(player) => {
            Restriction::gain_life(resolve_contextual_player_filter(player, refs)?)
        }
        Restriction::SearchOwnLibraryFromOwnEffects(player) => {
            Restriction::SearchOwnLibraryFromOwnEffects(resolve_contextual_player_filter(
                player, refs,
            )?)
        }
        Restriction::SearchLibraries(player) => {
            Restriction::search_libraries(resolve_contextual_player_filter(player, refs)?)
        }
        Restriction::PlayLandsMatching(player, filter) => Restriction::PlayLandsMatching(
            resolve_contextual_player_filter(player, refs)?,
            resolve_it_tag(filter, refs)?,
        ),
        Restriction::ActivateLoyaltyAbilitiesOf(filter) => {
            Restriction::ActivateLoyaltyAbilitiesOf(resolve_it_tag(filter, refs)?)
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
        Restriction::LoseGameForZeroLife(player) => {
            Restriction::LoseGameForZeroLife(resolve_contextual_player_filter(player, refs)?)
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
        Restriction::Attack(filter) => {
            Restriction::attack(resolve_combat_actor_it_tag(filter, refs)?)
        }
        Restriction::AttackPlayerOrPlaneswalkersControlledBy { attackers, player } => {
            Restriction::attack_player_or_planeswalkers_controlled_by(
                resolve_combat_actor_it_tag(attackers, refs)?,
                resolve_contextual_player_filter(player, refs)?,
            )
        }
        Restriction::AttackPlayer { attackers, player } => Restriction::attack_player(
            resolve_combat_actor_it_tag(attackers, refs)?,
            resolve_contextual_player_filter(player, refs)?,
        ),
        Restriction::Block(filter) => {
            Restriction::block(resolve_combat_actor_it_tag(filter, refs)?)
        }
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
        Restriction::MustAttack(filter) => Restriction::must_attack(resolve_it_tag(filter, refs)?),
        Restriction::MustAttackPlayer { attackers, player } => Restriction::must_attack_player(
            resolve_combat_actor_it_tag(attackers, refs)?,
            resolve_contextual_player_filter(player, refs)?,
        ),
        Restriction::MustBlock(filter) => Restriction::must_block(resolve_it_tag(filter, refs)?),
        Restriction::MustBeBlocked(filter) => {
            Restriction::must_be_blocked(resolve_it_tag(filter, refs)?)
        }
        Restriction::Untap(filter) => Restriction::untap(resolve_it_tag(filter, refs)?),
        Restriction::BecomeUntapped(filter) => {
            Restriction::become_untapped(resolve_it_tag(filter, refs)?)
        }
        Restriction::AttackBlockOrCrew(filter) => {
            Restriction::attack_block_or_crew(resolve_it_tag(filter, refs)?)
        }
        Restriction::BeAttachedBy(hosts, attachments) => Restriction::be_attached_by(
            resolve_it_tag(hosts, refs)?,
            resolve_it_tag(attachments, refs)?,
        ),
        Restriction::BeBlocked(filter) => Restriction::be_blocked(resolve_it_tag(filter, refs)?),
        Restriction::BeDestroyed(filter) => {
            Restriction::be_destroyed(resolve_it_tag(filter, refs)?)
        }
        Restriction::BeRegenerated(filter) => {
            Restriction::be_regenerated(resolve_it_tag(filter, refs)?)
        }
        Restriction::BeSacrificedByCause { filter, cause } => {
            let mut cause = cause.clone();
            cause.source_filter = cause
                .source_filter
                .as_ref()
                .map(|source| resolve_it_tag(source, refs))
                .transpose()?;
            Restriction::BeSacrificedByCause {
                filter: resolve_it_tag(filter, refs)?,
                cause,
            }
        }
        Restriction::MaximumBlockers { filter, maximum } => Restriction::MaximumBlockers {
            filter: resolve_it_tag(filter, refs)?,
            maximum: *maximum,
        },
        Restriction::BecomeSuspected(filter) => {
            Restriction::BecomeSuspected(resolve_it_tag(filter, refs)?)
        }
        Restriction::BeSacrificed(filter) => {
            Restriction::be_sacrificed(resolve_it_tag(filter, refs)?)
        }
        Restriction::HaveCountersPlaced(filter) => {
            Restriction::have_counters_placed(resolve_it_tag(filter, refs)?)
        }
        Restriction::HaveCounterTypePlaced(filter, counter_type) => {
            Restriction::have_counter_type_placed(resolve_it_tag(filter, refs)?, *counter_type)
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
        Restriction::PlayerHexproofFrom(player, source_filter) => {
            Restriction::player_hexproof_from(
                resolve_contextual_player_filter(player, refs)?,
                resolve_it_tag(source_filter, refs)?,
            )
        }
        Restriction::BeCountered(filter) => {
            Restriction::be_countered(resolve_it_tag(filter, refs)?)
        }
        Restriction::Transform(filter) => Restriction::transform(resolve_it_tag(filter, refs)?),
        Restriction::TurnFaceUp(filter) => Restriction::turn_face_up(resolve_it_tag(filter, refs)?),
        Restriction::PhaseOut(filter) => Restriction::phase_out(resolve_it_tag(filter, refs)?),
        Restriction::PhaseIn(filter) => Restriction::phase_in(resolve_it_tag(filter, refs)?),
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

/// The antecedent a "that spell" demonstrative keeps naming after a newer
/// object that can never be a spell (a created token, or a card revealed,
/// looked at, milled, or discarded) became the last object.
fn spell_demonstrative_prior_antecedent(refs: &ReferenceEnv) -> Option<TagKey> {
    let last = refs.known_last_object_tag()?;
    if !(is_noun_restricted_object_result_tag(last)
        || last.as_str().starts_with("destroyed_")
        || is_sacrificed_object_reference_tag(last.as_str()))
    {
        return None;
    }
    refs.snapshot_tag_aliases
        .iter()
        .find(|(alias, _)| {
            alias.as_str() == crate::tag::CompilerReferenceTag::PriorObjectAntecedent.as_str()
        })
        .map(|(_, prior)| prior.clone())
        .filter(|prior| !is_noun_restricted_object_result_tag(prior))
}

/// Alias marking that the trigger's event object (`triggering`) is a spell or
/// ability whose trigger condition named what it targets ("whenever you cast
/// a spell that targets only a single creature"). A demonstrative whose noun
/// names a permanent ("that creature", "those permanents") then denotes the
/// stack object's targets, never the stack object itself.
pub fn triggering_stack_targets_alias() -> TagKey {
    TagKey::from("__triggering_stack_targets__")
}

/// The permanent card types a demonstrative surface names ("that creature",
/// "those permanents", "that artifact or creature"). `None` for any other
/// surface, including "that spell", "that card", and bare pronouns.
fn demonstrative_permanent_noun_types(text: &str) -> Option<Vec<crate::types::CardType>> {
    use crate::types::CardType;
    let mut words = text.split_whitespace();
    if !matches!(words.next(), Some("that" | "those")) {
        return None;
    }
    let mut types = Vec::new();
    let mut saw_noun = false;
    for word in words {
        let singular = word.strip_suffix('s').unwrap_or(word);
        let found: &[CardType] = match singular {
            "or" | "and" => continue,
            "artifact" => &[CardType::Artifact],
            "creature" => &[CardType::Creature],
            "enchantment" => &[CardType::Enchantment],
            "land" => &[CardType::Land],
            "planeswalker" => &[CardType::Planeswalker],
            "battle" => &[CardType::Battle],
            "permanent" => &[
                CardType::Artifact,
                CardType::Creature,
                CardType::Enchantment,
                CardType::Land,
                CardType::Planeswalker,
                CardType::Battle,
            ],
            _ => return None,
        };
        saw_noun = true;
        for card_type in found {
            if !types.contains(card_type) {
                types.push(*card_type);
            }
        }
    }
    saw_noun.then_some(types)
}

/// Whether `It` currently resolves to a triggering stack object whose trigger
/// named its targets.
fn it_names_triggering_stack_object_with_targets(refs: &ReferenceEnv) -> bool {
    let Some(last) = refs.known_last_object_tag() else {
        return false;
    };
    let alias = triggering_stack_targets_alias();
    refs.snapshot_tag_aliases
        .iter()
        .any(|(existing, tag)| existing == &alias && tag == last)
}

/// "gain control of that creature" after "whenever you cast a spell that
/// targets only a single creature": the permanent the triggering stack object
/// targets.
fn triggering_stack_target_spec(
    hints: &[ChooseSpecSurfaceHint],
    refs: &ReferenceEnv,
) -> Option<ChooseSpec> {
    if !it_names_triggering_stack_object_with_targets(refs) {
        return None;
    }
    let types = hints.iter().find_map(|hint| match hint {
        ChooseSpecSurfaceHint::SourceReference(SourceReferenceSurface::ThisPermanentType(text)) => {
            demonstrative_permanent_noun_types(text)
        }
        _ => None,
    })?;
    let stack_object = refs.known_last_object_tag()?.clone();
    let mut filter = ObjectFilter::default();
    filter.zone = Some(Zone::Battlefield);
    filter.card_types = types;
    filter
        .tagged_constraints
        .push(crate::filter::TaggedObjectConstraint {
            tag: stack_object,
            relation: TaggedOpbjectRelation::TargetedByTaggedObject,
        });
    Some(ChooseSpec::Object(filter))
}

/// The filter form of [`triggering_stack_target_spec`]: "gain control of
/// those permanents" carries its noun as card types on an `It` filter.
fn rebind_triggering_stack_target_filter(
    filter: &ObjectFilter,
    refs: &ReferenceEnv,
) -> Option<ObjectFilter> {
    if !it_names_triggering_stack_object_with_targets(refs) {
        return None;
    }
    let it = crate::tag::CompilerReferenceTag::It.as_str();
    if !filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag.as_str() == it
            && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
    }) {
        return None;
    }
    let surface_types = match &filter.source_surface {
        Some(SourceReferenceSurface::ThisPermanentType(text)) => {
            demonstrative_permanent_noun_types(text)
        }
        _ => None,
    };
    let noun_is_permanent = surface_types.is_some()
        || (!filter.card_types.is_empty()
            && filter.card_types.iter().all(|card_type| {
                matches!(
                    card_type,
                    crate::types::CardType::Artifact
                        | crate::types::CardType::Creature
                        | crate::types::CardType::Enchantment
                        | crate::types::CardType::Land
                        | crate::types::CardType::Planeswalker
                        | crate::types::CardType::Battle
                )
            })
            && filter.demonstrative_antecedent_surface().is_some());
    if !noun_is_permanent {
        return None;
    }
    let stack_object = refs.known_last_object_tag()?.clone();
    let mut rebound = filter.clone();
    for constraint in &mut rebound.tagged_constraints {
        if constraint.tag.as_str() == it
            && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
        {
            constraint.tag = stack_object.clone();
            constraint.relation = TaggedOpbjectRelation::TargetedByTaggedObject;
        }
    }
    // "create a token that's a copy of that artifact or creature": the noun
    // union distributes the demonstrative into its first branch only. That
    // branch names the same targeted permanent as the outer filter, never the
    // triggering spell itself.
    retarget_triggering_stack_union_branches(&mut rebound, &stack_object);
    if rebound.card_types.is_empty()
        && let Some(types) = surface_types
    {
        rebound.card_types = types;
    }
    if rebound.zone.is_none() {
        rebound.zone = Some(Zone::Battlefield);
    }
    Some(rebound)
}

fn retarget_triggering_stack_union_branches(filter: &mut ObjectFilter, stack_object: &TagKey) {
    if filter.any_of.is_empty() {
        return;
    }
    let it = crate::tag::CompilerReferenceTag::It.as_str();
    for branch in &mut filter.any_of {
        for constraint in &mut branch.tagged_constraints {
            if constraint.tag.as_str() == it
                && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
            {
                constraint.tag = stack_object.clone();
                constraint.relation = TaggedOpbjectRelation::TargetedByTaggedObject;
            }
        }
        retarget_triggering_stack_union_branches(branch, stack_object);
    }
    // Once every branch only restates the outer targeted-permanent constraint
    // and one of the outer card types, the union adds nothing: fold it away.
    let outer_types = filter.card_types.clone();
    let outer_constraints = filter.tagged_constraints.clone();
    let redundant = filter.any_of.iter().all(|branch| {
        if branch.card_types.is_empty()
            || !branch
                .card_types
                .iter()
                .all(|card_type| outer_types.contains(card_type))
        {
            return false;
        }
        let mut residue = branch.clone();
        residue.card_types.clear();
        residue
            .tagged_constraints
            .retain(|constraint| !outer_constraints.contains(constraint));
        residue == ObjectFilter::default()
    });
    let covered = outer_types.iter().all(|card_type| {
        filter
            .any_of
            .iter()
            .any(|branch| branch.card_types.contains(card_type))
    });
    if redundant && covered {
        filter.any_of.clear();
    }
}

/// Alias recording the object antecedent an explicit source subject
/// superseded ("sacrifice this artifact and create X tokens, where X is that
/// creature's mana value": `that creature` is still the entering creature).
pub fn source_superseded_antecedent_alias() -> TagKey {
    TagKey::from("__source_superseded_antecedent__")
}

/// A demonstrative surface ("that creature", "that spell") never names the
/// source; when the source is the only live antecedent, it names the object
/// the source subject superseded.
fn demonstrative_superseded_antecedent(
    hints: &[ChooseSpecSurfaceHint],
    refs: &ReferenceEnv,
) -> Option<TagKey> {
    if refs.known_last_object_tag().is_some() || !refs.has_source_object_antecedent() {
        return None;
    }
    let demonstrative = hints.iter().any(|hint| {
        matches!(
            hint,
            ChooseSpecSurfaceHint::SourceReference(SourceReferenceSurface::ThisPermanentType(text))
                if text.starts_with("that ")
        )
    });
    if !demonstrative {
        return None;
    }
    let alias = source_superseded_antecedent_alias();
    refs.snapshot_tag_aliases
        .iter()
        .find(|(existing, _)| existing == &alias)
        .map(|(_, previous)| previous.clone())
}

pub fn resolve_choose_spec_it_tag(
    spec: &ChooseSpec,
    refs: &ReferenceEnv,
) -> Result<ChooseSpec, CardTextError> {
    let resolved = resolve_choose_spec_it_tag_preserving_selection(spec, refs, false)?;
    if resolved.is_target() && super::reference_resolution::target_reads_unpaid_counter_cost(&resolved)
        && !(refs.counter_removal_cost.is_some_and(|producer| producer.can_announce_quantity) && resolved.is_activation_counter_power_bound()) {
        return Err(CardTextError::ParseError("counter-cost target quantity requires prospective announcement admission".into()));
    }
    Ok(resolved)
}

fn resolve_choose_spec_it_tag_preserving_selection(
    spec: &ChooseSpec,
    refs: &ReferenceEnv,
    preserve_selection: bool,
) -> Result<ChooseSpec, CardTextError> {
    if let ChooseSpec::SurfaceHinted { spec: inner, hints } = spec
        && matches!(
            inner.as_ref(),
            ChooseSpec::Tagged(tag)
                if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
        )
        && hints.iter().any(|hint| {
            matches!(
                hint,
                crate::target::ChooseSpecSurfaceHint::SourceReference(
                    SourceReferenceSurface::ThisPermanentType(surface)
                ) if surface == "that spell"
            )
        })
        && let Some(prior) = spell_demonstrative_prior_antecedent(refs)
    {
        // "put X +1/+1 counters on it (X is that spell's mana value)": a
        // created token or a looked-at card is never "that spell".
        return Ok(ChooseSpec::SurfaceHinted {
            spec: Box::new(ChooseSpec::Tagged(prior)),
            hints: hints.clone(),
        });
    }
    if let ChooseSpec::SurfaceHinted { spec: inner, hints } = spec
        && matches!(
            inner.as_ref(),
            ChooseSpec::Tagged(tag)
                if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
        )
        && let Some(targets) = triggering_stack_target_spec(hints, refs)
    {
        return Ok(ChooseSpec::SurfaceHinted {
            spec: Box::new(targets),
            hints: hints.clone(),
        });
    }
    if let ChooseSpec::SurfaceHinted { spec: inner, hints } = spec
        && matches!(
            inner.as_ref(),
            ChooseSpec::Tagged(tag)
                if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
        )
        && let Some(previous) = demonstrative_superseded_antecedent(hints, refs)
    {
        return Ok(ChooseSpec::SurfaceHinted {
            spec: Box::new(ChooseSpec::Tagged(previous)),
            hints: hints.clone(),
        });
    }
    match spec {
        ChooseSpec::SurfaceHinted { spec, hints } => Ok(ChooseSpec::SurfaceHinted {
            spec: Box::new(resolve_choose_spec_it_tag_preserving_selection(
                spec,
                refs,
                preserve_selection,
            )?),
            hints: hints.clone(),
        }),
        ChooseSpec::Tagged(tag)
            if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str() =>
        {
            if refs
                .known_last_object_tag()
                .is_some_and(|tag| tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str())
            {
                // A player loop can still bind `__it__` through a choice made
                // by that player. Only an object loop supplies the runtime's
                // `iterated_object`; preserve the tag for per-player choices.
                return Ok(if refs.iterated_object {
                    ChooseSpec::Iterated
                } else {
                    ChooseSpec::Tagged(
                        (ironsmith_compiler_semantic::tag::declared_key(
                            crate::tag::CompilerReferenceTag::It.as_str(),
                        ))
                        .into(),
                    )
                });
            }
            if let Some(resolved) = refs.known_last_object_tag() {
                return Ok(ChooseSpec::Tagged(
                    (ironsmith_compiler_semantic::tag::declared_key(resolved.as_str())).into(),
                ));
            }
            if refs.has_source_object_antecedent() {
                return Ok(ChooseSpec::Source);
            }
            if let Some(tag) = looked_at_hand_antecedent(refs) {
                return Ok(ChooseSpec::Tagged(tag));
            }
            Ok(ChooseSpec::Source)
        }
        ChooseSpec::Tagged(tag) => Ok(ChooseSpec::Tagged(resolve_it_tag_key(tag, refs)?)),
        ChooseSpec::Object(filter) => {
            let resolved = resolve_it_tag(filter, refs)?;
            if resolved.source && resolved.zone.is_none() {
                Ok(source_reference_hinted_spec(
                    ChooseSpec::Source,
                    resolved.source_surface.clone(),
                ))
            } else if preserve_selection {
                // A targeted linked set is a candidate filter, not an
                // already-selected object. Preserve its zone and membership
                // constraints for announcement and resolution legality.
                Ok(ChooseSpec::Object(resolved))
            } else if let Some(tag) = object_filter_as_tagged_reference(&resolved) {
                let identity = resolve_choose_spec_it_tag(&ChooseSpec::Tagged(tag), refs)?;
                Ok(source_reference_hinted_spec(
                    identity,
                    resolved.source_surface.clone(),
                ))
            } else {
                Ok(ChooseSpec::Object(resolved))
            }
        }
        ChooseSpec::ObjectOrPlayer(object_filter, player_filter) => Ok(ChooseSpec::ObjectOrPlayer(
            resolve_it_tag(object_filter, refs)?,
            resolve_contextual_player_filter(player_filter, refs)?,
        )),
        ChooseSpec::Target(inner) => {
            let resolved = resolve_choose_spec_it_tag_preserving_selection(inner, refs, true)?;
            if matches!(resolved.base(), ChooseSpec::Source) {
                Ok(resolved)
            } else {
                Ok(ChooseSpec::Target(Box::new(resolved)))
            }
        }
        ChooseSpec::WithCount(inner, count) => Ok(ChooseSpec::WithCount(
            Box::new(resolve_choose_spec_it_tag_preserving_selection(
                inner,
                refs,
                preserve_selection,
            )?),
            *count,
        )),
        ChooseSpec::WithCountValue(inner, count, value) => Ok(ChooseSpec::WithCountValue(
            Box::new(resolve_choose_spec_it_tag_preserving_selection(
                inner,
                refs,
                preserve_selection,
            )?),
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

pub fn resolve_value_it_tag(value: &Value, refs: &ReferenceEnv) -> Result<Value, CardTextError> {
    match value {
        Value::EventValue(EventValueSpec::DieResultsAtLeast(_) | EventValueSpec::DieBatchTotal) => {
            if !refs.die_result_producers.is_empty() || refs.dice_event_grouped != Some(true) {
                return Err(CardTextError::ParseError("grouped die results require a proven grouped trigger; a local result-set binding is not available".into()));
            }
            Ok(value.clone())
        }

        Value::DamageHistory(query) => {
            let mut query = (**query).clone();
            for spec in query.reference_specs_mut() {
                *spec = resolve_choose_spec_it_tag(spec, refs)?;
            }
            for filter in query.object_filters_mut() {
                *filter = resolve_it_tag(filter, refs)?;
            }
            if let Some(player) = query.player_filter_mut() {
                *player = resolve_contextual_player_filter(player, refs)?;
            }
            Ok(Value::DamageHistory(Box::new(query)))
        }

        Value::PendingComparisonLeft
        | Value::PendingComparisonRight
        | Value::PendingComparisonDifference => {
            let crate::model::reference_state::RefState::Known((left, right)) =
                &refs.last_value_comparison
            else {
                return Err(CardTextError::ParseError(
                    "comparison reference requires an earlier explicit value comparison".into(),
                ));
            };
            Ok(match value {
                Value::PendingComparisonLeft => left.clone(),
                Value::PendingComparisonRight => right.clone(),
                _ => Value::absolute_difference(left.clone(), right.clone()).with_surface_hints([
                    ironsmith_core::ValueSurfaceHint::Difference,
                    ironsmith_core::ValueSurfaceHint::ComparisonDifferenceReference,
                ]),
            })
        }

        Value::CardsInHand(player) => Ok(Value::CardsInHand(resolve_contextual_player_filter(
            player, refs,
        )?)),
        Value::CardsInLibrary(player) => Ok(Value::CardsInLibrary(
            resolve_contextual_player_filter(player, refs)?,
        )),
        Value::CardsInGraveyard(player) => Ok(Value::CardsInGraveyard(
            resolve_contextual_player_filter(player, refs)?,
        )),
        Value::LifeTotal(player) => Ok(Value::LifeTotal(resolve_contextual_player_filter(
            player, refs,
        )?)),
        Value::StartingLifeTotal(player) => Ok(Value::StartingLifeTotal(
            resolve_contextual_player_filter(player, refs)?,
        )),
        Value::MaxCardsInHand(player) => Ok(Value::MaxCardsInHand(
            resolve_contextual_player_filter(player, refs)?,
        )),
        Value::LifeLostThisTurn(player) => Ok(Value::LifeLostThisTurn(
            resolve_contextual_player_filter(player, refs)?,
        )),
        Value::LifeGainedThisTurn(player) => Ok(Value::LifeGainedThisTurn(
            resolve_contextual_player_filter(player, refs)?,
        )),
        Value::X if refs.bind_unbound_x_to_last_effect && !refs.has_announced_x => {
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
        // "equal to half that card's mana value, rounded up": arithmetic
        // wrappers keep the object reference of the value they transform.
        Value::HalfRoundedDown(value) => Ok(Value::HalfRoundedDown(Box::new(
            resolve_value_it_tag(value, refs)?,
        ))),
        Value::DividedRoundedDown(value, divisor) => Ok(Value::DividedRoundedDown(
            Box::new(resolve_value_it_tag(value, refs)?),
            *divisor,
        )),
        Value::Min(left, right) => Ok(Value::Min(
            Box::new(resolve_value_it_tag(left, refs)?),
            Box::new(resolve_value_it_tag(right, refs)?),
        )),
        Value::SurfaceHinted { value, hints }
            if hints.contains(&ironsmith_core::ValueSurfaceHint::LifeGainedAmount)
                && !refs.allow_life_event_value
                && matches!(
                    value.unhinted(),
                    Value::EventValue(EventValueSpec::LifeAmount)
                ) =>
        {
            let id = refs.known_last_effect_id().ok_or_else(|| {
                CardTextError::ParseError(
                    "life-gain amount requires a triggering event or prior effect".to_string(),
                )
            })?;
            Ok(Value::SurfaceHinted {
                value: Box::new(Value::EffectMetric {
                    effect_id: id,
                    source: ironsmith_core::EffectMetricSource::Outcome,
                    metric: ironsmith_core::EffectMetric::LifeGained,
                }),
                hints: hints.clone(),
            })
        }
        Value::SurfaceHinted { value, hints }
            if hints.contains(&ironsmith_core::ValueSurfaceHint::PriorEffectResult)
                && matches!(value.unhinted(), Value::EventValue(EventValueSpec::Amount)) =>
        {
            let id = refs.known_last_effect_id().ok_or_else(|| {
                CardTextError::ParseError("prior-effect result requires a prior effect".into())
            })?;
            Ok(Value::SurfaceHinted {
                value: Box::new(Value::EffectValue(id)),
                hints: hints.clone(),
            })
        }
        Value::SurfaceHinted { value, hints } => Ok(Value::SurfaceHinted {
            value: Box::new(resolve_value_it_tag(value, refs)?),
            hints: hints.clone(),
        }),
        Value::Count(filter) => Ok(Value::Count(resolve_it_tag(filter, refs)?)),
        Value::CountScaled(filter, multiplier) => Ok(Value::CountScaled(
            resolve_it_tag(filter, refs)?,
            *multiplier,
        )),
        Value::GreatestCount(filter) => Ok(Value::GreatestCount(resolve_it_tag(filter, refs)?)),
        Value::GreatestSharedCreatureTypeCount(filter) => Ok(
            Value::GreatestSharedCreatureTypeCount(resolve_it_tag(filter, refs)?),
        ),
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
        Value::LeastPower(filter) => Ok(Value::LeastPower(resolve_it_tag(filter, refs)?)),
        Value::LeastToughness(filter) => Ok(Value::LeastToughness(resolve_it_tag(filter, refs)?)),
        Value::LeastManaValue(filter) => Ok(Value::LeastManaValue(resolve_it_tag(filter, refs)?)),
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
        Value::ColorPairsAmong(filter) => Ok(Value::ColorPairsAmong(resolve_it_tag(filter, refs)?)),
        Value::DistinctCounterTypesAmong(filter) => Ok(Value::DistinctCounterTypesAmong(
            resolve_it_tag(filter, refs)?,
        )),
        Value::DistinctNames(filter) => Ok(Value::DistinctNames(resolve_it_tag(filter, refs)?)),
        Value::DistinctManaValues(filter) => {
            Ok(Value::DistinctManaValues(resolve_it_tag(filter, refs)?))
        }
        Value::UnlockedDoorsAmong(filter) => {
            Ok(Value::UnlockedDoorsAmong(resolve_it_tag(filter, refs)?))
        }
        Value::DistinctPowers(filter) => Ok(Value::DistinctPowers(resolve_it_tag(filter, refs)?)),
        Value::TurnHistoryCount(query) => {
            use ironsmith_core::TurnHistoryCount;

            let query = match query {
                TurnHistoryCount::Died {
                    filter,
                    controller_surface,
                } => TurnHistoryCount::Died {
                    filter: resolve_it_tag(filter, refs)?,
                    controller_surface: *controller_surface,
                },
                TurnHistoryCount::EnteredBattlefield(filter) => {
                    TurnHistoryCount::EnteredBattlefield(resolve_it_tag(filter, refs)?)
                }
                TurnHistoryCount::TurnedFaceUp(player) => {
                    TurnHistoryCount::TurnedFaceUp(resolve_contextual_player_filter(player, refs)?)
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
                TurnHistoryCount::SacrificedCardTypes { player, filter } => {
                    TurnHistoryCount::SacrificedCardTypes {
                        player: resolve_contextual_player_filter(player, refs)?,
                        filter: resolve_it_tag(filter, refs)?,
                    }
                }
                TurnHistoryCount::CountersPutOn {
                    source_controller,
                    counter_type,
                    filter,
                } => TurnHistoryCount::CountersPutOn {
                    source_controller: source_controller
                        .as_ref()
                        .map(|player| resolve_contextual_player_filter(player, refs))
                        .transpose()?,
                    counter_type: *counter_type,
                    filter: resolve_it_tag(filter, refs)?,
                },
                TurnHistoryCount::CreaturesAttackedWith { player, filter } => {
                    TurnHistoryCount::CreaturesAttackedWith {
                        player: resolve_contextual_player_filter(player, refs)?,
                        filter: resolve_it_tag(filter, refs)?,
                    }
                }
                TurnHistoryCount::PlayersAttackedThisCombat(player) => {
                    TurnHistoryCount::PlayersAttackedThisCombat(resolve_contextual_player_filter(
                        player, refs,
                    )?)
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
                TurnHistoryCount::CardsDrawn(player) => {
                    TurnHistoryCount::CardsDrawn(resolve_contextual_player_filter(player, refs)?)
                }
                TurnHistoryCount::Cycled(player) => {
                    TurnHistoryCount::Cycled(resolve_contextual_player_filter(player, refs)?)
                }
                TurnHistoryCount::LandsPlayed(player) => {
                    TurnHistoryCount::LandsPlayed(resolve_contextual_player_filter(player, refs)?)
                }
                TurnHistoryCount::KeywordActionsPerformed { player, actions } => {
                    TurnHistoryCount::KeywordActionsPerformed {
                        player: resolve_contextual_player_filter(player, refs)?,
                        actions: actions.clone(),
                    }
                }
                TurnHistoryCount::CountersRemovedFrom {
                    counter_type,
                    filter,
                } => TurnHistoryCount::CountersRemovedFrom {
                    counter_type: *counter_type,
                    filter: resolve_it_tag(filter, refs)?,
                },
                TurnHistoryCount::PlayersLostLife(player) => TurnHistoryCount::PlayersLostLife(
                    resolve_contextual_player_filter(player, refs)?,
                ),
                TurnHistoryCount::UntappedLandsAtTurnStart(player) => {
                    TurnHistoryCount::UntappedLandsAtTurnStart(resolve_contextual_player_filter(
                        player, refs,
                    )?)
                }
                TurnHistoryCount::Descended(player) => {
                    TurnHistoryCount::Descended(resolve_contextual_player_filter(player, refs)?)
                }
                TurnHistoryCount::DamageDealtToSource => TurnHistoryCount::DamageDealtToSource,
                TurnHistoryCount::DamageDealtBySource => TurnHistoryCount::DamageDealtBySource,
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
                TurnHistoryCount::LibrarySearches {
                    player,
                    own_library_only,
                } => TurnHistoryCount::LibrarySearches {
                    player: resolve_contextual_player_filter(player, refs)?,
                    own_library_only: *own_library_only,
                },
                TurnHistoryCount::MaxEnteredBattlefieldByController { player, filter } => {
                    TurnHistoryCount::MaxEnteredBattlefieldByController {
                        player: resolve_contextual_player_filter(player, refs)?,
                        filter: resolve_it_tag(filter, refs)?,
                    }
                }
                TurnHistoryCount::DestroyedBy { filter, cause } => {
                    let mut cause = cause.clone();
                    cause.source_filter = cause
                        .source_filter
                        .as_ref()
                        .map(|filter| resolve_it_tag(filter, refs))
                        .transpose()?;
                    TurnHistoryCount::DestroyedBy {
                        filter: resolve_it_tag(filter, refs)?,
                        cause,
                    }
                }
                TurnHistoryCount::CastSpellsCounteredBy {
                    caster,
                    filter,
                    cause,
                } => {
                    let mut cause = cause.clone();
                    cause.source_filter = cause
                        .source_filter
                        .as_ref()
                        .map(|filter| resolve_it_tag(filter, refs))
                        .transpose()?;
                    TurnHistoryCount::CastSpellsCounteredBy {
                        caster: resolve_contextual_player_filter(caster, refs)?,
                        filter: resolve_it_tag(filter, refs)?,
                        cause,
                    }
                }
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
        Value::BasePowerOf(spec) => Ok(Value::BasePowerOf(Box::new(resolve_choose_spec_it_tag(
            spec, refs,
        )?))),
        Value::ToughnessOf(spec) => Ok(Value::ToughnessOf(Box::new(resolve_choose_spec_it_tag(
            spec, refs,
        )?))),
        Value::ColorsOf(spec) => Ok(Value::ColorsOf(Box::new(resolve_choose_spec_it_tag(
            spec, refs,
        )?))),
        Value::KicksPaidOf(spec) => Ok(Value::KicksPaidOf(Box::new(resolve_choose_spec_it_tag(
            spec, refs,
        )?))),
        Value::ManaSpentToCast(spec) => Ok(Value::ManaSpentToCast(Box::new(
            resolve_choose_spec_it_tag(spec, refs)?,
        ))),
        Value::ManaValueOf(spec) => Ok(Value::ManaValueOf(Box::new(resolve_choose_spec_it_tag(
            spec, refs,
        )?))),
        Value::CountersOn(spec, counter_type) => Ok(Value::CountersOn(
            Box::new(resolve_choose_spec_it_tag(spec, refs)?),
            *counter_type,
        )),
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
        Value::PendingPriorEffectMetric(query) => {
            if refs.known_last_effect_id() == Some(EffectId::ACTIVATION_COUNTER_COST) {
                return super::reference_resolution::bind_counter_cost_quantity(query, refs.counter_removal_cost);
            }

            if refs.known_last_effect_id().is_none()
                && let Some(value) =
                    super::reference_resolution::resolve_cost_quantity_query(query, refs)
            {
                return Ok(value);
            }

            if let Some(result) = super::reference_resolution::resolve_choice_quantity_query(query, refs) {
                return result;
            }
            if let Some(result) =
                super::reference_resolution::resolve_coin_quantity_query(query, refs)
            {
                return result;
            }
            if let Some(result) =
                super::reference_resolution::resolve_dice_quantity_query(query, refs)
            {
                return result;
            }
            if let Some(result) =
                super::reference_resolution::resolve_life_quantity_query(query, refs)
            {
                return result;
            }
            if refs.known_last_effect_id().is_none()
                && refs.milling_event_filter.as_deref().is_some_and(|filter| {
                    query.action == Some(ironsmith_core::PriorEffectAction::Milled)
                        && query.source == ironsmith_core::EffectMetricSource::AffectedObjects
                        && query.metric == ironsmith_core::EffectMetric::Count
                        && query.player.is_none()
                        && query.counter_type.is_none()
                        && query.filter.as_ref().unwrap_or(&ObjectFilter::default()) == filter
                })
            {
                return Ok(Value::EventValue(EventValueSpec::Amount));
            }
            let id = refs.known_last_effect_id().ok_or_else(|| {
                CardTextError::ParseError(
                    "pending filtered effect metric requires a prior memory-producing effect"
                        .to_string(),
                )
            })?;
            let mut query = query.clone();
            if let Some(filter) = query.filter.as_mut() {
                *filter = resolve_it_tag(filter, refs)?;
            }
            if let Some(player) = query.player.as_mut() {
                *player = resolve_contextual_player_filter(player, refs)?;
            }
            Ok(Value::PriorEffectMetric {
                effect_id: id,
                query,
            })
        }
        Value::PriorEffectMetric { effect_id, query } => {
            let mut query = query.clone();
            if let Some(filter) = query.filter.as_mut() {
                *filter = resolve_it_tag(filter, refs)?;
            }
            if let Some(player) = query.player.as_mut() {
                *player = resolve_contextual_player_filter(player, refs)?;
            }
            Ok(Value::PriorEffectMetric {
                effect_id: *effect_id,
                query,
            })
        }
        _ => Ok(value.clone()),
    }
}

pub fn resolve_total_cost_it_tags(
    cost: &crate::cost::TotalCost,
    refs: &ReferenceEnv,
) -> Result<crate::cost::TotalCost, CardTextError> {
    fn resolve_cost_effect(
        effect: &crate::effect::Effect,
        refs: &ReferenceEnv,
    ) -> Result<crate::effect::Effect, CardTextError> {
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            let mut tagged = tagged.clone();
            tagged.effect = Box::new(resolve_cost_effect(&tagged.effect, refs)?);
            return Ok(crate::effect::Effect::new(tagged));
        }
        if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
            let mut with_id = with_id.clone();
            with_id.effect = Box::new(resolve_cost_effect(&with_id.effect, refs)?);
            return Ok(crate::effect::Effect::new(with_id));
        }
        if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
            let mut sequence = sequence.clone();
            sequence.effects = sequence
                .effects
                .iter()
                .map(|effect| resolve_cost_effect(effect, refs))
                .collect::<Result<Vec<_>, _>>()?;
            return Ok(crate::effect::Effect::new(sequence));
        }
        if let Some(sacrifice) = effect.downcast_ref::<crate::effects::SacrificeTargetEffect>() {
            let target = resolve_choose_spec_it_tag(&sacrifice.target, refs)?;
            return Ok(crate::effect::Effect::new(
                crate::effects::SacrificeTargetEffect::new(target),
            ));
        }
        if let Some(sacrifice) = effect.downcast_ref::<crate::effects::SacrificeEffect>() {
            let mut sacrifice = sacrifice.clone();
            sacrifice.filter = resolve_it_tag(&sacrifice.filter, refs)?;
            return Ok(crate::effect::Effect::new(sacrifice));
        }
        Ok(effect.clone())
    }

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
            crate::costs::Cost::Sacrifice(filter) => {
                *filter = resolve_it_tag(filter, refs)?;
            }
            crate::costs::Cost::Effect(effect) => {
                *effect = resolve_cost_effect(effect, refs)?;
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

pub fn choose_spec_targets_object(spec: &ChooseSpec) -> bool {
    matches!(
        spec.base(),
        ChooseSpec::Object(_)
            | ChooseSpec::ObjectOrPlayer(_, _)
            | ChooseSpec::Tagged(_)
            | ChooseSpec::SpecificObject(_)
            | ChooseSpec::Source
    )
}

pub fn with_target_reference_surface_hint(spec: ChooseSpec, target: &TargetAst) -> ChooseSpec {
    let surface = match target {
        TargetAst::Object(filter, _, _) | TargetAst::ObjectOrPlayer(filter, _, _) => {
            filter.source_surface.clone()
        }
        TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, _, _) => {
            return with_target_reference_surface_hint(spec, inner);
        }
        _ => None,
    };
    source_reference_hinted_spec(spec, surface)
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

pub use ironsmith_compiler_semantic::model::ast::{
    choose_spec_for_target, source_reference_hinted_spec,
};

pub fn resolve_target_spec_with_choices(
    target: &TargetAst,
    refs: &ReferenceEnv,
) -> Result<(ChooseSpec, Vec<ChooseSpec>), CardTextError> {
    let mut spec = match target {
        // A direct `it` target is the stable current member of an object
        // loop. An earlier effect in the body may update last-object memory
        // (for example, a consult records the card it found), but that must
        // not retarget a later action away from the object being iterated.
        //
        // Keep this target-specific: `it` nested in a value such as "that
        // card's mana value" still intentionally follows the latest
        // antecedent recorded by the consult.
        TargetAst::Tagged(tag, _)
            if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
                && refs.iterated_object =>
        {
            ChooseSpec::Iterated
        }
        TargetAst::Tagged(tag, span)
            if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
                && implicit_it_reference_resolves_to_source(refs) =>
        {
            source_reference_hinted_spec(ChooseSpec::Source, implicit_source_pronoun_surface(*span))
        }
        _ => choose_spec_for_target(target),
    };
    if let TargetAst::Object(filter, None, _) = target
        && filter
            .additional_cost_object_surface()
            .is_some_and(|surface| {
                surface.action == ironsmith_core::AdditionalCostObjectAction::Exiled
            })
        && let Some(tag) = refs.known_last_object_tag()
        && crate::tag_support::is_exile_cost_collection_tag(tag)
    {
        spec = ChooseSpec::Object(
            filter
                .clone()
                .match_tagged(tag.clone(), TaggedOpbjectRelation::IsTaggedObject),
        );
    }
    if let TargetAst::Object(filter, explicit_target_span, reference_span) = target
        && refs.iterated_object
        && explicit_target_span.is_none()
        && reference_span.is_some()
        // "this creature" names the source even inside an object loop.
        && !filter.source
    {
        // An implicit demonstrative inside a `for each ...` object loop (for
        // example, "attach ... to that creature") denotes the current
        // iteration object.  The grammar may retain only the descriptive
        // filter, so resolve that reference from the loop's stable tag rather
        // than lowering it as an unresolvable object filter.  Explicit
        // `target ...` phrases intentionally remain ordinary target choices.
        let tag = refs.known_last_object_tag().cloned().unwrap_or_else(|| {
            (ironsmith_compiler_semantic::tag::declared_key(
                crate::tag::CompilerReferenceTag::It.as_str(),
            ))
            .into()
        });
        spec = ChooseSpec::Tagged(tag);
    }
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
    crate::selection_scope::validate_card_selection_spec(&spec)?;
    let mut choices = if spec.is_target() {
        vec![spec.clone()]
    } else {
        Vec::new()
    };
    // "Exile X target cards from target player's graveyard": the embedded
    // player target is declared whatever the object count.
    let mut object_target = target;
    while let TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, ..) = object_target
    {
        object_target = inner;
    }
    match object_target {
        TargetAst::Object(filter, _, _) | TargetAst::ObjectOrPlayer(filter, _, _) => {
            append_object_filter_target_player_choices(filter, &mut choices);
        }
        _ => {}
    }
    Ok((spec, choices))
}

pub fn resolve_attach_object_spec(
    object: &TargetAst,
    refs: &ReferenceEnv,
) -> Result<(ChooseSpec, Vec<ChooseSpec>), CardTextError> {
    match object {
        TargetAst::Source(_) => Ok((choose_spec_for_target(object), Vec::new())),
        TargetAst::Tagged(tag, _) => {
            let resolved_tag = if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str() {
                match refs.known_last_object_tag() {
                    Some(tag) => tag.clone(),
                    // "When this Equipment enters, if it was cast from your
                    // graveyard, attach it to target creature you control":
                    // the intervening-if names the source as the antecedent
                    // and no body object was tagged, so "it" is the source.
                    None => return Ok((ChooseSpec::Source, Vec::new())),
                }
            } else {
                tag.clone().into()
            };
            Ok((
                ChooseSpec::All(ObjectFilter::tagged(resolved_tag)),
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
            // An un-targeted attachment noun resolves to `All(filter)` when it
            // has no authored quantity ("attach all Auras ...").  A counted
            // noun is different: "attach an Equipment ..." is a resolving
            // choice, not an instruction to attach every matching Equipment.
            // Keep the filter in the ordinary `Object` form so the runtime's
            // counted-choice path asks for exactly the authored number.
            let spec = match base.unhinted() {
                ChooseSpec::All(filter) => ChooseSpec::Object(filter.clone()).with_count(*count),
                _ => base.with_count(*count),
            };
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
    use crate::model::reference_state::{RefState, ReferenceFrame, ReferenceImports};

    #[test]
    fn repeated_target_opponent_reference_requires_a_matching_announced_target() {
        let refs = ReferenceEnv {
            last_player_filter: RefState::Known(PlayerFilter::target_opponent()),
            ..ReferenceEnv::default()
        };

        assert_eq!(
            resolve_non_target_player_filter(PlayerAst::TargetOpponent, &refs)
                .expect("the already announced opponent target should remain in scope"),
            PlayerFilter::target_opponent()
        );

        let unrelated = ReferenceEnv {
            last_player_filter: RefState::Known(PlayerFilter::target_player()),
            ..ReferenceEnv::default()
        };
        assert!(
            resolve_non_target_player_filter(PlayerAst::TargetOpponent, &unrelated).is_err(),
            "an arbitrary player target must not satisfy a target-opponent reference"
        );
        assert!(
            resolve_non_target_player_filter(PlayerAst::TargetOpponent, &ReferenceEnv::default())
                .is_err(),
            "target language without an explicit target must still be rejected"
        );
    }

    #[test]
    fn counted_implicit_attach_object_remains_a_resolving_choice() {
        let object = TargetAst::WithCount(
            Box::new(TargetAst::Object(ObjectFilter::creature(), None, None)),
            crate::effect::ChoiceCount::exactly(1),
        );

        let (spec, choices) = resolve_attach_object_spec(&object, &ReferenceEnv::default())
            .expect("counted attachment object should resolve");

        assert!(choices.is_empty(), "a resolving choice is not a target");
        assert_eq!(spec.count(), crate::effect::ChoiceCount::exactly(1));
        assert!(
            matches!(spec.base(), ChooseSpec::Object(_)),
            "the counted filter must not degrade to All(filter): {spec:#?}"
        );
    }

    #[test]
    fn counter_count_resolves_its_object_reference() {
        let value = Value::CountersOn(
            Box::new(ChooseSpec::tagged(
                crate::tag::CompilerReferenceTag::It.as_str(),
            )),
            Some(crate::object::CounterType::PlusOnePlusOne),
        );
        let source = ReferenceEnv {
            source_object_antecedent: true,
            ..Default::default()
        };
        assert_eq!(
            resolve_value_it_tag(&value, &source).unwrap(),
            Value::CountersOn(
                Box::new(ChooseSpec::Source),
                Some(crate::object::CounterType::PlusOnePlusOne)
            )
        );
        let tagged = ReferenceEnv {
            last_object_tag: RefState::Known(TagKey::from("chosen")),
            ..Default::default()
        };
        assert_eq!(
            resolve_value_it_tag(&value, &tagged).unwrap(),
            Value::CountersOn(
                Box::new(ChooseSpec::tagged("chosen")),
                Some(crate::object::CounterType::PlusOnePlusOne)
            )
        );
    }

    #[test]
    fn target_wrapped_implicit_it_value_resolves_to_source() {
        let refs = ReferenceEnv {
            source_object_antecedent: true,
            last_player_filter: RefState::Known(PlayerFilter::You),
            ..Default::default()
        };

        let value = Value::PowerOf(Box::new(ChooseSpec::target(ChooseSpec::Object(
            ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.bind()),
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
            last_object_tag: RefState::Known(
                ironsmith_compiler_semantic::tag::declared_key(
                    "__sentence_helper_revealed_l0_s0_e7",
                )
                .into(),
            ),
            ..ReferenceEnv::default()
        };
        let value = Value::Count(ObjectFilter::tagged(
            crate::tag::CompilerReferenceTag::PublicRevealed.bind(),
        ));
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
    fn typed_revealed_it_count_uses_snapshot_collection_not_last_match() {
        let refs = ReferenceEnv {
            last_object_tag: RefState::Known(
                ironsmith_compiler_semantic::tag::declared_key(
                    "__sentence_helper_consult_match_l0_s0_e7",
                )
                .into(),
            ),
            snapshot_tag_aliases: vec![(
                ironsmith_compiler_semantic::tag::declared_key("__public_revealed").into(),
                ironsmith_compiler_semantic::tag::declared_key(
                    "__sentence_helper_revealed_l0_s0_e7",
                )
                .into(),
            )],
            ..ReferenceEnv::default()
        };
        let mut revealed = ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.bind());
        revealed.set_prior_effect_action_surface(Some(ironsmith_core::PriorEffectAction::Revealed));

        let resolved = resolve_it_tag(&revealed, &refs).expect("resolve typed revealed count");
        assert_eq!(
            resolved.tagged_constraints[0].tag.as_str(),
            "__sentence_helper_revealed_l0_s0_e7"
        );

        let ordinary = resolve_it_tag(
            &ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.bind()),
            &refs,
        )
        .expect("resolve ordinary singular result");
        assert_eq!(
            ordinary.tagged_constraints[0].tag.as_str(),
            "__sentence_helper_consult_match_l0_s0_e7"
        );
    }

    #[test]
    fn typed_exile_reference_survives_an_intervening_sacrifice() {
        let exiled = TagKey::from("exiled_pool");
        let sacrificed = TagKey::from("sacrificed_later");
        let refs = ReferenceEnv {
            last_object_tag: RefState::Known(sacrificed.clone()),
            snapshot_tag_aliases: vec![(
                crate::tag::CompilerReferenceTag::ExiledThisWay.key(),
                exiled.clone(),
            )],
            ..ReferenceEnv::default()
        };
        let mut filter =
            ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.bind()).in_zone(Zone::Exile);
        filter.set_prior_effect_action_surface(Some(ironsmith_core::PriorEffectAction::Exiled));
        let resolved = resolve_it_tag(&filter, &refs).unwrap();
        assert_eq!(resolved.tagged_constraints[0].tag, exiled);
        assert_eq!(resolved.zone, Some(Zone::Exile));
        filter.set_prior_effect_action_surface(None);
        assert_eq!(
            resolve_it_tag(&filter, &refs).unwrap().tagged_constraints[0].tag,
            sacrificed
        );
    }

    #[test]
    fn source_exiled_set_can_exclude_the_current_exile_result() {
        let refs = ReferenceEnv {
            last_object_tag: RefState::Known(
                ironsmith_compiler_semantic::tag::declared_key("exiled_7").into(),
            ),
            ..ReferenceEnv::default()
        };
        let filter = ObjectFilter::tagged(crate::tag::CompilerReferenceTag::SourceExiled.bind())
            .in_zone(Zone::Exile)
            .not_tagged(crate::tag::CompilerReferenceTag::It.bind());

        let resolved = resolve_it_tag(&filter, &refs)
            .expect("resolve the current-result exclusion without rebinding the source set");

        assert!(resolved.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == crate::tag::CompilerReferenceTag::SourceExiled.as_str()
                && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
        }));
        assert!(resolved.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == ironsmith_core::SOURCE_EXILED_THIS_RESOLUTION_TAG
                && constraint.relation == TaggedOpbjectRelation::IsNotTaggedObject
        }));
        assert!(!resolved.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == "exiled_7"
                && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
        }));

        let ordinary = ObjectFilter::tagged(crate::tag::CompilerReferenceTag::SourceExiled.bind())
            .in_zone(Zone::Exile);
        let ordinary = resolve_it_tag(&ordinary, &refs)
            .expect("an ordinary latest-exile collection should still rebind");
        assert!(ordinary.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == "exiled_7"
                && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
        }));
    }

    #[test]
    fn unresolved_it_relational_constraint_survives_for_runtime_trigger_binding() {
        let filter = ObjectFilter::default().match_tagged(
            crate::tag::CompilerReferenceTag::It.bind(),
            TaggedOpbjectRelation::ManaValueLtTagged,
        );

        let resolved =
            resolve_it_tag(&filter, &ReferenceEnv::default()).expect("preserve runtime relation");

        assert_eq!(resolved.tagged_constraints, filter.tagged_constraints);
    }

    #[test]
    fn unresolved_immediate_attachment_relation_does_not_leak_into_target_filter() {
        let filter = ObjectFilter::creature().match_tagged(
            crate::tag::CompilerReferenceTag::It.bind(),
            TaggedOpbjectRelation::AttachedToTaggedObject,
        );

        let resolved = resolve_it_tag(&filter, &ReferenceEnv::default())
            .expect("consume unbound immediate attachment relation");

        assert!(resolved.tagged_constraints.is_empty());
        assert_eq!(resolved.card_types, filter.card_types);
    }

    #[test]
    fn blocked_by_that_creature_resolves_the_nested_object_reference() {
        let mut filter = ObjectFilter::creature();
        filter.blocked = true;
        filter.blocked_by = Some(ObjectRef::Tagged(
            (crate::tag::CompilerReferenceTag::It.bind()).into(),
        ));
        let refs = ReferenceEnv {
            last_object_tag: RefState::Known(
                (crate::tag::CompilerReferenceTag::Targeted0.bind()).into(),
            ),
            ..ReferenceEnv::default()
        };

        let resolved = resolve_it_tag(&filter, &refs).expect("resolve blocking antecedent");

        assert!(matches!(
            resolved.blocked_by,
            Some(ObjectRef::Tagged(tag)) if tag.as_str() == "targeted_0"
        ));
    }

    #[test]
    fn in_combat_with_that_creature_resolves_the_nested_object_reference() {
        let mut filter = ObjectFilter::creature();
        filter.in_combat_with = Some(ObjectRef::Tagged(
            (crate::tag::CompilerReferenceTag::It.bind()).into(),
        ));
        let refs = ReferenceEnv {
            last_object_tag: RefState::Known(
                (crate::tag::CompilerReferenceTag::Targeted0.bind()).into(),
            ),
            ..ReferenceEnv::default()
        };

        let resolved = resolve_it_tag(&filter, &refs).expect("resolve combat antecedent");

        assert!(matches!(
            resolved.in_combat_with,
            Some(ObjectRef::Tagged(tag)) if tag.as_str() == "targeted_0"
        ));
    }

    #[test]
    fn exact_combat_event_tag_does_not_require_live_combat_role() {
        let mut filter = ObjectFilter::creature().match_tagged(
            crate::tag::CompilerReferenceTag::It.bind(),
            TaggedOpbjectRelation::IsTaggedObject,
        );
        filter.blocking = true;
        let refs = ReferenceEnv {
            last_object_tag: RefState::Known(
                (crate::tag::CompilerReferenceTag::Blocking.bind()).into(),
            ),
            ..ReferenceEnv::default()
        };

        let resolved = resolve_it_tag(&filter, &refs).expect("resolve exact blocker tag");

        assert!(!resolved.blocking, "{resolved:#?}");
        assert!(resolved.tagged_constraints.iter().any(|constraint| {
            constraint.relation == TaggedOpbjectRelation::IsTaggedObject
                && constraint.tag.as_str() == "blocking"
        }));
    }

    #[test]
    fn source_exiled_target_preserves_zone_and_count() {
        let mut filter =
            ObjectFilter::tagged(crate::tag::CompilerReferenceTag::SourceExiled.bind());
        filter.zone = Some(crate::zone::Zone::Exile);
        let spec = ChooseSpec::WithCount(
            Box::new(ChooseSpec::target(ChooseSpec::Object(filter))),
            crate::effect::ChoiceCount::up_to(1),
        );
        assert_eq!(
            resolve_choose_spec_it_tag(&spec, &ReferenceEnv::default()).unwrap(),
            spec,
        );
    }

    #[test]
    fn source_exiled_reference_does_not_bind_to_unrelated_sacrifice() {
        let filter = ObjectFilter::tagged(crate::tag::CompilerReferenceTag::SourceExiled.bind());
        let refs = ReferenceEnv {
            last_object_tag: RefState::Known(
                (crate::tag::CompilerReferenceTag::Sacrificed0.bind()).into(),
            ),
            ..ReferenceEnv::default()
        };

        let resolved = resolve_it_tag(&filter, &refs).expect("preserve source exile link");

        assert_eq!(
            resolved.tagged_constraints[0].tag.as_str(),
            crate::tag::CompilerReferenceTag::SourceExiled.as_str()
        );
    }

    #[test]
    fn source_exiled_reference_can_bind_to_local_exile_collection() {
        let filter = ObjectFilter::tagged(crate::tag::CompilerReferenceTag::SourceExiled.bind());
        let refs = ReferenceEnv {
            last_object_tag: RefState::Known(
                ironsmith_compiler_semantic::tag::declared_key("exiled_0").into(),
            ),
            ..ReferenceEnv::default()
        };

        let resolved = resolve_it_tag(&filter, &refs).expect("bind local exile collection");

        assert_eq!(resolved.tagged_constraints[0].tag.as_str(), "exiled_0");
    }

    #[test]
    fn additional_cost_alias_prefers_snapshot_over_newer_ordinary_antecedent() {
        let filter = ObjectFilter::default().match_tagged(
            crate::tag::CompilerReferenceTag::AdditionalCostObject.bind(),
            TaggedOpbjectRelation::SharesSubtypeWithTagged,
        );
        let refs = ReferenceEnv {
            last_object_tag: RefState::Known(
                ironsmith_compiler_semantic::tag::declared_key("destroyed_1").into(),
            ),
            snapshot_tag_aliases: vec![(
                (crate::tag::CompilerReferenceTag::AdditionalCostObject.bind()).into(),
                ironsmith_compiler_semantic::tag::declared_key("sacrifice_cost_0").into(),
            )],
            ..ReferenceEnv::default()
        };

        let resolved = resolve_it_tag(&filter, &refs).expect("resolve cost snapshot");
        assert_eq!(
            resolved.tagged_constraints[0].tag.as_str(),
            "sacrifice_cost_0"
        );
    }

    #[test]
    fn additional_cost_alias_survives_nested_reference_import_round_trip() {
        let frame = ReferenceFrame {
            last_object_tag: Some(
                ironsmith_compiler_semantic::tag::declared_key("damaged_0").into(),
            ),
            snapshot_tag_aliases: vec![(
                (crate::tag::CompilerReferenceTag::AdditionalCostObject.bind()).into(),
                ironsmith_compiler_semantic::tag::declared_key("sacrifice_cost_0").into(),
            )],
            ..ReferenceFrame::default()
        }
        .to_lowering_frame();
        let imports = ReferenceImports::from_lowering_frame(&frame);
        let nested_refs = ReferenceEnv::from_imports(&imports, false, false, false, None);

        assert_eq!(
            resolve_it_tag_key(
                &crate::tag::CompilerReferenceTag::AdditionalCostObject.bind(),
                &nested_refs
            )
            .expect("resolve imported cost snapshot")
            .as_str(),
            "sacrifice_cost_0"
        );
    }

    #[test]
    fn additional_cost_alias_falls_back_to_local_object_without_snapshot() {
        let refs = ReferenceEnv {
            last_object_tag: RefState::Known(
                ironsmith_compiler_semantic::tag::declared_key("exiled_0").into(),
            ),
            ..ReferenceEnv::default()
        };

        assert_eq!(
            resolve_it_tag_key(
                &crate::tag::CompilerReferenceTag::AdditionalCostObject.bind(),
                &refs
            )
            .expect("resolve local explicit exiled object")
            .as_str(),
            "exiled_0"
        );
    }

    #[test]
    fn plural_card_reference_after_source_exile_keeps_the_cost_set() {
        let mut filter = ObjectFilter::creature().in_zone(Zone::Battlefield);
        filter.set_explicit_card_noun(true);
        filter.set_plural_object_noun_surface(true);
        filter
            .tagged_constraints
            .push(crate::filter::TaggedObjectConstraint {
                tag: (crate::tag::CompilerReferenceTag::It.bind()).into(),
                relation: TaggedOpbjectRelation::IsTaggedObject,
            });
        let refs = ReferenceEnv {
            last_object_tag: RefState::Known(
                (crate::tag::CompilerReferenceTag::SourceExiled.bind()).into(),
            ),
            snapshot_tag_aliases: vec![(
                (crate::tag::CompilerReferenceTag::AdditionalCostObject.bind()).into(),
                ironsmith_compiler_semantic::tag::declared_key("sacrifice_cost_0").into(),
            )],
            ..ReferenceEnv::default()
        };

        let resolved = resolve_it_tag(&filter, &refs).expect("resolve plural cost set");
        assert_eq!(resolved.zone, None);
        assert!(resolved.tagged_constraints.iter().any(|constraint| {
            constraint.relation == TaggedOpbjectRelation::IsTaggedObject
                && constraint.tag.as_str() == "sacrifice_cost_0"
        }));
    }
}

/// Keep source-anaphor handling identical in reference planning and lowering.
pub fn sacrifice_filter_uses_source_antecedent(
    filter: &ObjectFilter,
    one_of_referenced_set: bool,
    refs: &ReferenceEnv,
) -> bool {
    !one_of_referenced_set
        && !refs.iterated_object
        && refs.has_source_object_antecedent()
        && refs.known_last_object_tag().is_none_or(|tag| {
            tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
                && !refs.last_it_choice_is_set
        })
        && object_filter_as_tagged_reference(filter)
            .is_some_and(|tag| tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str())
}

#[cfg(test)]
mod spell_quantity_after_object_actions_tests {
    use super::*;
    use crate::model::reference_state::RefState;
    #[test]
    fn a_spell_demonstrative_skips_destroyed_permanents_but_keeps_a_new_spell_target() {
        let value = ChooseSpec::Tagged(crate::tag::CompilerReferenceTag::It.key())
            .with_surface_hint(ChooseSpecSurfaceHint::SourceReference(
                SourceReferenceSurface::ThisPermanentType("that spell".into()),
            ));
        for current in ["destroyed_0", "sacrificed_0"] {
            let refs = ReferenceEnv {
                last_object_tag: RefState::Known(current.into()),
                snapshot_tag_aliases: vec![(
                    crate::tag::CompilerReferenceTag::PriorObjectAntecedent.key(),
                    "triggering".into(),
                )],
                ..Default::default()
            };
            let resolved = resolve_choose_spec_it_tag(&value, &refs).unwrap();
            assert_eq!(resolved.base(), &ChooseSpec::Tagged("triggering".into()));
        }
        let refs = ReferenceEnv {
            last_object_tag: RefState::Known("targeted_1".into()),
            snapshot_tag_aliases: vec![(
                crate::tag::CompilerReferenceTag::PriorObjectAntecedent.key(),
                "triggering".into(),
            )],
            ..Default::default()
        };
        assert_eq!(
            resolve_choose_spec_it_tag(&value, &refs).unwrap().base(),
            &ChooseSpec::Tagged("targeted_1".into())
        );
    }
}

#[cfg(test)]
mod target_participant_scalar_tests {
    use super::*;
    use crate::model::reference_state::RefState;

    #[test]
    fn scalar_player_binding_preserves_loops_and_requires_a_real_antecedent() {
        let value = Value::CardsInHand(PlayerFilter::IteratedPlayer);
        assert_eq!(
            resolve_value_it_tag(&value, &ReferenceEnv::default()).unwrap(),
            value
        );
        let target = PlayerFilter::target_opponent();
        let mut env = ReferenceEnv {
            last_player_filter: RefState::Known(target.clone()),
            ..ReferenceEnv::default()
        };
        assert_eq!(
            resolve_value_it_tag(&value, &env).unwrap(),
            Value::CardsInHand(target)
        );
        env.iterated_player = true;
        assert_eq!(
            resolve_value_it_tag(&value, &env).unwrap(),
            value,
            "a loop's participant remains local unless the typed caller opens a nearer explicit subject scope"
        );
        assert_eq!(
            resolve_value_it_tag(&Value::CardsInHand(PlayerFilter::You), &env).unwrap(),
            Value::CardsInHand(PlayerFilter::You)
        );
    }
}
