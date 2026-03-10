//! Runtime orchestration for `ChooseObjectsEffect`.

use crate::decisions::make_decision;
use crate::decisions::specs::ChooseObjectsSpec;
use crate::effect::{ChoiceCount, EffectOutcome, EffectResult};
use crate::effects::helpers::resolve_player_filter;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::filter::ObjectFilter;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;
use crate::zone::Zone;

use super::choose_objects::ChooseObjectsEffect;

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

fn graveyard_candidate_players(
    effect: &ChooseObjectsEffect,
    game: &GameState,
    filter_ctx: &crate::filter::FilterContext,
    chooser_id: PlayerId,
) -> Vec<PlayerId> {
    if let Some(owner_filter) = &effect.filter.owner {
        let owners = game
            .players
            .iter()
            .map(|player| player.id)
            .filter(|player_id| owner_filter.matches_player(*player_id, filter_ctx))
            .collect::<Vec<_>>();
        if !owners.is_empty() {
            return owners;
        }
    }

    if effect.filter.single_graveyard {
        return game.players.iter().map(|player| player.id).collect();
    }

    vec![chooser_id]
}

fn hand_candidate_players(
    effect: &ChooseObjectsEffect,
    game: &GameState,
    filter_ctx: &crate::filter::FilterContext,
    chooser_id: PlayerId,
) -> Vec<PlayerId> {
    if let Some(owner_filter) = &effect.filter.owner {
        let owners = game
            .players
            .iter()
            .map(|player| player.id)
            .filter(|player_id| owner_filter.matches_player(*player_id, filter_ctx))
            .collect::<Vec<_>>();
        if !owners.is_empty() {
            return owners;
        }
    }

    vec![chooser_id]
}

fn library_candidate_players(
    effect: &ChooseObjectsEffect,
    game: &GameState,
    filter_ctx: &crate::filter::FilterContext,
    chooser_id: PlayerId,
) -> Vec<PlayerId> {
    if let Some(owner_filter) = &effect.filter.owner {
        let owners = game
            .players
            .iter()
            .map(|player| player.id)
            .filter(|player_id| owner_filter.matches_player(*player_id, filter_ctx))
            .collect::<Vec<_>>();
        if !owners.is_empty() {
            return owners;
        }
    }
    vec![chooser_id]
}

fn effective_search_zones(
    effect: &ChooseObjectsEffect,
    game: &GameState,
    chooser_id: PlayerId,
) -> Vec<Zone> {
    let mut zones = effect.search_zones();
    if effect.is_search && zones.contains(&Zone::Library) && !game.can_search_library(chooser_id) {
        zones.retain(|zone| *zone != Zone::Library);
    }
    zones
}

fn collect_candidates_in_zone(
    effect: &ChooseObjectsEffect,
    game: &GameState,
    ctx: &ExecutionContext,
    chooser_id: PlayerId,
    search_zone: Zone,
) -> Result<Vec<ObjectId>, ExecutionError> {
    let filter_ctx = ctx.filter_context(game);
    let top_only_limit = effect.top_only_selection_limit(ctx.x_value);

    let candidates = match search_zone {
        Zone::Battlefield => game
            .battlefield
            .iter()
            .filter_map(|&id| game.object(id).map(|obj| (id, obj)))
            .filter(|(_, obj)| effect.filter.matches(obj, &filter_ctx, game))
            .map(|(id, _)| id)
            .collect(),
        Zone::Hand => hand_candidate_players(effect, game, &filter_ctx, chooser_id)
            .iter()
            .filter_map(|owner_id| game.player(*owner_id))
            .flat_map(|player| player.hand.iter())
            .filter_map(|&id| game.object(id).map(|obj| (id, obj)))
            .filter(|(_, obj)| effect.filter.matches(obj, &filter_ctx, game))
            .map(|(id, _)| id)
            .collect(),
        Zone::Graveyard => {
            let owner_ids = graveyard_candidate_players(effect, game, &filter_ctx, chooser_id);

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
                        if !effect.filter.matches(obj, &filter_ctx, game) {
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
                    .filter(|(_, obj)| effect.filter.matches(obj, &filter_ctx, game))
                    .map(|(id, _)| id)
                    .collect()
            }
        }
        Zone::Library => {
            let owner_ids = library_candidate_players(effect, game, &filter_ctx, chooser_id);
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
                        if !effect.filter.matches(obj, &filter_ctx, game) {
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
                    .filter(|(_, obj)| effect.filter.matches(obj, &filter_ctx, game))
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
    for zone in effective_search_zones(effect, game, chooser_id) {
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

fn normalize_chosen_objects(
    mut chosen: Vec<ObjectId>,
    candidates: &[ObjectId],
    min: usize,
    max: usize,
) -> Vec<ObjectId> {
    chosen.truncate(max);
    chosen.sort();
    chosen.dedup();

    if chosen.len() < min {
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

fn enforce_single_graveyard_choice_constraint(
    effect: &ChooseObjectsEffect,
    game: &GameState,
    candidates: &[ObjectId],
    mut chosen: Vec<ObjectId>,
    min: usize,
    max: usize,
) -> Vec<ObjectId> {
    let search_zone = effect.filter.zone.unwrap_or(effect.zone);
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

    if effect.is_search
        && effect.search_zones() == vec![Zone::Library]
        && !game.can_search_library(chooser_id)
    {
        return Ok(EffectOutcome::from_result(EffectResult::Prevented));
    }
    if effect.is_search && effect.search_zones().contains(&Zone::Library) {
        game.library_searches_this_turn.insert(chooser_id);
    }

    let candidates = collect_candidates(effect, game, ctx, chooser_id)?;
    if candidates.is_empty() {
        return Ok(EffectOutcome::count(0));
    }

    let (min, max) = if effect.count.dynamic_x {
        let x = ctx
            .x_value
            .ok_or_else(|| ExecutionError::UnresolvableValue("X value not set".to_string()))?
            as usize;

        if effect.count.up_to_x {
            (0, x.min(candidates.len()))
        } else if x > candidates.len() {
            return Err(ExecutionError::Impossible(format!(
                "Not enough candidates to choose X objects (X={}, {} available)",
                x,
                candidates.len()
            )));
        } else {
            (x, x)
        }
    } else {
        compute_choice_bounds(effect.count, candidates.len())
    };
    if max == 0 {
        return Ok(EffectOutcome::count(0));
    }

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
    let spec = ChooseObjectsSpec::new(ctx.source, description, candidates.clone(), min, Some(max));
    let chosen: Vec<ObjectId> =
        make_decision(game, ctx.decision_maker, chooser_id, Some(ctx.source), spec);
    if ctx.decision_maker.awaiting_choice() {
        ctx.clear_object_tag(effect.tag.as_str());
        return Ok(EffectOutcome::count(0));
    }
    let chosen = normalize_chosen_objects(chosen, &candidates, min, max);
    let chosen =
        enforce_single_graveyard_choice_constraint(effect, game, &candidates, chosen, min, max);

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

    Ok(EffectOutcome::from_result(EffectResult::Objects(chosen)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::DecisionMaker;
    use crate::effect::EffectResult;
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

        let normalized = normalize_chosen_objects(chosen, &candidates, 2, 2);
        assert_eq!(
            normalized,
            vec![ObjectId::from_raw(3), ObjectId::from_raw(1)]
        );
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

        let EffectResult::Objects(chosen) = outcome.result else {
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

        let EffectResult::Objects(chosen) = outcome.result else {
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

        let EffectResult::Objects(chosen) = outcome.result else {
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

        let EffectResult::Objects(chosen) = outcome.result else {
            panic!("expected object selection result");
        };
        assert_eq!(chosen.len(), 2);
        assert!(chosen.contains(&card_a));
        assert!(chosen.contains(&card_b));
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

        let EffectResult::Objects(chosen) = outcome.result else {
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

        let EffectResult::Objects(chosen) = outcome.result else {
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
        let EffectResult::Objects(first_choice) = first_outcome.result else {
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
        let EffectResult::Objects(second_choice) = second_outcome.result else {
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
        let EffectResult::Objects(first_choice) = first_outcome.result else {
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
        let EffectResult::Objects(second_choice) = second_outcome.result else {
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
            outcome.result,
            EffectResult::Count(0),
            "prompt discovery should not commit a fallback object choice"
        );
        assert!(
            ctx.get_tagged("chosen").is_none(),
            "stale chosen-object tags must be cleared while waiting for the real selection"
        );
    }
}
