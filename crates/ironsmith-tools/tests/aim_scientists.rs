//! Frozen atlas observation 25063051: verify existing connive support on the full card.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::ids::CardId;
use ironsmith::object::CounterType;
use ironsmith::triggers::{TriggerQueue, check_triggers};
use ironsmith::{AbilityKind, CardDefinition, CardType, GameState, PlayerId, Zone};
use ironsmith_tools::{
    ParseStatus, compile_authoritative_snapshot_from_payload, compile_definition_from_payload,
    default_cards_path, load_card_payloads_by_name,
};

fn inspector() -> CardDefinition {
    let payloads =
        load_card_payloads_by_name(default_cards_path().to_str().unwrap(), "A.I.M. Scientists")
            .unwrap();
    assert_eq!(payloads.len(), 1);
    let snapshot = compile_authoritative_snapshot_from_payload(&payloads[0]);
    assert_eq!(
        snapshot.parse_status,
        ParseStatus::StrictCompiled,
        "{snapshot:#?}"
    );
    assert!(!snapshot.has_unimplemented && !snapshot.parse_lossy);
    compile_definition_from_payload(&payloads[0]).unwrap()
}

#[test]
fn aim_scientists_connives_only_itself_and_counts_nonland_discards() {
    let definition = inspector();
    assert_eq!(
        definition
            .abilities
            .iter()
            .filter(|a| matches!(a.kind, AbilityKind::Triggered(_)))
            .count(),
        1
    );
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for discard_land in [true, false] {
        for leave_before_resolution in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let first = CardDefinitionBuilder::new(CardId::new(), "First discard")
                .card_types(vec![if discard_land {
                    CardType::Land
                } else {
                    CardType::Instant
                }])
                .build();
            let drawn = CardDefinitionBuilder::new(CardId::new(), "Drawn card")
                .card_types(vec![CardType::Instant])
                .build();
            let discard = game.create_object_from_definition(&first, alice, Zone::Hand);
            game.create_object_from_definition(&drawn, alice, Zone::Library);
            game.create_object_from_definition(&drawn, bob, Zone::Library);
            let bystander =
                game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            let hand = game.create_object_from_definition(&definition, alice, Zone::Hand);
            let source = game
                .move_object_with_etb_processing(hand, Zone::Battlefield)
                .unwrap()
                .new_id;
            assert!(!game.object_has_static_ability_id(
                source,
                ironsmith::static_abilities::StaticAbilityId::Flying
            ));
            let mut queue = TriggerQueue::new();
            for event in game.take_pending_trigger_events() {
                for entry in check_triggers(&game, &event) {
                    queue.add(entry);
                }
            }
            assert_eq!(queue.entries.len(), 1);
            let departed = leave_before_resolution
                .then(|| game.move_object_by_effect(source, Zone::Graveyard).unwrap());
            ironsmith::game_loop::run_priority_loop_with(
                &mut game,
                &mut queue,
                &mut SelectFirstDecisionMaker,
            )
            .unwrap();
            assert!(game.player(alice).unwrap().library.is_empty());
            assert_eq!(game.player(alice).unwrap().hand.len(), 1);
            assert_eq!(game.player(bob).unwrap().library.len(), 1);
            assert!(game.object(discard).is_none());
            let bystander_counters = game
                .object(bystander)
                .unwrap()
                .counters
                .get(&CounterType::PlusOnePlusOne)
                .copied()
                .unwrap_or(0);
            assert_eq!(bystander_counters, 0);
            if let Some(departed) = departed {
                assert_eq!(game.object(departed).unwrap().zone, Zone::Graveyard);
                assert_eq!(
                    game.object(departed)
                        .unwrap()
                        .counters
                        .get(&CounterType::PlusOnePlusOne)
                        .copied()
                        .unwrap_or(0),
                    0
                );
            } else {
                assert_eq!(
                    game.object(source)
                        .unwrap()
                        .counters
                        .get(&CounterType::PlusOnePlusOne)
                        .copied()
                        .unwrap_or(0),
                    u32::from(!discard_land)
                );
            }
        }
    }
}

#[test]
fn aim_scientists_connive_uses_current_or_last_controller() {
    let definition = inspector();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let filler = CardDefinitionBuilder::new(CardId::new(), "Controller draw")
        .card_types(vec![CardType::Instant])
        .build();
    for depart in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        for player in [alice, bob] {
            game.create_object_from_definition(&filler, player, Zone::Hand);
            game.create_object_from_definition(&filler, player, Zone::Library);
        }
        let hand = game.create_object_from_definition(&definition, alice, Zone::Hand);
        let source = game
            .move_object_with_etb_processing(hand, Zone::Battlefield)
            .unwrap()
            .new_id;
        let mut queue = TriggerQueue::new();
        for event in game.take_pending_trigger_events() {
            for entry in check_triggers(&game, &event) {
                queue.add(entry);
            }
        }
        assert_eq!(queue.entries.len(), 1);
        game.set_current_controller(source, bob);
        if depart {
            game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        }
        ironsmith::game_loop::run_priority_loop_with(
            &mut game,
            &mut queue,
            &mut SelectFirstDecisionMaker,
        )
        .unwrap();
        assert_eq!(
            game.player(alice).unwrap().library.len(),
            1,
            "ability controller must not draw instead of the conniving creature's controller; departed={depart}"
        );
        assert!(
            game.player(bob).unwrap().library.is_empty(),
            "current/last controller must draw; departed={depart}"
        );
    }
}

#[test]
fn aim_scientists_original_entry_does_not_connive_a_later_incarnation() {
    let definition = inspector();
    let alice = PlayerId::from_index(0);
    let filler = CardDefinitionBuilder::new(CardId::new(), "Discard for old entry")
        .card_types(vec![CardType::Instant])
        .build();
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.create_object_from_definition(&filler, alice, Zone::Hand);
    game.create_object_from_definition(&filler, alice, Zone::Library);
    let hand = game.create_object_from_definition(&definition, alice, Zone::Hand);
    let source = game
        .move_object_with_etb_processing(hand, Zone::Battlefield)
        .unwrap()
        .new_id;
    let mut queue = TriggerQueue::new();
    for event in game.take_pending_trigger_events() {
        game.turn_store
            .turn_history
            .record_event(&event, None, None);
        for entry in check_triggers(&game, &event) {
            queue.add(entry);
        }
    }
    assert_eq!(queue.entries.len(), 1);
    let exiled = game.move_object_by_effect(source, Zone::Exile).unwrap();
    let returned = game
        .move_object_with_etb_processing(exiled, Zone::Battlefield)
        .unwrap()
        .new_id;
    // Isolate the first entry's ability. The new entry has its own independent
    // trigger, already covered by the entry test; retain zone history for LKI.
    for event in game.take_pending_trigger_events() {
        game.turn_store
            .turn_history
            .record_event(&event, None, None);
    }
    ironsmith::game_loop::run_priority_loop_with(
        &mut game,
        &mut queue,
        &mut SelectFirstDecisionMaker,
    )
    .unwrap();
    assert!(game.player(alice).unwrap().library.is_empty());
    assert_eq!(game.player(alice).unwrap().hand.len(), 1);
    assert_eq!(
        game.object(returned)
            .unwrap()
            .counters
            .get(&CounterType::PlusOnePlusOne)
            .copied()
            .unwrap_or(0),
        0,
        "the old entry's connive must not put a counter on a later incarnation"
    );
}

#[test]
fn aim_scientists_basic_landcycling_pays_discards_searches_reveals_and_shuffles() {
    use ironsmith::ObjectId;
    use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
    use ironsmith::decisions::context::{SelectObjectsContext, ViewCardsContext};
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse};
    use ironsmith::game_state::Phase;
    use ironsmith::mana::ManaSymbol;
    use ironsmith::types::{Subtype, Supertype};
    struct SearchDm {
        decline: bool,
        public: Vec<ObjectId>,
    }
    impl DecisionMaker for SearchDm {
        fn decide_objects(
            &mut self,
            game: &GameState,
            ctx: &SelectObjectsContext,
        ) -> Vec<ObjectId> {
            let search = ctx
                .candidates
                .iter()
                .any(|c| game.object(c.id).is_some_and(|o| o.zone == Zone::Library));
            if search {
                for c in ctx.candidates.iter().filter(|c| c.legal) {
                    let object = game.object(c.id).unwrap();
                    assert!(
                        object.has_card_type(CardType::Land)
                            && object.supertypes.contains(&Supertype::Basic)
                    );
                }
                if self.decline {
                    return vec![];
                }
            }
            ctx.candidates
                .iter()
                .filter(|c| c.legal)
                .map(|c| c.id)
                .take(1)
                .collect()
        }
        fn view_cards(
            &mut self,
            _: &GameState,
            _: PlayerId,
            cards: &[ObjectId],
            ctx: &ViewCardsContext,
        ) {
            if ctx.public {
                self.public.extend_from_slice(cards);
            }
        }
    }
    let definition = inspector();
    let activated = definition
        .abilities
        .iter()
        .find(|a| matches!(a.kind, AbilityKind::Activated(_)))
        .unwrap();
    assert_eq!(activated.functional_zones, [Zone::Hand]);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for (has_basic, decline) in [(true, false), (true, true), (false, false)] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.phase = Phase::FirstMain;
        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        let source = game.create_object_from_definition(&definition, alice, Zone::Hand);
        let competing = game.create_object_from_definition(&definition, alice, Zone::Hand);
        let basic = CardDefinitionBuilder::new(CardId::new(), "Snow-covered basic fixture")
            .card_types(vec![CardType::Land])
            .supertypes(vec![Supertype::Basic, Supertype::Snow])
            .subtypes(vec![Subtype::Island])
            .build();
        let nonbasic = CardDefinitionBuilder::new(CardId::new(), "Nonbasic Island fixture")
            .card_types(vec![CardType::Land])
            .subtypes(vec![Subtype::Island])
            .build();
        let filler = CardDefinitionBuilder::new(CardId::new(), "Library filler")
            .card_types(vec![CardType::Instant])
            .build();
        game.create_object_from_definition(&nonbasic, alice, Zone::Library);
        game.create_object_from_definition(&filler, alice, Zone::Library);
        let found =
            has_basic.then(|| game.create_object_from_definition(&basic, alice, Zone::Library));
        let opponents_basic = game.create_object_from_definition(&basic, bob, Zone::Library);
        let offered = |game: &GameState| {
            compute_legal_actions(game, alice).into_iter().find(
                |a| matches!(a, LegalAction::ActivateAbility { source: id, .. } if *id == source),
            )
        };
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 1);
        // The menu permits opening the mana window before payment. Exercise
        // that window: with no other mana source the cost cannot finish.
        let mut insufficient = game.clone();
        let mut insufficient_queue = TriggerQueue::new();
        let mut insufficient_state = PriorityLoopState::new(insufficient.players_in_game());
        let mut insufficient_dm = SearchDm {
            decline,
            public: vec![],
        };
        let mut attempt = ironsmith::game_loop::apply_priority_response_with_dm(
            &mut insufficient,
            &mut insufficient_queue,
            &mut insufficient_state,
            &PriorityResponse::PriorityAction(offered(&game).unwrap()),
            &mut insufficient_dm,
        );
        for _ in 0..20 {
            let Ok(GameProgress::NeedsDecisionCtx(ctx)) = attempt else {
                break;
            };
            attempt = ironsmith::game_loop::apply_decision_context_with_dm(
                &mut insufficient,
                &mut insufficient_queue,
                &mut insufficient_state,
                &ctx,
                &mut insufficient_dm,
            );
        }
        assert!(
            insufficient.stack.is_empty(),
            "one mana cannot finish payment"
        );
        assert!(
            insufficient.player(alice).unwrap().hand.contains(&source),
            "failed mana payment must not discard the source"
        );
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 1);
        let action = offered(&game).expect("search need not find a card to activate");
        let mut queue = TriggerQueue::new();
        let mut state = PriorityLoopState::new(game.players_in_game());
        let mut dm = SearchDm {
            decline,
            public: vec![],
        };
        let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(
            &mut game,
            &mut queue,
            &mut state,
            &PriorityResponse::PriorityAction(action),
            &mut dm,
        )
        .unwrap();
        for _ in 0..20 {
            if !game.stack.is_empty() {
                break;
            }
            let GameProgress::NeedsDecisionCtx(ctx) = progress else {
                panic!("{progress:?}")
            };
            progress = ironsmith::game_loop::apply_decision_context_with_dm(
                &mut game, &mut queue, &mut state, &ctx, &mut dm,
            )
            .unwrap();
        }
        assert_eq!(game.stack.len(), 1);
        assert!(game.object(source).is_none());
        assert_eq!(game.player(alice).unwrap().graveyard.len(), 1);
        assert_eq!(game.player(alice).unwrap().hand.as_slice(), &[competing]);
        assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
        let remaining = game
            .player(alice)
            .unwrap()
            .library
            .iter()
            .copied()
            .filter(|id| !(has_basic && !decline && Some(*id) == found))
            .collect::<Vec<_>>();
        let expected_shuffle = remaining.iter().copied().rev().collect::<Vec<_>>();
        game.queue_transcript_library_shuffle_order(alice, remaining, expected_shuffle.clone());
        let grave_source = game.player(alice).unwrap().graveyard[0];
        game.move_object_by_effect(grave_source, Zone::Exile)
            .unwrap();
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.player(alice).unwrap().library, expected_shuffle);
        assert_eq!(game.player(bob).unwrap().library, vec![opponents_basic]);
        assert!(game.player(alice).unwrap().hand.contains(&competing));
        assert_eq!(
            game.player(alice).unwrap().hand.len(),
            if has_basic && !decline { 2 } else { 1 }
        );
        if has_basic && !decline {
            assert!(
                dm.public.contains(&found.unwrap()),
                "found land must be publicly revealed"
            );
            let drawn = game
                .player(alice)
                .unwrap()
                .hand
                .iter()
                .find(|id| **id != competing)
                .unwrap();
            assert!(
                game.object(*drawn)
                    .unwrap()
                    .supertypes
                    .contains(&Supertype::Basic)
            );
        } else {
            assert!(dm.public.is_empty());
        }
    }
}

#[test]
fn aim_scientists_landcycling_is_available_only_from_hand() {
    use ironsmith::decision::{LegalAction, compute_legal_actions};
    use ironsmith::mana::ManaSymbol;
    let definition = inspector();
    let alice = PlayerId::from_index(0);
    for zone in [
        Zone::Hand,
        Zone::Battlefield,
        Zone::Graveyard,
        Zone::Exile,
        Zone::Library,
        Zone::Command,
    ] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.priority_player = Some(alice);
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 2);
        let source = game.create_object_from_definition(&definition, alice, zone);
        let offered = compute_legal_actions(&game, alice)
            .iter()
            .any(|a| matches!(a, LegalAction::ActivateAbility {source: id, ..} if *id == source));
        assert_eq!(offered, zone == Zone::Hand, "{zone:?}");
    }
}
