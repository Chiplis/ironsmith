use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder};
use ironsmith::ids::CardId;
use ironsmith::{CardType, GameState, PlayerId, Zone};
fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Cursed Scroll",
    )
    .unwrap()
    .remove(0)
}
fn definition() -> CardDefinition {
    ironsmith_tools::compile_definition_from_payload(&payload()).unwrap()
}
struct NameChoice(&'static str);
impl ironsmith::decision::DecisionMaker for NameChoice {
    fn decide_text(&mut self, _: &GameState, _: &ironsmith::decisions::TextInputContext) -> String {
        self.0.into()
    }
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
fn matching_nonmatching_and_empty_hand_resolution_after_source_leaves() {
    let def = definition();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for leave in [false, true] {
        for (hand_name, expected) in [(Some("Island"), 18), (Some("Mountain"), 20), (None, 20)] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
            if let Some(name) = hand_name {
                let hand = CardDefinitionBuilder::new(CardId::new(), name)
                    .card_types(vec![CardType::Land])
                    .build();
                game.create_object_from_definition(&hand, alice, Zone::Hand);
            }
            let activation = def
                .abilities
                .iter()
                .find_map(|ability| match &ability.kind {
                    ironsmith::ability::AbilityKind::Activated(a) => Some(a),
                    _ => None,
                })
                .unwrap();
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
            if leave {
                game.move_object_by_effect(source, Zone::Graveyard).unwrap();
            }
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut NameChoice("Island"))
                .unwrap();
            assert_eq!(
                game.player(bob).unwrap().life,
                expected,
                "{hand_name:?}, source left={leave}"
            );
            assert_eq!(
                game.player(alice).unwrap().hand.len(),
                usize::from(hand_name.is_some())
            );
        }
    }
}

#[test]
fn repeated_activations_use_only_the_name_chosen_for_that_resolution() {
    let def = definition();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let hand = CardDefinitionBuilder::new(CardId::new(), "Island")
        .card_types(vec![CardType::Land])
        .build();
    game.create_object_from_definition(&hand, alice, Zone::Hand);
    let activation = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            ironsmith::ability::AbilityKind::Activated(a) => Some(a),
            _ => None,
        })
        .unwrap();
    for (name, expected) in [("Island", 18), ("Mountain", 18), ("Island", 16)] {
        game.push_to_stack(
            ironsmith::game_state::StackEntry::ability(source, alice, activation.effects.clone())
                .with_targets(vec![ironsmith::Target::Player(bob)]),
        );
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut NameChoice(name)).unwrap();
        assert_eq!(game.player(bob).unwrap().life, expected, "chosen {name}");
    }
}

#[test]
fn reveal_uses_random_selection_and_damage_follows_the_publicly_revealed_card() {
    use ironsmith::decisions::{SelectObjectsContext, TextInputContext, ViewCardsContext};
    struct Observe {
        revealed: Vec<ironsmith::ObjectId>,
    }
    impl ironsmith::decision::DecisionMaker for Observe {
        fn decide_text(&mut self, _: &GameState, _: &TextInputContext) -> String {
            "Island".into()
        }
        fn decide_objects(
            &mut self,
            _: &GameState,
            _: &SelectObjectsContext,
        ) -> Vec<ironsmith::ObjectId> {
            panic!("random reveal must not ask a player to select the card");
        }
        fn view_cards(
            &mut self,
            _: &GameState,
            _: PlayerId,
            cards: &[ironsmith::ObjectId],
            ctx: &ViewCardsContext,
        ) {
            assert!(ctx.public);
            self.revealed.extend_from_slice(cards);
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
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.set_random_seed(seed);
        let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
        for name in ["Island", "Mountain"] {
            let card = CardDefinitionBuilder::new(CardId::new(), name)
                .card_types(vec![CardType::Land])
                .build();
            game.create_object_from_definition(&card, alice, Zone::Hand);
        }
        let before_random = game.irreversible_random_count();
        let mut dm = Observe {
            revealed: Vec::new(),
        };
        game.push_to_stack(
            ironsmith::game_state::StackEntry::ability(source, alice, activation.effects.clone())
                .with_targets(vec![ironsmith::Target::Player(bob)]),
        );
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert!(game.irreversible_random_count() > before_random);
        dm.revealed.sort();
        dm.revealed.dedup();
        assert_eq!(dm.revealed.len(), 1);
        let name = game.object(dm.revealed[0]).unwrap().name.to_string();
        assert_eq!(
            game.player(bob).unwrap().life,
            if name == "Island" { 18 } else { 20 }
        );
        assert_eq!(game.player(alice).unwrap().hand.len(), 2);
        outcomes.insert(name);
    }
    assert_eq!(
        outcomes.len(),
        2,
        "fixed seeds must exercise both reveal outcomes"
    );
}

#[test]
fn activation_requires_three_mana_and_untapped_source_at_instant_speed() {
    use ironsmith::decision::{LegalAction, compute_legal_actions};
    let def = definition();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for mana in [0, 2, 3] {
        for tapped in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            game.turn.active_player = bob;
            game.turn.priority_player = Some(alice);
            game.turn.phase = ironsmith::game_state::Phase::FirstMain;
            let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
            game.player_mut(alice)
                .unwrap()
                .mana_pool
                .add(ironsmith::mana::ManaSymbol::Colorless, mana);
            if tapped {
                game.tap(source);
            }
            let available = |player| {
                compute_legal_actions(&game, player).iter().any(|a| matches!(a, LegalAction::ActivateAbility { source: id, .. } if *id == source))
            };
            // Legal actions deliberately expose activations before mana is
            // floated; affordability belongs to the cost-payment API.
            assert_eq!(available(alice), !tapped);
            let activation = def
                .abilities
                .iter()
                .find_map(|ability| match &ability.kind {
                    ironsmith::ability::AbilityKind::Activated(a) => Some(a),
                    _ => None,
                })
                .unwrap();
            assert_eq!(
                ironsmith::cost::can_pay_cost(&game, source, alice, &activation.mana_cost).is_ok(),
                mana >= 3 && !tapped,
                "mana={mana}, tapped={tapped}"
            );
            assert!(!available(bob));
        }
    }
}

#[test]
fn activation_pays_costs_before_name_choice_and_illegal_target_fizzles() {
    use ironsmith::decision::{DecisionMaker, LegalAction, compute_legal_actions};
    struct Choices {
        target: ironsmith::ObjectId,
        names: usize,
    }
    impl DecisionMaker for Choices {
        fn decide_targets(
            &mut self,
            _: &GameState,
            _: &ironsmith::decisions::context::TargetsContext,
        ) -> Vec<ironsmith::Target> {
            vec![ironsmith::Target::Object(self.target)]
        }
        fn decide_text(
            &mut self,
            _: &GameState,
            _: &ironsmith::decisions::TextInputContext,
        ) -> String {
            self.names += 1;
            "Island".into()
        }
    }
    let def = definition();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for target_leaves in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = bob;
        game.turn.priority_player = Some(alice);
        game.turn.phase = ironsmith::game_state::Phase::FirstMain;
        let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
        let creature = CardDefinitionBuilder::new(CardId::new(), "Target probe")
            .card_types(vec![CardType::Creature])
            .power_toughness(ironsmith::card::PowerToughness::fixed(3, 3))
            .build();
        let target = game.create_object_from_definition(&creature, bob, Zone::Battlefield);
        let hand = CardDefinitionBuilder::new(CardId::new(), "Island")
            .card_types(vec![CardType::Land])
            .build();
        game.create_object_from_definition(&hand, alice, Zone::Hand);
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ironsmith::mana::ManaSymbol::Colorless, 3);
        let action = compute_legal_actions(&game, alice)
            .into_iter()
            .find(|a| matches!(a, LegalAction::ActivateAbility { source: id, .. } if *id == source))
            .unwrap();
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let mut state = ironsmith::game_loop::PriorityLoopState::new(game.players_in_game());
        let mut choices = Choices { target, names: 0 };
        let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(
            &mut game,
            &mut queue,
            &mut state,
            &ironsmith::game_loop::PriorityResponse::PriorityAction(action),
            &mut choices,
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
                &mut game,
                &mut queue,
                &mut state,
                &ctx,
                &mut choices,
            )
            .unwrap();
        }
        assert_eq!(game.stack.len(), 1);
        assert!(game.is_tapped(source));
        assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
        assert_eq!(choices.names, 0, "name choice belongs to resolution");
        if target_leaves {
            game.move_object_by_effect(target, Zone::Graveyard).unwrap();
        }
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut choices).unwrap();
        assert_eq!(choices.names, usize::from(!target_leaves));
        if !target_leaves {
            assert_eq!(game.damage_on(target), 2);
        }
    }
}
