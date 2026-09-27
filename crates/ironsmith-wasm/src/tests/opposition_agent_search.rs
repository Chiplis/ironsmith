use super::*;
use ironsmith::ability::Ability;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::filter::ObjectFilter;
use ironsmith::game_state::{Phase, StackEntry};
use ironsmith::ids::CardId;
use ironsmith::static_abilities::StaticAbility;
use ironsmith::types::CardType;
use ironsmith_registry_test::cards::definitions::ornithopter;

#[test]
fn opposition_agent_replay_exposes_both_replacements_to_search_controller() {
    let _guard = crate::test_id_counter_guard();
    let mut wasm = WasmGame::new();
    wasm.initialize_empty_match(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20, 1);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    let definition = CardDefinitionBuilder::new(CardId::new(), "Opposition Agent")
        .card_types(vec![CardType::Creature])
        .power_toughness(ironsmith::card::PowerToughness::fixed(3, 2))
        .with_ability(Ability::static_ability(
            StaticAbility::control_opponents_while_searching_libraries(),
        ))
        .with_ability(Ability::static_ability(
            StaticAbility::opponent_search_exile_found_cards(),
        ))
        .build();
    let older = wasm
        .game
        .create_object_from_definition(&definition, bob, Zone::Battlefield);
    let newer = wasm
        .game
        .create_object_from_definition(&definition, charlie, Zone::Battlefield);
    let found = wasm
        .game
        .create_object_from_definition(&ornithopter(), alice, Zone::Library);
    let found_stable = wasm.game.object(found).unwrap().stable_id;
    wasm.game
        .create_object_from_definition(&ornithopter(), alice, Zone::Library);
    wasm.game.push_to_stack(StackEntry::ability(
        older,
        alice,
        vec![ironsmith::effect::Effect::search_library_to_hand(
            ObjectFilter::default(),
            false,
        )],
    ));
    wasm.game.turn.active_player = alice;
    wasm.game.turn.phase = Phase::FirstMain;
    wasm.game.turn.priority_player = Some(charlie);
    wasm.priority_state.restore_priority_tracker_for_sync(2, 3);
    let checkpoint = wasm.capture_replay_checkpoint();
    let root = ReplayRoot::Response(PriorityResponse::PriorityAction(LegalAction::PassPriority));
    let outcome = wasm.execute_with_replay(&checkpoint, &root, &[]).unwrap();
    let ReplayOutcome::NeedsDecision(DecisionContext::SelectObjects(search)) = outcome else {
        panic!("expected library search, got {outcome:?}");
    };
    assert_eq!(wasm.game.controlling_player_for(search.player), charlie);

    let answers = vec![ReplayDecisionAnswer::Objects(vec![found])];
    let outcome = wasm
        .execute_with_replay(&checkpoint, &root, &answers)
        .unwrap();
    let ReplayOutcome::NeedsDecision(DecisionContext::SelectOptions(replacements)) = outcome else {
        panic!("expected competing exile effects, got {outcome:?}");
    };
    assert_eq!(
        wasm.game.controlling_player_for(replacements.player),
        charlie
    );
    assert_eq!(
        replacements.description,
        "Choose which replacement effect to apply"
    );
    assert_eq!(replacements.options.len(), 2);
    assert_eq!(replacements.options[0].object_id, Some(older));
    assert_eq!(replacements.options[1].object_id, Some(newer));
    assert!(replacements.options[0].description.contains("Bob may play"));
    assert!(
        replacements.options[1]
            .description
            .contains("Charlie may play")
    );
    assert_eq!(wasm.game.object(found).unwrap().zone, Zone::Library);

    for selected in [0, 1] {
        let mut completed = answers.clone();
        completed.push(ReplayDecisionAnswer::Options(vec![selected]));
        let outcome = wasm
            .execute_with_replay(&checkpoint, &root, &completed)
            .unwrap();
        assert!(matches!(outcome, ReplayOutcome::Complete(_)), "{outcome:?}");
        let exiled = wasm.game.find_object_by_stable_id(found_stable).unwrap();
        assert_eq!(wasm.game.object(exiled).unwrap().zone, Zone::Exile);
        assert!(wasm.game.player(alice).unwrap().hand.is_empty());
        for player in [alice, bob, charlie] {
            assert_eq!(
                wasm.game
                    .effect_store
                    .grant_registry
                    .card_can_play_from_zone(&wasm.game, exiled, Zone::Exile, player),
                player == if selected == 0 { bob } else { charlie },
            );
        }
        assert_eq!(wasm.game.controlling_player_for(alice), alice);
    }
}

#[test]
fn local_cleanup_preserves_discard_choice_when_perspective_controls_opponent() {
    let _guard = crate::test_id_counter_guard();
    let mut wasm = WasmGame::new();
    wasm.initialize_empty_match(vec!["Alice".into(), "Bob".into()], 20, 1);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    wasm.perspective = alice;
    wasm.auto_cleanup_discard = true;
    wasm.game.turn.active_player = bob;
    wasm.game.turn.phase = Phase::Ending;
    wasm.game.turn.step = Some(ironsmith::game_state::Step::Cleanup);
    wasm.game.turn.priority_player = None;
    wasm.game.add_player_control(
        alice,
        bob,
        ironsmith::game_state::PlayerControlStart::Immediate,
        ironsmith::game_state::PlayerControlDuration::WholeTurn,
        None,
    );
    let mut hand = Vec::new();
    for index in 0..8 {
        let card = CardDefinitionBuilder::new(CardId::new(), format!("Cleanup Card {index}"))
            .card_types(vec![CardType::Artifact])
            .build();
        hand.push(
            wasm.game
                .create_object_from_definition(&card, bob, Zone::Hand),
        );
    }
    wasm.runner = Some(ironsmith::turn_runner::TurnRunner::from_state_for_sync(
        ironsmith::turn_runner::TurnState::CleanupDiscard,
    ));
    wasm.runner_awaiting_priority = false;
    wasm.advance_until_decision().unwrap();

    assert_eq!(wasm.game.player(bob).unwrap().hand.len(), 8);
    assert!(wasm.game.player(bob).unwrap().graveyard.is_empty());
    let Some(DecisionContext::SelectObjects(ref ctx)) = wasm.pending_decision else {
        panic!("Alice must choose Bob's cleanup discard");
    };
    assert_eq!(ctx.candidates.len(), 8);
    assert_eq!(ctx.min, 1);
    assert_eq!(wasm.game.controlling_player_for(ctx.player), alice);

    let chosen = hand[6];
    let chosen_stable = wasm.game.object(chosen).unwrap().stable_id;
    let pending = wasm.pending_decision.take().unwrap();
    wasm.apply_runner_decision(
        pending,
        UiCommand::SelectObjects {
            object_ids: vec![chosen.0],
            object_stable_ids: Vec::new(),
            object_hidden_refs: Vec::new(),
        },
    )
    .unwrap();
    assert_eq!(wasm.game.player(bob).unwrap().hand.len(), 7);
    let discarded = wasm.game.find_object_by_stable_id(chosen_stable).unwrap();
    assert_eq!(wasm.game.object(discarded).unwrap().zone, Zone::Graveyard);
    assert_eq!(wasm.game.player(bob).unwrap().graveyard, vec![discarded]);
}

#[test]
fn replay_option_validation_respects_legal_modes_points_and_repeat_limits() {
    use ironsmith::decisions::context::{SelectOptionsContext, SelectableOption};
    let options = SelectOptionsContext::new(
        PlayerId::from_index(0),
        None,
        "Choose three mode points",
        vec![
            SelectableOption::new(0, "Three-point mode").with_point_cost(3),
            SelectableOption::new(1, "Repeatable one-point mode").with_repeatability(true, Some(2)),
            SelectableOption::with_legality(2, "Unavailable mode", false),
            SelectableOption::new(3, "One-point mode"),
        ],
        3,
        3,
    );
    assert!(validate_replay_option_selection(&options, &[0]).is_ok());
    assert!(validate_replay_option_selection(&options, &[1, 1, 3]).is_ok());
    assert!(validate_replay_option_selection(&options, &[1, 1]).is_err());
    assert!(validate_replay_option_selection(&options, &[0, 3]).is_err());
    assert!(validate_replay_option_selection(&options, &[1, 1, 1]).is_err());
    assert!(validate_replay_option_selection(&options, &[3, 3, 3]).is_err());
    assert!(validate_replay_option_selection(&options, &[1, 2, 3]).is_err());
    assert!(validate_replay_option_selection(&options, &[99]).is_err());
}

#[test]
fn runner_attack_tax_colors_command_keeps_selected_blue_mana() {
    use ironsmith::color::Color;
    use ironsmith::cost::TotalCost;
    use ironsmith::costs::Cost;
    use ironsmith::effect::Effect;
    use ironsmith::mana::{ManaCost, ManaSymbol};
    use ironsmith::turn_runner::{TurnRunner, TurnState};

    let _guard = crate::test_id_counter_guard();
    let mut wasm = WasmGame::new();
    wasm.initialize_empty_match(vec!["Alice".into(), "Bob".into()], 20, 1);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let attacker =
        wasm.game
            .create_object_from_definition(&ornithopter(), alice, Zone::Battlefield);
    wasm.game.remove_summoning_sickness(attacker);
    let lotus = CardDefinitionBuilder::new(CardId::new(), "Gilded Lotus")
        .card_types(vec![CardType::Artifact])
        .with_ability(Ability::activated(
            TotalCost::from_cost(Cost::tap()),
            vec![Effect::add_mana_of_any_one_color(3)],
        ))
        .build();
    let lotus_id = wasm
        .game
        .create_object_from_definition(&lotus, alice, Zone::Battlefield);
    let prison = CardDefinitionBuilder::new(CardId::new(), "Ghostly Prison")
        .card_types(vec![CardType::Enchantment])
        .with_ability(Ability::static_ability(StaticAbility::attack_cost(
            ObjectFilter::creature(),
            false,
            TotalCost::mana(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]])),
            "Pay {2} to attack",
        )))
        .build();
    wasm.game
        .create_object_from_definition(&prison, bob, Zone::Battlefield);
    wasm.game.turn.active_player = alice;
    wasm.game.turn.phase = Phase::Combat;
    wasm.game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
    wasm.game.turn.priority_player = None;
    wasm.runner = Some(TurnRunner::from_state_for_sync(
        TurnState::DeclareAttackersDecision,
    ));
    wasm.runner_awaiting_priority = false;
    wasm.advance_until_decision().unwrap();
    let pending = wasm.pending_decision.take().unwrap();
    assert!(matches!(pending, DecisionContext::Attackers(_)));
    wasm.apply_runner_decision(
        pending,
        UiCommand::DeclareAttackers {
            declarations: vec![AttackerDeclarationInput {
                creature: attacker.0,
                target: AttackTargetInput::Player { player: 1 },
            }],
            bands: Vec::new(),
        },
    )
    .unwrap();

    let pending = wasm.pending_decision.take().unwrap();
    let DecisionContext::SelectOptions(ref payment) = pending else {
        panic!("expected attack-tax mana window, got {pending:?}");
    };
    let lotus_option = payment
        .options
        .iter()
        .find(|option| option.object_id == Some(lotus_id))
        .expect("Gilded Lotus should be an available mana ability")
        .index;
    wasm.apply_runner_decision(
        pending,
        UiCommand::SelectOptions {
            option_indices: vec![lotus_option],
        },
    )
    .unwrap();

    let pending = wasm.pending_decision.take().unwrap();
    let DecisionContext::Colors(ref colors) = pending else {
        panic!("Gilded Lotus must ask for a color, got {pending:?}");
    };
    assert!(!wasm.game.is_tapped(lotus_id));
    assert_eq!(wasm.game.player(alice).unwrap().mana_pool.total(), 0);
    let available = colors_for_context(colors);
    assert_eq!(available.len(), 5);
    let blue = available
        .iter()
        .position(|color| *color == Color::Blue)
        .unwrap();
    wasm.apply_runner_decision(
        pending,
        UiCommand::SelectOptions {
            option_indices: vec![blue],
        },
    )
    .unwrap();

    for _ in 0..8 {
        if !wasm.runner_pending_decision {
            break;
        }
        let pending = wasm.pending_decision.take().unwrap();
        let DecisionContext::SelectOptions(ref payment) = pending else {
            panic!("expected payment confirmation, got {pending:?}");
        };
        let confirm = payment
            .options
            .iter()
            .find(|option| {
                option.description == "Finish activating mana abilities"
                    || option.description == "Confirm payment"
            })
            .unwrap_or_else(|| panic!("unexpected payment prompt: {payment:?}"))
            .index;
        wasm.apply_runner_decision(
            pending,
            UiCommand::SelectOptions {
                option_indices: vec![confirm],
            },
        )
        .unwrap();
    }
    assert!(wasm.runner_awaiting_priority, "attack payment must finish");
    assert!(wasm.game.is_tapped(lotus_id));
    assert_eq!(wasm.game.player(alice).unwrap().mana_pool.blue, 1);
    assert_eq!(wasm.game.player(alice).unwrap().mana_pool.total(), 1);
    assert!(
        wasm.runner
            .as_ref()
            .unwrap()
            .combat()
            .attackers
            .iter()
            .any(|declared| declared.creature == attacker)
    );
}
