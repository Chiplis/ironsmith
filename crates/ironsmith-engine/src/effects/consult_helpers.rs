use crate::decisions::context::{OrderContext, ViewCardsContext};
use crate::effect::{EffectOutcome, ExecutionFact, OutcomeObjectMemory};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;
use crate::tag::{SOURCE_EXILED_TAG, TagKey};
use crate::triggers::TriggerEvent;
use crate::zone::Zone;
pub use ironsmith_core::{LibraryBottomOrder, LibraryConsultMode};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LibraryConsultStopRule {
    FirstMatch,
    MatchCount(u32),
    TotalManaValue(u32),
    FirstMatchOrExposedCount(u32),
}

impl LibraryConsultStopRule {
    pub fn required_matches(&self) -> u32 {
        match self {
            Self::FirstMatch => 1,
            Self::MatchCount(count) => *count,
            Self::TotalManaValue(0) => 0,
            Self::TotalManaValue(_) => u32::MAX,
            Self::FirstMatchOrExposedCount(_) => 1,
        }
    }

    fn max_exposed(&self) -> Option<usize> {
        match self {
            Self::FirstMatchOrExposedCount(count) => Some(*count as usize),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct LibraryConsultResult {
    pub exposed_snapshots: Vec<ObjectSnapshot>,
    pub matched_snapshots: Vec<ObjectSnapshot>,
    pub exposed_object_ids: Vec<ObjectId>,
    pub reveal_events: Vec<TriggerEvent>,
    /// Completed replacement observations retained by the owning consultation.
    pub operation_outcomes: Vec<EffectOutcome>,
}

impl LibraryConsultResult {
    pub fn attach_to_outcome(self, outcome: EffectOutcome) -> EffectOutcome {
        let exposed_memory = self
            .exposed_snapshots
            .iter()
            .map(OutcomeObjectMemory::from_snapshot)
            .collect::<Vec<_>>();
        let matched_ids = self
            .matched_snapshots
            .iter()
            .map(|snapshot| snapshot.object_id)
            .collect::<Vec<_>>();
        let matched_memory = self
            .matched_snapshots
            .iter()
            .map(OutcomeObjectMemory::from_snapshot)
            .collect::<Vec<_>>();

        let mut outcome = outcome
            .with_events(self.reveal_events)
            .with_affected_objects(self.exposed_object_ids)
            .with_affected_object_memory(exposed_memory);
        if !matched_ids.is_empty() {
            outcome = outcome.with_execution_fact(ExecutionFact::ChosenObjects(matched_ids));
        }
        let primary_status = outcome.status;
        let primary_value = outcome.value.clone();
        let mut outcomes = vec![outcome.with_chosen_object_memory(matched_memory)];
        outcomes.extend(self.operation_outcomes);
        let mut outcome = EffectOutcome::aggregate(outcomes);
        outcome.status = primary_status;
        outcome.value = primary_value;
        outcome
    }
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
    if ctx.decision_maker.awaiting_choice() { return Ok(LibraryConsultResult::default()); }
    let checkpoint = game.clone();
    let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
    let instruction = (|| -> Result<LibraryConsultResult, ExecutionError> {
    if let Some(tag) = all_tag {
        ctx.clear_object_tag(tag.as_str());
    }
    if let Some(tag) = match_tag {
        ctx.clear_object_tag(tag.as_str());
    }

    let required_matches = stop_rule.required_matches() as usize;
    let max_exposed = stop_rule.max_exposed();
    if required_matches == 0 || max_exposed == Some(0) {
        return Ok(LibraryConsultResult::default());
    }

    let mut result = LibraryConsultResult::default();
    let mut matched_mana_value = 0u32;
    let mut receipts = Vec::new();
    let additional = ctx.additional_replacement_effects_snapshot();
    let mut stalled_attempts = HashSet::new();

    match mode {
        LibraryConsultMode::Reveal => {
            let reveal_context_amount = ctx
                .triggering_event
                .as_ref()
                .and_then(|event| event.downcast::<crate::events::other::DieRolledEvent>())
                .filter(|roll| !roll.is_planar)
                .and_then(|roll| i32::try_from(roll.result).ok())
                .or(ctx.event_value_amount);
            let top_to_bottom: Vec<_> = game
                .player(player)
                .map(|library_owner| library_owner.library.iter().rev().copied().collect())
                .unwrap_or_default();

            for object_id in top_to_bottom {
                let Some(object) = game.object(object_id) else {
                    continue;
                };
                let snapshot = ObjectSnapshot::from_object(object, game);
                let mana_value = object
                    .mana_cost
                    .as_ref()
                    .map_or(0, |cost| cost.mana_value());
                let matched = is_match(object, game);

                result.exposed_object_ids.push(object_id);
                result.exposed_snapshots.push(snapshot.clone());
                result.reveal_events.push(TriggerEvent::new_with_provenance(
                    crate::events::CardRevealedEvent::new(
                        player,
                        object_id,
                        Zone::Library,
                        Some(ctx.source),
                        Some(snapshot.clone()),
                    )
                    .with_reveal_context_amount(reveal_context_amount),
                    ctx.provenance,
                ));
                if matched {
                    result.matched_snapshots.push(snapshot);
                    matched_mana_value = matched_mana_value.saturating_add(mana_value);
                    if result.matched_snapshots.len() >= required_matches
                        || matches!(stop_rule, LibraryConsultStopRule::TotalManaValue(threshold) if matched_mana_value >= threshold)
                    {
                        break;
                    }
                }
                if max_exposed.is_some_and(|maximum| result.exposed_snapshots.len() >= maximum) {
                    break;
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

            // One-shot prevention may leave the top card in place for a later attempt.
            // This temporary rejection boundary is deliberately not a claimed CR loop detector.
            // General repeated-state/optional-choice classification remains an open gate.
            let one_shots = game.effect_store.replacement_effects.one_shot_effects_snapshot();
            if !stalled_attempts.insert((top_card_id, one_shots)) {
                return Err(ExecutionError::InternalError("consultation made no progress; mandatory/optional loop classification required".into()));
            }
            let receipt = crate::effects::zones::apply_zone_change_with_context_and_additional_effects(
                game, top_card_id, Zone::Library, Zone::Exile, ctx.cause.clone(), ctx, &additional,
            )?;
            if ctx.decision_maker.awaiting_choice() { return Ok(LibraryConsultResult::default()); }
            let arrivals = match &receipt.original {
                crate::events::processing::EventOutcome::Proceed(change) => change.new_object_ids.clone(),
                crate::events::processing::EventOutcome::Replaced => {
                    let ids = game.take_zone_change_results(top_card_id);
                    if !ids.is_empty() { game.record_zone_change_results(top_card_id, ids.clone()); }
                    ids
                }
                _ => Vec::new(),
            };
            receipts.push((top_card_id, receipt));
            let mut stop = false;
            for exiled_id in arrivals {
                let Some(object) = game.object(exiled_id).filter(|object| object.zone == Zone::Exile) else { continue; };
                let snapshot = ObjectSnapshot::from_object(object, game);
                let mana_value = object.mana_cost.as_ref().map_or(0, |cost| cost.mana_value());
                let matched = is_match(object, game);
                game.add_exiled_with_source_link(ctx.source, exiled_id);
                ctx.tag_object(SOURCE_EXILED_TAG, snapshot.clone());
                result.exposed_object_ids.push(exiled_id);
                result.exposed_snapshots.push(snapshot.clone());
                if matched {
                    result.matched_snapshots.push(snapshot);
                    matched_mana_value = matched_mana_value.saturating_add(mana_value);
                    stop |= result.matched_snapshots.len() >= required_matches
                        || matches!(stop_rule, LibraryConsultStopRule::TotalManaValue(threshold) if matched_mana_value >= threshold);
                }
            }
            if stop { break; }
            if max_exposed.is_some_and(|maximum| result.exposed_snapshots.len() >= maximum) {
                break;
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

    // All original consultation tags and source links precede additional programs.
    let observations = crate::effects::zones::finish_zone_change_receipts(game, ctx, EffectOutcome::resolved(), receipts)?;
    if ctx.decision_maker.awaiting_choice() { return Ok(LibraryConsultResult::default()); }
    result.operation_outcomes.push(observations);
    Ok(result)
    })();
    if instruction.is_err() || ctx.decision_maker.awaiting_choice() {
        *game = checkpoint;
        context_checkpoint.restore(ctx);
    }
    instruction
}

pub fn move_tagged_remainder_to_library_bottom(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    tag: &TagKey,
    keep_tagged: Option<&TagKey>,
    order: LibraryBottomOrder,
    chooser: PlayerId,
) -> Result<EffectOutcome, ExecutionError> {
    if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
    let checkpoint = game.clone();
    let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
    let instruction = (|| -> Result<EffectOutcome, ExecutionError> {
        let Some(tagged) = ctx.get_tagged_all(tag.as_str()).cloned() else {
            return Ok(EffectOutcome::resolved());
        };
        let keep_ids = keep_tagged.and_then(|keep| ctx.get_tagged_all(keep.as_str()).cloned())
            .unwrap_or_default().into_iter().map(|snapshot| snapshot.object_id).collect::<HashSet<_>>();
        let mut owner_order = Vec::new();
        let mut by_owner: HashMap<PlayerId, Vec<BottomCandidate>> = HashMap::new();
        let mut selected = HashSet::new();
        for snapshot in tagged {
            if keep_ids.contains(&snapshot.object_id) || !selected.insert(snapshot.object_id) { continue; }
            let Some(candidate) = BottomCandidate::from_snapshot(game, snapshot) else { continue; };
            if !by_owner.contains_key(&candidate.owner) { owner_order.push(candidate.owner); }
            by_owner.entry(candidate.owner).or_default().push(candidate);
        }
        // Resolve all ordering choices before any member of the original instruction moves.
        let mut ordered_groups = Vec::new();
        for owner in owner_order {
            let candidates = by_owner.remove(&owner).unwrap_or_default();
            let ordered = order_bottom_candidates(game, ctx, chooser, &candidates, order);
            if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
            ordered_groups.push((owner, normalize_candidate_order(ordered, &candidates)));
        }
        let additional = ctx.additional_replacement_effects_snapshot();
        let opened_batch = game.open_simultaneous_action();
        let mut receipts = Vec::new();
        let mut moved_ids = Vec::new();
        for (owner, ordered) in ordered_groups {
            let mut arrivals = HashMap::<ObjectId, Vec<ObjectId>>::new();
            for candidate in &ordered {
                if candidate.zone == Zone::Library {
                    arrivals.insert(candidate.object_id, vec![candidate.object_id]);
                    continue;
                }
                let receipt = crate::effects::zones::apply_zone_change_with_context_and_additional_effects(
                    game, candidate.object_id, Zone::Exile, Zone::Library, ctx.cause.clone(), ctx, &additional,
                )?;
                if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
                let ids = match &receipt.original {
                    crate::events::processing::EventOutcome::Proceed(change) => change.new_object_ids.clone(),
                    crate::events::processing::EventOutcome::Replaced => {
                        let ids = game.take_zone_change_results(candidate.object_id);
                        if !ids.is_empty() { game.record_zone_change_results(candidate.object_id, ids.clone()); }
                        ids
                    }
                    _ => Vec::new(),
                };
                arrivals.insert(candidate.object_id, ids);
                receipts.push((candidate.object_id, receipt));
            }
            let mut seen = HashSet::new();
            let ordered_current_ids = ordered.iter().flat_map(|candidate|
                arrivals.get(&candidate.object_id).into_iter().flatten().copied())
                .filter(|id| game.object(*id).is_some_and(|object| object.zone == Zone::Library && object.owner == owner))
                .filter(|id| seen.insert(*id)).collect::<Vec<_>>();
            if ordered_current_ids.is_empty() { continue; }
            let bottom_ids = ordered_current_ids.iter().copied().collect::<HashSet<_>>();
            if let Some(player) = game.player(owner) {
                let mut after_order = player.library.iter().copied().filter(|id| !bottom_ids.contains(id)).collect::<Vec<_>>();
                after_order.splice(0..0, ordered_current_ids.clone());
                game.set_player_library_order_with_audit(owner, after_order, "consult effect put cards on bottom");
            }
            moved_ids.extend(ordered_current_ids);
        }
        game.close_simultaneous_action(opened_batch);
        let original = if moved_ids.is_empty() { EffectOutcome::resolved() }
            else { EffectOutcome::with_objects(moved_ids) };
        crate::effects::zones::finish_zone_change_receipts(game, ctx, original, receipts)
    })();
    let pending = ctx.decision_maker.awaiting_choice();
    if pending || instruction.is_err() { *game = checkpoint; context_checkpoint.restore(ctx); }
    if pending { return instruction.map(|_| EffectOutcome::count(0)); }
    instruction
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
    object_id: ObjectId,
    owner: PlayerId,
    zone: Zone,
    name: String,
}

impl BottomCandidate {
    fn from_snapshot(game: &GameState, snapshot: ObjectSnapshot) -> Option<Self> {
        let current_id = snapshot.object_id;
        // A tag names this incarnation, not a later object with the same stable ID.
        let object = game.object(current_id)?;
        if object.zone != Zone::Library && object.zone != Zone::Exile {
            return None;
        }

        Some(Self {
            object_id: current_id,
            owner: object.owner,
            zone: object.zone,
            name: object.name.to_string(),
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
            let mut ordered = candidates
                .iter()
                .map(|candidate| candidate.object_id)
                .collect::<Vec<_>>();
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
                    .map(|candidate| (candidate.object_id, candidate.name.to_string()))
                    .collect::<Vec<_>>(),
            );
            ctx.decision_maker.decide_order(game, &context)
        }
    }
}

fn normalize_candidate_order(
    response: Vec<ObjectId>,
    original: &[BottomCandidate],
) -> Vec<BottomCandidate> {
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
    use crate::effects::ExecutionContext;
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
            .map(|id| {
                game.object(*id)
                    .expect("library object exists")
                    .name
                    .to_string()
            })
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
            .map(|id| game.object(*id).expect("object exists").name.to_string())
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
        assert_eq!(result.reveal_events.len(), 2);
        let outcome = result
            .clone()
            .attach_to_outcome(EffectOutcome::with_objects(
                result.exposed_object_ids.clone(),
            ));
        assert_eq!(outcome.affected_objects(), Some([top, match_id].as_slice()));
        assert_eq!(outcome.chosen_objects(), Some([match_id].as_slice()));
        assert_eq!(
            outcome.affected_object_memory().map(|memory| memory.len()),
            Some(2)
        );
        assert_eq!(
            outcome.chosen_object_memory().map(|memory| memory.len()),
            Some(1)
        );
        assert_eq!(outcome.events.len(), 2);
        assert_eq!(snapshot_ids(&ctx, "all"), vec![top, match_id]);
        assert_eq!(snapshot_ids(&ctx, "match"), vec![match_id]);
        assert_eq!(game.player(alice).expect("alice exists").library, before);
        assert_eq!(
            game.object(bottom).expect("bottom card exists").zone,
            Zone::Library
        );
    }

    #[test]
    fn reveal_consult_first_match_or_count_stops_at_exposed_card_cap() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        game.create_object_from_card(
            &make_library_card("Bottom Artifact", vec![CardType::Artifact]),
            alice,
            Zone::Library,
        );
        let second = game.create_object_from_card(
            &make_library_card("Second Land", vec![CardType::Land]),
            alice,
            Zone::Library,
        );
        let first = game.create_object_from_card(
            &make_library_card("Top Creature", vec![CardType::Creature]),
            alice,
            Zone::Library,
        );

        let ctx = ExecutionContext::new_default(ObjectId::from_raw(1001), alice);
        let mut dm = AutoPassDecisionMaker;
        let mut ctx = ctx.with_decision_maker(&mut dm);

        let result = execute_library_consult(
            &mut game,
            &mut ctx,
            alice,
            LibraryConsultMode::Reveal,
            LibraryConsultStopRule::FirstMatchOrExposedCount(2),
            Some(&TagKey::from("all")),
            Some(&TagKey::from("match")),
            |object, _| object.card_types.contains(&CardType::Artifact),
        )
        .expect("bounded reveal consult should execute");

        assert_eq!(result.exposed_object_ids, vec![first, second]);
        assert!(result.matched_snapshots.is_empty());
        assert_eq!(snapshot_ids(&ctx, "all"), vec![first, second]);
        assert!(ctx.get_tagged_all("match").is_none());
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
            game.get_exiled_with_source_links(ctx.source),
            result.exposed_object_ids.as_slice(),
            "exile consultation must retain source-linked cleanup provenance"
        );
        assert_eq!(
            snapshot_ids(&ctx, SOURCE_EXILED_TAG),
            result.exposed_object_ids,
            "the shared source-exiled tag must cover matching and nonmatching cards"
        );
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
                ObjectSnapshot::from_object(
                    game.object(exile_one).expect("exile one exists"),
                    &game,
                ),
                ObjectSnapshot::from_object(
                    game.object(exile_two).expect("exile two exists"),
                    &game,
                ),
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
            HashSet::from(["Exile One".to_string(), "Exile Two".to_string(),])
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

#[cfg(test)]
mod replacement_bottom_owner_contract_tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::DecisionMaker;
    use crate::effect::{Effect, Value};
    use crate::ids::CardId;
    use crate::replacement::{ReplacementAction, ReplacementEffect};
    use crate::types::CardType;
    struct Answers {pause: bool, pending: bool, order_calls: usize, addition_calls: usize, originals: Vec<ObjectId>}
    impl DecisionMaker for Answers {
        fn decide_order(&mut self,game: &GameState,context: &OrderContext)->Vec<ObjectId> {
            self.order_calls+=1;
            assert!(self.originals.iter().all(|id|game.object(*id).unwrap().zone==Zone::Exile));
            context.items.iter().rev().map(|(id,_)|*id).collect()
        }
        fn decide_boolean(&mut self,game: &GameState,_: &crate::decisions::context::BooleanContext)->bool {
            self.addition_calls+=1;
            let names=game.player(PlayerId::from_index(0)).unwrap().library.iter().map(|id|game.object(*id).unwrap().name.to_string()).collect::<Vec<_>>();
            assert_eq!(names,vec!["Second","First","Existing"]);
            self.pending=self.pause; !self.pending
        }
        fn awaiting_choice(&self)->bool {self.pending}
    }
    fn card(game:&mut GameState,owner:PlayerId,name:&str,zone:Zone)->ObjectId {
        game.create_object_from_card(&CardBuilder::new(CardId::new(),name).card_types(vec![CardType::Artifact]).build(),owner,zone)
    }
    fn check(mode:u8) {
        let mut game=crate::tests::test_helpers::setup_two_player_game();let alice=PlayerId::from_index(0);let bob=PlayerId::from_index(1);
        let parent=card(&mut game,alice,"Parent",Zone::Battlefield);let source=card(&mut game,bob,"Replacement",Zone::Battlefield);
        let existing=card(&mut game,alice,"Existing",Zone::Library);let first=card(&mut game,alice,"First",Zone::Exile);let second=card(&mut game,alice,"Second",Zone::Exile);
        let tags=[first,second].into_iter().map(|id|ObjectSnapshot::from_object(game.object(id).unwrap(),&game)).collect::<Vec<_>>();
        let sentinel=ObjectSnapshot::from_object(game.object(parent).unwrap(),&game);
        let actions=if mode==1 {vec![Effect::gain_life(3),Effect::lose_life(Value::X)]}
            else {vec![Effect::gain_life(3),Effect::may(vec![Effect::gain_life(4)])]};
        let shield=game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source,bob,
            crate::events::zones::matchers::WouldChangeZoneMatcher::new(crate::target::ObjectFilter::default(),Some(Zone::Exile),Some(Zone::Library)),ReplacementAction::Additionally(actions)));
        game.take_pending_trigger_events();let ids=game.next_object_id_counter();let objects=game.objects_in_deterministic_order().len();
        let mut dm=Answers {pause:mode==2,pending:false,order_calls:0,addition_calls:0,originals:vec![first,second]};
        let mut ctx=ExecutionContext::new(parent,alice,&mut dm);ctx.set_tagged_objects("all",tags.clone());ctx.set_tagged_objects("it",vec![sentinel.clone()]);
        let result=move_tagged_remainder_to_library_bottom(&mut game,&mut ctx,&TagKey::from("all"),None,LibraryBottomOrder::ChooserChooses,alice);
        if mode==1 {assert!(matches!(result,Err(ExecutionError::UnresolvableValue(_))));}
        else if mode==2 {assert!(ctx.decision_maker.awaiting_choice());assert!(result.unwrap().events.is_empty());}
        else {
            let outcome=result.unwrap();let arrived=outcome.explicit_objects().unwrap();assert_eq!(arrived.len(),2);assert!(arrived.iter().all(|id|game.object(*id).unwrap().zone==Zone::Library));
            assert_eq!(game.player(alice).unwrap().life,20);assert_eq!(game.player(bob).unwrap().life,27);
            assert_eq!(outcome.events.iter().filter_map(|event|event.downcast::<crate::events::LifeGainEvent>()).map(|event|(event.player,event.amount)).collect::<Vec<_>>(),vec![(bob,3),(bob,4)]);
            assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
        }
        assert_eq!(ctx.get_tagged_all("all").unwrap().iter().map(|s|s.object_id).collect::<Vec<_>>(),vec![first,second]);assert_eq!(ctx.get_tagged_all("it").unwrap()[0].object_id,sentinel.object_id);
        if mode==1||mode==2 {
            assert_eq!(game.player(alice).unwrap().library,vec![existing]);assert_eq!(game.object(first).unwrap().zone,Zone::Exile);assert_eq!(game.object(second).unwrap().zone,Zone::Exile);
            assert_eq!(game.next_object_id_counter(),ids);assert_eq!(game.objects_in_deterministic_order().len(),objects);assert_eq!(game.player(bob).unwrap().life,20);
            assert!(game.effect_store.replacement_effects.get_effect(shield).is_some());assert!(game.take_pending_trigger_events().is_empty());
        }
        drop(ctx);assert_eq!(dm.order_calls,1);
        if mode==0 {assert_eq!(dm.addition_calls,1);}
        if mode==2 {
            assert_eq!(dm.addition_calls,1);dm.pause=false;dm.pending=false;
            let mut ctx=ExecutionContext::new(parent,alice,&mut dm);ctx.set_tagged_objects("all",tags);
            let outcome=move_tagged_remainder_to_library_bottom(&mut game,&mut ctx,&TagKey::from("all"),None,LibraryBottomOrder::ChooserChooses,alice).unwrap();assert_eq!(outcome.explicit_objects().unwrap().len(),2);
            assert_eq!(game.player(bob).unwrap().life,27);assert!(!ctx.decision_maker.awaiting_choice());drop(ctx);assert_eq!(dm.order_calls,2);assert_eq!(dm.addition_calls,2);
        }
    }
    #[test] fn additions_observe_whole_ordered_original_batch() {check(0);}
    #[test] fn error_restores_whole_ordered_batch() {check(1);}
    #[test] fn pending_replays_whole_ordered_batch() {check(2);}
    #[test] fn stale_tag_does_not_follow_a_later_incarnation() {
        let mut game=crate::tests::test_helpers::setup_two_player_game();let alice=PlayerId::from_index(0);let source=card(&mut game,alice,"Parent",Zone::Battlefield);
        let old=card(&mut game,alice,"Departed",Zone::Exile);let snapshot=ObjectSnapshot::from_object(game.object(old).unwrap(),&game);
        let cause=ExecutionContext::new_default(source,alice).cause; let later=game.move_object(old,Zone::Hand,cause.clone()).unwrap();let later=game.move_object(later,Zone::Exile,cause).unwrap();game.take_pending_trigger_events();
        let mut ctx=ExecutionContext::new_default(source,alice);ctx.set_tagged_objects("all",vec![snapshot]);
        let outcome=move_tagged_remainder_to_library_bottom(&mut game,&mut ctx,&TagKey::from("all"),None,LibraryBottomOrder::Random,alice).unwrap();
        assert!(game.object(later).is_some_and(|object| object.zone == Zone::Exile), "the later incarnation must remain untouched");assert!(outcome.explicit_objects().is_none());assert!(game.player(alice).unwrap().library.is_empty());
    }
}
