//! Look at top cards effect implementation.

use crate::decisions::context::ViewCardsContext;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::{
    resolve_player_filter, resolve_player_filter_as_chooser, resolve_value,
};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
pub use ironsmith_core::LookAtTopCardsEffect;

/// Effect that looks at the top N cards of a player's library and tags them.
impl EffectExecutor for LookAtTopCardsEffect {
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
        crate::effects::composition::execute_transaction(
            game,
            ctx,
            || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                let player_id = resolve_player_filter(game, &self.player, ctx)?;
                if !self.reveal || ctx.iteration.iterated_player.is_none() {
                    ctx.clear_object_tag(self.tag.as_str());
                }
                let Some(player) = game.player(player_id) else {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                };
                let count = resolve_value(game, &self.count, ctx)?.max(0) as usize;
                if count == 0 {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }

                let top_cards: Vec<_> = player.library.iter().rev().take(count).copied().collect();
                if top_cards.is_empty() {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }

                let viewer = resolve_player_filter_as_chooser(game, &self.viewer, ctx)?;
                let viewers =
                    game.private_information_viewers_for(viewer, crate::zone::Zone::Library);
                game.hydrate_verified_library_replay_view(&top_cards, &viewers, self.reveal);
                let snapshots: Vec<ObjectSnapshot> = top_cards
                    .iter()
                    .filter_map(|&id| {
                        game.object(id)
                            .map(|obj| ObjectSnapshot::from_object(obj, game))
                    })
                    .collect();
                if snapshots.is_empty() {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }

                if self.reveal {
                    let outcome = super::reveal_objects_with_outputs(
                        game,
                        ctx,
                        snapshots,
                        Some(player_id),
                        "Reveal cards from the top of a library",
                        None,
                    )?;
                    if !ctx.decision_maker.awaiting_choice() {
                        ctx.tag_objects_unique(
                            self.tag.clone(),
                            outcome
                                .outcome
                                .chosen_object_memory()
                                .unwrap_or_default()
                                .to_vec(),
                        );
                    }
                    Ok(outcome)
                } else {
                    let viewer = resolve_player_filter_as_chooser(game, &self.viewer, ctx)?;
                    let observed = super::look_at_cards_with_outputs(
                        game,
                        ctx,
                        viewer,
                        player_id,
                        crate::zone::Zone::Library,
                        &top_cards,
                        "Look at cards from the top of a library",
                    )?;
                    ctx.remember_face_down_exile_viewers(&top_cards, viewer);
                    ctx.set_tagged_objects(self.tag.clone(), snapshots);
                    Ok(observed)
                }
            },
        )
    }

    fn is_read_only_simultaneous_player_action(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::DecisionMaker;
    use crate::effect::{ChoiceCount, Effect};
    use crate::effects::{
        ChooseObjectsEffect, ForEachObject, ForPlayersEffect, MoveToZoneEffect, ResolvedTarget,
        SequenceEffect,
    };
    use crate::ids::{CardId, PlayerId};
    use crate::mana::ManaSymbol;
    use crate::tag::TagKey;
    use crate::target::{
        ChooseSpec, ObjectFilter, PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation,
    };
    use crate::test_prelude::*;
    use crate::types::CardType;
    use crate::zone::Zone;

    #[derive(Debug)]
    struct ViewCall {
        viewer: PlayerId,
        subject: PlayerId,
        zone: Zone,
        public: bool,
        cards: Vec<crate::ids::ObjectId>,
    }

    #[derive(Debug, Default)]
    struct CaptureViewDm {
        calls: Vec<ViewCall>,
    }

    impl DecisionMaker for CaptureViewDm {
        fn view_cards(
            &mut self,
            _game: &GameState,
            viewer: PlayerId,
            cards: &[crate::ids::ObjectId],
            ctx: &crate::decisions::context::ViewCardsContext,
        ) {
            self.calls.push(ViewCall {
                viewer,
                subject: ctx.subject,
                zone: ctx.zone,
                public: ctx.public,
                cards: cards.to_vec(),
            });
        }
    }

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    #[test]
    fn each_public_top_card_has_one_distinct_staged_and_committed_event() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        add_cards_to_library(&mut game, alice, 4);
        let parent = game.provenance_graph_mut().alloc_root_event(crate::events::EventKind::CardRevealed);
        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.provenance = parent;
        let outcome = LookAtTopCardsEffect::revealing(PlayerFilter::You, 3, "revealed")
            .execute(&mut game, &mut ctx).unwrap();
        assert_eq!(outcome.events.len(), 3);
        let ids: std::collections::HashSet<_> = outcome.events.iter().map(|event| event.provenance()).collect();
        assert_eq!(ids.len(), 3);
        assert!(!ids.contains(&parent));
        assert!(ids.iter().all(|id| game.provenance_graph().is_descendant_of(*id, parent)));
        let history = &mut game.turn_store.turn_history;
        for event in &outcome.events {
            history.stage_event(event, None, None);
            history.stage_event(event, None, None);
        }
        assert_eq!(history.event_kind_count(crate::events::EventKind::CardRevealed), 3);
        assert_eq!(history.staged_event_records.len(), 3);
        for event in &outcome.events {
            history.record_event(event, None, None);
            history.record_event(event, None, None);
            history.stage_event(event, None, None);
        }
        assert_eq!(history.event_records.len(), 3);
        assert!(history.staged_event_records.is_empty());
        assert_eq!(history.event_kind_count(crate::events::EventKind::CardRevealed), 3);
        assert!(LookAtTopCardsEffect::new(PlayerFilter::You, 3, "private")
            .execute(&mut game, &mut ctx).unwrap().events.is_empty());
        assert!(LookAtTopCardsEffect::revealing(PlayerFilter::You, 0, "empty")
            .execute(&mut game, &mut ctx).unwrap().events.is_empty());
    }

    #[test]
    fn delegated_look_hydrates_only_its_verified_top_set_for_the_named_viewer() {
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let card = crate::cards::CardDefinitionBuilder::new(CardId::from_raw(120_051), "Known top")
            .card_types(vec![CardType::Land]).build();
        for viewer in [alice, bob] {
            let mut game = setup_game();
            for slot in 0..3 {
                game.create_hidden_card_placeholder(alice, Zone::Library, slot, format!("ziffle:old:{slot}"));
            }
            game.queue_verified_hidden_library_epoch(alice, "pile-look".into(), 3, 0, None).unwrap();
            let info = crate::game_state::HiddenCardInfo {
                incarnation: Some(0),
                owner: alice, zone: Zone::Library, slot: 7, commitment: "manifest:7".into(),
                origin_slot: Some(2), origin_commitment: Some("ziffle:pile-look:2".into()),
                public_slot: Some(2), public_commitment: Some("ziffle:pile-look:2".into()),
            };
            assert!(game.queue_verified_hidden_library_replay_opening(&info, &card, bob).unwrap());
            game.shuffle_player_library(alice);
            let source = game.new_object_id();
            let top = *game.player(alice).unwrap().library.last().unwrap();
            let mut effect = LookAtTopCardsEffect::new(PlayerFilter::You, 1, "finite");
            effect.viewer = PlayerFilter::Specific(viewer);
            let mut dm = CaptureViewDm::default();
            let mut ctx = ExecutionContext::new(source, alice, &mut dm);
            effect.execute(&mut game, &mut ctx).unwrap();
            assert_eq!(game.is_hidden_card_placeholder(top), viewer != bob);
            assert!(game.player(alice).unwrap().library.iter().take(2).all(|id| game.is_hidden_card_placeholder(*id)));
            assert_eq!(ctx.get_tagged_all("finite").unwrap().len(), 1);
            assert_eq!(dm.calls.len(), 1);
            assert_eq!(dm.calls[0].viewer, viewer);
            assert!(!dm.calls[0].public);
        }
    }

    #[test]
    fn pending_top_view_rolls_back_tags_and_empty_reexecution_clears_the_pool() {
        #[derive(Default)]
        struct PauseView(bool);
        impl DecisionMaker for PauseView {
            fn view_cards(&mut self, _: &GameState, _: PlayerId, _: &[crate::ids::ObjectId], _: &ViewCardsContext) { self.0 = true; }
            fn awaiting_choice(&self) -> bool { self.0 }
        }
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        add_cards_to_library(&mut game, alice, 3);
        let source = game.new_object_id();
        let mut dm = PauseView::default();
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        LookAtTopCardsEffect::new(PlayerFilter::You, 2, "finite").execute(&mut game, &mut ctx).unwrap();
        assert!(ctx.get_tagged_all("finite").is_none());
        drop(ctx);
        let mut ctx = ExecutionContext::new_default(source, alice);
        LookAtTopCardsEffect::new(PlayerFilter::You, 2, "finite").execute(&mut game, &mut ctx).unwrap();
        assert_eq!(ctx.get_tagged_all("finite").unwrap().len(), 2);
        LookAtTopCardsEffect::new(PlayerFilter::You, 0, "finite").execute(&mut game, &mut ctx).unwrap();
        assert!(ctx.get_tagged_all("finite").is_none());
    }

    #[test]
    fn public_top_reveal_requires_every_identity_and_rolls_back_pending_or_missing_openings() {
        struct Opening { pause: bool, pending: bool, requests: Vec<Vec<crate::ids::ObjectId>>, views: usize }
        impl DecisionMaker for Opening {
            fn decide_objects(&mut self, _: &GameState, context: &crate::decisions::context::SelectObjectsContext) -> Vec<crate::ids::ObjectId> {
                assert_eq!(context.reveal_policy, crate::decisions::context::SelectionRevealPolicy::Public);
                let ids: Vec<_> = context.candidates.iter().map(|candidate| candidate.id).collect();
                assert_eq!(context.min, ids.len());
                assert_eq!(context.max, Some(ids.len()));
                self.requests.push(ids.clone()); self.pending = self.pause; ids
            }
            fn awaiting_choice(&self) -> bool { self.pending }
            fn view_cards(&mut self, game: &GameState, _: PlayerId, cards: &[crate::ids::ObjectId], context: &ViewCardsContext) {
                assert!(context.public);
                assert!(cards.iter().all(|id| !game.is_hidden_card_placeholder(*id)));
                self.views += 1;
            }
        }
        let alice = PlayerId::from_index(0);
        let definition = crate::cards::CardDefinitionBuilder::new(CardId::from_raw(120_052), "Known artifact")
            .card_types(vec![CardType::Artifact]).build();
        for known in [false, true] {
            for pause in [false, true] {
                let mut game = setup_game();
                let ids: Vec<_> = (0..3).map(|slot|
                    game.create_hidden_card_placeholder(alice, Zone::Library, slot, format!("library-{slot}"))).collect();
                if known {
                    for id in &ids[1..] { game.reveal_hidden_card_with_definition(*id, &definition).unwrap(); }
                }
                let source = game.new_object_id();
                let mut dm = Opening { pause, pending: false, requests: vec![], views: 0 };
                let mut ctx = ExecutionContext::new(source, alice, &mut dm);
                let outcome = LookAtTopCardsEffect::revealing(PlayerFilter::You, 2, "public_top")
                    .execute(&mut game, &mut ctx);
                assert!(game.is_hidden_card_placeholder(ids[0]));
                if pause || !known {
                    assert!(ctx.get_tagged_all("public_top").is_none());
                    assert!(game.publicly_revealed_hidden_cards().is_empty());
                    if !pause { assert!(matches!(outcome, Err(ExecutionError::IncompleteEvidence(_)))); }
                } else {
                    assert_eq!(outcome.unwrap().events.len(), 2);
                    assert_eq!(ctx.get_tagged_all("public_top").unwrap().len(), 2);
                }
                drop(ctx);
                assert_eq!(dm.requests, vec![vec![ids[2], ids[1]]]);
                assert_eq!(dm.views, if known && !pause { 2 } else { 0 });
            }
        }
    }

    #[test]
    fn known_top_cards_cannot_publish_reveals_after_a_malformed_opening_answer() {
        struct Incomplete(usize);
        impl DecisionMaker for Incomplete {
            fn decide_objects(&mut self, _: &GameState, context: &crate::decisions::context::SelectObjectsContext) -> Vec<crate::ids::ObjectId> {
                match self.0 {
                    0 => vec![],
                    1 => vec![context.candidates[0].id],
                    _ => vec![crate::ids::ObjectId::from_raw(9_999_999)],
                }
            }
            fn view_cards(&mut self, _: &GameState, _: PlayerId, _: &[crate::ids::ObjectId], _: &ViewCardsContext) {
                panic!("a rejected opening must not publish a public view");
            }
        }
        let alice = PlayerId::from_index(0);
        let definition = crate::cards::CardDefinitionBuilder::new(CardId::from_raw(120_053), "Known private top")
            .card_types(vec![CardType::Land]).build();
        for answer in 0..3 {
            let mut game = setup_game();
            let cards: Vec<_> = (0..2).map(|slot|
                game.create_hidden_card_placeholder(alice, Zone::Library, slot, format!("top-{slot}"))).collect();
            for id in &cards { game.reveal_hidden_card_with_definition(*id, &definition).unwrap(); }
            let source = game.new_object_id();
            let mut dm = Incomplete(answer);
            let mut ctx = ExecutionContext::new(source, alice, &mut dm);
            let result = LookAtTopCardsEffect::revealing(PlayerFilter::You, 2, "finite")
                .execute(&mut game, &mut ctx);
            assert!(matches!(result, Err(ExecutionError::IncompleteEvidence(_))));
            assert!(ctx.get_tagged_all("finite").is_none());
            assert!(game.publicly_revealed_hidden_cards().is_empty());
            assert_eq!(game.player(alice).unwrap().library, cards);
            assert_eq!(game.turn_store.turn_history.event_kind_count(crate::events::EventKind::CardRevealed), 0);
        }
    }

    fn add_cards_to_library(game: &mut GameState, owner: PlayerId, count: usize) {
        for idx in 0..count {
            let card = CardBuilder::new(
                CardId::from_raw(10_000 + idx as u32),
                format!("Library Card {idx}"),
            )
            .build();
            game.create_object_from_card(&card, owner, Zone::Library);
        }
    }

    fn add_named_card_to_library(
        game: &mut GameState,
        owner: PlayerId,
        name: &str,
        id: u32,
    ) -> crate::ids::ObjectId {
        let card = CardBuilder::new(CardId::from_raw(id), name).build();
        game.create_object_from_card(&card, owner, Zone::Library)
    }

    #[derive(Default)]
    struct CaptureSingletonChooser {
        choosers: Vec<PlayerId>,
    }

    impl DecisionMaker for CaptureSingletonChooser {
        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<crate::ids::ObjectId> {
            self.choosers.push(ctx.player);
            ctx.candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .map(|candidate| candidate.id)
                .take(1)
                .collect()
        }
    }

    fn add_typed_card_to_library(
        game: &mut GameState,
        owner: PlayerId,
        name: &str,
        id: u32,
        card_types: Vec<CardType>,
    ) {
        let card = CardBuilder::new(CardId::from_raw(id), name)
            .card_types(card_types)
            .build();
        game.create_object_from_card(&card, owner, Zone::Library);
    }

    #[test]
    fn look_at_top_fixed_count_tags_cards() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        add_cards_to_library(&mut game, alice, 5);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = LookAtTopCardsEffect::new(PlayerFilter::You, 2, "looked");
        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("execute look-at-top");

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        assert_eq!(
            ctx.tagged_objects
                .get(&TagKey::from("looked"))
                .map(|snapshots| snapshots.len()),
            Some(2)
        );
    }

    #[test]
    fn exact_singleton_look_program_moves_one_card_and_leaves_its_sibling() {
        for target_player_library in [false, true] {
            let mut game = setup_game();
            let alice = PlayerId::from_index(0);
            let bob = PlayerId::from_index(1);
            let alice_cards = [
                add_named_card_to_library(&mut game, alice, "Alice Lower", 21_001),
                add_named_card_to_library(&mut game, alice, "Alice Top", 21_002),
            ];
            let bob_cards = [
                add_named_card_to_library(&mut game, bob, "Bob Lower", 21_003),
                add_named_card_to_library(&mut game, bob, "Bob Top", 21_004),
            ];
            let library_owner = if target_player_library { bob } else { alice };
            let watched_cards = if target_player_library {
                bob_cards
            } else {
                alice_cards
            };
            let watched_stable_ids = watched_cards
                .map(|id| game.object(id).expect("looked card should exist").stable_id);
            let other_owner = if target_player_library { alice } else { bob };
            let other_library_before = game
                .player(other_owner)
                .expect("other player should exist")
                .library
                .clone();

            let looked_tag = crate::TagKey::from("looked_pool");
            let selected_tag = crate::TagKey::from("looked_selected");
            let look_player = if target_player_library {
                PlayerFilter::target_player()
            } else {
                PlayerFilter::You
            };
            let owner_filter = if target_player_library {
                PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Any))
            } else {
                PlayerFilter::You
            };
            let program = SequenceEffect::new(vec![
                Effect::look_at_top_cards(look_player, 2, looked_tag.clone()),
                Effect::new(
                    ChooseObjectsEffect::new(
                        ObjectFilter::tagged(looked_tag)
                            .in_zone(Zone::Library)
                            .owned_by(owner_filter),
                        ChoiceCount::exactly(1),
                        PlayerFilter::You,
                        selected_tag.clone(),
                    )
                    .in_zone(Zone::Library),
                ),
                Effect::new(MoveToZoneEffect::new(
                    ChooseSpec::Tagged(selected_tag),
                    Zone::Graveyard,
                    false,
                )),
            ]);
            let source = game.new_object_id();
            let mut dm = CaptureSingletonChooser::default();
            let mut ctx = ExecutionContext::new(source, alice, &mut dm);
            if target_player_library {
                ctx.targets.push(ResolvedTarget::Player(bob));
            }
            program
                .execute(&mut game, &mut ctx)
                .expect("exact singleton looked-card program should resolve");
            drop(ctx);

            assert_eq!(dm.choosers, vec![alice]);
            let resulting_zones = watched_stable_ids.map(|stable_id| {
                let current = game
                    .find_object_by_stable_id(stable_id)
                    .expect("looked card should remain in the game");
                game.object(current)
                    .expect("current card should exist")
                    .zone
            });
            assert_eq!(
                resulting_zones
                    .iter()
                    .filter(|zone| **zone == Zone::Graveyard)
                    .count(),
                1,
                "exactly the selected card should move"
            );
            assert_eq!(
                resulting_zones
                    .iter()
                    .filter(|zone| **zone == Zone::Library)
                    .count(),
                1,
                "the unselected sibling should remain in its library"
            );
            assert_eq!(
                game.player(library_owner)
                    .expect("library owner should exist")
                    .graveyard
                    .len(),
                1
            );
            assert_eq!(
                game.player(other_owner)
                    .expect("other player should exist")
                    .library,
                other_library_before,
                "the unrelated player's library must not be touched"
            );
        }
    }

    #[test]
    fn revealing_top_cards_accumulates_shared_tags_across_each_player() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        add_typed_card_to_library(
            &mut game,
            alice,
            "Alice Revealed Creature",
            20_001,
            vec![CardType::Creature],
        );
        add_typed_card_to_library(
            &mut game,
            bob,
            "Bob Revealed Creature",
            20_002,
            vec![CardType::Creature],
        );

        let tag = TagKey::from("revealed_this_way");
        let mut ctx = ExecutionContext::new_default(source, alice);
        let reveal_each = ForPlayersEffect::new(
            PlayerFilter::Any,
            vec![Effect::reveal_top_cards(
                PlayerFilter::IteratedPlayer,
                1,
                tag.clone(),
            )],
        );
        reveal_each
            .execute(&mut game, &mut ctx)
            .expect("execute each-player reveal");

        assert_eq!(
            ctx.get_tagged_all(&tag).map(|snapshots| snapshots.len()),
            Some(2)
        );

        let mut filter = ObjectFilter::default();
        filter.excluded_card_types.push(CardType::Land);
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: tag.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        let parley_reward = ForEachObject::new(
            filter,
            vec![
                Effect::add_mana(vec![ManaSymbol::Green]),
                Effect::gain_life(1),
            ],
        );
        parley_reward
            .execute(&mut game, &mut ctx)
            .expect("execute parley reward");

        assert_eq!(game.player(alice).expect("alice").mana_pool.green, 2);
        assert_eq!(game.player(alice).expect("alice").life, 22);
    }

    #[test]
    fn look_at_top_x_count_uses_context_x_value() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        add_cards_to_library(&mut game, alice, 6);

        let mut ctx = ExecutionContext::new_default(source, alice).with_x(3);
        let effect = LookAtTopCardsEffect::new(PlayerFilter::You, Value::X, "looked_x");
        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("execute look-at-top");

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(3));
        assert_eq!(
            ctx.tagged_objects
                .get(&TagKey::from("looked_x"))
                .map(|snapshots| snapshots.len()),
            Some(3)
        );
    }

    #[test]
    fn look_at_top_emits_private_view_cards_event() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        add_cards_to_library(&mut game, alice, 4);

        let mut dm = CaptureViewDm::default();
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        let effect = LookAtTopCardsEffect::new(PlayerFilter::You, 2, "looked");
        effect
            .execute(&mut game, &mut ctx)
            .expect("execute look-at-top");

        assert_eq!(dm.calls.len(), 1);
        let call = &dm.calls[0];
        assert_eq!(call.viewer, alice);
        assert_eq!(call.subject, alice);
        assert_eq!(call.zone, Zone::Library);
        assert!(!call.public);
        assert_eq!(call.cards.len(), 2);
    }
}
