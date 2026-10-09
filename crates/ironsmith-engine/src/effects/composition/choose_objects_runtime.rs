//! Runtime orchestration for `ChooseObjectsEffect`.
use crate::target::ChooseSpec;

use crate::decisions::context::DecisionHiddenCardVisibility;
use crate::decisions::make_decision;
use crate::decisions::specs::ChooseObjectsSpec;
use crate::effect::{ChoiceCount, EffectOutcome, ExecutionFact, SearchSelectionMode};
use crate::effects::cards::search_overrides::{
    LibrarySearchRequest, execute_library_search_scope,
    exile_found_cards_for_opposition_agent_with_outputs,
};
use crate::effects::helpers::{
    normalize_chosen_distinct_mana_values, normalize_chosen_one_per_card_type,
    resolve_player_filter_to_list, resolve_value, view_hidden_candidate_objects,
};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::filter::ObjectFilterExt as _;
use crate::filter::{ObjectFilter, ObjectRef, PlayerFilter};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;
use crate::zone::Zone;

use super::choose_objects::{ChooseObjectsEffect, search_zones, top_only_selection_limit};

/// The tag of `effect` when it is a library search whose chosen cards `next`
/// reveals ("search your library for up to two basic land cards, reveal those
/// cards, ..."). Such a search reveals its selection publicly, like a search
/// that says "reveal" itself (see `ExecutionContext::public_search_reveal_tag`).
pub(crate) fn revealed_search_tag(
    effect: &crate::effect::Effect,
    next: Option<&crate::effect::Effect>,
) -> Option<crate::tag::TagKey> {
    let choose = effect.downcast_ref::<ChooseObjectsEffect>()?;
    if !choose.is_search {
        return None;
    }
    let reveal = next?.downcast_ref::<crate::effects::RevealTaggedEffect>()?;
    (reveal.tag == choose.tag).then(|| choose.tag.clone())
}

/// The filter a chosen hidden library card must satisfy once opened
/// (ownership is resolved by `library_candidate_players`).
fn library_zone_filter(effect: &ChooseObjectsEffect) -> ObjectFilter {
    let mut filter = effect.filter.clone();
    filter.owner = None;
    filter
}

fn is_implicit_object_tag(tag: &str) -> bool {
    matches!(tag, "__it__" | "it")
}

fn should_accumulate_implicit_choice_tag(effect: &ChooseObjectsEffect) -> bool {
    let (PlayerFilter::AliasedOwnerOf(ObjectRef::Tagged(tag))
    | PlayerFilter::AliasedControllerOf(ObjectRef::Tagged(tag))) = &effect.chooser
    else {
        return false;
    };
    tag.as_str() == effect.tag.as_str() && is_implicit_object_tag(effect.tag.as_str())
}

fn object_filter_mentions_iterated_player(filter: &ObjectFilter) -> bool {
    filter
        .controller
        .as_ref()
        .is_some_and(PlayerFilter::mentions_iterated_player)
        || filter
            .owner
            .as_ref()
            .is_some_and(PlayerFilter::mentions_iterated_player)
        || filter
            .cast_by
            .as_ref()
            .is_some_and(PlayerFilter::mentions_iterated_player)
        || filter
            .targets_player
            .as_ref()
            .is_some_and(PlayerFilter::mentions_iterated_player)
        || filter
            .targets_only_player
            .as_ref()
            .is_some_and(PlayerFilter::mentions_iterated_player)
        || filter
            .attacking_player_or_planeswalker_controlled_by
            .as_ref()
            .is_some_and(PlayerFilter::mentions_iterated_player)
        || filter
            .protected_by
            .as_ref()
            .is_some_and(PlayerFilter::mentions_iterated_player)
        || filter
            .entered_battlefield_controller
            .as_ref()
            .is_some_and(PlayerFilter::mentions_iterated_player)
        || filter
            .counters_put_on_this_turn
            .as_ref()
            .is_some_and(|constraint| constraint.source_controller.mentions_iterated_player())
        || filter
            .targets_object
            .as_deref()
            .is_some_and(object_filter_mentions_iterated_player)
        || filter
            .targets_only_object
            .as_deref()
            .is_some_and(object_filter_mentions_iterated_player)
        || filter
            .attached_to_object
            .as_deref()
            .is_some_and(object_filter_mentions_iterated_player)
        || filter
            .no_shared_creature_types_with
            .iter()
            .any(object_filter_mentions_iterated_player)
        || filter
            .characteristic_relations
            .iter()
            .any(|relation| object_filter_mentions_iterated_player(&relation.comparison))
        || filter
            .any_of
            .iter()
            .any(object_filter_mentions_iterated_player)
}

fn value_mentions_iterated_player(value: &crate::effect::Value) -> bool {
    match value {
        crate::effect::Value::DamageHistory(query) => {
            query
                .object_filters()
                .any(object_filter_mentions_iterated_player)
                || query
                    .player_filter()
                    .is_some_and(PlayerFilter::mentions_iterated_player)
                || query.reference_specs().any(|spec| match spec.base() {
                    ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
                        object_filter_mentions_iterated_player(filter)
                    }
                    ChooseSpec::Player(player)
                    | ChooseSpec::EachPlayer(player)
                    | ChooseSpec::PlayerOrPlaneswalker(player) => player.mentions_iterated_player(),
                    ChooseSpec::ObjectOrPlayer(filter, player) => {
                        object_filter_mentions_iterated_player(filter)
                            || player.mentions_iterated_player()
                    }
                    _ => false,
                })
        }

        crate::effect::Value::Add(left, right) => {
            value_mentions_iterated_player(left) || value_mentions_iterated_player(right)
        }
        crate::effect::Value::Scaled(inner, _)
        | crate::effect::Value::DividedRoundedDown(inner, _)
        | crate::effect::Value::HalfRoundedDown(inner) => value_mentions_iterated_player(inner),
        crate::effect::Value::Count(filter)
        | crate::effect::Value::CountScaled(filter, _)
        | crate::effect::Value::GreatestCount(filter)
        | crate::effect::Value::LeastCount(filter)
        | crate::effect::Value::GreatestSharedCreatureTypeCount(filter)
        | crate::effect::Value::TotalPower(filter)
        | crate::effect::Value::TotalToughness(filter)
        | crate::effect::Value::TotalManaValue(filter)
        | crate::effect::Value::GreatestPower(filter)
        | crate::effect::Value::GreatestToughness(filter)
        | crate::effect::Value::GreatestManaValue(filter)
        | crate::effect::Value::LeastPower(filter)
        | crate::effect::Value::LeastToughness(filter)
        | crate::effect::Value::LeastManaValue(filter)
        | crate::effect::Value::BasicLandTypesAmong(filter)
        | crate::effect::Value::CreatureTypesAmong(filter)
        | crate::effect::Value::CardTypesAmong(filter)
        | crate::effect::Value::ColorsAmong(filter)
        | crate::effect::Value::DistinctNames(filter)
        | crate::effect::Value::DistinctManaValues(filter)
        | crate::effect::Value::UnlockedDoorsAmong(filter)
        | crate::effect::Value::DistinctPowers(filter) => {
            object_filter_mentions_iterated_player(filter)
        }
        crate::effect::Value::CreaturesDiedThisTurnControlledBy(player)
        | crate::effect::Value::CountPlayers(player)
        | crate::effect::Value::CountPlayersWithCardsInHandAtLeast(player, _)
        | crate::effect::Value::CountPlayersWithCardsInGraveyardAtLeast(player, _)
        | crate::effect::Value::CountPlayersWithPoisonCountersAtLeast(player, _)
        | crate::effect::Value::PartySize(player)
        | crate::effect::Value::LifeTotal(player)
        | crate::effect::Value::MaximumLifeTotal(player)
        | crate::effect::Value::CountPlayersBelowHalfStartingLifeTotal(player)
        | crate::effect::Value::LifeTotalDifference(player)
        | crate::effect::Value::Speed(player)
        | crate::effect::Value::StartingLifeTotal(player)
        | crate::effect::Value::HalfLifeTotalRoundedUp(player)
        | crate::effect::Value::HalfLifeTotalRoundedDown(player)
        | crate::effect::Value::HalfStartingLifeTotalRoundedUp(player)
        | crate::effect::Value::HalfStartingLifeTotalRoundedDown(player)
        | crate::effect::Value::CardsInHand(player)
        | crate::effect::Value::CardsInLibrary(player)
        | crate::effect::Value::DevotionToChosenColor(player)
        | crate::effect::Value::LifeGainedThisTurn(player)
        | crate::effect::Value::LifeLostThisTurn(player)
        | crate::effect::Value::AttractionsVisitedThisTurn(player)
        | crate::effect::Value::DamageDealtToPlayersThisTurn(player)
        | crate::effect::Value::NoncombatDamageDealtToPlayersThisTurn(player)
        | crate::effect::Value::MaxCardsDrawnThisTurn(player)
        | crate::effect::Value::MaxDiceRolledThisTurn(player)
        | crate::effect::Value::LandsEnteredBattlefieldThisTurn(player)
        | crate::effect::Value::MaxCardsInHand(player)
        | crate::effect::Value::CardsInGraveyard(player)
        | crate::effect::Value::SpellsCastThisTurn(player)
        | crate::effect::Value::SpellsCastBeforeThisTurn(player)
        | crate::effect::Value::CardTypesInGraveyard(player) => player.mentions_iterated_player(),
        crate::effect::Value::NoncombatDamageDealtBySourcesControlledThisTurn {
            player, ..
        } => player.mentions_iterated_player(),
        crate::effect::Value::Devotion { player, .. } => player.mentions_iterated_player(),
        crate::effect::Value::SpellsCastThisTurnMatching { player, filter, .. }
        | crate::effect::Value::TotalManaValueOfSpellsCastThisTurnMatching {
            player, filter, ..
        } => player.mentions_iterated_player() || object_filter_mentions_iterated_player(filter),
        crate::effect::Value::CommanderCastCount(player) => player.mentions_iterated_player(),
        crate::effect::Value::ThisAbilityResolvedThisTurnCount => false,
        _ => false,
    }
}

/// Build a human-readable prompt from an ObjectFilter when the
/// effect carries only the bare default description.
///
/// `verb` is the action word: "sacrifice", "discard", "choose", etc.
fn describe_choose_from_filter(
    filter: &ObjectFilter,
    min: usize,
    max: usize,
    verb: &str,
) -> String {
    let type_word = if filter.card_types.len() == 1 {
        filter.card_types[0].selection_name()
    } else if filter.card_types.is_empty() {
        "permanent"
    } else {
        // Multiple types like "creature or artifact"
        let types = filter
            .card_types
            .iter()
            .map(|card_type| card_type.name())
            .collect::<Vec<_>>()
            .join(" or ");
        let article = article_for_count(min, max);
        return capitalize_first(&format!("{verb} {article} {types}"));
    };

    let mut parts = Vec::new();
    if !filter.excluded_card_types.is_empty() {
        for card_type in &filter.excluded_card_types {
            parts.push(format!("non{}", card_type.name()));
        }
    }
    if !filter.subtypes.is_empty() {
        for st in &filter.subtypes {
            parts.push(format!("{st:?}"));
        }
    }
    parts.push(type_word.to_string());

    let noun = parts.join(" ");
    let article = article_for_count(min, max);
    capitalize_first(&format!("{verb} {article} {noun}"))
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

fn article_for_count(min: usize, max: usize) -> &'static str {
    if max == 1 {
        "a"
    } else if min == max {
        "exactly"
    } else {
        "up to"
    }
}

fn filter_has_same_name_constraint(filter: &ObjectFilter) -> bool {
    filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::SameNameAsTagged
    })
}

fn same_name_reference_label(filter: &ObjectFilter) -> &'static str {
    if filter.card_types.len() == 1 {
        filter.card_types[0].selection_name()
    } else {
        "object"
    }
}

fn same_name_reference_name(
    game: &GameState,
    ctx: &ExecutionContext,
    filter: &ObjectFilter,
) -> Option<String> {
    if let Some(object_id) = ctx.iteration.iterated_object
        && let Some(object) = game.object(object_id)
    {
        return Some(object.name.to_string());
    }

    let constraint = filter.tagged_constraints.iter().find(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::SameNameAsTagged
    })?;
    ctx.get_tagged(constraint.tag.as_str())
        .map(|snapshot| snapshot.name.to_string())
}

fn search_card_noun(filter: &ObjectFilter, singular: bool) -> String {
    if filter.card_types.is_empty() {
        return if singular {
            "card".to_string()
        } else {
            "cards".to_string()
        };
    }

    let types = filter
        .card_types
        .iter()
        .map(|card_type| card_type.selection_name())
        .collect::<Vec<_>>()
        .join(" or ");

    if singular {
        format!("{types} card")
    } else {
        format!("{types} cards")
    }
}

fn search_quantity_prefix(min: usize, max: usize) -> String {
    if max == 1 {
        if min == 0 {
            "up to one".to_string()
        } else {
            "a".to_string()
        }
    } else if min == 0 {
        format!("up to {max}")
    } else if min == max {
        max.to_string()
    } else {
        format!("{min} to {max}")
    }
}

pub(crate) fn friendly_same_name_search_prompt(
    game: &GameState,
    ctx: &ExecutionContext,
    filter: &ObjectFilter,
    min: usize,
    max: usize,
) -> Option<String> {
    if !filter_has_same_name_constraint(filter) || max == 0 {
        return None;
    }

    let quantity = search_quantity_prefix(min, max);
    let noun = search_card_noun(filter, max == 1);
    let reference = same_name_reference_name(game, ctx, filter)
        .unwrap_or_else(|| format!("that {}", same_name_reference_label(filter)));

    Some(format!(
        "Search your library for {quantity} {noun} with the same name as {reference}"
    ))
}

fn filter_references_tagged_pool(effect: &ChooseObjectsEffect) -> bool {
    effect.filter.tagged_constraints.iter().any(|constraint| {
        matches!(
            constraint.relation,
            crate::target::TaggedOpbjectRelation::IsTaggedObject
        )
    })
}

fn graveyard_candidate_players(
    effect: &ChooseObjectsEffect,
    game: &GameState,
    ctx: &ExecutionContext,
    filter_ctx: &crate::filter::FilterContext,
    chooser_id: PlayerId,
) -> Result<Vec<PlayerId>, ExecutionError> {
    if let Some(owner_filter) = &effect.filter.owner {
        if owner_filter.mentions_iterated_player() && filter_ctx.iterated_player.is_none() {
            return Err(ExecutionError::UnresolvableValue(
                "ChooseObjectsEffect graveyard search needs IteratedPlayer, but no triggering/iterated player is bound".to_string(),
            ));
        }
        let owners = resolve_player_filter_to_list(game, owner_filter, filter_ctx, ctx)?;
        if owners.is_empty() {
            return Err(ExecutionError::UnresolvableValue(format!(
                "ChooseObjectsEffect graveyard search owner filter matched no players: {owner_filter:?}"
            )));
        }
        return Ok(owners);
    }

    if effect.filter.single_graveyard {
        return Ok(game.players.iter().map(|player| player.id).collect());
    }

    if filter_references_tagged_pool(effect) {
        return Ok(game.players.iter().map(|player| player.id).collect());
    }

    Ok(vec![chooser_id])
}

fn hand_candidate_players(
    effect: &ChooseObjectsEffect,
    game: &GameState,
    ctx: &ExecutionContext,
    filter_ctx: &crate::filter::FilterContext,
    chooser_id: PlayerId,
) -> Result<Vec<PlayerId>, ExecutionError> {
    if let Some(owner_filter) = &effect.filter.owner {
        if owner_filter.mentions_iterated_player() && filter_ctx.iterated_player.is_none() {
            return Err(ExecutionError::UnresolvableValue(
                "ChooseObjectsEffect hand search needs IteratedPlayer, but no triggering/iterated player is bound".to_string(),
            ));
        }
        let owners = resolve_player_filter_to_list(game, owner_filter, filter_ctx, ctx)?;
        if owners.is_empty() {
            return Err(ExecutionError::UnresolvableValue(format!(
                "ChooseObjectsEffect hand search owner filter matched no players: {owner_filter:?}"
            )));
        }
        return Ok(owners);
    }

    if filter_references_tagged_pool(effect) {
        return Ok(game.players.iter().map(|player| player.id).collect());
    }

    Ok(vec![chooser_id])
}

fn library_candidate_players(
    effect: &ChooseObjectsEffect,
    game: &GameState,
    ctx: &ExecutionContext,
    filter_ctx: &crate::filter::FilterContext,
    chooser_id: PlayerId,
) -> Result<Vec<PlayerId>, ExecutionError> {
    if let Some(owner_filter) = &effect.filter.owner {
        if owner_filter.mentions_iterated_player() && filter_ctx.iterated_player.is_none() {
            return Err(ExecutionError::UnresolvableValue(
                "ChooseObjectsEffect library search needs IteratedPlayer, but no triggering/iterated player is bound".to_string(),
            ));
        }
        let owners = resolve_player_filter_to_list(game, owner_filter, filter_ctx, ctx)?;
        if owners.is_empty() {
            return Err(ExecutionError::UnresolvableValue(format!(
                "ChooseObjectsEffect library search owner filter matched no players: {owner_filter:?}"
            )));
        }
        return Ok(owners);
    }
    if filter_references_tagged_pool(effect) {
        return Ok(game.players.iter().map(|player| player.id).collect());
    }
    Ok(vec![chooser_id])
}

fn effective_search_zones(
    effect: &ChooseObjectsEffect,
    game: &GameState,
    chooser_id: PlayerId,
) -> Result<Vec<Zone>, ExecutionError> {
    let mut zones = search_zones(effect)?;
    if effect.is_search && zones.contains(&Zone::Library) && !game.can_search_library(chooser_id) {
        zones.retain(|zone| *zone != Zone::Library);
    }
    Ok(zones)
}

fn choice_filter_context(
    effect: &ChooseObjectsEffect,
    game: &GameState,
    ctx: &ExecutionContext,
    chooser_id: PlayerId,
) -> crate::filter::FilterContext {
    if object_filter_mentions_iterated_player(&effect.filter)
        && matches!(effect.chooser, PlayerFilter::Target(_))
    {
        let base_ctx = ctx.filter_context(game);
        base_ctx.with_iterated_player(Some(chooser_id))
    } else {
        ctx.filter_context(game)
    }
}

/// The filter applied to cards in a player's hand (ownership is resolved by
/// `hand_candidate_players`).
fn hand_zone_filter(effect: &ChooseObjectsEffect) -> ObjectFilter {
    let mut filter = effect.filter.clone();
    filter.owner = None;
    filter
}

fn hand_candidate_ids(
    effect: &ChooseObjectsEffect,
    game: &GameState,
    ctx: &ExecutionContext,
    filter_ctx: &crate::filter::FilterContext,
    chooser_id: PlayerId,
) -> Result<Vec<ObjectId>, ExecutionError> {
    Ok(
        hand_candidate_players(effect, game, ctx, filter_ctx, chooser_id)?
            .iter()
            .filter_map(|owner_id| game.player(*owner_id))
            .flat_map(|player| player.hand.iter().copied())
            .collect(),
    )
}

/// Whether this choice picks among hand cards whose identities some peer
/// cannot see (see `game_state::hidden_hand_choices`). The answer is the same
/// on every peer, so it may shape the prompt; the local filter result may not.
fn hidden_hand_choice(
    effect: &ChooseObjectsEffect,
    game: &GameState,
    ctx: &ExecutionContext,
    chooser_id: PlayerId,
) -> Result<bool, ExecutionError> {
    if effect.count.is_random()
        || !effective_search_zones(effect, game, chooser_id)?.contains(&Zone::Hand)
    {
        return Ok(false);
    }
    let filter_ctx = choice_filter_context(effect, game, ctx, chooser_id);
    let hand_ids = hand_candidate_ids(effect, game, ctx, &filter_ctx, chooser_id)?;
    Ok(game.hand_choice_depends_on_hidden_identity(&hand_zone_filter(effect), hand_ids))
}

/// Choices among a privately viewed, tagged library pool must survive on
/// peers that cannot evaluate the card qualities. Membership in that pool is
/// public; qualifying identities are checked when selected cards are opened.
fn hidden_library_pool_choice(
    effect: &ChooseObjectsEffect,
    game: &GameState,
    ctx: &ExecutionContext,
    chooser_id: PlayerId,
) -> Result<bool, ExecutionError> {
    if effect.count.is_random()
        || !filter_references_tagged_pool(effect)
        || !effective_search_zones(effect, game, chooser_id)?.contains(&Zone::Library)
    {
        return Ok(false);
    }
    let filter_ctx = choice_filter_context(effect, game, ctx, chooser_id);
    let generic = hidden_library_pool_filter(effect);
    Ok(
        library_candidate_players(effect, game, ctx, &filter_ctx, chooser_id)?
            .into_iter()
            .filter_map(|owner| game.player(owner))
            .flat_map(|player| player.library.iter())
            .any(|id| {
                game.hidden_card_info(*id).is_some()
                    && game
                        .object(*id)
                        .is_some_and(|object| generic.matches(object, &filter_ctx, game))
            }),
    )
}

fn hidden_library_pool_filter(effect: &ChooseObjectsEffect) -> ObjectFilter {
    let mut generic = ObjectFilter::default();
    generic.zone = effect.filter.zone;
    generic.controller = effect.filter.controller.clone();
    generic.owner = None;
    generic.tagged_constraints = effect
        .filter
        .tagged_constraints
        .iter()
        .filter(|constraint| {
            matches!(
                constraint.relation,
                crate::filter::TaggedOpbjectRelation::IsTaggedObject
            )
        })
        .cloned()
        .collect();
    generic
}

fn collect_candidates_in_zone(
    effect: &ChooseObjectsEffect,
    game: &GameState,
    ctx: &ExecutionContext,
    chooser_id: PlayerId,
    search_zone: Zone,
) -> Result<Vec<ObjectId>, ExecutionError> {
    let filter_ctx = choice_filter_context(effect, game, ctx, chooser_id);
    let top_only_limit = top_only_selection_limit(effect, ctx.x_value);
    let mut hidden_zone_filter = effect.filter.clone();
    if matches!(
        search_zone,
        Zone::Hand | Zone::Graveyard | Zone::Library | Zone::OutsideGame
    ) {
        hidden_zone_filter.owner = None;
    }

    let hidden_library_pool = hidden_library_pool_choice(effect, game, ctx, chooser_id)?;
    let pool_filter = hidden_library_pool_filter(effect);
    let candidates = match search_zone {
        Zone::Battlefield => game
            .battlefield
            .iter()
            .filter_map(|&id| game.object(id).map(|obj| (id, obj)))
            .filter(|(_, obj)| effect.filter.matches(obj, &filter_ctx, game))
            .map(|(id, _)| id)
            .collect(),
        Zone::Hand => {
            let hand_ids = hand_candidate_ids(effect, game, ctx, &filter_ctx, chooser_id)?;
            // A placeholder's filter result is unknown on this peer while the
            // owner knows it: keep it choosable (as hidden library searches
            // do) so every peer offers the owner's real choice. The chosen
            // card is validated once opened.
            let placeholders = if game.hand_choice_depends_on_hidden_identity(
                &hidden_zone_filter,
                hand_ids.iter().copied(),
            ) {
                game.hidden_hand_placeholder_candidates(
                    &hidden_zone_filter,
                    &filter_ctx,
                    hand_ids.iter().copied(),
                )
            } else {
                Vec::new()
            };
            hand_ids
                .into_iter()
                .filter(|id| {
                    placeholders.contains(id)
                        || game
                            .object(*id)
                            .is_some_and(|obj| hidden_zone_filter.matches(obj, &filter_ctx, game))
                })
                .collect()
        }
        Zone::Graveyard => {
            let owner_ids =
                graveyard_candidate_players(effect, game, ctx, &filter_ctx, chooser_id)?;

            if effect.top_only {
                let mut top_matches = Vec::new();
                for owner_id in owner_ids {
                    if top_matches.len() >= top_only_limit {
                        break;
                    }
                    let Some(player) = game.player(owner_id) else {
                        continue;
                    };
                    for (id, obj) in player
                        .graveyard
                        .iter()
                        .rev()
                        .filter_map(|&id| game.object(id).map(|obj| (id, obj)))
                    {
                        if !hidden_zone_filter.matches(obj, &filter_ctx, game) {
                            continue;
                        }
                        top_matches.push(id);
                        if top_matches.len() >= top_only_limit {
                            break;
                        }
                    }
                }
                top_matches
            } else {
                owner_ids
                    .iter()
                    .filter_map(|owner_id| game.player(*owner_id))
                    .flat_map(|player| player.graveyard.iter())
                    .filter_map(|&id| game.object(id).map(|obj| (id, obj)))
                    .filter(|(_, obj)| hidden_zone_filter.matches(obj, &filter_ctx, game))
                    .map(|(id, _)| id)
                    .collect()
            }
        }
        Zone::Library => {
            let mut library_ids: Vec<ObjectId> = {
                let owner_ids =
                    library_candidate_players(effect, game, ctx, &filter_ctx, chooser_id)?
                        .into_iter()
                        .filter(|owner| {
                            !effect.is_search
                                || game.can_search_library_from_effect(
                                    chooser_id,
                                    *owner,
                                    ctx.controller,
                                )
                        })
                        .collect::<Vec<_>>();
                if effect.top_only {
                    let mut top_matches = Vec::new();
                    for owner_id in owner_ids {
                        if top_matches.len() >= top_only_limit {
                            break;
                        }
                        let Some(player) = game.player(owner_id) else {
                            continue;
                        };
                        for (id, obj) in player
                            .library
                            .iter()
                            .rev()
                            .filter_map(|&id| game.object(id).map(|obj| (id, obj)))
                        {
                            if (effect.is_search
                                || (hidden_library_pool
                                    && pool_filter.matches(obj, &filter_ctx, game)))
                                && (game.is_hidden_card_placeholder(id)
                                    || (obj.zone == Zone::Library && obj.name == "Hidden Card"))
                            {
                                top_matches.push(id);
                                if top_matches.len() >= top_only_limit {
                                    break;
                                }
                                continue;
                            }
                            if !hidden_zone_filter.matches(obj, &filter_ctx, game) {
                                continue;
                            }
                            top_matches.push(id);
                            if top_matches.len() >= top_only_limit {
                                break;
                            }
                        }
                    }
                    top_matches
                } else {
                    owner_ids
                        .iter()
                        .filter_map(|owner_id| game.player(*owner_id))
                        .flat_map(|player| player.library.iter())
                        .filter_map(|&id| {
                            let obj = game.object(id)?;
                            if (effect.is_search
                                || (hidden_library_pool
                                    && pool_filter.matches(obj, &filter_ctx, game)))
                                && (game.is_hidden_card_placeholder(id)
                                    || (obj.zone == Zone::Library && obj.name == "Hidden Card"))
                            {
                                return Some(id);
                            }
                            hidden_zone_filter
                                .matches(obj, &filter_ctx, game)
                                .then_some(id)
                        })
                        .collect()
                }
            };
            if effect.is_search {
                // "that player searches the top four cards of that library
                // instead" (Aven Mindcensor).
                game.restrict_library_search_candidates(chooser_id, &mut library_ids);
            }
            library_ids
        }
        Zone::OutsideGame => {
            let owner_ids = library_candidate_players(effect, game, ctx, &filter_ctx, chooser_id)?;
            owner_ids
                .iter()
                .filter_map(|owner_id| game.player(*owner_id))
                .flat_map(|player| player.sideboard.iter())
                .filter_map(|&id| game.object(id).map(|obj| (id, obj)))
                .filter(|(_, obj)| hidden_zone_filter.matches(obj, &filter_ctx, game))
                .map(|(id, _)| id)
                .collect()
        }
        _ => game
            .objects_in_zone(search_zone)
            .into_iter()
            .filter_map(|id| game.object(id).map(|obj| (id, obj)))
            .filter(|(_, obj)| effect.filter.matches(obj, &filter_ctx, game))
            .map(|(id, _)| id)
            .collect(),
    };

    Ok(candidates)
}

fn collect_candidates(
    effect: &ChooseObjectsEffect,
    game: &GameState,
    ctx: &ExecutionContext,
    chooser_id: PlayerId,
) -> Result<Vec<ObjectId>, ExecutionError> {
    if !crate::object_query::require_captured_public_collections(
        game,
        &effect.filter,
        &choice_filter_context(effect, game, ctx, chooser_id),
    ) {
        return Err(ExecutionError::IncompleteEvidence(
            "public destination choice requires its producer collection".into(),
        ));
    }
    let mut candidates = Vec::new();
    let zones = effective_search_zones(effect, game, chooser_id)?;
    for zone in zones.iter().copied() {
        for id in collect_candidates_in_zone(effect, game, ctx, chooser_id, zone)? {
            if !candidates.contains(&id) {
                candidates.push(id);
            }
        }
    }
    // A choice among an already-tagged pool ("An opponent chooses two of
    // them", Turtles Forever) names those exact cards wherever they are. Cards
    // revealed from outside the game stay there until moved, so the pool
    // includes tagged cards outside the game as well.
    let chooses_tagged_pool = !effect.is_search
        && effect.filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
        });
    if chooses_tagged_pool && !zones.contains(&Zone::OutsideGame) {
        let filter_ctx = choice_filter_context(effect, game, ctx, chooser_id);
        for id in game.objects_in_zone(Zone::OutsideGame) {
            if candidates.contains(&id) {
                continue;
            }
            if game
                .object(id)
                .is_some_and(|obj| effect.filter.matches(obj, &filter_ctx, game))
            {
                candidates.push(id);
            }
        }
    }
    Ok(candidates)
}

fn hidden_library_search_candidates(
    effect: &ChooseObjectsEffect,
    game: &GameState,
    ctx: &ExecutionContext,
    owner: PlayerId,
) -> Vec<ObjectId> {
    let Some(player) = game.player(owner) else {
        return Vec::new();
    };
    let mut ids: Box<dyn Iterator<Item = ObjectId> + '_> = if effect.top_only {
        Box::new(player.library.iter().rev().copied())
    } else {
        Box::new(player.library.iter().copied())
    };
    let mut candidates = Vec::new();
    let limit = if effect.top_only || effect.bottom_only {
        top_only_selection_limit(effect, ctx.x_value)
    } else {
        usize::MAX
    };
    for id in ids.by_ref() {
        if candidates.len() >= limit {
            break;
        }
        if game.is_hidden_card_placeholder(id) {
            candidates.push(id);
        }
    }
    candidates
}

fn compute_choice_bounds(count: ChoiceCount, candidate_count: usize) -> (usize, usize) {
    let min = count.min.min(candidate_count);
    let max = count.max.unwrap_or(candidate_count).min(candidate_count);
    (min, max)
}

fn compute_search_required_count(mode: SearchSelectionMode, max: usize) -> usize {
    match mode {
        SearchSelectionMode::Exact => max,
        SearchSelectionMode::Optional | SearchSelectionMode::AllMatching => 0,
    }
}

fn normalize_chosen_objects(
    mut chosen: Vec<ObjectId>,
    candidates: &[ObjectId],
    min: usize,
    max: usize,
    fill_to_min: bool,
    preserve_order: bool,
) -> Vec<ObjectId> {
    chosen.truncate(max);
    if preserve_order {
        let mut seen = std::collections::HashSet::new();
        chosen.retain(|id| seen.insert(*id));
    } else {
        chosen.sort();
        chosen.dedup();
    }

    if fill_to_min && chosen.len() < min {
        for id in candidates {
            if chosen.len() >= min {
                break;
            }
            if !chosen.contains(id) {
                chosen.push(*id);
            }
        }
    }

    chosen
}

fn normalize_chosen_distinct_names(
    game: &GameState,
    chosen: Vec<ObjectId>,
    candidates: &[ObjectId],
    min: usize,
    max: usize,
    fill_to_min: bool,
) -> Vec<ObjectId> {
    let mut names = std::collections::HashSet::new();
    let mut normalized = Vec::new();
    for id in chosen {
        if normalized.len() >= max {
            break;
        }
        let Some(object) = game.object(id) else {
            continue;
        };
        let name = object.name.to_ascii_lowercase();
        if names.insert(name) {
            normalized.push(id);
        }
    }

    if fill_to_min && normalized.len() < min {
        for id in candidates {
            if normalized.len() >= min || normalized.len() >= max {
                break;
            }
            let Some(object) = game.object(*id) else {
                continue;
            };
            let name = object.name.to_ascii_lowercase();
            if names.insert(name) {
                normalized.push(*id);
            }
        }
    }

    normalized
}

fn object_power_for_distinct_choice(game: &GameState, id: ObjectId) -> Option<i32> {
    game.calculated_power(id)
        .or_else(|| game.object(id).and_then(|object| object.power()))
}

fn normalize_chosen_distinct_powers(
    game: &GameState,
    chosen: Vec<ObjectId>,
    candidates: &[ObjectId],
    min: usize,
    max: usize,
    fill_to_min: bool,
) -> Vec<ObjectId> {
    let mut powers = std::collections::HashSet::new();
    let mut normalized = Vec::new();
    for id in chosen {
        if normalized.len() >= max {
            break;
        }
        let Some(power) = object_power_for_distinct_choice(game, id) else {
            continue;
        };
        if powers.insert(power) {
            normalized.push(id);
        }
    }

    if fill_to_min && normalized.len() < min {
        for id in candidates {
            if normalized.len() >= min || normalized.len() >= max {
                break;
            }
            let Some(power) = object_power_for_distinct_choice(game, *id) else {
                continue;
            };
            if powers.insert(power) {
                normalized.push(*id);
            }
        }
    }

    normalized
}

/// Selection-set constraint "that share a land type" (Myriad Landscape): keep
/// chosen objects while the land types common to the whole selection stay
/// non-empty. Illegal responses are trimmed rather than rejected, like the
/// other set-level normalizers.
fn normalize_chosen_shared_land_type(
    game: &GameState,
    chosen: Vec<ObjectId>,
    candidates: &[ObjectId],
    min: usize,
    max: usize,
    fill_to_min: bool,
) -> Vec<ObjectId> {
    let land_types = |id: ObjectId| -> Vec<crate::types::Subtype> {
        game.object(id)
            .map(|object| {
                object
                    .subtypes
                    .iter()
                    .copied()
                    .filter(|subtype| subtype.is_land_subtype())
                    .collect()
            })
            .unwrap_or_default()
    };
    let mut shared: Option<Vec<crate::types::Subtype>> = None;
    let mut normalized = Vec::new();
    let consider = |id: ObjectId,
                    shared: &mut Option<Vec<crate::types::Subtype>>,
                    normalized: &mut Vec<ObjectId>| {
        if normalized.len() >= max || normalized.contains(&id) {
            return;
        }
        let types = land_types(id);
        let next: Vec<_> = match shared {
            Some(current) => current
                .iter()
                .copied()
                .filter(|subtype| types.contains(subtype))
                .collect(),
            None => types,
        };
        if next.is_empty() {
            return;
        }
        *shared = Some(next);
        normalized.push(id);
    };
    for id in chosen {
        consider(id, &mut shared, &mut normalized);
    }
    if fill_to_min && normalized.len() < min {
        for id in candidates {
            if normalized.len() >= min {
                break;
            }
            consider(*id, &mut shared, &mut normalized);
        }
    }
    normalized
}

fn normalize_chosen_distinct_creature_types(
    game: &GameState,
    chosen: Vec<ObjectId>,
    candidates: &[ObjectId],
    min: usize,
    max: usize,
    fill_to_min: bool,
) -> Vec<ObjectId> {
    let mut used_types = std::collections::HashSet::new();
    let mut normalized = Vec::new();
    for id in chosen {
        if normalized.len() >= max {
            break;
        }
        let Some(object) = game.object(id) else {
            continue;
        };
        if object
            .subtypes
            .iter()
            .all(|subtype| !used_types.contains(subtype))
        {
            used_types.extend(object.subtypes.iter().copied());
            normalized.push(id);
        }
    }

    if fill_to_min && normalized.len() < min {
        for id in candidates {
            if normalized.len() >= min || normalized.len() >= max {
                break;
            }
            let Some(object) = game.object(*id) else {
                continue;
            };
            if object
                .subtypes
                .iter()
                .all(|subtype| !used_types.contains(subtype))
            {
                used_types.extend(object.subtypes.iter().copied());
                normalized.push(*id);
            }
        }
    }

    normalized
}

fn normalize_chosen_aggregate_constraint(
    game: &GameState,
    chosen: Vec<ObjectId>,
    candidates: &[ObjectId],
    min: usize,
    max: usize,
    fill_to_min: bool,
    constraint: crate::effect::ChoiceAggregateConstraint,
) -> Vec<ObjectId> {
    let maximum = match constraint.maximum.unhinted() {
        crate::effect::Value::Fixed(maximum) => *maximum,
        _ => return chosen,
    };
    let minimum = match constraint.minimum.as_ref().map(|value| value.unhinted()) {
        Some(crate::effect::Value::Fixed(minimum)) => *minimum,
        Some(_) => return chosen,
        None => i32::MIN,
    };
    if constraint.metric == crate::effect::ChoiceAggregateMetric::DistinctCardTypes {
        let total = |ids: &[ObjectId]| {
            crate::targeting::aggregate_object_set_value(
                game,
                ids.iter().copied(),
                constraint.metric,
            )
        };
        if chosen.len() >= min && total(&chosen) >= minimum && total(&chosen) <= maximum {
            return chosen;
        }
        // Keep a selection for each type union and cardinality, so a
        // minimum card count is not lost when several cards share types.
        let mut states =
            std::collections::BTreeMap::from([((0i32, 0usize), Vec::<ObjectId>::new())]);
        for id in chosen.iter().chain(candidates.iter()).copied() {
            let contribution =
                crate::targeting::aggregate_object_value(game, id, constraint.metric);
            for ((mask, _), selection) in states.clone() {
                if selection.contains(&id) || selection.len() >= max {
                    continue;
                }
                let next_mask = mask | contribution;
                if next_mask.count_ones() as i32 > maximum {
                    continue;
                }
                let mut next = selection;
                next.push(id);
                states.entry((next_mask, next.len())).or_insert(next);
            }
        }
        return states
            .into_iter()
            .filter(|((mask, _), selection)| {
                mask.count_ones() as i32 >= minimum && selection.len() >= min
            })
            .min_by_key(|(_, selection)| selection.len())
            .map(|(_, selection)| selection)
            .unwrap_or_default();
    }
    let chosen_total = crate::targeting::aggregate_object_set_value(
        game,
        chosen.iter().copied(),
        constraint.metric,
    );
    if chosen_total >= minimum && chosen_total <= maximum {
        return chosen;
    }

    // Decision makers are expected to submit a legal aggregate selection. As
    // with cardinality normalization above, keep malformed responses safe and
    // deterministic. Sorting by contribution also preserves valid choices
    // such as a positive-power creature offset by a negative-power creature.
    let (mut total, mut normalized) = if chosen_total <= maximum {
        (chosen_total, chosen)
    } else {
        let mut ranked = chosen
            .into_iter()
            .map(|id| {
                (
                    crate::targeting::aggregate_object_value(game, id, constraint.metric),
                    id,
                )
            })
            .collect::<Vec<_>>();
        ranked.sort_by_key(|(value, id)| (*value, *id));

        let mut total = 0i32;
        let mut normalized = Vec::new();
        for (value, id) in ranked {
            if normalized.len() >= max {
                break;
            }
            if total.saturating_add(value) <= maximum {
                total = total.saturating_add(value);
                normalized.push(id);
            }
        }
        (total, normalized)
    };

    if fill_to_min && normalized.len() < min {
        let mut remaining = candidates
            .iter()
            .copied()
            .filter(|id| !normalized.contains(id))
            .map(|id| {
                (
                    crate::targeting::aggregate_object_value(game, id, constraint.metric),
                    id,
                )
            })
            .collect::<Vec<_>>();
        remaining.sort_by_key(|(value, id)| (*value, *id));
        for (value, id) in remaining {
            if normalized.len() >= min || normalized.len() >= max {
                break;
            }
            if total.saturating_add(value) <= maximum {
                total = total.saturating_add(value);
                normalized.push(id);
            }
        }
    }

    if total < minimum && normalized.len() < max {
        let mut remaining = candidates
            .iter()
            .copied()
            .filter(|id| !normalized.contains(id))
            .map(|id| {
                (
                    crate::targeting::aggregate_object_value(game, id, constraint.metric),
                    id,
                )
            })
            .collect::<Vec<_>>();
        // Reach a lower aggregate bound with the fewest deterministic
        // additions. A decision maker's already-legal selection is preserved
        // by the early return above.
        remaining.sort_by_key(|(value, id)| (std::cmp::Reverse(*value), *id));
        for (value, id) in remaining {
            if normalized.len() >= max || total >= minimum {
                break;
            }
            if total.saturating_add(value) <= maximum {
                total = total.saturating_add(value);
                normalized.push(id);
            }
        }
    }

    normalized.sort();
    normalized
}

fn public_search_candidates(game: &GameState, candidates: &[ObjectId]) -> Vec<ObjectId> {
    candidates
        .iter()
        .copied()
        .filter(|id| {
            game.object(*id).is_some_and(|obj| {
                !obj.zone.is_hidden()
                    && obj.zone != Zone::Library
                    && !game.is_hidden_card_placeholder(*id)
            })
        })
        .collect()
}

fn enforce_public_search_choice_constraint(
    game: &GameState,
    candidates: &[ObjectId],
    chosen: Vec<ObjectId>,
    required_public_count: usize,
    max: usize,
) -> Vec<ObjectId> {
    if required_public_count == 0 {
        return chosen;
    }

    let public_candidates = public_search_candidates(game, candidates);
    let mut chosen_public: Vec<ObjectId> = public_candidates
        .iter()
        .copied()
        .filter(|id| chosen.contains(id))
        .collect();
    let mut chosen_hidden: Vec<ObjectId> = candidates
        .iter()
        .copied()
        .filter(|id| !public_candidates.contains(id) && chosen.contains(id))
        .collect();

    for id in &public_candidates {
        if chosen_public.len() >= required_public_count {
            break;
        }
        if !chosen_public.contains(id) {
            chosen_public.push(*id);
        }
    }

    let max_hidden = max.saturating_sub(chosen_public.len());
    chosen_hidden.truncate(max_hidden);

    let mut normalized = chosen_public;
    normalized.extend(chosen_hidden);
    normalized
}

fn enforce_single_graveyard_choice_constraint(
    effect: &ChooseObjectsEffect,
    game: &GameState,
    candidates: &[ObjectId],
    mut chosen: Vec<ObjectId>,
    min: usize,
    max: usize,
) -> Vec<ObjectId> {
    let Some(search_zone) = effect.filter.zone.or(effect.zone) else {
        return chosen;
    };
    if search_zone != Zone::Graveyard || !effect.filter.single_graveyard {
        return chosen;
    }

    let mut owner_groups: Vec<(PlayerId, Vec<ObjectId>)> = Vec::new();
    for &id in candidates {
        let Some(owner) = game.object(id).map(|obj| obj.owner) else {
            continue;
        };
        if let Some((_, ids)) = owner_groups
            .iter_mut()
            .find(|(group_owner, _)| *group_owner == owner)
        {
            ids.push(id);
        } else {
            owner_groups.push((owner, vec![id]));
        }
    }

    if owner_groups.is_empty() {
        return chosen;
    }

    let mut preferred_owner = chosen
        .first()
        .and_then(|id| game.object(*id).map(|obj| obj.owner))
        .or_else(|| owner_groups.first().map(|(owner, _)| *owner));

    if let Some(owner) = preferred_owner {
        let available_for_owner = owner_groups
            .iter()
            .find(|(group_owner, _)| *group_owner == owner)
            .map(|(_, ids)| ids.len())
            .unwrap_or(0);
        if available_for_owner < min
            && let Some((best_owner, _)) = owner_groups.iter().max_by_key(|(_, ids)| ids.len())
        {
            preferred_owner = Some(*best_owner);
        }
    }

    let Some(preferred_owner) = preferred_owner else {
        return chosen;
    };
    chosen.retain(|id| {
        game.object(*id)
            .is_some_and(|obj| obj.owner == preferred_owner)
    });
    chosen.truncate(max);
    chosen.sort();
    chosen.dedup();

    if chosen.len() < min
        && let Some((_, owner_candidates)) = owner_groups
            .iter()
            .find(|(group_owner, _)| *group_owner == preferred_owner)
    {
        for id in owner_candidates {
            if chosen.len() >= min || chosen.len() >= max {
                break;
            }
            if !chosen.contains(id) {
                chosen.push(*id);
            }
        }
    }

    chosen
}

fn snapshot_chosen_objects(game: &GameState, chosen: &[ObjectId]) -> Vec<ObjectSnapshot> {
    chosen
        .iter()
        .filter_map(|&id| {
            game.object(id)
                .map(|obj| ObjectSnapshot::from_object(obj, game))
        })
        .collect()
}

/// Whether this choice asks for a fixed number of objects that the chooser
/// cannot supply.
///
/// A mandatory instruction does as much as it can — "each player sacrifices two
/// creatures" takes the only creature a player controls (CR 608.2). An
/// *optional* one is all-or-nothing: "any player may sacrifice two creatures of
/// their choice" is simply not an option for a player who controls fewer than
/// two, so the offer is withheld rather than partially performed.
///
/// Only fixed counts are judged here. Searches may legally fail to find, and
/// their candidate set includes hidden library cards this function does not
/// collect; X-based and "up to" counts have no hard floor to miss.
pub(crate) fn fixed_choice_requirement_is_unmet(
    effect: &ChooseObjectsEffect,
    game: &GameState,
    ctx: &ExecutionContext,
) -> Result<bool, ExecutionError> {
    if effect.is_search
        || effect.count.dynamic_x
        || effect.count.up_to_x
        || effect.count_value.is_some()
        || effect.count.min == 0
    {
        return Ok(false);
    }

    let chooser_id =
        crate::effects::helpers::resolve_player_filter_as_chooser(game, &effect.chooser, ctx)?;
    // Whether a hidden hand holds enough matches is known only to its owner;
    // withholding the offer on that basis would desync the peers.
    if hidden_hand_choice(effect, game, ctx, chooser_id)?
        || hidden_library_pool_choice(effect, game, ctx, chooser_id)?
    {
        return Ok(false);
    }
    let mut candidates = collect_candidates(effect, game, ctx, chooser_id)?;
    if !game.source_snapshot_is_exempt_from_range(Some(ctx.source), ctx.source_snapshot.as_ref()) {
        candidates
            .retain(|object| game.object_is_within_range(chooser_id, *object, Some(ctx.source)));
    }

    Ok(candidates.len() < effect.count.min)
}

pub(crate) fn run_choose_objects(
    effect: &ChooseObjectsEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Result<EffectOutcome, ExecutionError> {
    run_choose_objects_with_outputs(effect, game, ctx)
        .map(crate::effects::CompletedEffectOutputs::into_outcome)
}

/// Inputs selected once by the native chooser, before disclosure or movement.
/// The proposal carries no completed action or inferred observer snapshot.
#[derive(Debug)]
struct SelectedObjectChoice {
    effect: ChooseObjectsEffect,
    chosen: Vec<ObjectId>,
    chooser_id: PlayerId,
    search_viewer: PlayerId,
    search_zones: Vec<Zone>,
    search_override: Option<crate::effects::cards::search_overrides::OppositionAgentSearch>,
    search_event: Option<crate::triggers::TriggerEvent>,
}

fn complete_selected_object_choice(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    selected: SelectedObjectChoice,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    let SelectedObjectChoice {
        effect,
        chosen,
        chooser_id,
        search_viewer,
        search_zones,
        search_override,
        search_event,
    } = selected;
    let effect = &effect;
    let mut retained_children = Vec::new();
    let mut disclosures = Vec::new();
    if effect.reveal && !chosen.is_empty() {
        if effect.reveal_is_presentation_only == Some(true) {
            view_hidden_candidate_objects(
                game,
                ctx,
                search_viewer,
                &chosen,
                "Reveal chosen hidden card",
                true,
            );
        } else {
            let selected = chosen
                .iter()
                .map(|id| {
                    ObjectSnapshot::try_from_object_id(game, *id)?.ok_or_else(|| {
                        ExecutionError::IncompleteEvidence(
                            "selected reveal card has no object snapshot".into(),
                        )
                    })
                })
                .collect::<Result<Vec<_>, ExecutionError>>()?;
            let outputs = crate::effects::cards::reveal_objects_with_outputs(
                game,
                ctx,
                selected,
                Some(chooser_id),
                "Reveal chosen hidden card",
                None,
            )?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::count(0),
                ));
            }
            disclosures.push(outputs.outcome.clone());
            retained_children.push(outputs);
        }
    }
    let chosen_memory: Vec<_> = chosen
        .iter()
        .filter_map(|id| ObjectSnapshot::from_object_id(game, *id))
        .collect();
    if search_zones.iter().any(Zone::is_hidden) {
        ctx.remember_face_down_exile_viewers(&chosen, chooser_id);
    }

    let mut published_outputs = Vec::new();
    let (objects_for_tags, outcome_objects, receipts) = if search_override.is_some() {
        let (found, published) =
            exile_found_cards_for_opposition_agent_with_outputs(game, ctx, &chosen, chooser_id)?;
        published_outputs = published;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        (Vec::new(), found.moved_ids, found.receipts)
    } else {
        (chosen.clone(), chosen.clone(), Vec::new())
    };

    let snapshots = snapshot_chosen_objects(game, &objects_for_tags);
    if effect.remember_as_chosen_object
        && let [chosen] = snapshots.as_slice()
    {
        game.set_chosen_object(ctx.source, chosen.clone());
    }
    if !snapshots.is_empty() {
        if effect.replace_tagged_objects
            || (is_implicit_object_tag(effect.tag.as_str())
                && !should_accumulate_implicit_choice_tag(effect))
        {
            ctx.set_tagged_objects(effect.tag.clone(), snapshots);
        } else {
            ctx.tag_objects(effect.tag.clone(), snapshots);
        }
    } else if effect.replace_tagged_objects || is_implicit_object_tag(effect.tag.as_str()) {
        ctx.set_tagged_objects(effect.tag.clone(), Vec::new());
    } else {
        ctx.tagged_objects.entry(effect.tag.clone()).or_default();
    }

    let outcome = EffectOutcome::with_objects(outcome_objects.clone())
        .with_result_objects(outcome_objects.clone())
        .with_execution_fact(ExecutionFact::ChosenObjects(outcome_objects))
        .with_chosen_object_memory(chosen_memory);
    let original = if let Some(search_event) = search_event {
        outcome.with_event(search_event)
    } else {
        outcome
    };
    // Tags, chosen-object memory and permission links belong to the
    // original instruction and must be complete before additions run.
    let original = EffectOutcome::aggregate_replacement_outcomes(original, disclosures);
    let mut original = crate::effects::CompletedEffectOutputs::from_children(
        std::mem::take(&mut retained_children),
        |_| original,
    );
    original.retain_published_references(published_outputs);
    let outputs = crate::effects::zones::finish_zone_change_receipts_with_outputs(
        game, ctx, original, receipts,
    )?;
    Ok(outputs)
}

#[derive(Debug)]
enum PreparedObjectChoice {
    Selected(SelectedObjectChoice),
    /// A completed empty input choice has no physical disclosure. Retain its
    /// actual native packet until original commitment, not as earlier history.
    Empty(crate::effects::CompletedEffectOutputs),
}

#[derive(Debug)]
struct ObjectChoiceProposal {
    choice: PreparedObjectChoice,
}
impl crate::effects::SimultaneousEffectProposal for ObjectChoiceProposal {
    fn commit_original_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        ExecutionError,
    > {
        crate::effects::composition::execute_transaction(
            game,
            ctx,
            || {
                crate::effects::SimultaneousEffectCommit::finished(
                    crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
                )
            },
            |game, ctx| {
                let outputs = match self.choice {
                    PreparedObjectChoice::Selected(selected) => {
                        complete_selected_object_choice(game, ctx, selected)?
                    }
                    PreparedObjectChoice::Empty(outputs) => outputs,
                };
                Ok(crate::effects::SimultaneousEffectCommit::finished(outputs))
            },
        )
    }
    fn commit_original(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::SimultaneousEffectCommit, ExecutionError> {
        self.commit_original_with_outputs(game, ctx)
            .map(crate::effects::SimultaneousEffectCommit::into_aggregate)
    }
    fn commit(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.commit_original_with_outputs(game, ctx)
            .map(|receipt| receipt.outcome.into_outcome())
    }
}

/// Select through the same native chooser with a mutable evidence/RNG world.
/// Public disclosure, chosen-object persistence and physical completion wait
/// for the generic prepared-original coordinator to commit this proposal.
pub(crate) fn prepare_choose_objects_program(
    effect: &ChooseObjectsEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Result<Option<Box<dyn crate::effects::ActionProgramCursor>>, ExecutionError> {
    if effect.is_search || !effect.reveal || effect.reveal_is_presentation_only != Some(false) {
        return Err(ExecutionError::Impossible(
            "selection has no prepared disclosure program".into(),
        ));
    }
    let mut selected = None;
    let empty = run_choose_objects_phase_with_outputs(effect, game, ctx, Some(&mut selected))?;
    if ctx.decision_maker.awaiting_choice() {
        return Ok(None);
    }
    let choice = selected.map_or_else(
        || PreparedObjectChoice::Empty(empty),
        PreparedObjectChoice::Selected,
    );
    Ok(Some(
        super::action_program::prepared_original_program_cursor(
            crate::effect::Effect::new(effect.clone()),
            Box::new(ObjectChoiceProposal { choice }),
        ),
    ))
}

pub(crate) fn run_choose_objects_with_outputs(
    effect: &ChooseObjectsEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    run_choose_objects_phase_with_outputs(effect, game, ctx, None)
}

fn run_choose_objects_phase_with_outputs(
    effect: &ChooseObjectsEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    mut prepared_selection: Option<&mut Option<SelectedObjectChoice>>,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    if ctx.decision_maker.awaiting_choice() {
        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
            EffectOutcome::count(0),
        ));
    }
    if effect.reveal && effect.reveal_is_presentation_only.is_none() {
        return Err(ExecutionError::IncompleteEvidence(
            "serialized public selection lacks its reveal policy; recompile its artifact".into(),
        ));
    }
    game.clear_pending_decision_controllers();
    let mut published_cast_outputs = Vec::new();
    let mut retained_children = Vec::new();
    let mut pending_selection_cleared_tag = false;
    let instruction = crate::effects::composition::execute_result_transaction(
        game,
        ctx,
        |game, ctx| {
            let chooser_id = crate::effects::helpers::resolve_player_filter_as_chooser(
                game,
                &effect.chooser,
                ctx,
            )?;

            let search_zones = search_zones(effect)?;
            let library_owner = if effect.is_search && search_zones.contains(&Zone::Library) {
                let filter_ctx = if object_filter_mentions_iterated_player(&effect.filter)
                    && matches!(effect.chooser, PlayerFilter::Target(_))
                {
                    let base_ctx = ctx.filter_context(game);
                    if base_ctx.iterated_player.is_none() {
                        base_ctx.with_iterated_player(Some(chooser_id))
                    } else {
                        base_ctx
                    }
                } else {
                    ctx.filter_context(game)
                };
                let owners = library_candidate_players(effect, game, ctx, &filter_ctx, chooser_id)?;
                if owners.len() == 1 {
                    Some(owners[0])
                } else {
                    None
                }
            } else {
                None
            };
            execute_library_search_scope(
                game,
                ctx,
                LibrarySearchRequest {
                    chooser: chooser_id,
                    library_owner,
                    search_library: effect.is_search && search_zones.contains(&Zone::Library),
                    require_library_access: effect.is_search && search_zones == vec![Zone::Library],
                    // This adapter's existing initial presentation has its own policy;
                    // candidate restriction remains with its selected-object programme.
                    restrict_initial_view: false,
                    refresh_library_access: true,
                },
                |game, ctx, search| {
                    published_cast_outputs.extend(search.completed_outputs);
                    let search_viewer = search.chooser;
                    let search_override = search.found_card_policy;
                    let search_event = search.event;
                    let mut candidates = collect_candidates(effect, game, ctx, chooser_id)?;
                    let relation_cost = ctx.cause.cause_type
                        == crate::events::cause::CauseType::Cost
                        && super::selection_relations::has_relations(&effect.filter);
                    if relation_cost {
                        candidates
                            .retain(|id| !ctx.replacement.entry_reserved_objects.contains(id));
                    }

                    if !game.source_snapshot_is_exempt_from_range(
                        Some(ctx.source),
                        ctx.source_snapshot.as_ref(),
                    ) {
                        candidates.retain(|object| {
                            game.object_is_within_range(chooser_id, *object, Some(ctx.source))
                        });
                    }
                    let hidden_library_candidates = if effect.is_search
                        && search_zones.contains(&Zone::Library)
                    {
                        library_owner
                            .map(|owner| hidden_library_search_candidates(effect, game, ctx, owner))
                            .unwrap_or_default()
                    } else {
                        Vec::new()
                    };
                    for id in &hidden_library_candidates {
                        if !candidates.contains(id) {
                            candidates.push(*id);
                        }
                    }
                    if candidates.is_empty()
                        && effect.is_search
                        && search_zones.contains(&Zone::Library)
                    {
                        for player in &game.players {
                            if !game.can_search_library_from_effect(
                                chooser_id,
                                player.id,
                                ctx.controller,
                            ) {
                                continue;
                            }
                            for &id in &player.library {
                                let is_hidden_library_card = game.is_hidden_card_placeholder(id)
                                    || game.object(id).is_some_and(|obj| {
                                        obj.zone == Zone::Library && obj.name == "Hidden Card"
                                    });
                                if is_hidden_library_card && !candidates.contains(&id) {
                                    candidates.push(id);
                                }
                            }
                        }
                    }
                    if effect.is_search && search_zones.contains(&Zone::Library) {
                        // The placeholders added above bypassed the zone scan's "search
                        // the top N cards instead" restriction (Aven Mindcensor).
                        game.restrict_library_search_candidates(chooser_id, &mut candidates);
                    }
                    // A random pick among qualifying hand cards ("exile a nonland card at
                    // random from your hand"): every peer must shuffle the same set, so
                    // the owners reveal the qualifying private cards first (see
                    // `game_state::hidden_hand_choices`).
                    if effect.count.is_random()
                        && effective_search_zones(effect, game, chooser_id)?.contains(&Zone::Hand)
                    {
                        let filter_ctx = choice_filter_context(effect, game, ctx, chooser_id);
                        let hand_ids =
                            hand_candidate_ids(effect, game, ctx, &filter_ctx, chooser_id)?;
                        if !game.settle_hidden_hand_random_pool(
                            &mut *ctx.decision_maker,
                            ctx.source,
                            &hand_zone_filter(effect),
                            &filter_ctx,
                            &hand_ids,
                            &mut candidates,
                        ) {
                            ctx.clear_object_tag(effect.tag.as_str());
                            return Ok(EffectOutcome::count(0));
                        }
                    }
                    // Symmetric across peers; see `game_state::hidden_hand_choices`.
                    let hidden_library_pool =
                        hidden_library_pool_choice(effect, game, ctx, chooser_id)?;
                    let hidden_identity_choice =
                        hidden_hand_choice(effect, game, ctx, chooser_id)? || hidden_library_pool;
                    // CR 107.3f: an X that appears
                    // only in the text and isn't defined is chosen by the controller as
                    // the ability resolves. For "reveal X cards" that choice is the
                    // number of objects chosen, so the choice itself binds X.
                    let binds_undefined_x = effect.count.dynamic_x
                        && effect.count_value.is_none()
                        && ctx.x_value.is_none()
                        && !effect.is_search;
                    if candidates.is_empty() && !hidden_identity_choice {
                        if binds_undefined_x {
                            ctx.x_value = Some(0);
                        }
                        if effect.replace_tagged_objects
                            || is_implicit_object_tag(effect.tag.as_str())
                        {
                            ctx.set_tagged_objects(effect.tag.clone(), Vec::new());
                        } else {
                            // An accumulating choice keeps earlier members, but an
                            // actually completed empty choice still publishes its tag.
                            ctx.tagged_objects.entry(effect.tag.clone()).or_default();
                        }
                        let outcome = EffectOutcome::count(0)
                            .with_result_objects(Vec::new())
                            .with_execution_fact(ExecutionFact::ChosenObjects(Vec::new()));
                        return Ok(if let Some(search_event) = search_event.clone() {
                            outcome.with_event(search_event)
                        } else {
                            outcome
                        });
                    }

                    let (base_min, max) = if binds_undefined_x {
                        (0, candidates.len())
                    } else if effect.count.dynamic_x || effect.count_value.is_some() {
                        let x = if let Some(count_value) = effect.count_value.as_ref() {
                            let previous_iterated_player = ctx.iteration.iterated_player;
                            if matches!(effect.chooser, PlayerFilter::Target(_))
                                && value_mentions_iterated_player(count_value)
                            {
                                ctx.iteration.iterated_player = Some(chooser_id);
                            }
                            let resolved = resolve_value(game, count_value, ctx);
                            ctx.iteration.iterated_player = previous_iterated_player;
                            resolved?.max(0) as usize
                        } else {
                            ctx.x_value.ok_or_else(|| {
                                ExecutionError::UnresolvableValue("X value not set".to_string())
                            })? as usize
                        };

                        let optional_dynamic_choice = effect.count.up_to_x
                            || (effect.is_search
                                && effect.search_mode == SearchSelectionMode::Optional);
                        if optional_dynamic_choice {
                            (0, x.min(candidates.len()))
                        } else {
                            let bounded = x.min(candidates.len());
                            (bounded, bounded)
                        }
                    } else {
                        compute_choice_bounds(effect.count, candidates.len())
                    };
                    if max == 0 && !hidden_identity_choice {
                        if binds_undefined_x {
                            ctx.x_value = Some(0);
                        }
                        let outcome = EffectOutcome::count(0);
                        return Ok(if let Some(search_event) = search_event.clone() {
                            outcome.with_event(search_event)
                        } else {
                            outcome
                        });
                    }

                    let has_hidden_search_zones = effect.is_search
                        && (search_zones.iter().any(Zone::is_hidden)
                            || !hidden_library_candidates.is_empty());
                    if has_hidden_search_zones && library_owner.is_none() {
                        view_hidden_candidate_objects(
                            game,
                            ctx,
                            search_viewer,
                            &candidates,
                            "Search hidden zone",
                            false,
                        );
                    }
                    let has_search_stated_quality = effect.filter.has_search_stated_quality();
                    let search_required_count =
                        compute_search_required_count(effect.search_mode, max);
                    let allow_hidden_partial =
                        effect.is_search && has_hidden_search_zones && has_search_stated_quality;
                    let min = if effect.is_search {
                        if allow_hidden_partial {
                            0
                        } else {
                            search_required_count.max(base_min)
                        }
                    } else {
                        base_min
                    };
                    let required_public_count = if allow_hidden_partial {
                        let public_count = public_search_candidates(game, &candidates).len();
                        match effect.search_mode {
                            SearchSelectionMode::Exact => search_required_count.min(public_count),
                            SearchSelectionMode::Optional => 0,
                            SearchSelectionMode::AllMatching => public_count,
                        }
                    } else {
                        0
                    };

                    let description = if effect.is_search
                        && matches!(
                            effect.description.as_str(),
                            "Choose" | "card" | "cards" | "objects"
                        )
                        && let Some(prompt) =
                            friendly_same_name_search_prompt(game, ctx, &effect.filter, min, max)
                    {
                        prompt
                    } else if effect.description == "Choose" {
                        let tag_str = effect.tag.as_str();
                        let verb = if tag_str.starts_with("sacrificed") {
                            "sacrifice"
                        } else if tag_str.starts_with("discarded") {
                            "discard"
                        } else if tag_str.starts_with("exiled") {
                            "exile"
                        } else if tag_str.starts_with("returned") {
                            "return"
                        } else {
                            "choose"
                        };
                        describe_choose_from_filter(&effect.filter, min, max, verb)
                    } else {
                        effect.description.clone()
                    };
                    let aggregate_constraint = effect
                        .aggregate_constraint
                        .as_ref()
                        .map(|constraint| {
                            let maximum = resolve_value(game, &constraint.maximum, ctx)?;
                            let minimum = constraint
                                .minimum
                                .as_ref()
                                .map(|minimum| resolve_value(game, minimum, ctx))
                                .transpose()?;
                            let mut resolved = crate::effect::ChoiceAggregateConstraint::at_most(
                                constraint.metric,
                                maximum,
                            );
                            resolved.minimum = minimum.map(crate::effect::Value::Fixed);
                            Ok::<_, ExecutionError>(resolved)
                        })
                        .transpose()?;
                    let reveals_selection_publicly = effect.is_search
                        && has_hidden_search_zones
                        && (effect.reveal
                            || ctx.public_search_reveal_tag.as_ref() == Some(&effect.tag));
                    let chosen: Vec<ObjectId> = if effect.count.is_random() {
                        let mut randomized = candidates.clone();
                        game.shuffle_slice(&mut randomized);
                        randomized.truncate(max);
                        randomized
                    } else {
                        let mut spec = ChooseObjectsSpec::new(
                            ctx.source,
                            description.clone(),
                            candidates.clone(),
                            min,
                            Some(max),
                        );
                        if super::selection_relations::has_relations(&effect.filter) {
                            spec = spec.with_relation_filter(effect.filter.clone());
                        }
                        if let Some(constraint) = aggregate_constraint.clone() {
                            spec = spec.with_aggregate_constraint(constraint);
                        }
                        if allow_hidden_partial || hidden_identity_choice {
                            spec = spec.allow_partial_completion();
                        }
                        if has_hidden_search_zones || hidden_identity_choice {
                            spec = spec.require_explicit_choice();
                        }
                        if has_hidden_search_zones {
                            spec = spec.with_hidden_card_visibility(
                                DecisionHiddenCardVisibility::PrivateToDecisionPlayer,
                            );
                        }
                        if reveals_selection_publicly || relation_cost {
                            // The chosen cards are revealed: open them on every peer
                            // before the answer is replayed (as `SearchSpec` does for a
                            // revealed search), so every engine filters the real cards.
                            spec = spec.with_selection_reveal_policy(
                                crate::decisions::context::SelectionRevealPolicy::Public,
                            );
                        }
                        make_decision(game, ctx.decision_maker, chooser_id, Some(ctx.source), spec)
                    };
                    if !effect.count.is_random() && ctx.decision_maker.awaiting_choice() {
                        pending_selection_cleared_tag = true;
                        ctx.clear_object_tag(effect.tag.as_str());
                        let outcome = EffectOutcome::count(0);
                        return Ok(if let Some(search_event) = search_event {
                            outcome.with_event(search_event)
                        } else {
                            outcome
                        });
                    }
                    if (effect.is_search || hidden_library_pool)
                        && !effect.count.is_random()
                        && let Some(rejected) = chosen.iter().copied().find(|id| {
                            !candidates.contains(id)
                                && game
                                    .object(*id)
                                    .is_some_and(|object| object.zone == Zone::Library)
                        })
                    {
                        // A library card that is not a candidate was chosen: on a peer
                        // this is a card opened before the replay (a publicly revealed
                        // selection) that fails the search's filter. Reject it as a
                        // violated hidden choice instead of silently dropping it, which
                        // would desync this engine from the chooser's.
                        let name = game
                            .object(rejected)
                            .map(|object| object.name.to_string())
                            .unwrap_or_default();
                        return Err(ExecutionError::Impossible(format!(
                            "{}: {name} does not satisfy the hidden choice \"{description}\"",
                            crate::game_state::HIDDEN_IDENTITY_VIOLATION_PREFIX
                        )));
                    }
                    // A payment selects an exact legal set. Resolution normalization
                    // must not fill, truncate, deduplicate or substitute what was paid.
                    let exact_cost_choice = ctx.cause.cause_type
                        == crate::events::cause::CauseType::Cost
                        && !effect.is_search;
                    if exact_cost_choice {
                        let mut seen = std::collections::HashSet::new();
                        if chosen.len() < min
                            || chosen.len() > max
                            || chosen
                                .iter()
                                .any(|id| !candidates.contains(id) || !seen.insert(*id))
                        {
                            return Err(ExecutionError::Impossible(
                                "cost choice must contain the required distinct legal objects"
                                    .into(),
                            ));
                        }
                    }
                    let preserve_order = effect.count_value.as_ref().is_some_and(|value| {
                        value.has_surface_hint(ironsmith_core::ValueSurfaceHint::ChooseAllInOrder)
                    });
                    // Never fill a hidden identity choice up to its minimum: the fill would
                    // pick different cards on the owner and on peers holding placeholders.
                    let fill_to_min = !allow_hidden_partial && !hidden_identity_choice;
                    let chosen = normalize_chosen_objects(
                        chosen,
                        &candidates,
                        min,
                        max,
                        fill_to_min,
                        preserve_order,
                    );
                    if binds_undefined_x {
                        ctx.x_value = Some(chosen.len() as u32);
                    }
                    if hidden_identity_choice && chosen.iter().any(|id| !candidates.contains(id)) {
                        // A known card outside the candidates failed the filter (for a
                        // peer, after the chosen card was opened): reject the choice.
                        return Err(ExecutionError::InvalidTarget);
                    }
                    // Identity-dependent normalizations (names, mana values, powers,
                    // types, aggregate bounds) cannot be evaluated for placeholders. For
                    // an honest choice they are no-ops on the owner, so skip them wherever
                    // a placeholder was chosen instead of rewriting the choice differently
                    // from the owner.
                    //
                    // A library search that picked placeholders is the same: the owner's
                    // engine knows the cards, peers only see "Hidden Card" (which every
                    // name-based normalization would collapse).
                    let chose_library_placeholder = effect.is_search
                        && chosen.iter().any(|id| {
                            game.is_hidden_card_placeholder(*id)
                                && game
                                    .object(*id)
                                    .is_some_and(|object| object.zone == Zone::Library)
                        });
                    let chose_placeholder = (hidden_identity_choice
                        && chosen.iter().any(|id| game.is_hidden_card_placeholder(*id)))
                        || chose_library_placeholder;
                    let allow_hidden_partial = allow_hidden_partial || hidden_identity_choice;
                    let chosen = enforce_public_search_choice_constraint(
                        game,
                        &candidates,
                        chosen,
                        required_public_count,
                        max,
                    );
                    let chosen = enforce_single_graveyard_choice_constraint(
                        effect,
                        game,
                        &candidates,
                        chosen,
                        min,
                        max,
                    );
                    if (effect.filter.shares_name || effect.filter.shares_color || effect.filter.shares_card_type || relation_cost)
                        && !super::selection_relations::allows(
                            game,
                            &effect.filter,
                            &chosen,
                            chose_placeholder && !relation_cost,
                        )
                    {
                        return Err(ExecutionError::Impossible(
                            "chosen objects do not satisfy the whole-selection relation".into(),
                        ));
                    }
                    let chosen = if effect.filter.distinct_names && !chose_placeholder {
                        normalize_chosen_distinct_names(
                            game,
                            chosen,
                            &candidates,
                            min,
                            max,
                            !allow_hidden_partial,
                        )
                    } else {
                        chosen
                    };
                    let chosen = if effect.filter.distinct_mana_values && !chose_placeholder {
                        normalize_chosen_distinct_mana_values(
                            game,
                            chosen,
                            &candidates,
                            min,
                            max,
                            !allow_hidden_partial,
                        )
                    } else {
                        chosen
                    };
                    let chosen = if effect.filter.distinct_powers && !chose_placeholder {
                        normalize_chosen_distinct_powers(
                            game,
                            chosen,
                            &candidates,
                            min,
                            max,
                            !allow_hidden_partial,
                        )
                    } else {
                        chosen
                    };
                    let chosen = if effect.filter.distinct_creature_types && !chose_placeholder {
                        normalize_chosen_distinct_creature_types(
                            game,
                            chosen,
                            &candidates,
                            min,
                            max,
                            !allow_hidden_partial,
                        )
                    } else {
                        chosen
                    };
                    let chosen = if effect.filter.shares_land_type && !chose_placeholder {
                        normalize_chosen_shared_land_type(
                            game,
                            chosen,
                            &candidates,
                            min,
                            max,
                            !allow_hidden_partial,
                        )
                    } else {
                        chosen
                    };
                    let chosen = if effect.filter.one_per_card_type && !chose_placeholder {
                        normalize_chosen_one_per_card_type(
                            game,
                            chosen,
                            &candidates,
                            min,
                            max,
                            !allow_hidden_partial,
                            &effect.filter.card_types,
                        )
                    } else {
                        chosen
                    };
                    let chosen = if let Some(constraint) = aggregate_constraint.clone()
                        && !chose_placeholder
                    {
                        normalize_chosen_aggregate_constraint(
                            game,
                            chosen,
                            &candidates,
                            min,
                            max,
                            !allow_hidden_partial,
                            constraint,
                        )
                    } else {
                        chosen
                    };
                    if let Some(constraint) = aggregate_constraint.as_ref()
                        && !chose_placeholder
                        && let Some(crate::effect::Value::Fixed(minimum)) =
                            constraint.minimum.as_ref().map(|value| value.unhinted())
                    {
                        let chosen_total = crate::targeting::aggregate_object_set_value(
                            game,
                            chosen.iter().copied(),
                            constraint.metric,
                        );
                        if chosen_total < *minimum {
                            return Err(ExecutionError::Impossible(format!(
                                "chosen objects have aggregate value {chosen_total}, below required minimum {minimum}"
                            )));
                        }
                    }
                    // Claims enter the shared ledger, so their description must be the
                    // same on every peer; the prompt text above embeds locally clamped
                    // bounds and names.
                    let claim_description = if effect.description == "Choose" {
                        format!("Choose {}", effect.filter.description())
                    } else {
                        effect.description.clone()
                    };
                    if effect.is_search && !effect.count.is_random() {
                        // Every library card chosen by a filtered search must satisfy the
                        // search's own filter once opened (moved to a public zone,
                        // revealed, or disclosed at the end of the match). The chosen
                        // list is identical on every peer, so the claim subjects (and
                        // library anchors, should the card be shuffled back) are
                        // symmetric; only peers holding placeholders record obligations.
                        // An unfiltered search ("search for a card") records nothing.
                        let library_choices: Vec<ObjectId> = chosen
                            .iter()
                            .copied()
                            .filter(|id| {
                                game.hidden_card_info(*id).is_some()
                                    && game
                                        .object(*id)
                                        .is_some_and(|object| object.zone == Zone::Library)
                            })
                            .collect();
                        if !library_choices.is_empty() {
                            let filter_ctx = choice_filter_context(effect, game, ctx, chooser_id);
                            game.record_hidden_identity_obligations(
                                &library_choices,
                                &library_zone_filter(effect),
                                &filter_ctx,
                                &claim_description,
                            );
                        }
                    }
                    if hidden_identity_choice {
                        // Symmetric condition (the owner chooses no placeholder), so every
                        // peer records the same claims about the chosen private cards.
                        let filter_ctx = choice_filter_context(effect, game, ctx, chooser_id);
                        let hidden_choices: Vec<ObjectId> = chosen
                            .iter()
                            .copied()
                            .filter(|id| game.hidden_card_info(*id).is_some())
                            .collect();
                        game.record_hidden_identity_obligations(
                            &hidden_choices,
                            &if hidden_library_pool {
                                library_zone_filter(effect)
                            } else {
                                hand_zone_filter(effect)
                            },
                            &filter_ctx,
                            &claim_description,
                        );
                    }
                    let selected = SelectedObjectChoice {
                        effect: effect.clone(),
                        chosen,
                        chooser_id,
                        search_viewer,
                        search_zones: search_zones.clone(),
                        search_override,
                        search_event,
                    };
                    if let Some(receiver) = prepared_selection.as_deref_mut() {
                        *receiver = Some(selected);
                        // Private control flow only: no completed original is
                        // exposed to the program cursor during input selection.
                        return Ok(EffectOutcome::resolved());
                    }
                    let outputs = complete_selected_object_choice(game, ctx, selected)?;
                    let outcome = outputs.outcome.clone();
                    retained_children.push(outputs);
                    Ok(outcome)
                },
            )
        },
    );
    let pending = ctx.decision_maker.awaiting_choice();
    if pending {
        if pending_selection_cleared_tag {
            ctx.clear_object_tag(effect.tag.as_str());
        }
        return instruction.map(|_| {
            crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0))
        });
    }
    instruction.map(|outcome| {
        let mut outputs =
            crate::effects::CompletedEffectOutputs::from_children(retained_children, |_| outcome);
        outputs.retain_published_references(published_cast_outputs);
        outputs
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::decision::DecisionMaker;
    use crate::effect::ExecutionFact;
    use crate::effects::{EffectExecutor, ExecutionContext, MoveToZoneEffect};
    use crate::filter::ObjectFilter;
    use crate::ids::{CardId, PlayerId};
    use crate::tag::TagKey;
    use crate::target::{ChooseSpec, PlayerFilter};
    use crate::types::CardType;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn create_graveyard_card(game: &mut GameState, name: &str, owner: PlayerId) -> ObjectId {
        let card = CardBuilder::new(CardId::from_raw(game.new_object_id().0 as u32), name)
            .card_types(vec![CardType::Creature])
            .build();
        game.create_object_from_card(&card, owner, Zone::Graveyard)
    }

    fn create_library_card(game: &mut GameState, name: &str, owner: PlayerId) -> ObjectId {
        let card = CardBuilder::new(CardId::from_raw(game.new_object_id().0 as u32), name)
            .card_types(vec![CardType::Creature])
            .build();
        game.create_object_from_card(&card, owner, Zone::Library)
    }

    fn create_exiled_card(game: &mut GameState, name: &str, owner: PlayerId) -> ObjectId {
        let card = CardBuilder::new(CardId::from_raw(game.new_object_id().0 as u32), name)
            .card_types(vec![CardType::Creature])
            .build();
        game.create_object_from_card(&card, owner, Zone::Exile)
    }

    fn create_sideboard_card(game: &mut GameState, name: &str, owner: PlayerId) -> ObjectId {
        let card = CardBuilder::new(CardId::from_raw(game.new_object_id().0 as u32), name)
            .card_types(vec![CardType::Artifact])
            .build();
        game.create_object_from_card(&card, owner, Zone::OutsideGame)
    }

    fn create_library_creature_with_power(
        game: &mut GameState,
        name: &str,
        owner: PlayerId,
        power: i32,
    ) -> ObjectId {
        let card = CardBuilder::new(CardId::from_raw(game.new_object_id().0 as u32), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(power, 1))
            .build();
        game.create_object_from_card(&card, owner, Zone::Library)
    }

    #[test]
    fn choose_objects_can_select_owned_sideboard_cards_outside_the_game() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let sideboard_card = create_sideboard_card(&mut game, "Wish Target", alice);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let filter = ObjectFilter::artifact()
            .owned_by(PlayerFilter::You)
            .in_zone(Zone::OutsideGame);
        let effect = ChooseObjectsEffect::new(filter, 1, PlayerFilter::You, "wished")
            .as_optional_search()
            .in_zone(Zone::OutsideGame);

        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.objects(), Some([sideboard_card].as_slice()));
        assert_eq!(
            ctx.get_tagged("wished").map(|snapshot| snapshot.object_id),
            Some(sideboard_card)
        );
    }

    #[test]
    fn library_search_prompts_with_hidden_placeholders() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let first = game.create_hidden_card_placeholder(
            alice,
            Zone::Library,
            0,
            "first-hidden-library-card".to_string(),
        );
        let second = game.create_hidden_card_placeholder(
            alice,
            Zone::Library,
            1,
            "second-hidden-library-card".to_string(),
        );
        let source = game.new_object_id();
        let mut dm = SearchPromptDecisionMaker::default();
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        let filter = ObjectFilter::default()
            .with_type(CardType::Instant)
            .owned_by(PlayerFilter::You)
            .in_zone(Zone::Library);
        let effect = ChooseObjectsEffect::new(filter, 1, PlayerFilter::You, "found")
            .as_search()
            .reveal()
            .in_zone(Zone::Library);

        effect
            .execute(&mut game, &mut ctx)
            .expect("hidden library search should pause for a choice");

        assert!(dm.captured, "search should surface a decision prompt");
        assert_eq!(dm.candidates, vec![first, second]);
        assert!(
            dm.viewed_cards
                .iter()
                .any(|(viewer, subject, zone, public, cards)| {
                    *viewer == alice
                        && *subject == alice
                        && *zone == Zone::Library
                        && !*public
                        && cards == &vec![first, second]
                }),
            "hidden library placeholders should be privately viewed before choosing"
        );
    }

    #[test]
    fn sequenced_library_search_stops_on_hidden_placeholder_prompt() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let first = game.create_hidden_card_placeholder(
            alice,
            Zone::Library,
            0,
            "first-hidden-library-card".to_string(),
        );
        let second = game.create_hidden_card_placeholder(
            alice,
            Zone::Library,
            1,
            "second-hidden-library-card".to_string(),
        );
        let source = game.new_object_id();
        let mut dm = SearchPromptDecisionMaker::default();
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        let filter = ObjectFilter::default()
            .with_type(CardType::Instant)
            .owned_by(PlayerFilter::You)
            .in_zone(Zone::Library);
        let choose = ChooseObjectsEffect::new(filter, 1, PlayerFilter::You, "found")
            .as_search()
            .reveal()
            .in_zone(Zone::Library);
        let sequence = crate::effects::SequenceEffect::new(vec![
            crate::effect::Effect::new(choose),
            crate::effect::Effect::shuffle_library_player(PlayerFilter::You),
        ]);

        sequence
            .execute(&mut game, &mut ctx)
            .expect("hidden library sequence should pause for a choice");

        assert!(dm.captured, "sequence should preserve the search prompt");
        assert_eq!(dm.candidates, vec![first, second]);
        assert_eq!(
            game.player(alice).expect("Alice should exist").library,
            vec![first, second],
            "sequence should not run later effects while the search prompt is pending"
        );
    }

    fn create_hand_card(game: &mut GameState, name: &str, owner: PlayerId) -> ObjectId {
        let card = CardBuilder::new(CardId::from_raw(game.new_object_id().0 as u32), name)
            .card_types(vec![CardType::Creature])
            .build();
        game.create_object_from_card(&card, owner, Zone::Hand)
    }

    fn create_battlefield_card(game: &mut GameState, name: &str, owner: PlayerId) -> ObjectId {
        let card = CardBuilder::new(CardId::from_raw(game.new_object_id().0 as u32), name)
            .card_types(vec![CardType::Creature])
            .build();
        game.create_object_from_card(&card, owner, Zone::Battlefield)
    }

    fn create_battlefield_creature_with_power(
        game: &mut GameState,
        name: &str,
        owner: PlayerId,
        power: i32,
    ) -> ObjectId {
        let card = CardBuilder::new(CardId::from_raw(game.new_object_id().0 as u32), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(power, 2))
            .build();
        game.create_object_from_card(&card, owner, Zone::Battlefield)
    }

    fn create_battlefield_artifact_with_mana_value(
        game: &mut GameState,
        name: &str,
        owner: PlayerId,
        mana_value: u8,
    ) -> ObjectId {
        let card = CardBuilder::new(CardId::from_raw(game.new_object_id().0 as u32), name)
            .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
                crate::mana::ManaSymbol::Generic(mana_value),
            ]]))
            .card_types(vec![CardType::Artifact])
            .build();
        game.create_object_from_card(&card, owner, Zone::Battlefield)
    }

    struct PromptCapturingDecisionMaker {
        captured: bool,
    }

    impl DecisionMaker for PromptCapturingDecisionMaker {
        fn awaiting_choice(&self) -> bool {
            self.captured
        }

        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            self.captured = true;
            ctx.candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .map(|candidate| candidate.id)
                .take(ctx.min)
                .collect()
        }
    }

    #[test]
    fn aggregate_choice_constraint_is_exposed_and_enforced() {
        struct SelectAllDecisionMaker {
            seen_constraint: Option<crate::effect::ChoiceAggregateConstraint>,
        }

        impl DecisionMaker for SelectAllDecisionMaker {
            fn decide_objects(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::SelectObjectsContext,
            ) -> Vec<ObjectId> {
                self.seen_constraint = ctx.aggregate_constraint.clone();
                ctx.candidates
                    .iter()
                    .filter(|candidate| candidate.legal)
                    .map(|candidate| candidate.id)
                    .collect()
            }
        }

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let three = create_battlefield_creature_with_power(&mut game, "Three", alice, 3);
        let two = create_battlefield_creature_with_power(&mut game, "Two", alice, 2);
        let source = game.new_object_id();
        let constraint = crate::effect::ChoiceAggregateConstraint::total_power_at_most(4);
        let mut dm = SelectAllDecisionMaker {
            seen_constraint: None,
        };
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        let effect = ChooseObjectsEffect::new(
            ObjectFilter::creature().controlled_by(PlayerFilter::You),
            crate::effect::ChoiceCount::any_number(),
            PlayerFilter::You,
            "kept",
        )
        .with_aggregate_constraint(constraint.clone());

        let outcome = run_choose_objects(&effect, &mut game, &mut ctx)
            .expect("aggregate-constrained choice should resolve");
        let chosen = outcome.objects().expect("choice should return objects");
        drop(ctx);

        assert_eq!(dm.seen_constraint, Some(constraint));
        assert_eq!(chosen, &[two]);
        assert!(!chosen.contains(&three));
    }

    #[test]
    fn aggregate_choice_constraint_counts_negative_power_in_the_total() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let five = create_battlefield_creature_with_power(&mut game, "Five", alice, 5);
        let minus_two = create_battlefield_creature_with_power(&mut game, "Minus Two", alice, -2);
        let chosen = vec![five, minus_two];

        let normalized = normalize_chosen_aggregate_constraint(
            &game,
            chosen.clone(),
            &chosen,
            0,
            chosen.len(),
            true,
            crate::effect::ChoiceAggregateConstraint::total_power_at_most(4),
        );

        assert_eq!(normalized, chosen, "5 + -2 is a legal total power of 3");
    }

    #[test]
    fn aggregate_choice_constraint_resolves_dynamic_mana_value_from_sacrificed_lki() {
        struct SelectAllDecisionMaker {
            seen_constraint: Option<crate::effect::ChoiceAggregateConstraint>,
        }

        impl DecisionMaker for SelectAllDecisionMaker {
            fn decide_objects(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::SelectObjectsContext,
            ) -> Vec<ObjectId> {
                self.seen_constraint = ctx.aggregate_constraint.clone();
                ctx.candidates
                    .iter()
                    .filter(|candidate| candidate.legal)
                    .map(|candidate| candidate.id)
                    .collect()
            }
        }

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let sacrificed =
            create_battlefield_artifact_with_mana_value(&mut game, "Sacrificed", alice, 4);
        let two = create_battlefield_artifact_with_mana_value(&mut game, "Two", alice, 2);
        let three = create_battlefield_artifact_with_mana_value(&mut game, "Three", alice, 3);
        let sacrificed_snapshot = ObjectSnapshot::from_object(
            game.object(sacrificed).expect("sacrificed artifact"),
            &game,
        );
        game.move_object_by_effect(sacrificed, Zone::Graveyard)
            .expect("sacrifice should move the artifact");

        let source = game.new_object_id();
        let mut dm = SelectAllDecisionMaker {
            seen_constraint: None,
        };
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        ctx.tag_objects("sacrifice_cost_0", vec![sacrificed_snapshot]);
        let effect = ChooseObjectsEffect::new(
            ObjectFilter::artifact().controlled_by(PlayerFilter::You),
            crate::effect::ChoiceCount::any_number(),
            PlayerFilter::You,
            "chosen",
        )
        .in_zone(Zone::Battlefield)
        .with_aggregate_constraint(
            crate::effect::ChoiceAggregateConstraint::total_mana_value_at_most(
                crate::effect::Value::ManaValueOf(Box::new(crate::target::ChooseSpec::Tagged(
                    crate::tag::TagKey::from("sacrifice_cost_0"),
                ))),
            ),
        );

        let outcome = run_choose_objects(&effect, &mut game, &mut ctx)
            .expect("dynamic aggregate-constrained choice should resolve");
        let chosen = outcome.objects().expect("choice should return objects");
        drop(ctx);

        assert_eq!(
            dm.seen_constraint,
            Some(crate::effect::ChoiceAggregateConstraint::total_mana_value_at_most(4))
        );
        assert_eq!(chosen, &[two]);
        assert!(!chosen.contains(&three));
    }

    #[derive(Default)]
    struct SearchPromptDecisionMaker {
        captured: bool,
        candidates: Vec<ObjectId>,
        viewed_cards: Vec<(PlayerId, PlayerId, Zone, bool, Vec<ObjectId>)>,
    }

    impl DecisionMaker for SearchPromptDecisionMaker {
        fn awaiting_choice(&self) -> bool {
            self.captured
        }

        fn view_cards(
            &mut self,
            _game: &GameState,
            viewer: PlayerId,
            cards: &[ObjectId],
            ctx: &crate::decisions::context::ViewCardsContext,
        ) {
            self.viewed_cards
                .push((viewer, ctx.subject, ctx.zone, ctx.public, cards.to_vec()));
        }

        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            self.captured = true;
            self.candidates = ctx
                .candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .map(|candidate| candidate.id)
                .collect();
            Vec::new()
        }
    }

    #[test]
    fn test_compute_choice_bounds_clamps_to_candidates() {
        let (min, max) = compute_choice_bounds(ChoiceCount::exactly(3), 2);
        assert_eq!(min, 2);
        assert_eq!(max, 2);
    }

    #[test]
    fn test_normalize_chosen_objects_truncates_dedups_and_fills() {
        let candidates = vec![
            ObjectId::from_raw(1),
            ObjectId::from_raw(2),
            ObjectId::from_raw(3),
        ];
        let chosen = vec![
            ObjectId::from_raw(3),
            ObjectId::from_raw(3),
            ObjectId::from_raw(99),
            ObjectId::from_raw(2),
        ];

        let normalized = normalize_chosen_objects(chosen, &candidates, 2, 2, true, false);
        assert_eq!(
            normalized,
            vec![ObjectId::from_raw(3), ObjectId::from_raw(1)]
        );
    }

    #[test]
    fn test_normalize_chosen_distinct_powers_replaces_duplicate_power() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let first_two = create_library_creature_with_power(&mut game, "First Two", alice, 2);
        let second_two = create_library_creature_with_power(&mut game, "Second Two", alice, 2);
        let three = create_library_creature_with_power(&mut game, "Three", alice, 3);
        let candidates = vec![first_two, second_two, three];

        let normalized = normalize_chosen_distinct_powers(
            &game,
            vec![first_two, second_two],
            &candidates,
            2,
            2,
            true,
        );

        assert_eq!(normalized, vec![first_two, three]);
    }

    #[test]
    fn test_stated_quality_search_with_single_candidate_can_fail_to_find() {
        struct FailToFindDecisionMaker;

        impl DecisionMaker for FailToFindDecisionMaker {
            fn decide_objects(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::SelectObjectsContext,
            ) -> Vec<ObjectId> {
                assert!(
                    ctx.allow_partial_completion,
                    "search prompts should allow partial completion"
                );
                Vec::new()
            }
        }

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let _only = create_library_card(&mut game, "Only Match", alice);
        let source = game.new_object_id();
        let mut dm = FailToFindDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        let filter = ObjectFilter::default()
            .in_zone(Zone::Library)
            .with_type(CardType::Creature);
        let effect =
            ChooseObjectsEffect::new(filter, ChoiceCount::up_to(1), PlayerFilter::You, "chosen")
                .in_zone(Zone::Library)
                .as_search();
        let outcome = run_choose_objects(&effect, &mut game, &mut ctx).expect("search resolves");

        let crate::effect::OutcomeValue::Objects(chosen) = outcome.value else {
            panic!("expected object selection result");
        };
        assert!(
            chosen.is_empty(),
            "single-candidate searches must still allow failing to find"
        );
    }

    #[test]
    fn test_stated_quality_exact_search_can_partially_complete() {
        struct ChooseOneDecisionMaker;

        impl DecisionMaker for ChooseOneDecisionMaker {
            fn decide_objects(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::SelectObjectsContext,
            ) -> Vec<ObjectId> {
                assert!(
                    ctx.allow_partial_completion,
                    "exact-count searches should still allow stopping early"
                );
                ctx.candidates
                    .iter()
                    .filter(|candidate| candidate.legal)
                    .map(|candidate| candidate.id)
                    .take(1)
                    .collect()
            }
        }

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let first = create_library_card(&mut game, "First Match", alice);
        let _second = create_library_card(&mut game, "Second Match", alice);
        let source = game.new_object_id();
        let mut dm = ChooseOneDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        let filter = ObjectFilter::default()
            .in_zone(Zone::Library)
            .with_type(CardType::Creature);
        let effect =
            ChooseObjectsEffect::new(filter, ChoiceCount::up_to(2), PlayerFilter::You, "chosen")
                .in_zone(Zone::Library)
                .as_search();
        let outcome = run_choose_objects(&effect, &mut game, &mut ctx).expect("search resolves");

        let crate::effect::OutcomeValue::Objects(chosen) = outcome.value else {
            panic!("expected object selection result");
        };
        assert_eq!(chosen, vec![first]);
    }

    #[test]
    fn test_quantity_only_search_with_single_candidate_cannot_fail_to_find() {
        struct FailToFindDecisionMaker;

        impl DecisionMaker for FailToFindDecisionMaker {
            fn decide_objects(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::SelectObjectsContext,
            ) -> Vec<ObjectId> {
                assert!(
                    !ctx.allow_partial_completion,
                    "quantity-only searches should not allow partial completion"
                );
                Vec::new()
            }
        }

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let only = create_library_card(&mut game, "Only Match", alice);
        let source = game.new_object_id();
        let mut dm = FailToFindDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        let filter = ObjectFilter::default().in_zone(Zone::Library);
        let effect =
            ChooseObjectsEffect::new(filter, ChoiceCount::up_to(1), PlayerFilter::You, "chosen")
                .in_zone(Zone::Library)
                .as_search();
        let outcome = run_choose_objects(&effect, &mut game, &mut ctx).expect("search resolves");

        let crate::effect::OutcomeValue::Objects(chosen) = outcome.value else {
            panic!("expected object selection result");
        };
        assert_eq!(chosen, vec![only]);
    }

    #[test]
    fn test_mixed_public_and_hidden_exact_search_requires_public_match() {
        struct FailToFindDecisionMaker;

        impl DecisionMaker for FailToFindDecisionMaker {
            fn decide_objects(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::SelectObjectsContext,
            ) -> Vec<ObjectId> {
                assert!(
                    ctx.allow_partial_completion,
                    "mixed hidden/public stated-quality searches should still allow hidden misses"
                );
                Vec::new()
            }
        }

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let graveyard_match = create_graveyard_card(&mut game, "Public Match", alice);
        let _library_match = create_library_card(&mut game, "Hidden Match", alice);
        let source = game.new_object_id();
        let mut dm = FailToFindDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        let filter = ObjectFilter::default().with_type(CardType::Creature);
        let effect =
            ChooseObjectsEffect::new(filter, ChoiceCount::up_to(1), PlayerFilter::You, "chosen")
                .in_zones(vec![Zone::Graveyard, Zone::Library])
                .as_search();
        let outcome = run_choose_objects(&effect, &mut game, &mut ctx).expect("search resolves");

        let crate::effect::OutcomeValue::Objects(chosen) = outcome.value else {
            panic!("expected object selection result");
        };
        assert_eq!(chosen, vec![graveyard_match]);
    }

    #[test]
    fn test_all_matching_search_auto_includes_public_matches() {
        struct FailToFindDecisionMaker;

        impl DecisionMaker for FailToFindDecisionMaker {
            fn decide_objects(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::SelectObjectsContext,
            ) -> Vec<ObjectId> {
                assert!(
                    ctx.allow_partial_completion,
                    "all-matching hidden searches should still allow hidden misses"
                );
                Vec::new()
            }
        }

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let graveyard_match = create_graveyard_card(&mut game, "Public Match", alice);
        let _library_match = create_library_card(&mut game, "Hidden Match", alice);
        let source = game.new_object_id();
        let mut dm = FailToFindDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        let filter = ObjectFilter::default().with_type(CardType::Creature);
        let effect = ChooseObjectsEffect::new(
            filter,
            ChoiceCount::any_number(),
            PlayerFilter::You,
            "chosen",
        )
        .in_zones(vec![Zone::Graveyard, Zone::Library])
        .as_all_matching_search();
        let outcome = run_choose_objects(&effect, &mut game, &mut ctx).expect("search resolves");

        let crate::effect::OutcomeValue::Objects(chosen) = outcome.value else {
            panic!("expected object selection result");
        };
        assert_eq!(chosen, vec![graveyard_match]);
    }

    #[test]
    fn test_single_graveyard_filter_considers_all_graveyards() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let bob_card = create_graveyard_card(&mut game, "Bob Card", bob);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let filter = ObjectFilter::default()
            .in_zone(Zone::Graveyard)
            .single_graveyard();
        let effect = ChooseObjectsEffect::new(filter, 1, PlayerFilter::You, "chosen")
            .in_zone(Zone::Graveyard);
        let outcome = run_choose_objects(&effect, &mut game, &mut ctx).expect("choose resolves");

        let crate::effect::OutcomeValue::Objects(chosen) = outcome.value else {
            panic!("expected object selection result");
        };
        assert_eq!(chosen, vec![bob_card]);
    }

    #[test]
    fn test_single_graveyard_filter_normalizes_mixed_owner_selection() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let alice_card = create_graveyard_card(&mut game, "Alice Card", alice);
        let bob_card_a = create_graveyard_card(&mut game, "Bob Card A", bob);
        let bob_card_b = create_graveyard_card(&mut game, "Bob Card B", bob);

        let filter = ObjectFilter::default()
            .in_zone(Zone::Graveyard)
            .single_graveyard();
        let effect = ChooseObjectsEffect::new(filter, 3, PlayerFilter::You, "chosen")
            .in_zone(Zone::Graveyard);
        let candidates = vec![alice_card, bob_card_a, bob_card_b];
        let chosen = vec![alice_card, bob_card_a];

        let normalized =
            enforce_single_graveyard_choice_constraint(&effect, &game, &candidates, chosen, 0, 3);
        assert_eq!(normalized, vec![alice_card]);
    }

    #[test]
    fn test_top_only_library_selects_top_matching_card() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let _bottom = create_library_card(&mut game, "Bottom Card", alice);
        let top = create_library_card(&mut game, "Top Card", alice);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let filter = ObjectFilter::default()
            .in_zone(Zone::Library)
            .owned_by(PlayerFilter::You);
        let effect = ChooseObjectsEffect::new(filter, 1, PlayerFilter::You, "chosen").top_only();
        let outcome = run_choose_objects(&effect, &mut game, &mut ctx).expect("choose resolves");

        let crate::effect::OutcomeValue::Objects(chosen) = outcome.value else {
            panic!("expected object selection result");
        };
        assert_eq!(chosen, vec![top], "expected top library card to be chosen");
    }

    #[test]
    fn test_top_only_library_selects_top_two_matching_cards() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bottom = create_library_card(&mut game, "Bottom Card", alice);
        let middle = create_library_card(&mut game, "Middle Card", alice);
        let top = create_library_card(&mut game, "Top Card", alice);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let filter = ObjectFilter::default()
            .in_zone(Zone::Library)
            .owned_by(PlayerFilter::You);
        let effect = ChooseObjectsEffect::new(filter, 2, PlayerFilter::You, "chosen").top_only();
        let outcome = run_choose_objects(&effect, &mut game, &mut ctx).expect("choose resolves");

        let crate::effect::OutcomeValue::Objects(chosen) = outcome.value else {
            panic!("expected object selection result");
        };
        assert_eq!(chosen.len(), 2, "expected exactly two chosen cards");
        assert!(chosen.contains(&top), "expected top card to be chosen");
        assert!(
            chosen.contains(&middle),
            "expected second-from-top card to be chosen"
        );
        assert!(
            !chosen.contains(&bottom),
            "bottom library card should not be chosen"
        );
    }

    #[test]
    fn test_dynamic_x_choice_count_requires_x_value() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let _card = create_graveyard_card(&mut game, "Card", alice);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let filter = ObjectFilter::default().in_zone(Zone::Graveyard);
        let mut effect = ChooseObjectsEffect::new(
            filter,
            ChoiceCount::dynamic_x(),
            PlayerFilter::You,
            "chosen",
        )
        .in_zone(Zone::Graveyard);

        effect.count_value = Some(crate::effect::Value::X);
        let err = run_choose_objects(&effect, &mut game, &mut ctx).expect_err("missing X errors");
        assert!(
            matches!(err, ExecutionError::UnresolvableValue(_)),
            "expected X resolution error, got {err:?}"
        );
    }

    #[test]
    fn test_undefined_dynamic_x_is_bound_by_choice() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let card = create_graveyard_card(&mut game, "Card", alice);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let filter = ObjectFilter::default().in_zone(Zone::Graveyard);
        let effect = ChooseObjectsEffect::new(
            filter,
            ChoiceCount::dynamic_x(),
            PlayerFilter::You,
            "chosen",
        )
        .in_zone(Zone::Graveyard);

        let outcome = run_choose_objects(&effect, &mut game, &mut ctx)
            .expect("undefined X is chosen during resolution");
        assert_eq!(ctx.x_value, Some(1));
        assert_eq!(outcome.objects().unwrap(), &[card]);
    }

    #[test]
    fn test_dynamic_x_choice_count_picks_exactly_x() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let card_a = create_graveyard_card(&mut game, "A", alice);
        let card_b = create_graveyard_card(&mut game, "B", alice);
        let _card_c = create_graveyard_card(&mut game, "C", alice);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice).with_x(2);

        let filter = ObjectFilter::default().in_zone(Zone::Graveyard);
        let effect = ChooseObjectsEffect::new(
            filter,
            ChoiceCount::dynamic_x(),
            PlayerFilter::You,
            "chosen",
        )
        .in_zone(Zone::Graveyard);
        let outcome = run_choose_objects(&effect, &mut game, &mut ctx).expect("choose resolves");

        let crate::effect::OutcomeValue::Objects(chosen) = outcome.value else {
            panic!("expected object selection result");
        };
        assert_eq!(chosen.len(), 2);
        assert!(chosen.contains(&card_a));
        assert!(chosen.contains(&card_b));
    }

    #[test]
    fn test_value_backed_optional_search_uses_resolved_count() {
        struct ChooseOneDecisionMaker;

        impl DecisionMaker for ChooseOneDecisionMaker {
            fn decide_objects(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::SelectObjectsContext,
            ) -> Vec<ObjectId> {
                assert_eq!(
                    ctx.max,
                    Some(2),
                    "expected resolved count value to set max choices"
                );
                ctx.candidates
                    .iter()
                    .filter(|candidate| candidate.legal)
                    .map(|candidate| candidate.id)
                    .take(1)
                    .collect()
            }
        }

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let first = create_library_card(&mut game, "First", alice);
        let _second = create_library_card(&mut game, "Second", alice);
        let _third = create_library_card(&mut game, "Third", alice);
        let source = game.new_object_id();
        let mut dm = ChooseOneDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        let filter = ObjectFilter::default().in_zone(Zone::Library);
        let effect = ChooseObjectsEffect::new(
            filter,
            ChoiceCount::dynamic_x(),
            PlayerFilter::You,
            "chosen",
        )
        .with_count_value(crate::effect::Value::Fixed(2))
        .in_zone(Zone::Library)
        .as_optional_search();
        let outcome = run_choose_objects(&effect, &mut game, &mut ctx).expect("search resolves");

        let crate::effect::OutcomeValue::Objects(chosen) = outcome.value else {
            panic!("expected object selection result");
        };
        assert_eq!(chosen, vec![first]);
    }

    #[test]
    fn test_library_search_only_searches_choosers_library() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Put creatures in both libraries
        let alice_card = create_library_card(&mut game, "Alice Creature", alice);
        let _bob_card = create_library_card(&mut game, "Bob Creature", bob);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        // Search library for creature cards (like Buried Alive)
        let filter = ObjectFilter::default().with_type(CardType::Creature);
        let effect = ChooseObjectsEffect::new(filter, 1, PlayerFilter::You, "found")
            .in_zone(Zone::Library)
            .as_search();
        let outcome = run_choose_objects(&effect, &mut game, &mut ctx).expect("choose resolves");

        let crate::effect::OutcomeValue::Objects(chosen) = outcome.value else {
            panic!("expected object selection result");
        };
        // Should only find Alice's creature, not Bob's
        assert_eq!(chosen.len(), 1);
        assert_eq!(
            chosen[0], alice_card,
            "should only search chooser's library"
        );
    }

    #[test]
    fn test_library_search_errors_when_iterated_owner_is_unbound() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let _bob_card = create_library_card(&mut game, "Bob Creature", bob);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let filter = ObjectFilter::default()
            .with_type(CardType::Creature)
            .owned_by(PlayerFilter::IteratedPlayer);
        let effect = ChooseObjectsEffect::new(filter, 1, PlayerFilter::You, "chosen")
            .in_zone(Zone::Library)
            .top_only();
        let err =
            run_choose_objects(&effect, &mut game, &mut ctx).expect_err("missing binding errors");

        assert!(
            matches!(err, ExecutionError::UnresolvableValue(_)),
            "expected unresolvable iterated-player error, got {err:?}"
        );
        assert!(
            format!("{err:?}").contains("IteratedPlayer"),
            "error should mention the missing iterated-player binding, got {err:?}"
        );
    }

    #[test]
    fn test_target_chooser_binds_iterated_owner_for_graveyard_choice() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let _alice_card = create_graveyard_card(&mut game, "Alice Creature", alice);
        let bob_card = create_graveyard_card(&mut game, "Bob Creature", bob);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![crate::effects::ResolvedTarget::Player(bob)]);

        let filter = ObjectFilter::default()
            .with_type(CardType::Creature)
            .owned_by(PlayerFilter::IteratedPlayer);
        let effect = ChooseObjectsEffect::new(filter, 1, PlayerFilter::target_opponent(), "chosen")
            .in_zone(Zone::Graveyard);
        let outcome = run_choose_objects(&effect, &mut game, &mut ctx)
            .expect("target chooser should bind that player's graveyard");

        let crate::effect::OutcomeValue::Objects(chosen) = outcome.value else {
            panic!("expected object selection result");
        };
        assert_eq!(
            chosen,
            vec![bob_card],
            "target opponent should choose only from their own graveyard"
        );
    }

    #[test]
    fn test_multi_zone_search_collects_hand_and_library_candidates() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let hand_card = create_hand_card(&mut game, "Hand Creature", bob);
        let library_card = create_library_card(&mut game, "Library Creature", bob);
        let _alice_card = create_library_card(&mut game, "Alice Creature", alice);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let filter = ObjectFilter::default()
            .with_type(CardType::Creature)
            .owned_by(PlayerFilter::Opponent);
        let effect = ChooseObjectsEffect::new(filter, 2, PlayerFilter::You, "chosen")
            .in_zones(vec![Zone::Hand, Zone::Library])
            .as_search();
        let outcome = run_choose_objects(&effect, &mut game, &mut ctx).expect("choose resolves");

        let crate::effect::OutcomeValue::Objects(chosen) = outcome.value else {
            panic!("expected object selection result");
        };
        assert_eq!(chosen.len(), 2);
        assert!(chosen.contains(&hand_card));
        assert!(chosen.contains(&library_card));
    }

    #[test]
    fn test_choose_objects_accumulates_existing_tagged_objects_by_default() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let first = create_graveyard_card(&mut game, "First", alice);
        let second = create_graveyard_card(&mut game, "Second", alice);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let tag = crate::tag::TagKey::from("chosen");

        let first_effect = ChooseObjectsEffect::new(
            ObjectFilter::default().in_zone(Zone::Graveyard),
            1,
            PlayerFilter::You,
            tag.clone(),
        )
        .in_zone(Zone::Graveyard);
        let first_outcome =
            run_choose_objects(&first_effect, &mut game, &mut ctx).expect("first choose resolves");
        let crate::effect::OutcomeValue::Objects(first_choice) = first_outcome.value else {
            panic!("expected object selection result");
        };
        assert_eq!(first_choice, vec![first]);

        let second_effect = ChooseObjectsEffect::new(
            ObjectFilter::default()
                .in_zone(Zone::Graveyard)
                .not_tagged(tag.clone()),
            1,
            PlayerFilter::You,
            tag.clone(),
        )
        .in_zone(Zone::Graveyard);
        let second_outcome = run_choose_objects(&second_effect, &mut game, &mut ctx)
            .expect("second choose resolves");
        let crate::effect::OutcomeValue::Objects(second_choice) = second_outcome.value else {
            panic!("expected object selection result");
        };
        assert_eq!(second_choice, vec![second]);

        let tagged = ctx
            .tagged_objects
            .get(&tag)
            .expect("tag should remain populated");
        let tagged_ids: Vec<ObjectId> = tagged.iter().map(|snapshot| snapshot.object_id).collect();
        assert_eq!(tagged_ids, vec![first, second]);
    }

    #[test]
    fn test_choose_objects_can_replace_existing_tagged_objects() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let first = create_graveyard_card(&mut game, "First", alice);
        let second = create_graveyard_card(&mut game, "Second", alice);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let tag = crate::tag::TagKey::from("chosen");

        let first_effect = ChooseObjectsEffect::new(
            ObjectFilter::default().in_zone(Zone::Graveyard),
            1,
            PlayerFilter::You,
            tag.clone(),
        )
        .in_zone(Zone::Graveyard)
        .replace_tagged_objects();
        let first_outcome =
            run_choose_objects(&first_effect, &mut game, &mut ctx).expect("first choose resolves");
        let crate::effect::OutcomeValue::Objects(first_choice) = first_outcome.value else {
            panic!("expected object selection result");
        };
        assert_eq!(first_choice, vec![first]);

        let second_effect = ChooseObjectsEffect::new(
            ObjectFilter::default()
                .in_zone(Zone::Graveyard)
                .not_tagged(tag.clone()),
            1,
            PlayerFilter::You,
            tag.clone(),
        )
        .in_zone(Zone::Graveyard)
        .replace_tagged_objects();
        let second_outcome = run_choose_objects(&second_effect, &mut game, &mut ctx)
            .expect("second choose resolves");
        let crate::effect::OutcomeValue::Objects(second_choice) = second_outcome.value else {
            panic!("expected object selection result");
        };
        assert_eq!(second_choice, vec![second]);

        let tagged = ctx
            .tagged_objects
            .get(&tag)
            .expect("tag should remain populated");
        let tagged_ids: Vec<ObjectId> = tagged.iter().map(|snapshot| snapshot.object_id).collect();
        assert_eq!(tagged_ids, vec![second]);
    }

    #[test]
    fn test_choose_objects_replaces_existing_implicit_it_tag() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = create_battlefield_card(&mut game, "Brain Maggot", alice);
        let chosen_card = create_hand_card(&mut game, "Bloodflow Connoisseur", bob);
        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.tag_objects(
            "__it__",
            vec![crate::snapshot::ObjectSnapshot::from_object(
                game.object(source).expect("source should exist"),
                &game,
            )],
        );

        let effect = ChooseObjectsEffect::new(
            ObjectFilter::default()
                .in_zone(Zone::Hand)
                .owned_by(PlayerFilter::Opponent),
            1,
            PlayerFilter::You,
            "__it__",
        )
        .in_zone(Zone::Hand);
        let outcome = run_choose_objects(&effect, &mut game, &mut ctx).expect("choose resolves");

        let crate::effect::OutcomeValue::Objects(choice) = outcome.value else {
            panic!("expected object selection result");
        };
        assert_eq!(choice, vec![chosen_card]);

        let tagged = ctx
            .tagged_objects
            .get(&crate::tag::TagKey::from("__it__"))
            .expect("implicit tag should remain populated");
        let tagged_ids: Vec<ObjectId> = tagged.iter().map(|snapshot| snapshot.object_id).collect();
        assert_eq!(tagged_ids, vec![chosen_card]);
    }

    #[test]
    fn test_choose_objects_does_not_commit_fallback_choice_while_prompt_is_pending() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let first = create_hand_card(&mut game, "First", alice);
        let _second = create_hand_card(&mut game, "Second", alice);
        let mut dm = PromptCapturingDecisionMaker { captured: false };
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);
        ctx.tag_objects(
            "chosen",
            vec![crate::snapshot::ObjectSnapshot::from_object(
                game.object(first).expect("first object should exist"),
                &game,
            )],
        );

        let effect = ChooseObjectsEffect::new(
            ObjectFilter::default().in_zone(Zone::Hand),
            1,
            PlayerFilter::You,
            "chosen",
        )
        .in_zone(Zone::Hand)
        .replace_tagged_objects();

        let outcome = run_choose_objects(&effect, &mut game, &mut ctx).expect("choose resolves");

        assert_eq!(
            outcome.value,
            crate::effect::OutcomeValue::Count(0),
            "prompt discovery should not commit a fallback object choice"
        );
        assert!(
            ctx.get_tagged("chosen").is_none(),
            "stale chosen-object tags must be cleared while waiting for the real selection"
        );
    }

    #[test]
    fn hidden_hand_partial_selection_does_not_restrict_unselected_cards() {
        struct SelectCards(Vec<ObjectId>);
        impl DecisionMaker for SelectCards {
            fn decide_objects(
                &mut self,
                _game: &GameState,
                context: &crate::decisions::context::SelectObjectsContext,
            ) -> Vec<ObjectId> {
                assert!(context.allow_partial_completion);
                self.0.clone()
            }
        }

        // Exercise both an empty answer (the Grazer failure) and a partial
        // answer, from the owner's and a peer's views of the same hand.
        for known in [false, true] {
            for selected_count in [0, 1] {
                let mut game = setup_game();
                let alice = PlayerId::from_index(0);
                let source = game.new_object_id();
                let land = crate::cards::CardDefinitionBuilder::new(
                    CardId::from_raw(120_002),
                    "Hidden Land",
                )
                .card_types(vec![CardType::Land])
                .build();
                let creature = crate::cards::CardDefinitionBuilder::new(
                    CardId::from_raw(120_003),
                    "Wrong Type",
                )
                .card_types(vec![CardType::Creature])
                .build();
                let cards: Vec<_> = (0..2)
                    .map(|slot| {
                        let id = game.create_hidden_card_placeholder(
                            alice,
                            Zone::Hand,
                            slot,
                            format!("ziffle:selection:{slot}"),
                        );
                        if known {
                            game.reveal_hidden_card_with_definition(id, &land).unwrap();
                        }
                        id
                    })
                    .collect();
                let mut dm = SelectCards(cards[..selected_count].to_vec());
                let mut ctx =
                    ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);
                let effect = ChooseObjectsEffect::new(
                    ObjectFilter::default().with_type(CardType::Land),
                    2,
                    PlayerFilter::You,
                    "chosen",
                )
                .in_zone(Zone::Hand);
                let outcome = run_choose_objects(&effect, &mut game, &mut ctx).unwrap();
                assert_eq!(
                    outcome.value,
                    crate::effect::OutcomeValue::Objects(cards[..selected_count].to_vec())
                );
                for &id in &cards[selected_count..] {
                    assert!(!game.has_hidden_identity_obligation(id));
                    assert!(
                        game.hidden_identity_obligation_violation(id, &land)
                            .is_none()
                    );
                    game.reveal_hidden_card_with_definition(id, &land).unwrap();
                }
                for &id in &cards[..selected_count] {
                    assert!(game.has_hidden_identity_obligation(id));
                    assert!(
                        game.hidden_identity_obligation_violation(id, &creature)
                            .is_some()
                    );
                }
            }
        }
    }

    #[test]
    fn test_choose_objects_auto_resolves_single_required_candidate() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let chosen_card = create_graveyard_card(&mut game, "Only Card", alice);
        let mut dm = PromptCapturingDecisionMaker { captured: false };
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        let effect = ChooseObjectsEffect::new(
            ObjectFilter::default().in_zone(Zone::Graveyard),
            1,
            PlayerFilter::You,
            "chosen",
        )
        .in_zone(Zone::Graveyard);

        let outcome = run_choose_objects(&effect, &mut game, &mut ctx).expect("choose resolves");

        assert!(
            !dm.captured,
            "single required candidate should resolve without surfacing a decision"
        );
        let crate::effect::OutcomeValue::Objects(chosen) = outcome.value else {
            panic!("expected object selection result");
        };
        assert_eq!(chosen, vec![chosen_card]);
    }

    #[test]
    fn test_choose_objects_keeps_optional_single_candidate_prompt() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let _chosen_card = create_graveyard_card(&mut game, "Only Card", alice);
        let mut dm = PromptCapturingDecisionMaker { captured: false };
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        let effect = ChooseObjectsEffect::new(
            ObjectFilter::default().in_zone(Zone::Graveyard),
            ChoiceCount::up_to(1),
            PlayerFilter::You,
            "chosen",
        )
        .in_zone(Zone::Graveyard);

        let outcome = run_choose_objects(&effect, &mut game, &mut ctx).expect("choose resolves");

        assert_eq!(
            outcome.value,
            crate::effect::OutcomeValue::Count(0),
            "optional singleton choices should still prompt because the player may decline"
        );
        assert!(
            dm.captured,
            "optional singleton choices should still surface a decision"
        );
    }

    #[test]
    fn test_choose_objects_emits_chosen_objects_fact() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let chosen_card = create_graveyard_card(&mut game, "Chosen Card", alice);
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ChooseObjectsEffect::new(
            ObjectFilter::default().in_zone(Zone::Graveyard),
            1,
            PlayerFilter::You,
            "chosen",
        )
        .in_zone(Zone::Graveyard);

        let outcome = run_choose_objects(&effect, &mut game, &mut ctx).expect("choose resolves");

        assert!(
            outcome
                .execution_facts()
                .contains(&ExecutionFact::ChosenObjects(vec![chosen_card]))
        );
    }

    #[test]
    fn random_source_exiled_choice_selects_only_linked_card_and_returns_it_to_owner_hand() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let library_card = create_library_card(&mut game, "Linked Card", alice);
        let unrelated_exiled = create_exiled_card(&mut game, "Unrelated Exiled Card", alice);
        let mut ctx = ExecutionContext::new_default(source, alice);

        let linked_exiled =
            MoveToZoneEffect::new(ChooseSpec::SpecificObject(library_card), Zone::Exile, false)
                .execute(&mut game, &mut ctx)
                .expect("source should exile the selected library card")
                .affected_objects()
                .and_then(|objects| objects.first().copied())
                .expect("exile move should report the new identity");
        assert_eq!(
            game.get_exiled_with_source_links(source),
            &[linked_exiled],
            "the exile move must retain exact source provenance"
        );

        let mut count = ChoiceCount::exactly(1);
        count.random = true;
        let choose = ChooseObjectsEffect::new(
            ObjectFilter::tagged(crate::tag::SOURCE_EXILED_TAG).in_zone(Zone::Exile),
            count,
            PlayerFilter::You,
            "chosen_linked_card",
        )
        .in_zone(Zone::Exile);
        let outcome = run_choose_objects(&choose, &mut game, &mut ctx)
            .expect("random source-linked choice should resolve");
        assert_eq!(
            outcome.objects(),
            Some([linked_exiled].as_slice()),
            "an unrelated card in exile must not enter the random selection pool"
        );

        let returned = MoveToZoneEffect::new(
            ChooseSpec::Tagged(TagKey::from("chosen_linked_card")),
            Zone::Hand,
            false,
        )
        .execute(&mut game, &mut ctx)
        .expect("the chosen linked card should move to its owner's hand")
        .affected_objects()
        .and_then(|objects| objects.first().copied())
        .expect("hand move should report the new identity");
        assert!(
            game.player(alice)
                .is_some_and(|player| player.hand.contains(&returned))
        );
        assert!(game.exile.contains(&unrelated_exiled));
    }
}
