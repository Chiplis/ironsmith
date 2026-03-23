use crate::decisions::context::{OrderContext, ViewCardsContext};
use crate::effect::EffectOutcome;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId, StableId};
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;
use crate::zone::Zone;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryConsultMode {
    Reveal,
    Exile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryBottomOrder {
    Random,
    ChooserChooses,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LibraryConsultStopRule {
    FirstMatch,
    MatchCount(u32),
}

impl LibraryConsultStopRule {
    pub fn required_matches(&self) -> u32 {
        match self {
            Self::FirstMatch => 1,
            Self::MatchCount(count) => *count,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct LibraryConsultResult {
    pub exposed_snapshots: Vec<ObjectSnapshot>,
    pub matched_snapshots: Vec<ObjectSnapshot>,
    pub exposed_object_ids: Vec<ObjectId>,
}

pub fn execute_library_consult(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    player: PlayerId,
    mode: LibraryConsultMode,
    stop_rule: LibraryConsultStopRule,
    all_tag: Option<&TagKey>,
    match_tag: Option<&TagKey>,
    mut is_match: impl FnMut(&crate::object::Object, &GameState) -> bool,
) -> Result<LibraryConsultResult, ExecutionError> {
    if let Some(tag) = all_tag {
        ctx.clear_object_tag(tag.as_str());
    }
    if let Some(tag) = match_tag {
        ctx.clear_object_tag(tag.as_str());
    }

    let required_matches = stop_rule.required_matches() as usize;
    if required_matches == 0 {
        return Ok(LibraryConsultResult::default());
    }

    let mut result = LibraryConsultResult::default();

    match mode {
        LibraryConsultMode::Reveal => {
            let top_to_bottom: Vec<_> = game
                .player(player)
                .map(|library_owner| library_owner.library.iter().rev().copied().collect())
                .unwrap_or_default();

            for object_id in top_to_bottom {
                let Some(object) = game.object(object_id) else {
                    continue;
                };
                let snapshot = ObjectSnapshot::from_object(object, game);
                let matched = is_match(object, game);

                result.exposed_object_ids.push(object_id);
                result.exposed_snapshots.push(snapshot.clone());
                if matched {
                    result.matched_snapshots.push(snapshot);
                    if result.matched_snapshots.len() >= required_matches {
                        break;
                    }
                }
            }

            reveal_consulted_cards(game, ctx, player, &result.exposed_object_ids);
        }
        LibraryConsultMode::Exile => loop {
            let Some(top_card_id) = game
                .player(player)
                .and_then(|library_owner| library_owner.library.last().copied())
            else {
                break;
            };

            let Some((exiled_id, final_zone)) = game.move_object_with_commander_options(
                top_card_id,
                Zone::Exile,
                ctx.cause.clone(),
                &mut *ctx.decision_maker,
            ) else {
                break;
            };
            if final_zone != Zone::Exile {
                continue;
            }

            let Some(object) = game.object(exiled_id) else {
                continue;
            };
            let snapshot = ObjectSnapshot::from_object(object, game);
            let matched = is_match(object, game);

            result.exposed_object_ids.push(exiled_id);
            result.exposed_snapshots.push(snapshot.clone());
            if matched {
                result.matched_snapshots.push(snapshot);
                if result.matched_snapshots.len() >= required_matches {
                    break;
                }
            }
        },
    }

    if let Some(tag) = all_tag
        && !result.exposed_snapshots.is_empty()
    {
        ctx.set_tagged_objects(tag.clone(), result.exposed_snapshots.clone());
    }
    if let Some(tag) = match_tag
        && !result.matched_snapshots.is_empty()
    {
        ctx.set_tagged_objects(tag.clone(), result.matched_snapshots.clone());
    }

    Ok(result)
}

pub fn move_tagged_remainder_to_library_bottom(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    tag: &TagKey,
    keep_tagged: Option<&TagKey>,
    order: LibraryBottomOrder,
    chooser: PlayerId,
) -> Result<EffectOutcome, ExecutionError> {
    let Some(tagged) = ctx.get_tagged_all(tag.as_str()).cloned() else {
        return Ok(EffectOutcome::resolved());
    };

    let keep_stable_ids = keep_tagged
        .and_then(|keep| ctx.get_tagged_all(keep.as_str()).cloned())
        .unwrap_or_default()
        .into_iter()
        .map(|snapshot| snapshot.stable_id)
        .collect::<HashSet<_>>();

    let mut owner_order = Vec::new();
    let mut by_owner: HashMap<PlayerId, Vec<BottomCandidate>> = HashMap::new();
    for snapshot in tagged {
        if keep_stable_ids.contains(&snapshot.stable_id) {
            continue;
        }

        let Some(candidate) = BottomCandidate::from_snapshot(game, snapshot) else {
            continue;
        };
        if !by_owner.contains_key(&candidate.owner) {
            owner_order.push(candidate.owner);
        }
        by_owner.entry(candidate.owner).or_default().push(candidate);
    }

    let mut moved_ids = Vec::new();
    for owner in owner_order {
        let Some(candidates) = by_owner.remove(&owner) else {
            continue;
        };
        if candidates.is_empty() {
            continue;
        }

        let ordered = order_bottom_candidates(game, ctx, chooser, &candidates, order);
        let ordered = normalize_candidate_order(ordered, &candidates);

        let mut stable_to_current = HashMap::<StableId, ObjectId>::new();
        for candidate in &ordered {
            if candidate.zone == Zone::Library {
                stable_to_current.insert(candidate.stable_id, candidate.object_id);
            }
        }

        for candidate in &ordered {
            if candidate.zone != Zone::Exile {
                continue;
            }

            let Some((new_id, final_zone)) = game.move_object_with_commander_options(
                candidate.object_id,
                Zone::Library,
                ctx.cause.clone(),
                &mut *ctx.decision_maker,
            ) else {
                continue;
            };
            if final_zone == Zone::Library {
                stable_to_current.insert(candidate.stable_id, new_id);
            }
        }

        let ordered_current_ids = ordered
            .iter()
            .filter_map(|candidate| stable_to_current.get(&candidate.stable_id).copied())
            .collect::<Vec<_>>();
        if ordered_current_ids.is_empty() {
            continue;
        }

        let bottom_ids = ordered_current_ids.iter().copied().collect::<HashSet<_>>();
        if let Some(player) = game.player_mut(owner) {
            player.library.retain(|id| !bottom_ids.contains(id));
            player.library.splice(0..0, ordered_current_ids.clone());
        }
        moved_ids.extend(ordered_current_ids);
    }

    if moved_ids.is_empty() {
        Ok(EffectOutcome::resolved())
    } else {
        Ok(EffectOutcome::with_objects(moved_ids))
    }
}

fn reveal_consulted_cards(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    subject: PlayerId,
    card_ids: &[ObjectId],
) {
    if card_ids.is_empty() {
        return;
    }

    for viewer_idx in 0..game.players.len() {
        let viewer = PlayerId::from_index(viewer_idx as u8);
        let view_ctx = ViewCardsContext::new(
            viewer,
            subject,
            Some(ctx.source),
            Zone::Library,
            "Reveal consulted cards",
        )
        .with_public(true);
        ctx.decision_maker
            .view_cards(game, viewer, card_ids, &view_ctx);
    }
}

#[derive(Debug, Clone)]
struct BottomCandidate {
    stable_id: StableId,
    object_id: ObjectId,
    owner: PlayerId,
    zone: Zone,
    name: String,
}

impl BottomCandidate {
    fn from_snapshot(game: &GameState, snapshot: ObjectSnapshot) -> Option<Self> {
        let current_id = if game.object(snapshot.object_id).is_some() {
            snapshot.object_id
        } else {
            game.find_object_by_stable_id(snapshot.stable_id)?
        };
        let object = game.object(current_id)?;
        if object.zone != Zone::Library && object.zone != Zone::Exile {
            return None;
        }

        Some(Self {
            stable_id: snapshot.stable_id,
            object_id: current_id,
            owner: object.owner,
            zone: object.zone,
            name: object.name.clone(),
        })
    }
}

fn order_bottom_candidates(
    game: &GameState,
    ctx: &mut ExecutionContext,
    chooser: PlayerId,
    candidates: &[BottomCandidate],
    order: LibraryBottomOrder,
) -> Vec<ObjectId> {
    match order {
        LibraryBottomOrder::Random => {
            let mut ordered = candidates.iter().map(|candidate| candidate.object_id).collect::<Vec<_>>();
            game.shuffle_slice(&mut ordered);
            ordered
        }
        LibraryBottomOrder::ChooserChooses => {
            if candidates.len() <= 1 {
                return candidates
                    .iter()
                    .map(|candidate| candidate.object_id)
                    .collect::<Vec<_>>();
            }

            let context = OrderContext::new(
                chooser,
                Some(ctx.source),
                "Order the selected cards for the bottom of your library. The first option becomes the bottom-most card.",
                candidates
                    .iter()
                    .map(|candidate| (candidate.object_id, candidate.name.clone()))
                    .collect::<Vec<_>>(),
            );
            ctx.decision_maker.decide_order(game, &context)
        }
    }
}

fn normalize_candidate_order(response: Vec<ObjectId>, original: &[BottomCandidate]) -> Vec<BottomCandidate> {
    let mut remaining = original.to_vec();
    let mut ordered = Vec::with_capacity(original.len());

    for object_id in response {
        if let Some(position) = remaining
            .iter()
            .position(|candidate| candidate.object_id == object_id)
        {
            ordered.push(remaining.remove(position));
        }
    }

    ordered.extend(remaining);
    ordered
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::{AutoPassDecisionMaker, DecisionMaker};
    use crate::decisions::context::OrderContext;
    use crate::executor::ExecutionContext;
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::types::CardType;

    fn make_library_card(name: &str, card_types: Vec<CardType>) -> crate::card::Card {
        CardBuilder::new(CardId::new(), name)
            .card_types(card_types)
            .build()
    }

    fn library_names(game: &GameState, player: PlayerId) -> Vec<String> {
        game.player(player)
            .expect("player exists")
            .library
            .iter()
            .map(|id| game.object(*id).expect("library object exists").name.clone())
            .collect()
    }

    fn snapshot_ids(ctx: &ExecutionContext, tag: &str) -> Vec<ObjectId> {
        ctx.get_tagged_all(tag)
            .expect("tag should exist")
            .iter()
            .map(|snapshot| snapshot.object_id)
            .collect()
    }

    fn names_for_ids(game: &GameState, ids: &[ObjectId]) -> Vec<String> {
        ids.iter()
            .map(|id| game.object(*id).expect("object exists").name.clone())
            .collect()
    }

    struct ReverseOrderDecisionMaker;

    impl DecisionMaker for ReverseOrderDecisionMaker {
        fn decide_order(&mut self, _game: &GameState, ctx: &OrderContext) -> Vec<ObjectId> {
            let mut ids = ctx.items.iter().map(|(id, _)| *id).collect::<Vec<_>>();
            ids.reverse();
            ids
        }
    }

    #[test]
    fn reveal_consult_tags_exposed_and_matched_without_moving_cards() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bottom = game.create_object_from_card(
            &make_library_card("Bottom Land", vec![CardType::Land]),
            alice,
            Zone::Library,
        );
        let match_id = game.create_object_from_card(
            &make_library_card("Match Artifact", vec![CardType::Artifact]),
            alice,
            Zone::Library,
        );
        let top = game.create_object_from_card(
            &make_library_card("Top Creature", vec![CardType::Creature]),
            alice,
            Zone::Library,
        );
        let before = game.player(alice).expect("alice exists").library.clone();

        let ctx = ExecutionContext::new_default(ObjectId::from_raw(999), alice);
        let mut dm = AutoPassDecisionMaker;
        let mut ctx = ctx.with_decision_maker(&mut dm);

        let result = execute_library_consult(
            &mut game,
            &mut ctx,
            alice,
            LibraryConsultMode::Reveal,
            LibraryConsultStopRule::FirstMatch,
            Some(&TagKey::from("all")),
            Some(&TagKey::from("match")),
            |object, _| object.card_types.contains(&CardType::Artifact),
        )
        .expect("reveal consult should execute");

        assert_eq!(result.exposed_object_ids, vec![top, match_id]);
        assert_eq!(snapshot_ids(&ctx, "all"), vec![top, match_id]);
        assert_eq!(snapshot_ids(&ctx, "match"), vec![match_id]);
        assert_eq!(game.player(alice).expect("alice exists").library, before);
        assert_eq!(
            game.object(bottom).expect("bottom card exists").zone,
            Zone::Library
        );
    }

    #[test]
    fn exile_consult_match_count_stops_on_second_match_and_tags_all_exiled_cards() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        game.create_object_from_card(
            &make_library_card("Bottom Creature", vec![CardType::Creature]),
            alice,
            Zone::Library,
        );
        game.create_object_from_card(
            &make_library_card("Second Match", vec![CardType::Artifact]),
            alice,
            Zone::Library,
        );
        game.create_object_from_card(
            &make_library_card("Middle Land", vec![CardType::Land]),
            alice,
            Zone::Library,
        );
        game.create_object_from_card(
            &make_library_card("First Match", vec![CardType::Artifact]),
            alice,
            Zone::Library,
        );
        game.create_object_from_card(
            &make_library_card("Top Instant", vec![CardType::Instant]),
            alice,
            Zone::Library,
        );

        let ctx = ExecutionContext::new_default(ObjectId::from_raw(1000), alice);
        let mut dm = AutoPassDecisionMaker;
        let mut ctx = ctx.with_decision_maker(&mut dm);

        let result = execute_library_consult(
            &mut game,
            &mut ctx,
            alice,
            LibraryConsultMode::Exile,
            LibraryConsultStopRule::MatchCount(2),
            Some(&TagKey::from("all")),
            Some(&TagKey::from("match")),
            |object, _| object.card_types.contains(&CardType::Artifact),
        )
        .expect("exile consult should execute");

        assert_eq!(
            names_for_ids(&game, &result.exposed_object_ids),
            vec![
                "Top Instant".to_string(),
                "First Match".to_string(),
                "Middle Land".to_string(),
                "Second Match".to_string(),
            ]
        );
        assert_eq!(snapshot_ids(&ctx, "all"), result.exposed_object_ids);
        assert_eq!(
            names_for_ids(&game, &snapshot_ids(&ctx, "match")),
            vec!["First Match".to_string(), "Second Match".to_string()]
        );
        for object_id in result.exposed_object_ids {
            assert_eq!(
                game.object(object_id).expect("exposed card exists").zone,
                Zone::Exile
            );
        }
        assert_eq!(
            library_names(&game, alice),
            vec!["Bottom Creature".to_string()]
        );
    }

    #[test]
    fn chooser_order_bottoming_reorders_library_remainder_bottom_most_first() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        game.create_object_from_card(
            &make_library_card("Existing Bottom", vec![CardType::Land]),
            alice,
            Zone::Library,
        );
        let first = game.create_object_from_card(
            &make_library_card("First Candidate", vec![CardType::Artifact]),
            alice,
            Zone::Library,
        );
        let second = game.create_object_from_card(
            &make_library_card("Second Candidate", vec![CardType::Creature]),
            alice,
            Zone::Library,
        );
        let kept = game.create_object_from_card(
            &make_library_card("Kept Top", vec![CardType::Instant]),
            alice,
            Zone::Library,
        );

        let ctx = ExecutionContext::new_default(ObjectId::from_raw(1001), alice);
        let mut dm = ReverseOrderDecisionMaker;
        let mut ctx = ctx.with_decision_maker(&mut dm);

        ctx.set_tagged_objects(
            "all",
            vec![
                ObjectSnapshot::from_object(game.object(first).expect("first exists"), &game),
                ObjectSnapshot::from_object(game.object(second).expect("second exists"), &game),
                ObjectSnapshot::from_object(game.object(kept).expect("kept exists"), &game),
            ],
        );
        ctx.set_tagged_objects(
            "keep",
            vec![ObjectSnapshot::from_object(
                game.object(kept).expect("kept exists"),
                &game,
            )],
        );

        move_tagged_remainder_to_library_bottom(
            &mut game,
            &mut ctx,
            &TagKey::from("all"),
            Some(&TagKey::from("keep")),
            LibraryBottomOrder::ChooserChooses,
            alice,
        )
        .expect("bottoming remainder should execute");

        assert_eq!(
            library_names(&game, alice),
            vec![
                "Second Candidate".to_string(),
                "First Candidate".to_string(),
                "Existing Bottom".to_string(),
                "Kept Top".to_string(),
            ]
        );
    }

    #[test]
    fn random_bottoming_returns_exiled_remainder_to_library_bottom() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        game.create_object_from_card(
            &make_library_card("Existing Bottom", vec![CardType::Land]),
            alice,
            Zone::Library,
        );
        game.create_object_from_card(
            &make_library_card("Existing Top", vec![CardType::Creature]),
            alice,
            Zone::Library,
        );
        let exile_one = game.create_object_from_card(
            &make_library_card("Exile One", vec![CardType::Artifact]),
            alice,
            Zone::Exile,
        );
        let exile_two = game.create_object_from_card(
            &make_library_card("Exile Two", vec![CardType::Instant]),
            alice,
            Zone::Exile,
        );

        let ctx = ExecutionContext::new_default(ObjectId::from_raw(1002), alice);
        let mut dm = AutoPassDecisionMaker;
        let mut ctx = ctx.with_decision_maker(&mut dm);

        ctx.set_tagged_objects(
            "all",
            vec![
                ObjectSnapshot::from_object(game.object(exile_one).expect("exile one exists"), &game),
                ObjectSnapshot::from_object(game.object(exile_two).expect("exile two exists"), &game),
            ],
        );

        move_tagged_remainder_to_library_bottom(
            &mut game,
            &mut ctx,
            &TagKey::from("all"),
            None,
            LibraryBottomOrder::Random,
            alice,
        )
        .expect("random bottoming should execute");

        let library = library_names(&game, alice);
        let bottom_two = library[..2].iter().cloned().collect::<HashSet<_>>();
        assert_eq!(
            bottom_two,
            HashSet::from([
                "Exile One".to_string(),
                "Exile Two".to_string(),
            ])
        );
        assert_eq!(
            library.iter().cloned().collect::<HashSet<_>>(),
            HashSet::from([
                "Exile One".to_string(),
                "Exile Two".to_string(),
                "Existing Bottom".to_string(),
                "Existing Top".to_string(),
            ])
        );
    }
}
