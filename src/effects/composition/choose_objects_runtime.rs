//! Runtime orchestration for `ChooseObjectsEffect`.

use crate::decisions::make_decision;
use crate::decisions::specs::ChooseObjectsSpec;
use crate::effect::{ChoiceCount, EffectOutcome, ExecutionFact, SearchSelectionMode};
use crate::effects::helpers::{
    resolve_player_filter, resolve_player_filter_to_list, resolve_value,
};
use crate::events::SearchLibraryEvent;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::filter::{ObjectFilter, PlayerFilter};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;
use crate::triggers::TriggerEvent;
use crate::zone::Zone;

use super::choose_objects::ChooseObjectsEffect;

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
            .entered_battlefield_controller
            .as_ref()
            .is_some_and(PlayerFilter::mentions_iterated_player)
        || filter
            .targets_object
            .as_deref()
            .is_some_and(object_filter_mentions_iterated_player)
        || filter
            .targets_only_object
            .as_deref()
            .is_some_and(object_filter_mentions_iterated_player)
        || filter
            .any_of
            .iter()
            .any(object_filter_mentions_iterated_player)
}

fn value_mentions_iterated_player(value: &crate::effect::Value) -> bool {
    match value {
        crate::effect::Value::Add(left, right) => {
            value_mentions_iterated_player(left) || value_mentions_iterated_player(right)
        }
        crate::effect::Value::Scaled(inner, _) | crate::effect::Value::HalfRoundedDown(inner) => {
            value_mentions_iterated_player(inner)
        }
        crate::effect::Value::Count(filter)
        | crate::effect::Value::CountScaled(filter, _)
        | crate::effect::Value::TotalPower(filter)
        | crate::effect::Value::TotalToughness(filter)
        | crate::effect::Value::TotalManaValue(filter)
        | crate::effect::Value::GreatestPower(filter)
        | crate::effect::Value::GreatestToughness(filter)
        | crate::effect::Value::GreatestManaValue(filter)
        | crate::effect::Value::BasicLandTypesAmong(filter)
        | crate::effect::Value::ColorsAmong(filter)
        | crate::effect::Value::DistinctNames(filter) => {
            object_filter_mentions_iterated_player(filter)
        }
        crate::effect::Value::CreaturesDiedThisTurnControlledBy(player)
        | crate::effect::Value::CountPlayers(player)
        | crate::effect::Value::PartySize(player)
        | crate::effect::Value::LifeTotal(player)
        | crate::effect::Value::HalfLifeTotalRoundedUp(player)
        | crate::effect::Value::HalfLifeTotalRoundedDown(player)
        | crate::effect::Value::HalfStartingLifeTotalRoundedUp(player)
        | crate::effect::Value::HalfStartingLifeTotalRoundedDown(player)
        | crate::effect::Value::CardsInHand(player)
        | crate::effect::Value::CardsInLibrary(player)
        | crate::effect::Value::DevotionToChosenColor(player)
        | crate::effect::Value::LifeGainedThisTurn(player)
        | crate::effect::Value::LifeLostThisTurn(player)
        | crate::effect::Value::NoncombatDamageDealtToPlayersThisTurn(player)
        | crate::effect::Value::MaxCardsDrawnThisTurn(player)
        | crate::effect::Value::MaxCardsInHand(player)
        | crate::effect::Value::CardsInGraveyard(player)
        | crate::effect::Value::SpellsCastThisTurn(player)
        | crate::effect::Value::SpellsCastBeforeThisTurn(player)
        | crate::effect::Value::CardTypesInGraveyard(player) => player.mentions_iterated_player(),
        crate::effect::Value::Devotion { player, .. } => player.mentions_iterated_player(),
        crate::effect::Value::SpellsCastThisTurnMatching { player, filter, .. } => {
            player.mentions_iterated_player() || object_filter_mentions_iterated_player(filter)
        }
        crate::effect::Value::CommanderCastCount(player) => player.mentions_iterated_player(),
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

fn should_auto_choose_single_candidate(candidates: &[ObjectId], min: usize, max: usize) -> bool {
    candidates.len() == 1 && min == 1 && max == 1
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
    Ok(vec![chooser_id])
}

fn effective_search_zones(
    effect: &ChooseObjectsEffect,
    game: &GameState,
    chooser_id: PlayerId,
) -> Result<Vec<Zone>, ExecutionError> {
    let mut zones = effect.search_zones()?;
    if effect.is_search && zones.contains(&Zone::Library) && !game.can_search_library(chooser_id) {
        zones.retain(|zone| *zone != Zone::Library);
    }
    Ok(zones)
}

fn collect_candidates_in_zone(
    effect: &ChooseObjectsEffect,
    game: &GameState,
    ctx: &ExecutionContext,
    chooser_id: PlayerId,
    search_zone: Zone,
) -> Result<Vec<ObjectId>, ExecutionError> {
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
    let top_only_limit = effect.top_only_selection_limit(ctx.x_value);
    let mut hidden_zone_filter = effect.filter.clone();
    if matches!(search_zone, Zone::Hand | Zone::Graveyard | Zone::Library) {
        hidden_zone_filter.owner = None;
    }

    let candidates = match search_zone {
        Zone::Battlefield => game
            .battlefield
            .iter()
            .filter_map(|&id| game.object(id).map(|obj| (id, obj)))
            .filter(|(_, obj)| effect.filter.matches(obj, &filter_ctx, game))
            .map(|(id, _)| id)
            .collect(),
        Zone::Hand => hand_candidate_players(effect, game, ctx, &filter_ctx, chooser_id)?
            .iter()
            .filter_map(|owner_id| game.player(*owner_id))
            .flat_map(|player| player.hand.iter())
            .filter_map(|&id| game.object(id).map(|obj| (id, obj)))
            .filter(|(_, obj)| hidden_zone_filter.matches(obj, &filter_ctx, game))
            .map(|(id, _)| id)
            .collect(),
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
            let owner_ids = library_candidate_players(effect, game, ctx, &filter_ctx, chooser_id)?;
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
                    .filter_map(|&id| game.object(id).map(|obj| (id, obj)))
                    .filter(|(_, obj)| hidden_zone_filter.matches(obj, &filter_ctx, game))
                    .map(|(id, _)| id)
                    .collect()
            }
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
    let mut candidates = Vec::new();
    for zone in effective_search_zones(effect, game, chooser_id)? {
        for id in collect_candidates_in_zone(effect, game, ctx, chooser_id, zone)? {
            if !candidates.contains(&id) {
                candidates.push(id);
            }
        }
    }
    Ok(candidates)
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
) -> Vec<ObjectId> {
    chosen.truncate(max);
    chosen.sort();
    chosen.dedup();

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

fn public_search_candidates(game: &GameState, candidates: &[ObjectId]) -> Vec<ObjectId> {
    candidates
        .iter()
        .copied()
        .filter(|id| game.object(*id).is_some_and(|obj| !obj.zone.is_hidden()))
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

pub(crate) fn run_choose_objects(
    effect: &ChooseObjectsEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Result<EffectOutcome, ExecutionError> {
    let chooser_id = resolve_player_filter(game, &effect.chooser, ctx)?;

    let search_zones = effect.search_zones()?;

    if effect.is_search
        && search_zones == vec![Zone::Library]
        && !game.can_search_library(chooser_id)
    {
        return Ok(EffectOutcome::prevented());
    }
    let search_event = (effect.is_search && search_zones.contains(&Zone::Library)).then(|| {
        TriggerEvent::new_with_provenance(SearchLibraryEvent::new(chooser_id, None), ctx.provenance)
    });

    let candidates = collect_candidates(effect, game, ctx, chooser_id)?;
    if candidates.is_empty() {
        let outcome = EffectOutcome::count(0);
        return Ok(if let Some(search_event) = search_event.clone() {
            outcome.with_event(search_event)
        } else {
            outcome
        });
    }

    let (base_min, max) = if effect.count.dynamic_x || effect.count_value.is_some() {
        let x = if let Some(count_value) = effect.count_value.as_ref() {
            let previous_iterated_player = ctx.iterated_player;
            if previous_iterated_player.is_none()
                && matches!(effect.chooser, PlayerFilter::Target(_))
                && value_mentions_iterated_player(count_value)
            {
                ctx.iterated_player = Some(chooser_id);
            }
            let resolved = resolve_value(game, count_value, ctx);
            ctx.iterated_player = previous_iterated_player;
            resolved?.max(0) as usize
        } else {
            ctx.x_value
                .ok_or_else(|| ExecutionError::UnresolvableValue("X value not set".to_string()))?
                as usize
        };

        let optional_dynamic_choice = effect.count.up_to_x
            || (effect.is_search && effect.search_mode == SearchSelectionMode::Optional);
        if optional_dynamic_choice {
            (0, x.min(candidates.len()))
        } else if x > candidates.len() && !effect.is_search {
            return Err(ExecutionError::Impossible(format!(
                "Not enough candidates to choose dynamic-count objects ({x}, {} available)",
                candidates.len()
            )));
        } else {
            let bounded = x.min(candidates.len());
            (bounded, bounded)
        }
    } else {
        compute_choice_bounds(effect.count, candidates.len())
    };
    if max == 0 {
        let outcome = EffectOutcome::count(0);
        return Ok(if let Some(search_event) = search_event.clone() {
            outcome.with_event(search_event)
        } else {
            outcome
        });
    }

    let has_hidden_search_zones = effect.is_search && search_zones.iter().any(Zone::is_hidden);
    let has_search_stated_quality = effect.filter.has_search_stated_quality();
    let search_required_count = compute_search_required_count(effect.search_mode, max);
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

    let description = if effect.description == "Choose" {
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
        effect.description.to_string()
    };
    let chosen: Vec<ObjectId> =
        if !effect.is_search && should_auto_choose_single_candidate(&candidates, min, max) {
            candidates.clone()
        } else {
            let mut spec =
                ChooseObjectsSpec::new(ctx.source, description, candidates.clone(), min, Some(max));
            if allow_hidden_partial {
                spec = spec.allow_partial_completion();
            }
            make_decision(game, ctx.decision_maker, chooser_id, Some(ctx.source), spec)
        };
    if ctx.decision_maker.awaiting_choice() {
        ctx.clear_object_tag(effect.tag.as_str());
        let outcome = EffectOutcome::count(0);
        return Ok(if let Some(search_event) = search_event {
            outcome.with_event(search_event)
        } else {
            outcome
        });
    }
    let chosen = normalize_chosen_objects(chosen, &candidates, min, max, !allow_hidden_partial);
    let chosen = enforce_public_search_choice_constraint(
        game,
        &candidates,
        chosen,
        required_public_count,
        max,
    );
    let chosen =
        enforce_single_graveyard_choice_constraint(effect, game, &candidates, chosen, min, max);
    if search_zones.iter().any(Zone::is_hidden) {
        ctx.remember_face_down_exile_viewers(&chosen, chooser_id);
    }

    let snapshots = snapshot_chosen_objects(game, &chosen);
    if !snapshots.is_empty() {
        if effect.replace_tagged_objects {
            ctx.set_tagged_objects(effect.tag.clone(), snapshots);
        } else {
            ctx.tag_objects(effect.tag.clone(), snapshots);
        }
    } else {
        ctx.clear_object_tag(effect.tag.as_str());
    }

    let outcome = EffectOutcome::with_objects(chosen.clone())
        .with_execution_fact(ExecutionFact::ChosenObjects(chosen));
    Ok(if let Some(search_event) = search_event {
        outcome.with_event(search_event)
    } else {
        outcome
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::DecisionMaker;
    use crate::effect::ExecutionFact;
    use crate::executor::ExecutionContext;
    use crate::filter::ObjectFilter;
    use crate::ids::{CardId, PlayerId};
    use crate::target::PlayerFilter;
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

    fn create_hand_card(game: &mut GameState, name: &str, owner: PlayerId) -> ObjectId {
        let card = CardBuilder::new(CardId::from_raw(game.new_object_id().0 as u32), name)
            .card_types(vec![CardType::Creature])
            .build();
        game.create_object_from_card(&card, owner, Zone::Hand)
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

        let normalized = normalize_chosen_objects(chosen, &candidates, 2, 2, true);
        assert_eq!(
            normalized,
            vec![ObjectId::from_raw(3), ObjectId::from_raw(1)]
        );
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
        let effect = ChooseObjectsEffect::new(
            filter,
            ChoiceCount::dynamic_x(),
            PlayerFilter::You,
            "chosen",
        )
        .in_zone(Zone::Graveyard);

        let err = run_choose_objects(&effect, &mut game, &mut ctx).expect_err("missing X errors");
        assert!(
            matches!(err, ExecutionError::UnresolvableValue(_)),
            "expected X resolution error, got {err:?}"
        );
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
}
