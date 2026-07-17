use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{DecisionMaker, GameResult};
use ironsmith::events::ZoneChangeEvent;
use ironsmith::rules::state_based::{LoseReason, StateBasedAction};
use ironsmith::{
    ArchenemyVariant, CardBuilder, CardDefinition, CardId, CardType, Effect, GameProgress,
    GameState, Object, ObjectKind, PlanarCardKind, PlayerFilter, PlayerId, Step, TriggerQueue,
    Value, Zone,
};

struct TestDecisionMaker;
impl DecisionMaker for TestDecisionMaker {}

fn card(name: impl Into<String>, card_type: CardType) -> CardDefinition {
    CardDefinition::new(
        CardBuilder::new(CardId::new(), name)
            .card_types(vec![card_type])
            .build(),
    )
}

fn add_libraries(game: &mut GameState, players: &[PlayerId], count: usize) {
    for player in players {
        for index in 0..count {
            game.create_object_from_definition(
                &card(
                    format!("P{} Card {index}", player.index()),
                    CardType::Artifact,
                ),
                *player,
                Zone::Library,
            );
        }
    }
}

fn vanguard(name: &str, hand_modifier: i32, life_modifier: i32) -> CardDefinition {
    CardDefinition::new(
        CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Vanguard])
            .vanguard_modifiers(hand_modifier, life_modifier)
            .build(),
    )
}

fn planar_deck(prefix: &str) -> Vec<(CardDefinition, PlanarCardKind)> {
    (0..10)
        .map(|index| {
            (
                card(format!("{prefix} Plane {index}"), CardType::Plane),
                PlanarCardKind::Plane,
            )
        })
        .collect()
}

fn scheme_deck(prefix: &str) -> Vec<CardDefinition> {
    (0..20)
        .map(|index| card(format!("{prefix} Scheme {index}"), CardType::Scheme))
        .collect()
}

fn shahrazad_continuation() -> Vec<Effect> {
    vec![Effect::lose_life_player(
        Value::HalfLifeTotalRoundedUp(PlayerFilter::IteratedPlayer),
        PlayerFilter::IteratedPlayer,
    )]
}

#[test]
fn u065_canonical_subgame_oracle_compiles_to_typed_continuation_and_renders() {
    let oracle = "Players play a Magic subgame, using their libraries as their decks. Each player who doesn't win the subgame loses half their life, rounded up.";
    let definition = CardDefinitionBuilder::new(CardId::new(), "Shahrazad")
        .card_types(vec![CardType::Sorcery])
        .parse_text(oracle)
        .expect("canonical subgame instruction should compile");
    let program = definition.spell_effect.as_ref().expect("spell effect");
    let effects = &program.segments[0].default_effects;
    let [effect] = effects.as_slice() else {
        panic!("expected one typed subgame effect, got {effects:#?}");
    };
    let subgame = effect
        .downcast_ref::<ironsmith::effects::PlaySubgameEffect>()
        .expect("typed play-subgame runtime effect");
    let [continuation] = subgame.nonwinner_effects.as_slice() else {
        panic!("expected one nonwinner continuation");
    };
    let loss = continuation
        .downcast_ref::<ironsmith::effects::LoseLifeEffect>()
        .expect("typed half-life continuation");
    assert_eq!(
        loss.amount,
        Value::HalfLifeTotalRoundedUp(PlayerFilter::IteratedPlayer)
    );
    assert!(matches!(
        loss.player.base(),
        ironsmith::ChooseSpec::Player(PlayerFilter::IteratedPlayer)
    ));
    assert_eq!(
        ironsmith::compiled_text::compiled_text_lines(&definition).join("\n"),
        oracle
    );
}

#[test]
fn u065_subgame_isolates_parent_and_resumes_with_nonwinner_continuation() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    add_libraries(&mut game, &[alice, bob], 10);
    game.player_mut(alice).unwrap().life = 17;
    game.player_mut(alice).unwrap().poison_counters = 2;
    game.player_mut(bob).unwrap().life = 15;
    let source = game.create_object_from_definition(
        &card("Subgame Spell", CardType::Sorcery),
        alice,
        Zone::Stack,
    );
    let parent_random_count = game.irreversible_random_count();

    game.begin_subgame(Some(source), alice, shahrazad_continuation())
        .expect("create isolated child game");

    assert_eq!(game.subgame_depth(), 1);
    assert!([alice, bob].contains(&game.turn.active_player));
    assert_eq!(game.player(alice).unwrap().life, 20);
    assert_eq!(game.player(alice).unwrap().poison_counters, 0);
    assert_eq!(game.player(alice).unwrap().hand.len(), 7);
    assert_eq!(game.player(alice).unwrap().library.len(), 3);
    assert_eq!(game.player(bob).unwrap().hand.len(), 7);
    game.player_mut(alice).unwrap().life = 1;
    game.player_mut(alice).unwrap().poison_counters = 9;

    let completion = game
        .finish_subgame_with(GameResult::Winner(alice), &mut TestDecisionMaker)
        .expect("resume parent game");

    assert_eq!(completion.nonwinners, vec![bob]);
    assert_eq!(completion.resumed_depth, 0);
    assert_eq!(game.player(alice).unwrap().life, 17);
    assert_eq!(game.player(alice).unwrap().poison_counters, 2);
    assert_eq!(game.player(bob).unwrap().life, 7);
    assert_eq!(game.player(alice).unwrap().library.len(), 10);
    assert_eq!(game.player(bob).unwrap().library.len(), 10);
    assert!(game.irreversible_random_count() > parent_random_count);
    assert!(game.player(alice).unwrap().graveyard.iter().any(|object| {
        game.object(*object)
            .is_some_and(|card| card.name == "Subgame Spell")
    }));
}

#[test]
fn u065_priority_waits_for_starting_procedure_then_automatically_resumes_parent() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    add_libraries(&mut game, &[alice, bob], 10);
    game.player_mut(bob).unwrap().life = 15;
    game.begin_subgame(None, alice, shahrazad_continuation())
        .unwrap();

    assert!(game.subgame_starting_procedure_pending());
    let mut queue = TriggerQueue::new();
    let error = ironsmith::game_loop::advance_priority_with_dm(
        &mut game,
        &mut queue,
        &mut TestDecisionMaker,
    )
    .expect_err("priority must wait for the subgame starting procedure");
    assert!(error.to_string().contains("starting procedure"));

    game.complete_subgame_starting_procedure();
    game.player_mut(bob).unwrap().has_lost = true;
    let progress = ironsmith::game_loop::advance_priority_with_dm(
        &mut game,
        &mut queue,
        &mut TestDecisionMaker,
    )
    .expect("the child result should restore the parent");

    assert!(matches!(progress, GameProgress::StackResolved));
    assert_eq!(game.subgame_depth(), 0);
    assert_eq!(game.player(bob).unwrap().life, 7);
    assert!(game.take_subgame_just_resumed());
}

#[test]
fn u065_draw_makes_every_participant_a_nonwinner() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    add_libraries(&mut game, &[alice, bob], 8);
    game.player_mut(alice).unwrap().life = 11;
    game.player_mut(bob).unwrap().life = 10;

    game.begin_subgame(None, alice, shahrazad_continuation())
        .unwrap();
    let completion = game
        .finish_subgame_with(GameResult::Draw, &mut TestDecisionMaker)
        .unwrap();

    assert_eq!(completion.nonwinners, vec![alice, bob]);
    assert_eq!(game.player(alice).unwrap().life, 5);
    assert_eq!(game.player(bob).unwrap().life, 5);
}

#[test]
fn u065_multiplayer_result_applies_continuation_only_to_nonwinners() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let cara = PlayerId::from_index(2);
    add_libraries(&mut game, &[alice, bob, cara], 8);
    for player in [alice, bob, cara] {
        game.player_mut(player).unwrap().life = 13;
    }

    game.begin_subgame(None, alice, shahrazad_continuation())
        .unwrap();
    let completion = game
        .finish_subgame_with(
            GameResult::Remaining(vec![alice, cara]),
            &mut TestDecisionMaker,
        )
        .unwrap();

    assert_eq!(completion.nonwinners, vec![bob]);
    assert_eq!(game.player(alice).unwrap().life, 13);
    assert_eq!(game.player(bob).unwrap().life, 6);
    assert_eq!(game.player(cara).unwrap().life, 13);
}

#[test]
fn u065_cards_return_from_all_child_zones_while_child_tokens_cease() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    add_libraries(&mut game, &[alice, bob], 12);
    game.begin_subgame(None, alice, Vec::new()).unwrap();

    let selected = game.player(alice).unwrap().hand[..4].to_vec();
    let battlefield = game
        .move_object_by_effect(selected[0], Zone::Battlefield)
        .unwrap();
    let _graveyard = game
        .move_object_by_effect(selected[1], Zone::Graveyard)
        .unwrap();
    let _exile = game
        .move_object_by_effect(selected[2], Zone::Exile)
        .unwrap();
    let phased = game
        .move_object_by_effect(selected[3], Zone::Battlefield)
        .unwrap();
    game.phase_out(phased);
    assert!(game.is_phased_out(phased));
    assert_eq!(game.object(battlefield).unwrap().zone, Zone::Battlefield);

    let token_id = game.new_object_id();
    game.add_object(Object::new_token(
        token_id,
        alice,
        "Child Token".to_string(),
        vec![CardType::Creature],
        Vec::new(),
        Some(1),
        Some(1),
        ironsmith::ColorSet::COLORLESS,
    ));

    game.finish_subgame_with(GameResult::Winner(alice), &mut TestDecisionMaker)
        .unwrap();

    assert_eq!(game.player(alice).unwrap().library.len(), 12);
    assert!(game.player(alice).unwrap().hand.is_empty());
    assert!(game.player(alice).unwrap().graveyard.is_empty());
    assert!(game.exile.is_empty());
    assert!(
        game.objects_in_deterministic_order()
            .into_iter()
            .all(|object| { object.kind != ObjectKind::Token && object.name != "Child Token" })
    );
}

#[test]
fn u065_nested_subgame_and_restart_preserve_each_suspended_parent() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    add_libraries(&mut game, &[alice, bob], 12);

    game.begin_subgame(None, alice, Vec::new()).unwrap();
    assert_eq!(game.subgame_depth(), 1);
    game.begin_subgame(None, bob, Vec::new()).unwrap();
    assert_eq!(game.subgame_depth(), 2);

    game.restart_game(bob, &[]);
    assert_eq!(
        game.subgame_depth(),
        2,
        "restart affects only the active child"
    );
    game.finish_subgame_with(GameResult::Winner(bob), &mut TestDecisionMaker)
        .unwrap();
    assert_eq!(game.subgame_depth(), 1);
    assert_eq!(game.player(alice).unwrap().hand.len(), 7);
    game.finish_subgame_with(GameResult::Winner(alice), &mut TestDecisionMaker)
        .unwrap();
    assert_eq!(game.subgame_depth(), 0);
    assert_eq!(game.player(alice).unwrap().library.len(), 12);
    assert_eq!(game.player(bob).unwrap().library.len(), 12);
}

#[test]
fn u065_explicit_parent_card_import_defers_parent_zone_event_until_resume() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    add_libraries(&mut game, &[alice, bob], 8);
    let imported = game.create_object_from_definition(
        &card("Imported Relic", CardType::Artifact),
        alice,
        Zone::Battlefield,
    );

    game.begin_subgame(None, alice, Vec::new()).unwrap();
    let _opening_hand_events = game.take_pending_trigger_events();
    let child_object = game
        .bring_parent_card_into_subgame(imported)
        .expect("bring a parent card into the child");
    assert_eq!(game.object(child_object).unwrap().zone, Zone::OutsideGame);
    assert!(
        game.take_pending_trigger_events().is_empty(),
        "parent event queue must not leak into the active child"
    );

    game.finish_subgame_with(GameResult::Winner(alice), &mut TestDecisionMaker)
        .unwrap();
    assert!(game.player(alice).unwrap().library.iter().any(|object| {
        game.object(*object)
            .is_some_and(|card| card.name == "Imported Relic")
    }));
    assert!(game.take_pending_trigger_events().iter().any(|event| {
        event.downcast::<ZoneChangeEvent>().is_some_and(|change| {
            change.from == Zone::Battlefield && change.to == Zone::OutsideGame
        })
    }));
}

#[test]
fn u065_child_preserves_sparse_player_ids_and_short_deck_loss_state() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let cara = PlayerId::from_index(2);
    assert!(game.leave_game(bob));
    add_libraries(&mut game, &[alice, cara], 5);

    game.begin_subgame(None, cara, Vec::new()).unwrap();

    assert!(game.player(bob).unwrap().has_left_game);
    assert_eq!(game.player(cara).unwrap().name, "Cara");
    assert_eq!(game.player(cara).unwrap().hand.len(), 5);
    assert!(game.player(cara).unwrap().attempted_draw_from_empty_library);
    game.turn.step = Some(Step::Upkeep);
    assert!(ironsmith::rules::check_state_based_actions(&game).contains(
        &StateBasedAction::PlayerLoses {
            player: cara,
            reason: LoseReason::DrewFromEmptyLibrary,
        }
    ));
}

#[test]
fn u065_vanguards_and_commanders_transfer_to_child_command_and_back() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    add_libraries(&mut game, &[alice, bob], 12);
    game.enable_vanguard(vec![
        (alice, vanguard("Alice Avatar", 3, -3)),
        (bob, vanguard("Bob Avatar", -1, 5)),
    ])
    .unwrap();
    let commander = game.create_object_from_definition(
        &card("Alice Commander", CardType::Creature),
        alice,
        Zone::Command,
    );
    let commander_stable = game.object(commander).unwrap().stable_id;
    game.set_as_commander(commander, alice);

    game.begin_subgame(None, alice, Vec::new()).unwrap();

    assert_eq!(game.player(alice).unwrap().life, 17);
    assert_eq!(game.player(bob).unwrap().life, 25);
    assert_eq!(game.player(alice).unwrap().max_hand_size, 10);
    assert_eq!(game.player(bob).unwrap().max_hand_size, 6);
    assert_eq!(game.player(alice).unwrap().hand.len(), 10);
    assert_eq!(game.player(bob).unwrap().hand.len(), 6);
    for player in [alice, bob] {
        let avatar = game.vanguard_card(player).expect("child Vanguard card");
        assert_eq!(game.object(avatar).unwrap().zone, Zone::Command);
    }
    let child_commander = game
        .objects_in_deterministic_order()
        .into_iter()
        .find(|object| object.stable_id == commander_stable)
        .expect("child commander")
        .id;
    assert!(game.is_commander(child_commander));
    assert_eq!(game.object(child_commander).unwrap().zone, Zone::Command);

    game.finish_subgame_with(GameResult::Winner(alice), &mut TestDecisionMaker)
        .unwrap();
    for player in [alice, bob] {
        assert!(game.vanguard_card(player).is_some());
    }
    let returned_commander = game
        .objects_in_deterministic_order()
        .into_iter()
        .find(|object| object.stable_id == commander_stable)
        .expect("returned commander")
        .id;
    assert!(game.is_commander(returned_commander));
    assert_eq!(game.object(returned_commander).unwrap().zone, Zone::Command);
}

#[test]
fn u065_individual_planar_and_scheme_decks_transfer_shuffle_and_return() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    add_libraries(&mut game, &[alice, bob], 8);
    game.enable_planechase(vec![
        (alice, planar_deck("Alice")),
        (bob, planar_deck("Bob")),
    ])
    .unwrap();
    game.enable_archenemy(
        ArchenemyVariant::Default,
        vec![(alice, scheme_deck("Alice"))],
    )
    .unwrap();

    game.begin_subgame(None, alice, Vec::new()).unwrap();

    let child_planes = game.planechase.as_ref().unwrap();
    assert_eq!(child_planes.face_up.len(), 1);
    assert_eq!(
        child_planes.decks.values().map(Vec::len).sum::<usize>() + child_planes.face_up.len(),
        20
    );
    assert_eq!(game.scheme_deck(alice).unwrap().len(), 20);

    game.finish_subgame_with(GameResult::Winner(alice), &mut TestDecisionMaker)
        .unwrap();
    let parent_planes = game.planechase.as_ref().unwrap();
    assert!(parent_planes.face_up.is_empty());
    assert_eq!(
        parent_planes.decks.values().map(Vec::len).sum::<usize>(),
        20
    );
    assert_eq!(game.scheme_deck(alice).unwrap().len(), 20);
}

#[test]
fn u065_communal_planar_deck_transfers_and_returns_as_communal() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    add_libraries(&mut game, &[alice, bob], 8);
    let mut communal = planar_deck("Shared A");
    communal.extend(planar_deck("Shared B"));
    game.enable_planechase_communal(communal).unwrap();

    game.begin_subgame(None, alice, Vec::new()).unwrap();
    let child = game.planechase.as_ref().unwrap();
    assert_eq!(child.face_up.len(), 1);
    assert_eq!(child.communal_deck.as_ref().unwrap().len(), 19);

    game.finish_subgame_with(GameResult::Winner(alice), &mut TestDecisionMaker)
        .unwrap();
    let parent = game.planechase.as_ref().unwrap();
    assert!(parent.face_up.is_empty());
    assert_eq!(parent.communal_deck.as_ref().unwrap().len(), 20);
}
