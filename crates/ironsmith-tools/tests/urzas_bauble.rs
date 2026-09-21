use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder};
use ironsmith::ids::CardId;
use ironsmith::{CardType, GameState, PlayerId, Zone};
fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Urza's Bauble",
    )
    .unwrap()
    .remove(0)
}
fn definition() -> CardDefinition {
    ironsmith_tools::compile_definition_from_payload(&payload()).unwrap()
}
#[test]
fn strict_snapshot_and_full_quality_gate() {
    let s = ironsmith_tools::compile_authoritative_snapshot_from_payload(&payload());
    assert_eq!(
        s.parse_status,
        ironsmith_tools::ParseStatus::StrictCompiled,
        "{:?}",
        s.parse_error
    );
    assert!(!s.parse_lossy && !s.has_unimplemented && s.parse_error.is_none());
    assert!(
        s.similarity_score >= 0.99,
        "{}: {:?}",
        s.similarity_score,
        s.compiled_text
    );
}

#[test]
fn random_hand_look_is_private_and_schedules_draw_even_for_empty_hand() {
    use ironsmith::decisions::{SelectObjectsContext, ViewCardsContext};
    struct Observe {
        viewed: Vec<(PlayerId, ironsmith::ObjectId)>,
    }
    impl ironsmith::decision::DecisionMaker for Observe {
        fn decide_objects(
            &mut self,
            _: &GameState,
            _: &SelectObjectsContext,
        ) -> Vec<ironsmith::ObjectId> {
            panic!("random selection must not ask a player to choose a card");
        }
        fn view_cards(
            &mut self,
            _: &GameState,
            viewer: PlayerId,
            cards: &[ironsmith::ObjectId],
            ctx: &ViewCardsContext,
        ) {
            assert!(!ctx.public, "a look must not become a public reveal");
            self.viewed.extend(cards.iter().map(|id| (viewer, *id)));
        }
    }
    let def = definition();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let activation = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            ironsmith::ability::AbilityKind::Activated(a) => Some(a),
            _ => None,
        })
        .unwrap();
    let mut outcomes = std::collections::HashSet::new();
    for seed in 1..=16 {
        for empty in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            game.set_random_seed(seed);
            let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
            if !empty {
                for name in ["Island", "Mountain"] {
                    let card = CardDefinitionBuilder::new(CardId::new(), name)
                        .card_types(vec![CardType::Land])
                        .build();
                    game.create_object_from_definition(&card, bob, Zone::Hand);
                }
            }
            let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object(
                game.object(source).unwrap(),
                &game,
            );
            game.push_to_stack(
                ironsmith::game_state::StackEntry::ability(
                    source,
                    alice,
                    activation.effects.clone(),
                )
                .with_source_snapshot(snapshot)
                .with_targets(vec![ironsmith::Target::Player(bob)]),
            );
            game.move_object_by_effect(source, Zone::Graveyard).unwrap();
            let mut dm = Observe { viewed: Vec::new() };
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            // The view and hidden-information bookkeeping notify the same viewer.
            dm.viewed.sort();
            dm.viewed.dedup();
            assert_eq!(
                dm.viewed.len(),
                if empty { 0 } else { 1 },
                "views: {:?}",
                dm.viewed
            );
            if let Some((viewer, id)) = dm.viewed.first() {
                assert_eq!(*viewer, alice);
                outcomes.insert(game.object(*id).unwrap().name.clone());
            }
            assert_eq!(
                game.player(bob).unwrap().hand.len(),
                if empty { 0 } else { 2 }
            );
            assert!(
                game.player(alice).unwrap().hand.is_empty(),
                "draw must be delayed"
            );
            assert_eq!(
                game.effect_store.delayed_triggers.len(),
                1,
                "empty hand must not prevent the delayed draw"
            );
        }
    }
    assert_eq!(outcomes.len(), 2);
}

#[test]
fn delayed_draw_waits_for_next_turn_and_triggers_only_once_for_activator() {
    let def = definition();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for next_player in [alice, bob] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
        for _ in 0..2 {
            let card = CardDefinitionBuilder::new(CardId::new(), "Draw probe")
                .card_types(vec![CardType::Land])
                .build();
            game.create_object_from_definition(&card, alice, Zone::Library);
        }
        let activation = def
            .abilities
            .iter()
            .find_map(|a| match &a.kind {
                ironsmith::ability::AbilityKind::Activated(a) => Some(a),
                _ => None,
            })
            .unwrap();
        let snapshot =
            ironsmith::snapshot::ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
        game.push_to_stack(
            ironsmith::game_state::StackEntry::ability(source, alice, activation.effects.clone())
                .with_source_snapshot(snapshot)
                .with_targets(vec![ironsmith::Target::Player(bob)]),
        );
        game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        let upkeep = |player| {
            ironsmith::triggers::TriggerEvent::new_with_provenance(
                ironsmith::events::BeginningOfUpkeepEvent::new(player),
                Default::default(),
            )
        };
        assert!(
            ironsmith::triggers::check_delayed_triggers(&mut game, &upkeep(alice)).is_empty(),
            "an extra upkeep this turn is too early"
        );
        game.turn.turn_number += 1;
        game.turn.active_player = next_player;
        let triggers = ironsmith::triggers::check_delayed_triggers(&mut game, &upkeep(next_player));
        assert_eq!(triggers.len(), 1);
        assert_eq!(triggers[0].controller, alice);
        assert!(
            game.player(alice).unwrap().hand.is_empty(),
            "draw waits for trigger resolution"
        );
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        for entry in triggers {
            queue.add(entry);
        }
        ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert_eq!(game.player(alice).unwrap().hand.len(), 1);
        assert!(game.player(bob).unwrap().hand.is_empty());
        assert!(
            ironsmith::triggers::check_delayed_triggers(&mut game, &upkeep(next_player)).is_empty()
        );
        game.turn.turn_number += 1;
        assert!(ironsmith::triggers::check_delayed_triggers(&mut game, &upkeep(alice)).is_empty());
    }
}

#[test]
fn activation_taps_and_sacrifices_before_private_look() {
    use ironsmith::decision::{DecisionMaker, LegalAction, compute_legal_actions};
    struct Observer {
        target: PlayerId,
        viewed: Vec<PlayerId>,
    }
    impl DecisionMaker for Observer {
        fn decide_targets(
            &mut self,
            _: &GameState,
            _: &ironsmith::decisions::context::TargetsContext,
        ) -> Vec<ironsmith::Target> {
            vec![ironsmith::Target::Player(self.target)]
        }
        fn view_cards(
            &mut self,
            _: &GameState,
            viewer: PlayerId,
            _: &[ironsmith::ObjectId],
            ctx: &ironsmith::decisions::ViewCardsContext,
        ) {
            assert!(!ctx.public);
            self.viewed.push(viewer);
        }
    }
    let def = definition();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = bob;
    game.turn.priority_player = Some(alice);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let card = CardDefinitionBuilder::new(CardId::new(), "Island")
        .card_types(vec![CardType::Land])
        .build();
    game.create_object_from_definition(&card, bob, Zone::Hand);
    let action_for = |game: &GameState, player| {
        compute_legal_actions(game, player)
            .into_iter()
            .find(|a| matches!(a, LegalAction::ActivateAbility { source: id, .. } if *id == source))
    };
    assert!(action_for(&game, bob).is_none());
    game.tap(source);
    assert!(action_for(&game, alice).is_none());
    game.untap(source);
    let action = action_for(&game, alice).unwrap();
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut state = ironsmith::game_loop::PriorityLoopState::new(game.players_in_game());
    let mut dm = Observer {
        target: bob,
        viewed: Vec::new(),
    };
    let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(
        &mut game,
        &mut queue,
        &mut state,
        &ironsmith::game_loop::PriorityResponse::PriorityAction(action),
        &mut dm,
    )
    .unwrap();
    for _ in 0..24 {
        if !game.stack.is_empty() {
            break;
        }
        let ironsmith::decision::GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}");
        };
        progress = ironsmith::game_loop::apply_decision_context_with_dm(
            &mut game, &mut queue, &mut state, &ctx, &mut dm,
        )
        .unwrap();
    }
    assert_eq!(game.stack.len(), 1);
    assert!(!game.battlefield.contains(&source));
    assert_eq!(game.player(alice).unwrap().graveyard.len(), 1);
    assert!(
        dm.viewed.is_empty(),
        "look happens on resolution, after sacrifice"
    );
    assert!(game.effect_store.delayed_triggers.is_empty());
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    assert!(!dm.viewed.is_empty());
    assert!(dm.viewed.iter().all(|viewer| *viewer == alice));
    assert_eq!(game.player(bob).unwrap().hand.len(), 1);
    assert_eq!(game.effect_store.delayed_triggers.len(), 1);
}
