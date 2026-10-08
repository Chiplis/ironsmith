//! Search a library for multiple differently-constrained cards as one search.

use crate::effects::zones::{
    apply_zone_change_with_context_and_additional_effects_with_outputs,
    finish_zone_change_receipts_with_outputs,
};
use crate::events::processing::EventOutcome;
use crate::filter::ObjectFilterExt as _;
use std::collections::HashSet;

use crate::decision::FallbackStrategy;
use crate::decisions::{SearchSpec, make_decision_with_fallback};
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::cards::search_overrides::{
    LibrarySearchRequest, execute_library_search_scope,
    exile_found_cards_for_opposition_agent_with_outputs,
};
use crate::effects::context::ObjectSelectionProgress;
use crate::effects::helpers::view_hidden_candidate_objects;
use crate::effects::zones::{
    BattlefieldEntryOptions, BattlefieldEntryOutcome, move_to_battlefield_batch_with_options,
};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::snapshot::ObjectSnapshot;
use crate::zone::Zone;

pub type SearchLibrarySlot = ironsmith_core::SearchLibrarySlot;
pub type SearchLibrarySlotsEffect = ironsmith_core::SearchLibrarySlotsEffect;

impl EffectExecutor for SearchLibrarySlotsEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        game.clear_pending_decision_controllers();
        let checkpoint = game.clone();
        let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        let mut pending_selection_progress = None;
        let mut retained_children = Vec::new();
        let mut published_entry_outputs = Vec::new();
        let instruction = (|| -> Result<EffectOutcome, ExecutionError> {
            let chooser_id = crate::effects::helpers::resolve_player_filter_as_chooser(
                game,
                &self.chooser,
                ctx,
            )?;
            let player_id =
                crate::effects::helpers::resolve_player_filter(game, &self.player, ctx)?;
            execute_library_search_scope(
                game,
                ctx,
                LibrarySearchRequest {
                    chooser: chooser_id,
                    library_owner: Some(player_id),
                    search_library: true,
                    require_library_access: true,
                    restrict_initial_view: true,
                    refresh_library_access: false,
                },
                |game, ctx, search| {
                    let search_override = search.found_card_policy;
                    let search_event = search
                        .event
                        .expect("admitted library search supplies its observation");
                    let search_viewer = search.chooser;
                    let progress_key = (
                        ctx.source,
                        ctx.controller,
                        chooser_id,
                        player_id,
                        self.progress_tag.clone(),
                    );
                    let mut progress = ctx
                        .object_selection_progress
                        .get(&progress_key)
                        .cloned()
                        .unwrap_or_default();
                    if progress.next_request > self.slots.len()
                        || progress.chosen.len() > progress.next_request
                    {
                        progress = ObjectSelectionProgress::default();
                        ctx.object_selection_progress.remove(&progress_key);
                    }
                    let mut chosen = progress.chosen;
                    let mut next_request = progress.next_request;
                    if chosen.is_empty() {
                        ctx.clear_object_tag(self.progress_tag.as_str());
                    } else {
                        ctx.set_tagged_objects(self.progress_tag.clone(), chosen.clone());
                    }

                    // Reopen the accepted selection's presentation on resume.
                    // This is not another semantic Reveal action.
                    if self.reveal && !chosen.is_empty() {
                        let ids = chosen
                            .iter()
                            .map(|snapshot| snapshot.object_id)
                            .collect::<Vec<_>>();
                        view_hidden_candidate_objects(
                            game,
                            ctx,
                            search_viewer,
                            &ids,
                            "Reveal searched card",
                            true,
                        );
                    }

                    for (slot_index, slot) in self.slots.iter().enumerate().skip(next_request) {
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
                                game.restrict_library_search_candidates(
                                    chooser_id,
                                    &mut candidates,
                                );
                                candidates
                                    .into_iter()
                                    .filter(|id| !already_chosen.contains(id))
                                    .filter(|id| {
                                        game.object(*id).is_some_and(|obj| {
                                            slot.filter.matches(obj, &filter_ctx, game)
                                        })
                                    })
                                    .collect()
                            })
                            .unwrap_or_default();

                        if matching_cards.is_empty() {
                            next_request = slot_index + 1;
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
                            pending_selection_progress = Some((
                                progress_key.clone(),
                                ObjectSelectionProgress {
                                    next_request,
                                    chosen: chosen.clone(),
                                },
                            ));
                            return Ok(EffectOutcome::count(0).with_event(search_event));
                        }

                        next_request = slot_index + 1;
                        let Some(card_id) = chosen_card else {
                            continue;
                        };
                        let Some(snapshot) = game
                            .object(card_id)
                            .map(|obj| ObjectSnapshot::from_object(obj, game))
                        else {
                            continue;
                        };
                        chosen.push(snapshot);
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
                        ctx.set_tagged_objects(self.progress_tag.clone(), chosen.clone());
                    }
                    // Later entry, replacement and shuffle choices may also suspend.
                    // Preserve the whole accepted selection, including declined slots.
                    pending_selection_progress = Some((
                        progress_key.clone(),
                        ObjectSelectionProgress {
                            next_request,
                            chosen: chosen.clone(),
                        },
                    ));

                    // The authored reveal applies to the complete selected set as
                    // one action, before any of the selected cards move. A pending
                    // attempt rolls it back with the world; resume executes this
                    // same group afresh rather than publishing discarded receipts.
                    if self.reveal && !chosen.is_empty() {
                        let outputs = super::reveal_objects_with_outputs(
                            game,
                            ctx,
                            chosen.clone(),
                            Some(chooser_id),
                            "Reveal searched cards",
                            None,
                        )?;
                        if ctx.decision_maker.awaiting_choice() {
                            return Ok(EffectOutcome::count(0));
                        }
                        if let Some(revealed) = outputs.outcome.chosen_object_memory() {
                            chosen = revealed.to_vec();
                            ctx.set_tagged_objects(self.progress_tag.clone(), chosen.clone());
                            pending_selection_progress = Some((
                                progress_key.clone(),
                                ObjectSelectionProgress {
                                    next_request,
                                    chosen: chosen.clone(),
                                },
                            ));
                        }
                        retained_children.push(outputs);
                    }

                    let mut moved_ids = Vec::new();
                    let mut receipts = Vec::new();
                    let chosen_ids = chosen
                        .iter()
                        .map(|snapshot| snapshot.object_id)
                        .collect::<Vec<_>>();
                    if search_override.is_some() {
                        let (found, published) =
                            exile_found_cards_for_opposition_agent_with_outputs(
                                game,
                                ctx,
                                &chosen_ids,
                                chooser_id,
                            )?;
                        crate::effects::PublishedEffectOutputs::append_distinct(
                            &mut published_entry_outputs,
                            published,
                        );
                        moved_ids = found.moved_ids;
                        receipts = found.receipts;
                        if ctx.decision_maker.awaiting_choice() {
                            return Ok(EffectOutcome::count(0));
                        }
                    } else if self.destination == Zone::Battlefield {
                        let entries = move_to_battlefield_batch_with_options(
                            game,
                            ctx,
                            chosen_ids
                                .iter()
                                .copied()
                                .map(|id| (id, BattlefieldEntryOptions::preserve(false)))
                                .collect(),
                        )?;
                        if ctx.decision_maker.awaiting_choice() {
                            return Ok(EffectOutcome::count(0));
                        }
                        if entries.len() != chosen_ids.len() {
                            return Err(ExecutionError::InternalError(
                                "search entry batch lost a receipt".into(),
                            ));
                        }
                        for (original, entry) in chosen_ids.iter().zip(entries) {
                            match &entry.outcome {
                                BattlefieldEntryOutcome::Moved(id) => moved_ids.push(*id),
                                BattlefieldEntryOutcome::Redirected(change) => {
                                    moved_ids.extend(change.new_object_ids.iter().copied())
                                }
                                BattlefieldEntryOutcome::Prevented => {}
                            }
                            let ((object, receipt), published) =
                                entry.into_zone_receipt_with_outputs();
                            crate::effects::PublishedEffectOutputs::append_distinct(
                                &mut published_entry_outputs,
                                published,
                            );
                            if object != *original {
                                return Err(ExecutionError::InternalError(
                                    "search entry receipt changed original identity".into(),
                                ));
                            }
                            receipts.push((object, receipt));
                        }
                    } else {
                        let opened_batch = game.open_simultaneous_action();
                        let moves = (|| -> Result<(), ExecutionError> {
                            for id in &chosen_ids {
                                let Some(from) = game.object(*id).map(|card| card.zone) else {
                                    continue;
                                };
                                if self.destination == Zone::Library && from == Zone::Library {
                                    moved_ids.push(*id);
                                    continue;
                                }
                                let additional = ctx.additional_replacement_effects_snapshot();
                                let committed =
                                    apply_zone_change_with_context_and_additional_effects_with_outputs(
                                        game,
                                        *id,
                                        from,
                                        self.destination,
                                        ctx.cause.clone(),
                                        ctx,
                                        &additional,
                                    )?;
                                if ctx.decision_maker.awaiting_choice() {
                                    return Ok(());
                                }
                                crate::effects::PublishedEffectOutputs::append_distinct(
                                    &mut published_entry_outputs,
                                    committed.published_outputs,
                                );
                                let receipt = committed.receipt;
                                match &receipt.original {
                                    EventOutcome::Proceed(change) => {
                                        moved_ids.extend(change.new_object_ids.iter().copied())
                                    }
                                    EventOutcome::Replaced => {
                                        let ids = game.take_zone_change_results(*id);
                                        if !ids.is_empty() {
                                            game.record_zone_change_results(*id, ids.clone());
                                        }
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
                        if ctx.decision_maker.awaiting_choice() {
                            return Ok(EffectOutcome::count(0));
                        }
                    }
                    // The put/exile instruction and permission links complete first;
                    // replacement additions precede the subsequent library shuffle.
                    let original = if moved_ids.is_empty() {
                        EffectOutcome::count(0)
                    } else {
                        EffectOutcome::with_objects(moved_ids.clone())
                    }
                    .with_event(search_event);
                    let original = EffectOutcome::aggregate_replacement_outcomes(
                        original,
                        retained_children.iter().map(|child| child.outcome.clone()),
                    );
                    let mut original =
                        crate::effects::CompletedEffectOutputs::aggregate_only(original);
                    original
                        .retain_published_references(std::mem::take(&mut published_entry_outputs));
                    let zone_outputs =
                        finish_zone_change_receipts_with_outputs(game, ctx, original, receipts)?;
                    let outcome = zone_outputs.outcome.clone();
                    retained_children.push(zone_outputs);
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(EffectOutcome::count(0));
                    }
                    let shuffle_action =
                        if self.destination == Zone::Library && search_override.is_none() {
                            let remaining = moved_ids
                                .into_iter()
                                .filter(|id| {
                                    game.object(*id)
                                        .is_some_and(|card| card.zone == Zone::Library)
                                })
                                .collect::<Vec<_>>();
                            super::shuffle_library_action(
                                player_id,
                                &remaining,
                                1,
                                "searched cards put on top after library shuffle",
                            )
                        } else {
                            super::shuffle_library_action(
                                player_id,
                                &[],
                                1,
                                "library shuffled after search",
                            )
                        };
                    let shuffle_outputs =
                        crate::effects::execute_effect_with_outputs(game, &shuffle_action, ctx)?;
                    let shuffle = shuffle_outputs.outcome.clone();
                    retained_children.push(shuffle_outputs);
                    ctx.clear_object_tag(self.progress_tag.as_str());
                    ctx.object_selection_progress.remove(&progress_key);
                    Ok(EffectOutcome::aggregate_with_primary_result(
                        outcome,
                        [shuffle],
                    ))
                },
            )
        })();
        let pending = ctx.decision_maker.awaiting_choice();
        if pending || instruction.is_err() {
            game.restore_execution_checkpoint(checkpoint, pending && instruction.is_ok());
            context_checkpoint.restore(ctx);
        }
        if pending {
            if let Some((key, progress)) = pending_selection_progress {
                ctx.set_tagged_objects(self.progress_tag.clone(), progress.chosen.clone());
                ctx.object_selection_progress.insert(key, progress);
            }
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        instruction.map(|outcome| {
            let mut outputs =
                crate::effects::CompletedEffectOutputs::from_children(retained_children, |_| {
                    outcome
                });
            outputs.retain_published_references(published_entry_outputs);
            outputs
        })
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
