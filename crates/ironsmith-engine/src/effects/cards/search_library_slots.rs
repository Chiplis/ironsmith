//! Search a library for multiple differently-constrained cards as one search.

use crate::effects::zones::{apply_zone_change_with_context_and_additional_effects, finish_zone_change_receipts};
use crate::events::processing::EventOutcome;
use crate::filter::ObjectFilterExt as _;
use std::collections::HashSet;

use crate::decision::FallbackStrategy;
use crate::decisions::{SearchSpec, make_decision_with_fallback};
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::cards::search_overrides::{
    begin_opposition_agent_search_control, exile_found_cards_for_opposition_agent,
    finish_opposition_agent_search_control, offer_library_search_casts, opposition_agent_search,
};
use crate::effects::helpers::{resolve_player_filter, view_hidden_candidate_objects};
use crate::effects::zones::{
    BattlefieldEntryOptions, BattlefieldEntryOutcome, move_to_battlefield_batch_with_options,
};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::{SearchLibraryEvent, ShuffleLibraryEvent};
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::snapshot::ObjectSnapshot;
use crate::triggers::TriggerEvent;
use crate::zone::Zone;

pub type SearchLibrarySlot = ironsmith_core::SearchLibrarySlot;
pub type SearchLibrarySlotsEffect = ironsmith_core::SearchLibrarySlotsEffect;

impl EffectExecutor for SearchLibrarySlotsEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
        game.clear_pending_decision_controllers();
        let checkpoint = game.clone();
        let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        let mut pending_selection_progress = None;
        let instruction = (|| -> Result<EffectOutcome, ExecutionError> {
        let chooser_id =
            crate::effects::helpers::resolve_player_filter_as_chooser(game, &self.chooser, ctx)?;
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let search_override = opposition_agent_search(game, chooser_id, player_id);

        if !game.can_search_library_from_effect(chooser_id, player_id, ctx.controller) {
            return Ok(EffectOutcome::prevented());
        }
        let search_control =
            begin_opposition_agent_search_control(game, chooser_id, search_override);
        let result = (|| -> Result<EffectOutcome, ExecutionError> {
            let search_viewer = chooser_id;
            let mut library_cards = game
                .player(player_id)
                .map(|player| player.library.to_vec())
                .unwrap_or_default();
            game.restrict_library_search_candidates(chooser_id, &mut library_cards);
            view_hidden_candidate_objects(
                game,
                ctx,
                search_viewer,
                &library_cards,
                "Search library",
                false,
            );

            offer_library_search_casts(game, ctx, player_id)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }

            let search_event = TriggerEvent::new_with_provenance(
                SearchLibraryEvent::new(chooser_id, Some(player_id)),
                ctx.provenance,
            );
            let shuffle_event = TriggerEvent::new_with_provenance(
                ShuffleLibraryEvent::new(player_id, ctx.cause.clone()),
                ctx.provenance,
            );

            let mut chosen: Vec<ObjectSnapshot> = ctx
                .get_tagged_all(self.progress_tag.as_str())
                .cloned()
                .unwrap_or_default();
            if chosen.len() > self.slots.len() {
                chosen.clear();
                ctx.clear_object_tag(self.progress_tag.as_str());
            }

            for slot in self.slots.iter().skip(chosen.len()) {
                let filter_ctx = ctx.filter_context(game);
                let already_chosen: HashSet<ObjectId> =
                    chosen.iter().map(|snapshot| snapshot.object_id).collect();
                let matching_cards: Vec<ObjectId> = game
                    .player(player_id)
                    .map(|player| {
                        let candidates: Vec<ObjectId> = match slot.filter.zone {
                            Some(Zone::Graveyard) => player.graveyard.to_vec(),
                            Some(Zone::Library) => player.library.to_vec(),
                            None => player
                                .library
                                .iter()
                                .chain(player.graveyard.iter())
                                .copied()
                                .collect(),
                            _ => player.library.to_vec(),
                        };
                        let mut candidates = candidates;
                        game.restrict_library_search_candidates(chooser_id, &mut candidates);
                        candidates
                            .into_iter()
                            .filter(|id| !already_chosen.contains(id))
                            .filter(|id| {
                                game.object(*id)
                                    .is_some_and(|obj| slot.filter.matches(obj, &filter_ctx, game))
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                if matching_cards.is_empty() {
                    continue;
                }

                let chosen_card = if slot.optional {
                    make_decision_with_fallback(
                        game,
                        &mut ctx.decision_maker,
                        chooser_id,
                        Some(ctx.source),
                        SearchSpec::new(ctx.source, matching_cards, self.reveal),
                        FallbackStrategy::Decline,
                    )
                } else {
                    make_decision_with_fallback(
                        game,
                        &mut ctx.decision_maker,
                        chooser_id,
                        Some(ctx.source),
                        SearchSpec::mandatory(ctx.source, matching_cards, self.reveal),
                        FallbackStrategy::FirstOption,
                    )
                };

                if ctx.decision_maker.awaiting_choice() {
                    pending_selection_progress = Some(ctx.get_tagged_all(self.progress_tag.as_str()).cloned().unwrap_or_default());
                    return Ok(EffectOutcome::count(0).with_event(search_event));
                }

                let Some(card_id) = chosen_card else {
                    continue;
                };
                let Some(snapshot) = game
                    .object(card_id)
                    .map(|obj| ObjectSnapshot::from_object(obj, game))
                else {
                    continue;
                };
                if self.reveal {
                    view_hidden_candidate_objects(
                        game,
                        ctx,
                        search_viewer,
                        &[card_id],
                        "Reveal searched card",
                        true,
                    );
                }
                chosen.push(snapshot.clone());
                ctx.tag_object(self.progress_tag.clone(), snapshot);
            }

            let mut moved_ids = Vec::new();
            let mut receipts = Vec::new();
            let chosen_ids = chosen.iter().map(|snapshot| snapshot.object_id).collect::<Vec<_>>();
            if search_override.is_some() {
                let found = exile_found_cards_for_opposition_agent(game, ctx, &chosen_ids, chooser_id)?;
                moved_ids = found.moved_ids;
                receipts = found.receipts;
                if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
            } else if self.destination == Zone::Battlefield {
                let entries = move_to_battlefield_batch_with_options(game, ctx, chosen_ids.iter().copied()
                    .map(|id| (id, BattlefieldEntryOptions::preserve(false))).collect())?;
                if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
                if entries.len() != chosen_ids.len() { return Err(ExecutionError::InternalError("search entry batch lost a receipt".into())); }
                for (original, entry) in chosen_ids.iter().zip(entries) {
                    match &entry.outcome {
                        BattlefieldEntryOutcome::Moved(id) => moved_ids.push(*id),
                        BattlefieldEntryOutcome::Redirected(change) => moved_ids.extend(change.new_object_ids.iter().copied()),
                        BattlefieldEntryOutcome::Prevented => {}
                    }
                    let (object, receipt) = entry.into_zone_receipt();
                    if object != *original { return Err(ExecutionError::InternalError("search entry receipt changed original identity".into())); }
                    receipts.push((object, receipt));
                }
            } else {
                let opened_batch = game.open_simultaneous_action();
                let moves = (|| -> Result<(), ExecutionError> {
                    for id in &chosen_ids {
                        let Some(from) = game.object(*id).map(|card| card.zone) else { continue; };
                        if self.destination == Zone::Library && from == Zone::Library {
                            moved_ids.push(*id); continue;
                        }
                        let additional = ctx.additional_replacement_effects_snapshot();
                        let receipt = apply_zone_change_with_context_and_additional_effects(
                            game, *id, from, self.destination, ctx.cause.clone(), ctx, &additional,
                        )?;
                        if ctx.decision_maker.awaiting_choice() { return Ok(()); }
                        match &receipt.original {
                            EventOutcome::Proceed(change) => moved_ids.extend(change.new_object_ids.iter().copied()),
                            EventOutcome::Replaced => {
                                let ids = game.take_zone_change_results(*id);
                                if !ids.is_empty() { game.record_zone_change_results(*id, ids.clone()); }
                                moved_ids.extend(ids);
                            }
                            EventOutcome::Prevented | EventOutcome::NotApplicable => {}
                        }
                        receipts.push((*id, receipt));
                    }
                    Ok(())
                })();
                game.close_simultaneous_action(opened_batch);
                moves?;
                if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
            }
            // The put/exile instruction and permission links complete first;
            // replacement additions precede the subsequent library shuffle.
            let original = if moved_ids.is_empty() { EffectOutcome::count(0) }
                else { EffectOutcome::with_objects(moved_ids.clone()) }.with_event(search_event);
            let mut outcome = finish_zone_change_receipts(game, ctx, original, receipts)?;
            if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
            if self.destination == Zone::Library && search_override.is_none() {
                let remaining = moved_ids.into_iter().filter(|id| game.object(*id).is_some_and(|card| card.zone == Zone::Library)).collect::<Vec<_>>();
                game.shuffle_library_except_then_put_on_top(player_id, &remaining,
                    "searched cards put on top after library shuffle");
            } else { game.shuffle_player_library(player_id); }
            ctx.clear_object_tag(self.progress_tag.as_str());
            outcome.events.push(shuffle_event);
            Ok(outcome)
        })();

        if result.is_ok() && ctx.decision_maker.awaiting_choice() {
            game.capture_pending_decision_controllers();
        }
        // Active scopes always unwind. Only the pending routing view survives.
        finish_opposition_agent_search_control(game, search_control);
        result
        })();
        let pending = ctx.decision_maker.awaiting_choice();
        if pending || instruction.is_err() { game.restore_execution_checkpoint(checkpoint, pending && instruction.is_ok()); context_checkpoint.restore(ctx); }
        if pending {
            if let Some(progress) = pending_selection_progress {
                ctx.set_tagged_objects(self.progress_tag.clone(), progress);
            }
            return Ok(EffectOutcome::count(0));
        }
        instruction
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::{DecisionMaker, SelectFirstDecisionMaker};
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::ManaCost;
    use crate::test_prelude::*;
    use crate::types::{CardType, Subtype, Supertype};

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn push_basic_land_in_zone(
        game: &mut GameState,
        controller: PlayerId,
        name: &str,
        subtype: Subtype,
        zone: Zone,
    ) {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Land])
            .supertypes(vec![Supertype::Basic])
            .subtypes(vec![subtype])
            .mana_cost(ManaCost::new())
            .build();
        game.create_object_from_card(&card, controller, zone);
    }

    fn push_land_with_subtypes(
        game: &mut GameState,
        controller: PlayerId,
        name: &str,
        subtypes: Vec<Subtype>,
    ) {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Land])
            .subtypes(subtypes)
            .mana_cost(ManaCost::new())
            .build();
        game.create_object_from_card(&card, controller, Zone::Library);
    }

    fn push_basic_land(game: &mut GameState, controller: PlayerId, name: &str, subtype: Subtype) {
        push_basic_land_in_zone(game, controller, name, subtype, Zone::Library);
    }

    fn gaeas_balance_search_effect() -> SearchLibrarySlotsEffect {
        SearchLibrarySlotsEffect::new(
            [
                Subtype::Plains,
                Subtype::Island,
                Subtype::Swamp,
                Subtype::Mountain,
                Subtype::Forest,
            ]
            .into_iter()
            .map(|subtype| {
                SearchLibrarySlot::optional(
                    ObjectFilter::default()
                        .in_zone(Zone::Library)
                        .with_type(CardType::Land)
                        .with_subtype(subtype),
                )
            })
            .collect(),
            Zone::Battlefield,
            PlayerFilter::You,
            PlayerFilter::You,
            false,
            "gaeas_balance_progress",
        )
    }

    struct PendingOnSecondChoiceDm {
        calls: usize,
    }

    impl DecisionMaker for PendingOnSecondChoiceDm {
        fn awaiting_choice(&self) -> bool {
            self.calls >= 2
        }

        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            self.calls += 1;
            if self.calls == 1 {
                ctx.candidates
                    .iter()
                    .find(|candidate| candidate.legal)
                    .map(|candidate| vec![candidate.id])
                    .unwrap_or_default()
            } else {
                Vec::new()
            }
        }
    }

    #[test]
    fn search_library_slots_moves_multiple_different_cards_to_hand() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        push_basic_land(&mut game, alice, "Forest", Subtype::Forest);
        push_basic_land(&mut game, alice, "Plains", Subtype::Plains);

        let source = ObjectId::from_raw(999);
        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = SearchLibrarySlotsEffect::to_hand(
            vec![
                SearchLibrarySlot::optional(
                    ObjectFilter::default()
                        .in_zone(Zone::Library)
                        .with_type(CardType::Land)
                        .with_supertype(Supertype::Basic)
                        .with_subtype(Subtype::Forest),
                ),
                SearchLibrarySlot::optional(
                    ObjectFilter::default()
                        .in_zone(Zone::Library)
                        .with_type(CardType::Land)
                        .with_supertype(Supertype::Basic)
                        .with_subtype(Subtype::Plains),
                ),
            ],
            PlayerFilter::You,
            true,
            "progress",
        );

        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("search should resolve");

        assert_eq!(outcome.output_objects().len(), 2);
        let hand_names: Vec<_> = game
            .player(alice)
            .expect("alice exists")
            .hand
            .iter()
            .filter_map(|id| game.object(*id).map(|obj| obj.name.to_string()))
            .collect();
        assert!(hand_names.iter().any(|name| name == "Forest"));
        assert!(hand_names.iter().any(|name| name == "Plains"));
        assert!(ctx.get_tagged_all("progress").is_none());
    }

    #[test]
    fn search_library_slots_keeps_progress_across_resume() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        push_basic_land(&mut game, alice, "Forest", Subtype::Forest);
        push_basic_land(&mut game, alice, "Plains", Subtype::Plains);

        let effect = SearchLibrarySlotsEffect::to_hand(
            vec![
                SearchLibrarySlot::optional(
                    ObjectFilter::default()
                        .in_zone(Zone::Library)
                        .with_type(CardType::Land)
                        .with_supertype(Supertype::Basic)
                        .with_subtype(Subtype::Forest),
                ),
                SearchLibrarySlot::optional(
                    ObjectFilter::default()
                        .in_zone(Zone::Library)
                        .with_type(CardType::Land)
                        .with_supertype(Supertype::Basic)
                        .with_subtype(Subtype::Plains),
                ),
            ],
            PlayerFilter::You,
            true,
            "progress",
        );
        let source = ObjectId::from_raw(1000);
        let ctx = ExecutionContext::new_default(source, alice);

        let mut pending_dm = PendingOnSecondChoiceDm { calls: 0 };
        let mut ctx = ctx.with_decision_maker(&mut pending_dm);
        let first = effect
            .execute(&mut game, &mut ctx)
            .expect("first pass should execute");
        assert_eq!(first.value, crate::effect::OutcomeValue::Count(0));
        assert_eq!(
            ctx.get_tagged_all("progress")
                .expect("first selected card should be remembered")
                .len(),
            1
        );
        assert_eq!(game.player(alice).expect("alice exists").hand.len(), 0);

        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ctx.with_decision_maker(&mut dm);
        let second = effect
            .execute(&mut game, &mut ctx)
            .expect("resume should execute");
        assert_eq!(second.output_objects().len(), 2);
        assert!(ctx.get_tagged_all("progress").is_none());
        assert_eq!(game.player(alice).expect("alice exists").hand.len(), 2);
    }

    #[test]
    fn gaeas_balance_slots_put_each_basic_land_type_onto_battlefield() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        for (name, subtype) in [
            ("Plains", Subtype::Plains),
            ("Island", Subtype::Island),
            ("Swamp", Subtype::Swamp),
            ("Mountain", Subtype::Mountain),
            ("Forest", Subtype::Forest),
        ] {
            push_basic_land(&mut game, alice, name, subtype);
        }

        let source = ObjectId::from_raw(2001);
        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = gaeas_balance_search_effect()
            .execute(&mut game, &mut ctx)
            .expect("Gaea's Balance search should resolve");

        assert_eq!(outcome.output_objects().len(), 5);
        let battlefield_names: Vec<_> = game
            .battlefield
            .iter()
            .filter_map(|id| game.object(*id).map(|obj| obj.name.to_string()))
            .collect();
        for name in ["Plains", "Island", "Swamp", "Mountain", "Forest"] {
            assert!(
                battlefield_names.iter().any(|candidate| candidate == name),
                "Gaea's Balance should put {name} onto the battlefield, got {battlefield_names:?}"
            );
        }
        assert!(ctx.get_tagged_all("gaeas_balance_progress").is_none());
    }

    #[test]
    fn gaeas_balance_slots_do_not_find_missing_type_from_graveyard() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        for (name, subtype) in [
            ("Plains", Subtype::Plains),
            ("Island", Subtype::Island),
            ("Mountain", Subtype::Mountain),
            ("Forest", Subtype::Forest),
        ] {
            push_basic_land(&mut game, alice, name, subtype);
        }
        push_basic_land_in_zone(
            &mut game,
            alice,
            "Graveyard Swamp",
            Subtype::Swamp,
            Zone::Graveyard,
        );

        let source = ObjectId::from_raw(2002);
        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = gaeas_balance_search_effect()
            .execute(&mut game, &mut ctx)
            .expect("Gaea's Balance partial search should resolve");

        assert_eq!(outcome.output_objects().len(), 4);
        assert!(
            !game.battlefield.iter().any(|id| game
                .object(*id)
                .is_some_and(|obj| obj.name == "Graveyard Swamp")),
            "Gaea's Balance must not find a missing basic land type from the graveyard"
        );
        let graveyard_names: Vec<_> = game
            .player(alice)
            .expect("alice exists")
            .graveyard
            .iter()
            .filter_map(|id| game.object(*id).map(|obj| obj.name.to_string()))
            .collect();
        assert!(
            graveyard_names.iter().any(|name| name == "Graveyard Swamp"),
            "Gaea's Balance should leave the graveyard Swamp in the graveyard, got {graveyard_names:?}"
        );
    }

    #[test]
    fn gaeas_balance_slots_use_multitype_land_for_only_one_slot() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        push_land_with_subtypes(
            &mut game,
            alice,
            "Savannah",
            vec![Subtype::Plains, Subtype::Forest],
        );

        let source = ObjectId::from_raw(2003);
        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = gaeas_balance_search_effect()
            .execute(&mut game, &mut ctx)
            .expect("Gaea's Balance dual-land search should resolve");

        assert_eq!(
            outcome.output_objects().len(),
            1,
            "one land card with two basic land types must not satisfy two slots"
        );
        assert!(
            game.battlefield
                .iter()
                .any(|id| game.object(*id).is_some_and(|obj| obj.name == "Savannah"))
        );
    }
}
