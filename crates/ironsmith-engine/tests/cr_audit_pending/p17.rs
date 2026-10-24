//! CR audit (pending-entry pass, package p17): assertions express the rules, so confirmed defects fail.
//! Do not use `should_panic`, `ignore`, or assertions of current broken behavior.

#![allow(unused_imports, dead_code)]

use ironsmith::{EffectContext, GameState, PlayerId};

use ironsmith::EffectExecutor;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::events::{KeywordActionEvent, KeywordActionKind};
use ironsmith::triggers::Trigger;
use ironsmith::{
    Ability, ArchenemyVariant, CardBuilder, CardDefinition, CardId, CardType, CastingMethod,
    ColorSet, Effect, LegalAction, ObjectId, Phase, PlanarCardKind, PlanarDieFace, PlayerFilter,
    StackEntry, StateBasedAction, Supertype, TotalCost, TriggerEvent, TriggerQueue, Zone,
    apply_state_based_actions, check_state_based_actions, check_triggers, put_triggers_on_stack,
    resolve_stack_entry,
};

// ---------------------------------------------------------------------------
// Shared fixtures
// ---------------------------------------------------------------------------

fn p(index: u8) -> PlayerId {
    PlayerId::from_index(index)
}

fn names(count: usize) -> Vec<String> {
    (0..count).map(|index| format!("P{index}")).collect()
}

fn planar_card(name: &str, kind: PlanarCardKind) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![match kind {
            PlanarCardKind::Plane => CardType::Plane,
            PlanarCardKind::Phenomenon => CardType::Phenomenon,
        }])
        .build()
}

fn planar_deck(prefix: &str, count: usize) -> Vec<(CardDefinition, PlanarCardKind)> {
    (0..count)
        .map(|index| {
            (
                planar_card(&format!("{prefix} Plane {index}"), PlanarCardKind::Plane),
                PlanarCardKind::Plane,
            )
        })
        .collect()
}

fn planechase_game(players: usize) -> GameState {
    let mut game = GameState::new(names(players), 20);
    let decks = (0..players)
        .map(|index| (p(index as u8), planar_deck(&format!("P{index}"), 10)))
        .collect();
    game.enable_planechase(decks).expect("valid planar decks");
    game
}

fn planar_object_named(game: &GameState, name: &str) -> ObjectId {
    game.planechase
        .as_ref()
        .expect("Planechase enabled")
        .card_kinds
        .keys()
        .copied()
        .find(|id| game.object(*id).is_some_and(|object| object.name == name))
        .unwrap_or_else(|| panic!("missing planar card {name}"))
}

fn put_planar_on_top(game: &mut GameState, player: PlayerId, object: ObjectId) {
    let state = game.planechase.as_mut().expect("Planechase enabled");
    let deck = match state.communal_deck.as_mut() {
        Some(deck) => deck,
        None => state.decks.get_mut(&player).expect("individual deck"),
    };
    let index = deck
        .iter()
        .position(|candidate| *candidate == object)
        .expect("card in planar deck");
    deck.remove(index);
    deck.push(object);
}

fn keyword_events(events: &[TriggerEvent], action: KeywordActionKind) -> Vec<&TriggerEvent> {
    events
        .iter()
        .filter(|event| {
            event
                .downcast::<KeywordActionEvent>()
                .is_some_and(|event| event.action == action)
        })
        .collect()
}

fn set_main_phase(game: &mut GameState, player: PlayerId) {
    game.turn.active_player = player;
    game.turn.priority_player = Some(player);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
}

fn gain_life_trigger_on(action: KeywordActionKind) -> Ability {
    Ability::triggered(
        Trigger::keyword_action_from_source(action, PlayerFilter::You),
        vec![Effect::gain_life(1)],
    )
}

// ---------------------------------------------------------------------------
// CR 901 Planechase
// ---------------------------------------------------------------------------

#[test]
fn cr_901_2_planechase_supports_two_player_and_multiplayer_games() {
    for players in [2usize, 3, 4] {
        let mut game = planechase_game(players);
        assert!(
            game.planechase.is_some(),
            "CR 901.2: a {players}-player game can use Planechase"
        );
        let plane = game.reveal_starting_plane().expect("starting plane");
        assert_eq!(game.face_up_planar_objects(), &[plane]);
        for index in 0..players {
            assert_eq!(
                game.planar_deck(p(index as u8)).map(|deck| deck.len()),
                Some(if index == 0 { 9 } else { 10 }),
                "CR 901.2/901.3: every player of a {players}-player game has a planar deck"
            );
        }
    }
}

#[test]
fn cr_901_3_planar_deck_minimum_phenomenon_limit_and_unique_names() {
    let try_deck = |deck: Vec<(CardDefinition, PlanarCardKind)>| {
        let mut game = GameState::new(names(2), 20);
        game.enable_planechase(vec![(p(0), deck), (p(1), planar_deck("Other", 10))])
    };
    assert!(try_deck(planar_deck("Ten", 10)).is_ok(), "CR 901.3: ten planes are legal");
    assert!(
        try_deck(planar_deck("Nine", 9)).is_err(),
        "CR 901.3: a planar deck needs at least ten cards"
    );
    let mut two_phenomena = planar_deck("TwoPh", 10);
    for index in 0..2 {
        two_phenomena[index] = (
            planar_card(&format!("TwoPh Phenomenon {index}"), PlanarCardKind::Phenomenon),
            PlanarCardKind::Phenomenon,
        );
    }
    assert!(try_deck(two_phenomena.clone()).is_ok(), "CR 901.3: two phenomena are legal");
    let mut three_phenomena = two_phenomena;
    three_phenomena[2] = (
        planar_card("TwoPh Phenomenon 2", PlanarCardKind::Phenomenon),
        PlanarCardKind::Phenomenon,
    );
    assert!(
        try_deck(three_phenomena).is_err(),
        "CR 901.3: no more than two phenomenon cards"
    );
    let mut duplicate = planar_deck("Dup", 10);
    duplicate[1] = duplicate[0].clone();
    assert!(
        try_deck(duplicate).is_err(),
        "CR 901.3: each card in a planar deck must have a different English name"
    );
}

#[test]
fn cr_901_3a_planar_die_has_one_planeswalker_one_chaos_and_four_blank_faces() {
    let mut faces = Vec::new();
    for raw in 1..=6u32 {
        let mut game = planechase_game(2);
        game.reveal_starting_plane().unwrap();
        game.force_next_die_roll(raw);
        faces.push(game.roll_planar_die(p(0), false).expect("planar roll"));
    }
    let planeswalker = faces.iter().filter(|face| **face == PlanarDieFace::Planeswalker).count();
    let chaos = faces.iter().filter(|face| **face == PlanarDieFace::Chaos).count();
    let blank = faces.iter().filter(|face| **face == PlanarDieFace::Blank).count();
    assert_eq!(
        (planeswalker, chaos, blank),
        (1, 1, 4),
        "CR 901.3a: six faces, one Planeswalker symbol, one chaos symbol, four blank"
    );
}

#[test]
fn cr_901_4_planar_cards_remain_in_the_command_zone() {
    let mut game = planechase_game(2);
    let face_up = game.reveal_starting_plane().unwrap();
    let face_down = game.planar_deck(p(1)).unwrap()[0];
    for (object, label) in [(face_up, "face-up plane"), (face_down, "planar deck card")] {
        let stable = game.object(object).unwrap().stable_id;
        for zone in [Zone::Graveyard, Zone::Hand, Zone::Battlefield, Zone::Exile, Zone::Library] {
            game.move_object_by_effect(object, zone);
            let current = game.find_object_by_stable_id(stable).unwrap_or(object);
            assert_eq!(
                game.object(current).map(|object| object.zone),
                Some(Zone::Command),
                "CR 901.4: a {label} remains in the command zone after an attempted move to {zone:?}"
            );
        }
    }
}

#[test]
fn cr_901_5_starting_plane_skips_phenomena_to_the_bottom_without_triggers() {
    let mut game = GameState::new(names(2), 20);
    let mut deck = planar_deck("Alice", 10);
    deck[8] = (
        {
            let mut definition = planar_card("Alice Phenomenon", PlanarCardKind::Phenomenon);
            definition
                .abilities
                .push(gain_life_trigger_on(KeywordActionKind::EncounterPhenomenon));
            definition
        },
        PlanarCardKind::Phenomenon,
    );
    deck[9].0.abilities.push(gain_life_trigger_on(KeywordActionKind::Planeswalk));
    game.enable_planechase(vec![(p(0), deck), (p(1), planar_deck("Bob", 10))])
        .unwrap();
    let phenomenon = planar_object_named(&game, "Alice Phenomenon");
    let plane = planar_object_named(&game, "Alice Plane 9");
    let phenomenon_stable = game.object(phenomenon).unwrap().stable_id;
    put_planar_on_top(&mut game, p(0), plane);
    put_planar_on_top(&mut game, p(0), phenomenon);
    let revealed = game.reveal_starting_plane().expect("starting plane");
    assert_eq!(revealed, plane, "CR 901.5: the starting plane is the first plane turned face up");
    assert_eq!(game.face_up_planar_objects(), &[plane]);
    let bottom = game.planar_deck(p(0)).unwrap()[0];
    assert_eq!(
        game.object(bottom).unwrap().stable_id,
        phenomenon_stable,
        "CR 901.5: a phenomenon turned face up is put on the bottom of the planar deck"
    );
    let events = game.take_pending_trigger_events();
    let triggered: usize = events.iter().map(|event| check_triggers(&game, event).len()).sum();
    assert_eq!(triggered, 0, "CR 901.5: no abilities trigger during the starting-plane process");
    assert_eq!(game.planeswalk_count(), Some(0), "CR 901.5/901.11: setting the starting plane is not planeswalking");
}

#[test]
fn cr_901_6_owner_is_deck_owner_and_controller_follows_the_active_player() {
    let mut game = GameState::new(names(3), 20);
    let mut deck = planar_deck("P0", 10);
    deck[9].0.abilities.push(gain_life_trigger_on(KeywordActionKind::ChaosEnsues));
    game.enable_planechase(vec![
        (p(0), deck),
        (p(1), planar_deck("P1", 10)),
        (p(2), planar_deck("P2", 10)),
    ])
    .unwrap();
    let plane = planar_object_named(&game, "P0 Plane 9");
    put_planar_on_top(&mut game, p(0), plane);
    game.reveal_starting_plane().unwrap();
    assert_eq!(game.object(plane).unwrap().owner, p(0), "CR 901.6: owner is the deck's starting owner");
    assert_eq!(game.planar_controller(), Some(p(0)));
    assert_eq!(game.controller_of_id(plane), Some(p(0)));
    game.next_turn();
    assert_eq!(game.turn.active_player, p(1));
    assert_eq!(game.object(plane).unwrap().owner, p(0), "CR 901.6: ownership never changes");
    assert_eq!(
        game.planar_controller(),
        Some(p(1)),
        "CR 901.6: the planar controller is normally the active player"
    );
    assert_eq!(
        game.controller_of_id(plane),
        Some(p(1)),
        "CR 901.6: the planar controller controls the face-up plane"
    );
    game.take_pending_trigger_events();
    game.chaos_ensues(p(1), plane).unwrap();
    let events = game.take_pending_trigger_events();
    let chaos = keyword_events(&events, KeywordActionKind::ChaosEnsues);
    let entries = check_triggers(&game, chaos[0]);
    assert_eq!(entries.len(), 1);
    assert_eq!(
        entries[0].controller,
        p(1),
        "CR 901.6: the face-up plane's triggered ability is controlled by the planar controller"
    );
    // The current planar controller leaving makes the next player in turn order the controller.
    assert!(game.leave_game(p(1)).expect("departure"));
    assert_eq!(
        game.planar_controller(),
        Some(p(2)),
        "CR 901.6: the next player in turn order becomes planar controller when the controller leaves"
    );
}

#[test]
fn cr_901_7_face_up_plane_abilities_function_from_command_and_face_down_are_dormant() {
    let mut game = GameState::new(names(2), 20);
    let mut deck = planar_deck("Alice", 10);
    deck[9].0.abilities.push(Ability::activated(TotalCost::free(), vec![Effect::gain_life(1)]));
    deck[9].0.abilities.push(gain_life_trigger_on(KeywordActionKind::ChaosEnsues));
    deck[8].0.abilities.push(Ability::activated(TotalCost::free(), vec![Effect::gain_life(1)]));
    game.enable_planechase(vec![(p(0), deck), (p(1), planar_deck("Bob", 10))])
        .unwrap();
    set_main_phase(&mut game, p(0));
    let active = planar_object_named(&game, "Alice Plane 9");
    let dormant = planar_object_named(&game, "Alice Plane 8");
    put_planar_on_top(&mut game, p(0), active);
    game.reveal_starting_plane().unwrap();
    let actions = ironsmith::decision::compute_legal_actions(&game, p(0)).expect("legal actions");
    assert!(
        actions.iter().any(|action| matches!(action, LegalAction::ActivateAbility { source, .. } if *source == active)),
        "CR 901.7: a face-up plane's activated ability may be activated from the command zone"
    );
    assert!(
        !actions.iter().any(|action| matches!(action, LegalAction::ActivateAbility { source, .. } if *source == dormant)),
        "CR 901.7: abilities of face-down planar cards do not function"
    );
    game.chaos_ensues(p(0), active).unwrap();
    let events = game.take_pending_trigger_events();
    let chaos = keyword_events(&events, KeywordActionKind::ChaosEnsues);
    assert_eq!(
        check_triggers(&game, chaos[0]).len(),
        1,
        "CR 901.7: a face-up plane's triggered ability may trigger"
    );
}

#[test]
fn cr_901_8_901_9c_planeswalker_roll_triggers_a_stack_ability_that_planeswalks() {
    let mut game = planechase_game(2);
    game.reveal_starting_plane().unwrap();
    set_main_phase(&mut game, p(0));
    let before = game.planeswalk_count().unwrap();
    game.force_next_die_roll(1);
    assert_eq!(game.roll_planar_die(p(0), true).unwrap(), PlanarDieFace::Planeswalker);
    assert_eq!(
        game.planeswalk_count().unwrap(),
        before,
        "CR 901.8: rolling the Planeswalker symbol triggers; it doesn't planeswalk immediately"
    );
    let mut queue = TriggerQueue::new();
    put_triggers_on_stack(&mut game, &mut queue).unwrap();
    assert_eq!(game.stack.len(), 1, "CR 901.9c: the planeswalking ability is put on the stack");
    assert_eq!(game.stack[0].controller, p(0));
    resolve_stack_entry(&mut game).unwrap();
    assert_eq!(
        game.planeswalk_count().unwrap(),
        before + 1,
        "CR 901.8: the planeswalking ability planeswalks on resolution"
    );
}

#[test]
fn cr_901_9b_chaos_roll_makes_chaos_ensue_and_active_player_keeps_priority() {
    let mut game = GameState::new(names(2), 20);
    let mut deck = planar_deck("Alice", 10);
    for (definition, _) in &mut deck {
        definition
            .abilities
            .push(gain_life_trigger_on(KeywordActionKind::ChaosEnsues));
    }
    game.enable_planechase(vec![(p(0), deck), (p(1), planar_deck("Bob", 10))])
        .unwrap();
    game.reveal_starting_plane().unwrap();
    set_main_phase(&mut game, p(0));
    game.force_next_die_roll(2);
    let mut decisions = ironsmith::decision::SelectFirstDecisionMaker;
    ironsmith::special_actions::perform(
        ironsmith::SpecialAction::RollPlanarDie,
        &mut game,
        p(0),
        &mut decisions,
    )
    .expect("free planar roll");
    let events = game.take_pending_trigger_events();
    let chaos = keyword_events(&events, KeywordActionKind::ChaosEnsues);
    assert_eq!(chaos.len(), 1, "CR 901.9b: rolling the chaos symbol makes chaos ensue");
    assert_eq!(check_triggers(&game, chaos[0]).len(), 1);
    assert_eq!(
        game.turn.priority_player,
        Some(p(0)),
        "CR 901.9b: the active player gets priority after rolling"
    );
}

#[test]
fn cr_901_10_901_11a_owner_leaving_makes_planar_controller_planeswalk_immediately() {
    let mut game = planechase_game(3);
    let plane = planar_object_named(&game, "P0 Plane 9");
    put_planar_on_top(&mut game, p(0), plane);
    game.reveal_starting_plane().unwrap();
    game.next_turn();
    assert_eq!(game.planar_controller(), Some(p(1)));
    let bob_top = *game.planar_deck(p(1)).unwrap().last().unwrap();
    let before = game.planeswalk_count().unwrap();
    game.take_pending_trigger_events();
    assert!(game.leave_game(p(0)).expect("departure"));
    assert!(game.object(plane).is_none(), "CR 901.10: the departed owner's plane leaves the game");
    assert_eq!(
        game.face_up_planar_objects(),
        &[bob_top],
        "CR 901.10: the planar controller turns the top card of their planar deck face up immediately"
    );
    assert_eq!(
        game.planeswalk_count().unwrap(),
        before + 1,
        "CR 901.11a: this counts as planeswalking"
    );
    let events = game.take_pending_trigger_events();
    assert_eq!(
        keyword_events(&events, KeywordActionKind::Planeswalk).len(),
        1,
        "CR 901.11: abilities that trigger when a player planeswalks can trigger"
    );
}

#[test]
fn cr_901_10a_planeswalking_ability_ceases_to_exist_when_its_plane_leaves_the_game() {
    let mut game = planechase_game(3);
    let plane = planar_object_named(&game, "P0 Plane 9");
    put_planar_on_top(&mut game, p(0), plane);
    game.reveal_starting_plane().unwrap();
    game.next_turn();
    set_main_phase(&mut game, p(1));
    assert_eq!(game.planar_controller(), Some(p(1)));
    game.force_next_die_roll(1);
    assert_eq!(game.roll_planar_die(p(1), true).unwrap(), PlanarDieFace::Planeswalker);
    let mut queue = TriggerQueue::new();
    put_triggers_on_stack(&mut game, &mut queue).unwrap();
    assert_eq!(game.stack.len(), 1);
    assert_eq!(game.stack[0].controller, p(1));
    assert!(game.leave_game(p(0)).expect("departure"));
    assert!(game.object(plane).is_none());
    assert!(
        game.stack.is_empty(),
        "CR 901.10a: the planeswalking ability ceases to exist when its plane leaves the game; stack still holds {} entr(ies)",
        game.stack.len()
    );
}

#[test]
fn cr_901_10b_phenomenon_abilities_survive_owner_and_pass_to_new_planar_controller() {
    let mut game = GameState::new(names(3), 20);
    let mut deck = planar_deck("P0", 10);
    deck[8] = (
        {
            let mut definition = planar_card("P0 Phenomenon", PlanarCardKind::Phenomenon);
            definition
                .abilities
                .push(gain_life_trigger_on(KeywordActionKind::EncounterPhenomenon));
            definition
        },
        PlanarCardKind::Phenomenon,
    );
    game.enable_planechase(vec![
        (p(0), deck),
        (p(1), planar_deck("P1", 10)),
        (p(2), planar_deck("P2", 10)),
    ])
    .unwrap();
    let start = planar_object_named(&game, "P0 Plane 9");
    let phenomenon = planar_object_named(&game, "P0 Phenomenon");
    put_planar_on_top(&mut game, p(0), start);
    game.reveal_starting_plane().unwrap();
    put_planar_on_top(&mut game, p(0), phenomenon);
    assert_eq!(game.planeswalk(p(0), start).unwrap(), phenomenon);
    let mut queue = TriggerQueue::new();
    put_triggers_on_stack(&mut game, &mut queue).unwrap();
    assert_eq!(game.stack.len(), 1);
    assert_eq!(game.stack[0].controller, p(0));
    assert!(game.leave_game(p(0)).expect("departure"));
    assert_eq!(
        game.stack.len(),
        1,
        "CR 901.10b: abilities from phenomena owned by a departed player remain on the stack"
    );
    assert_eq!(
        game.stack[0].controller,
        p(1),
        "CR 901.10b: the surviving phenomenon ability is controlled by the new planar controller"
    );
}

#[test]
fn cr_901_11_planeswalk_moves_top_card_face_up_and_triggers_planeswalk_abilities() {
    let mut game = GameState::new(names(2), 20);
    let mut deck = planar_deck("Alice", 10);
    deck[9].0.abilities.push(Ability::triggered(
        Trigger::keyword_action(KeywordActionKind::Planeswalk, PlayerFilter::You),
        vec![Effect::gain_life(1)],
    ));
    game.enable_planechase(vec![(p(0), deck), (p(1), planar_deck("Bob", 10))])
        .unwrap();
    let start = planar_object_named(&game, "Alice Plane 9");
    let destination = planar_object_named(&game, "Alice Plane 8");
    put_planar_on_top(&mut game, p(0), start);
    game.reveal_starting_plane().unwrap();
    put_planar_on_top(&mut game, p(0), destination);
    let start_stable = game.object(start).unwrap().stable_id;
    assert_eq!(
        game.planeswalk(p(0), start).unwrap(),
        destination,
        "CR 901.11b: the card turned face up is the plane planeswalked to"
    );
    assert_eq!(game.face_up_planar_objects(), &[destination]);
    let events = game.take_pending_trigger_events();
    let planeswalk = keyword_events(&events, KeywordActionKind::Planeswalk);
    assert_eq!(planeswalk.len(), 1, "CR 901.11: planeswalking is an observable event");
    let entries = check_triggers(&game, planeswalk[0]);
    assert_eq!(
        entries.len(),
        1,
        "CR 901.11/901.11b: the plane planeswalked away from triggers via last known information"
    );
    assert_eq!(entries[0].source_stable_id, start_stable);
}

#[test]
fn cr_901_11a_phenomenon_planeswalks_after_its_trigger_leaves_the_stack() {
    let mut game = GameState::new(names(2), 20);
    let mut deck = planar_deck("Alice", 10);
    deck[8] = (
        {
            let mut definition = planar_card("Alice Phenomenon", PlanarCardKind::Phenomenon);
            definition
                .abilities
                .push(gain_life_trigger_on(KeywordActionKind::EncounterPhenomenon));
            definition
        },
        PlanarCardKind::Phenomenon,
    );
    game.enable_planechase(vec![(p(0), deck), (p(1), planar_deck("Bob", 10))])
        .unwrap();
    let start = planar_object_named(&game, "Alice Plane 9");
    let phenomenon = planar_object_named(&game, "Alice Phenomenon");
    let next = planar_object_named(&game, "Alice Plane 7");
    put_planar_on_top(&mut game, p(0), start);
    game.reveal_starting_plane().unwrap();
    put_planar_on_top(&mut game, p(0), phenomenon);
    game.planeswalk(p(0), start).unwrap();
    let mut queue = TriggerQueue::new();
    put_triggers_on_stack(&mut game, &mut queue).unwrap();
    assert_eq!(game.stack.len(), 1);
    assert!(
        !check_state_based_actions(&game).contains(&StateBasedAction::PlaneswalkFromPhenomenon(phenomenon)),
        "CR 704.6f: no planeswalk while the encounter ability is on the stack"
    );
    resolve_stack_entry(&mut game).unwrap();
    put_planar_on_top(&mut game, p(0), next);
    let before = game.planeswalk_count().unwrap();
    apply_state_based_actions(&mut game).unwrap();
    assert_eq!(
        game.face_up_planar_objects(),
        &[next],
        "CR 901.11a: a phenomenon's planeswalk happens once its triggered ability leaves the stack"
    );
    assert_eq!(game.planeswalk_count().unwrap(), before + 1);
}

#[test]
fn cr_901_11c_planeswalking_with_several_face_up_planes_leaves_all_of_them() {
    let mut game = planechase_game(2);
    let first = game.reveal_starting_plane().unwrap();
    // A second face-up plane (as Spatial Merging produces) controlled by the same planar controller.
    let second = game.planar_deck(p(0)).unwrap()[0];
    {
        let state = game.planechase.as_mut().unwrap();
        state.decks.get_mut(&p(0)).unwrap().retain(|id| *id != second);
        state.face_up.push(second);
        state.face_up_controllers.insert(second, p(0));
    }
    assert_eq!(game.face_up_planar_objects().len(), 2);
    let destination = game.planeswalk(p(0), first).unwrap();
    assert_eq!(
        game.face_up_planar_objects(),
        &[destination],
        "CR 901.11c: the player planeswalks away from all face-up planes"
    );
    assert!(game.object(first).is_none() && game.object(second).is_none());
}

#[test]
fn cr_901_12a_d_two_headed_giant_planechase_controller_and_individual_roll_costs() {
    let mut game = GameState::new(names(4), 20);
    game.enable_two_headed_giant(vec![vec![p(0), p(1)], vec![p(2), p(3)]])
        .expect("2HG");
    let decks = (0..4)
        .map(|index| (p(index), planar_deck(&format!("P{index}"), 10)))
        .collect();
    game.enable_planechase(decks)
        .expect("CR 901.12a: each player has their own planar deck");
    for index in 0..4 {
        assert_eq!(game.planar_deck(p(index)).map(|deck| deck.len()), Some(10));
    }
    let active = game.turn.active_player;
    let (team, other_team) = if active == p(1) {
        ([p(0), p(1)], [p(2), p(3)])
    } else {
        ([p(2), p(3)], [p(0), p(1)])
    };
    assert_eq!(active, team[1], "fixture: 2HG starts with the starting team's primary player");
    assert_eq!(
        game.planar_controller(),
        Some(team[1]),
        "CR 901.12b: the planar controller is the primary player of the active team"
    );
    game.reveal_starting_plane().unwrap();
    // CR 901.12d: both members of the active team may roll; each pays based on their own rolls.
    game.force_next_die_roll(4);
    game.roll_planar_die(team[1], true).unwrap();
    game.force_next_die_roll(4);
    game.roll_planar_die(team[1], true).unwrap();
    game.force_next_die_roll(4);
    game.roll_planar_die(team[0], true).unwrap();
    assert_eq!(game.planar_die_roll_cost(team[1]), Some(2), "CR 901.12d: primary rolled twice");
    assert_eq!(
        game.planar_die_roll_cost(team[0]),
        Some(1),
        "CR 901.12d: the teammate's cost counts only the teammate's own rolls"
    );
    // CR 901.12c: a teammate's planeswalking ability moves the team's planar zone.
    let current = game.face_up_planar_objects()[0];
    assert!(game.planeswalk(team[0], current).is_ok(), "CR 901.12c: 'you' covers both team members");
    game.next_turn();
    assert_eq!(
        game.planar_controller(),
        Some(other_team[1]),
        "CR 901.12b: when the other team becomes active its primary player becomes planar controller"
    );
}

#[test]
fn cr_901_13_planar_sources_are_exempt_from_limited_range_of_influence() {
    let mut game = planechase_game(5);
    game.enable_limited_range_of_influence((0..5).map(p).collect(), vec![1; 5])
        .expect("range of influence");
    let plane = game.reveal_starting_plane().unwrap();
    assert!(!game.player_is_within_range(p(0), p(2)), "fixture: P2 is outside P0's range");
    assert!(
        game.source_is_exempt_from_range(Some(plane)),
        "CR 901.13: plane abilities are exempt from the limited range of influence option"
    );
    let ordinary = CardDefinitionBuilder::new(CardId::new(), "Ordinary enchantment")
        .card_types(vec![CardType::Enchantment])
        .build();
    let ordinary = game.create_object_from_definition(&ordinary, p(0), Zone::Battlefield);
    assert!(!game.source_is_exempt_from_range(Some(ordinary)), "control: ordinary sources are limited");
}

#[test]
fn cr_901_14_grand_melee_planechase_has_one_planar_controller_and_plane_per_marker() {
    let mut game = GameState::new(names(8), 20);
    game.enable_grand_melee().expect("Grand Melee");
    let decks = (0..8)
        .map(|index| (p(index), planar_deck(&format!("P{index}"), 10)))
        .collect();
    game.enable_planechase(decks).unwrap();
    let faces = game
        .reveal_grand_melee_starting_planes()
        .expect("CR 901.14a: each turn-marker holder sets a starting plane");
    assert_eq!(faces.len(), 2, "CR 901.14: two turn markers give two face-up planes");
    assert_eq!(game.face_up_planar_objects().len(), 2);
    let controllers = game.planar_controllers();
    assert_eq!(controllers.len(), 2, "CR 901.14a: each marker holder is a planar controller");
    for face in faces {
        let controller = game.planar_controller_of_face(face).unwrap();
        assert_eq!(game.object(face).unwrap().owner, controller);
        assert!(controllers.contains(&controller));
    }
}

#[test]
fn cr_901_14b_marker_reducing_departure_bottoms_planes_without_planeswalking() {
    let mut game = GameState::new(names(8), 20);
    game.enable_grand_melee().expect("Grand Melee");
    let decks = (0..8)
        .map(|index| (p(index), planar_deck(&format!("P{index}"), 10)))
        .collect();
    game.enable_planechase(decks).unwrap();
    game.reveal_grand_melee_starting_planes().unwrap();
    let controllers = game.planar_controllers();
    let departing = controllers[1];
    let departing_face = game
        .face_up_planar_objects()
        .iter()
        .copied()
        .find(|face| game.planar_controller_of_face(*face) == Some(departing))
        .unwrap();
    let before = game.planeswalk_count().unwrap();
    // Eight players with two markers: any departure reduces the number of turn markers.
    assert!(game.leave_game(departing).expect("departure"));
    assert!(
        !game.planar_controllers().contains(&departing),
        "CR 901.14b: the departing player ceases to be a planar controller"
    );
    assert_eq!(
        game.planar_controllers().len(),
        1,
        "CR 901.14b: no other player becomes a planar controller"
    );
    assert!(!game.face_up_planar_objects().contains(&departing_face));
    assert_eq!(
        game.face_up_planar_objects().len(),
        1,
        "CR 901.14b: the departing controller's face-up plane is put on the bottom, nothing replaces it"
    );
    assert_eq!(
        game.planeswalk_count().unwrap(),
        before,
        "CR 901.14b: no player is considered to have planeswalked"
    );
}

#[test]
fn cr_901_15_communal_planar_deck_size_ownership_and_shared_reference() {
    for players in [2usize, 3, 5] {
        let minimum = 40usize.min(players * 10);
        let mut short = GameState::new(names(players), 20);
        assert!(
            short
                .enable_planechase_communal(planar_deck("Short", minimum - 1))
                .is_err(),
            "CR 901.15a: a communal deck needs at least min(40, 10 x players) = {minimum} cards"
        );
        let mut too_many_phenomena = planar_deck("Ph", minimum);
        for index in 0..=(players * 2) {
            too_many_phenomena[index] = (
                planar_card(&format!("Ph Phenomenon {index}"), PlanarCardKind::Phenomenon),
                PlanarCardKind::Phenomenon,
            );
        }
        assert!(
            GameState::new(names(players), 20)
                .enable_planechase_communal(too_many_phenomena)
                .is_err(),
            "CR 901.15a: no more phenomena than twice the number of players"
        );
        let mut duplicate = planar_deck("Dup", minimum);
        duplicate[1] = duplicate[0].clone();
        assert!(
            GameState::new(names(players), 20)
                .enable_planechase_communal(duplicate)
                .is_err(),
            "CR 901.15a: names must differ"
        );
        let mut game = GameState::new(names(players), 20);
        game.enable_planechase_communal(planar_deck("Shared", minimum))
            .expect("legal communal deck");
        game.reveal_starting_plane().unwrap();
        let first = game.planar_deck(p(0)).unwrap().to_vec();
        for index in 1..players {
            assert_eq!(
                game.planar_deck(p(index as u8)).unwrap(),
                first.as_slice(),
                "CR 901.15c: every player's planar deck is the communal deck"
            );
        }
        let all = game.planechase.as_ref().unwrap().card_kinds.keys().copied().collect::<Vec<_>>();
        assert!(
            all.iter().all(|id| game.object(*id).unwrap().owner == p(0)),
            "CR 901.15b: the planar controller owns every communal card"
        );
        game.next_turn();
        assert!(
            all.iter().all(|id| game.object(*id).unwrap().owner == p(1)),
            "CR 901.15b: ownership follows the planar controller"
        );
    }
}


// ---------------------------------------------------------------------------
// CR 902 Vanguard
// ---------------------------------------------------------------------------

fn vanguard_card(name: &str, hand_modifier: i32, life_modifier: i32) -> CardDefinition {
    CardDefinition::new(
        CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Vanguard])
            .vanguard_modifiers(hand_modifier, life_modifier)
            .build(),
    )
}

#[test]
fn cr_902_1_902_2_902_3_each_player_has_one_face_up_vanguard_in_command() {
    for players in [2usize, 3] {
        let mut game = GameState::new(names(players), 20);
        let cards = (0..players)
            .map(|index| {
                (
                    p(index as u8),
                    vanguard_card(&format!("Avatar {index}"), index as i32, 2 * index as i32),
                )
            })
            .collect();
        game.enable_vanguard(cards)
            .expect("CR 902.2: Vanguard works in two-player and multiplayer games");
        for index in 0..players {
            let player = p(index as u8);
            let card = game.vanguard_card(player).expect("CR 902.1: one vanguard per player");
            assert_eq!(game.object(card).unwrap().owner, player);
            assert_eq!(game.object(card).unwrap().zone, Zone::Command, "CR 902.3: vanguard is in command");
            assert_eq!(
                game.player(player).unwrap().life,
                20 + 2 * index as i32,
                "CR 902.1: the vanguard's characteristics affect the game"
            );
            for zone in [Zone::Graveyard, Zone::Hand, Zone::Battlefield, Zone::Exile] {
                game.move_object_by_effect(card, zone);
                assert_eq!(
                    game.object(card).map(|object| object.zone),
                    Some(Zone::Command),
                    "CR 902.3: vanguard cards remain in the command zone ({zone:?})"
                );
            }
        }
        assert_eq!(game.vanguard_cards().len(), players);
    }
    let mut missing = GameState::new(names(3), 20);
    assert!(
        missing
            .enable_vanguard(vec![(p(0), vanguard_card("A", 0, 0)), (p(1), vanguard_card("B", 0, 0))])
            .is_err(),
        "CR 902.3: each player needs a vanguard card"
    );
}

#[test]
fn cr_902_5a_vanguard_starting_hand_size_includes_the_hand_modifier() {
    let mut game = GameState::new(names(2), 20);
    game.enable_vanguard(vec![(p(0), vanguard_card("Plus", 2, 0)), (p(1), vanguard_card("Minus", -1, 0))])
        .unwrap();
    assert_eq!(
        game.vanguard_starting_hand_size(p(0)),
        9,
        "CR 902.5a: a +2 hand modifier draws nine cards for each mulligan"
    );
    assert_eq!(game.vanguard_starting_hand_size(p(1)), 6);
}

// ---------------------------------------------------------------------------
// CR 903 Commander
// ---------------------------------------------------------------------------

fn compile(name: &str, text: &str) -> CardDefinition {
    ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false)
        .unwrap_or_else(|error| panic!("{name} should compile: {error:?}"))
}

fn wubrg() -> ColorSet {
    ColorSet::WHITE
        .union(ColorSet::BLUE)
        .union(ColorSet::BLACK)
        .union(ColorSet::RED)
        .union(ColorSet::GREEN)
}

fn legendary_creature(name: &str, power: i32, indicator: Option<ColorSet>) -> CardDefinition {
    let mut builder = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .supertypes(vec![Supertype::Legendary])
        .power_toughness(ironsmith::PowerToughness::fixed(power, power.max(1)));
    if let Some(colors) = indicator {
        builder = builder.color_indicator(colors);
    }
    builder.build()
}

fn identity_mana_source(game: &mut GameState, owner: PlayerId) -> ObjectId {
    use ironsmith::effects::AddManaFromCommanderColorIdentityEffect;
    let definition = CardDefinitionBuilder::new(CardId::new(), "Identity signet")
        .card_types(vec![CardType::Artifact])
        .with_ability(Ability::mana_with_effects(
            TotalCost::free(),
            vec![Effect::new(AddManaFromCommanderColorIdentityEffect::you(1))],
        ))
        .build();
    game.create_object_from_definition(&definition, owner, Zone::Battlefield)
}

fn pool_total(game: &GameState, player: PlayerId) -> u32 {
    let pool = &game.player(player).unwrap().mana_pool;
    pool.white + pool.blue + pool.black + pool.red + pool.green + pool.colorless
}

#[test]
fn cr_903_3a_can_be_your_commander_ability_compiles_as_a_deck_construction_static() {
    let grist = compile(
        "Grist, the Hunger Tide",
        "Mana cost: {1}{B}{G}\nType: Legendary Planeswalker — Grist\nLoyalty: 3\nAs long as Grist isn't on the battlefield, it's a 1/1 Insect creature in addition to its other types.\n+1: Create a 1/1 black and green Insect creature token, then mill a card. If an Insect card was milled this way, put a loyalty counter on Grist and repeat this process.\n−2: You may sacrifice a creature. When you do, destroy target creature or planeswalker.\n−5: Each opponent loses life equal to the number of creature cards in your graveyard.\nGrist, the Hunger Tide can be your commander.",
    );
    assert!(
        grist.abilities.iter().any(|ability| matches!(
            &ability.kind,
            ironsmith::AbilityKind::Static(ability)
                if ability.id() == ironsmith::static_abilities::StaticAbilityId::CanBeCommander
        )),
        "CR 903.3a: 'can be your commander' is a deck-construction static ability"
    );
}

#[test]
fn cr_903_3d_controlling_a_commander_means_a_commander_permanent_on_the_battlefield() {
    let mut game = GameState::new(names(2), 40);
    let definition = legendary_creature("Zone commander", 2, Some(ColorSet::WHITE));
    let commander = game.create_object_from_definition(&definition, p(0), Zone::Command);
    game.set_as_commander(commander, p(0));
    assert!(
        !game.player_controls_a_commander(p(0)) && !game.player_controls_own_commander(p(0)),
        "CR 903.3d: a commander in the command zone is not controlled"
    );
    let on_battlefield = game.move_object_by_effect(commander, Zone::Battlefield).unwrap();
    assert!(game.is_commander(on_battlefield));
    assert!(
        game.player_controls_a_commander(p(0)) && game.player_controls_own_commander(p(0)),
        "CR 903.3d: a commander permanent on the battlefield is controlled"
    );
    assert!(!game.player_controls_a_commander(p(1)));
}

#[test]
fn cr_903_3e_your_commander_is_found_in_library_and_hand() {
    for zone in [Zone::Library, Zone::Hand, Zone::Graveyard, Zone::Exile] {
        let mut game = GameState::new(names(2), 40);
        let definition = legendary_creature("Roaming commander", 2, Some(ColorSet::BLUE));
        let commander = game.create_object_from_definition(&definition, p(0), Zone::Command);
        game.set_as_commander(commander, p(0));
        let moved = game.move_object_by_effect(commander, zone).unwrap();
        assert!(game.is_commander(moved), "CR 903.3e: the commander is still found in {zone:?}");
        let source = identity_mana_source(&mut game, p(0));
        ironsmith::special_actions::perform_activate_mana_ability(
            &mut game,
            p(0),
            source,
            0,
            &mut ironsmith::decision::SelectFirstDecisionMaker,
        )
        .unwrap();
        assert_eq!(
            game.player(p(0)).unwrap().mana_pool.blue,
            1,
            "CR 903.3e: 'your commander' references see the commander while it is in {zone:?}"
        );
    }
}

#[test]
fn cr_903_4_color_identity_counts_cost_rules_text_indicator_and_cdas() {
    let bosh = compile(
        "Bosh, Iron Golem",
        "Mana cost: {8}\nType: Legendary Artifact Creature — Golem\nPower/Toughness: 6/7\nTrample\n{3}{R}, Sacrifice an artifact: Bosh deals damage equal to the sacrificed artifact's mana value to any target.",
    );
    assert_eq!(bosh.card.color_identity(), ColorSet::RED, "CR 903.4 example: Bosh is red");
    let courier = compile(
        "Transguild Courier",
        "Mana cost: {4}\nType: Artifact Creature — Golem\nPower/Toughness: 3/3\nTransguild Courier is all colors.",
    );
    assert_eq!(
        courier.card.color_identity(),
        wubrg(),
        "CR 903.4: colors defined by a characteristic-defining ability are in the identity"
    );
    let indicator = legendary_creature("Indicator card", 1, Some(ColorSet::GREEN));
    assert_eq!(indicator.card.color_identity(), ColorSet::GREEN, "CR 903.4: color indicator counts");
    let hybrid = compile(
        "Hybrid identity",
        "Mana cost: {2}{W/U}\nType: Creature — Bird\nPower/Toughness: 1/1\n{B}: This creature gets +1/+0 until end of turn.",
    );
    assert_eq!(
        hybrid.card.color_identity(),
        ColorSet::WHITE.union(ColorSet::BLUE).union(ColorSet::BLACK),
        "CR 903.4: hybrid cost symbols and rules-text symbols both count"
    );
}

#[test]
fn cr_903_4_cda_that_disclaims_identity_does_not_add_colors() {
    let wayfarer = compile(
        "Fallaji Wayfarer",
        "Mana cost: {2}{G}\nType: Creature — Human Scout\nPower/Toughness: 3/3\nFallaji Wayfarer is all colors. This ability doesn't affect its color identity.\nMulticolored spells you cast have convoke.",
    );
    assert_eq!(
        wayfarer.card.color_identity(),
        ColorSet::GREEN,
        "CR 903.4: an ability stating it doesn't affect color identity adds no identity colors"
    );
}

#[test]
fn cr_903_4a_commander_identity_is_fixed_when_designated() {
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    let mut game = GameState::new(names(2), 40);
    let definition = legendary_creature("Shifting commander", 2, Some(ColorSet::BLUE));
    let commander = game.create_object_from_definition(&definition, p(0), Zone::Battlefield);
    game.set_as_commander(commander, p(0));
    let mut ctx = EffectContext::new_default(commander, p(0));
    ApplyContinuousEffect::new(
        EffectTarget::Specific(commander),
        Modification::SetColors(ColorSet::RED),
        ironsmith::Until::Forever,
    )
    .execute(&mut game, &mut ctx)
    .unwrap();
    game.refresh_continuous_state().unwrap();
    assert_eq!(
        game.get_commander_color_identity(p(0)),
        ColorSet::BLUE,
        "CR 903.4a: in-game color identity is the identity established as the game began"
    );
}

#[test]
fn cr_903_4b_prismatic_piper_chosen_color_is_its_identity() {
    let piper = compile(
        "The Prismatic Piper",
        "Mana cost: {5}\nType: Legendary Creature — Shapeshifter\nPower/Toughness: 3/3\nIf The Prismatic Piper is your commander, choose a color before the game begins. The Prismatic Piper is the chosen color.\nPartner (You can have two commanders if both have partner.)",
    );
    let mut game = GameState::new(names(2), 40);
    let commander = game.create_object_from_definition(&piper, p(0), Zone::Command);
    assert!(
        piper.abilities.iter().any(|ability| matches!(
            &ability.kind,
            ironsmith::AbilityKind::Static(ability)
                if matches!(ability.pregame_action_kind(), Some(ironsmith::static_abilities::PregameActionKind::ChooseColor))
        )),
        "fixture: Piper compiles a typed choose-color pregame action"
    );
    // The pregame action records the chosen color on the card before it is designated.
    game.set_chosen_color(commander, ironsmith::Color::Blue);
    game.set_as_commander(commander, p(0));
    game.refresh_continuous_state().unwrap();
    assert_eq!(
        game.current_colors(commander),
        Some(ColorSet::BLUE),
        "CR 903.4b: the color chosen before the game applies throughout the game (Piper is the chosen color in the command zone)"
    );
    assert_eq!(
        game.get_commander_color_identity(p(0)),
        ColorSet::BLUE,
        "CR 903.4b: the color chosen before the game affects the commander's color identity"
    );
}

#[test]
fn cr_903_4c_reminder_text_is_ignored_for_color_identity() {
    let card = CardBuilder::new(CardId::new(), "Extort reminder")
        .card_types(vec![CardType::Creature])
        .oracle_text("Extort (Whenever you cast a spell, you may pay {W/B}. If you do, each opponent loses 1 life and you gain that much life.)")
        .build();
    assert_eq!(
        card.color_identity(),
        ColorSet::COLORLESS,
        "CR 903.4c: mana symbols in reminder text don't count"
    );
    let control = CardBuilder::new(CardId::new(), "Real symbol")
        .card_types(vec![CardType::Creature])
        .oracle_text("{T}: Add {G}.")
        .build();
    assert_eq!(control.color_identity(), ColorSet::GREEN);
}

#[test]
fn cr_903_4f_identity_mana_does_nothing_without_a_commander() {
    let mut game = GameState::new(names(2), 20);
    let source = identity_mana_source(&mut game, p(0));
    let _ = ironsmith::special_actions::perform_activate_mana_ability(
        &mut game,
        p(0),
        source,
        0,
        &mut ironsmith::decision::SelectFirstDecisionMaker,
    );
    assert_eq!(
        pool_total(&game, p(0)),
        0,
        "CR 903.4f: with no commander the identity is undefined and the ability adds no mana; pool is {:?}",
        game.player(p(0)).unwrap().mana_pool
    );
}

#[test]
fn cr_903_8_commander_tax_adds_two_per_previous_command_zone_cast_and_only_owner_casts() {
    let mut game = GameState::new(names(2), 40);
    set_main_phase(&mut game, p(0));
    let definition = CardDefinitionBuilder::new(CardId::new(), "Taxed commander")
        .card_types(vec![CardType::Creature])
        .supertypes(vec![Supertype::Legendary])
        .mana_cost(ironsmith::ManaCost::from_symbols(vec![ironsmith::ManaSymbol::Generic(1)]))
        .power_toughness(ironsmith::PowerToughness::fixed(1, 1))
        .build();
    let commander = game.create_object_from_definition(&definition, p(0), Zone::Command);
    game.set_as_commander(commander, p(0));
    let mut values = Vec::new();
    for _ in 0..3 {
        let cost = ironsmith::decision::spell_mana_cost_for_cast(
            &game,
            p(0),
            game.object(commander).unwrap(),
            &CastingMethod::Normal,
            Zone::Command,
        )
        .expect("commander cost");
        values.push(cost.mana_value());
        game.record_commander_cast_from_command_zone(commander);
    }
    assert_eq!(values, vec![1, 3, 5], "CR 903.8: commander tax is {{2}} per previous command-zone cast");
    assert!(
        !ironsmith::decision::compute_commander_actions(&game, p(1))
            .unwrap()
            .iter()
            .any(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == commander)),
        "CR 903.8: only the commander's owner may cast it from the command zone"
    );
}

fn commander_combat(game: &mut GameState, attacker: ObjectId, defender: PlayerId) {
    use ironsmith::combat_state::{self, AttackTarget};
    use ironsmith::{Step, TurnAction, TurnRunner, TurnRunnerState};
    game.turn.active_player = game.object(attacker).unwrap().owner;
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(Step::DeclareAttackers);
    let mut combat = combat_state::new_combat();
    combat_state::declare_attackers(game, &mut combat, vec![(attacker, AttackTarget::Player(defender))])
        .unwrap();
    game.turn.step = Some(Step::DeclareBlockers);
    game.combat = Some(combat.clone());
    let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::CombatDamageRegular);
    *runner.combat_mut() = combat;
    let mut queue = TriggerQueue::new();
    for _ in 0..2 {
        assert!(matches!(runner.advance(game, &mut queue).unwrap(), TurnAction::Continue));
    }
}

fn hasty_creature(name: &str, power: i32, legendary: bool) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .supertypes(if legendary { vec![Supertype::Legendary] } else { vec![] })
        .power_toughness(ironsmith::PowerToughness::fixed(power, 1))
        .with_ability(Ability::static_ability(ironsmith::StaticAbility::haste()))
        .build()
}

#[test]
fn cr_903_10a_twenty_one_combat_damage_from_one_commander_loses_the_game() {
    for (power, is_commander, loses) in [(21, true, true), (20, true, false), (21, false, false)] {
        let mut game = GameState::new(names(2), 40);
        let attacker = game.create_object_from_definition(
            &hasty_creature("Combat commander", power, true),
            p(0),
            Zone::Battlefield,
        );
        if is_commander {
            game.set_as_commander(attacker, p(0));
        }
        commander_combat(&mut game, attacker, p(1));
        assert_eq!(game.player(p(1)).unwrap().life, 40 - power);
        apply_state_based_actions(&mut game).unwrap();
        assert_eq!(
            game.player(p(1)).unwrap().has_lost,
            loses,
            "CR 903.10a: power={power}, commander={is_commander}: 21+ combat damage from one commander loses"
        );
    }
    let mut game = GameState::new(names(2), 40);
    for power in [11, 10] {
        let attacker = game.create_object_from_definition(
            &hasty_creature(&format!("Partner {power}"), power, true),
            p(0),
            Zone::Battlefield,
        );
        game.set_as_commander(attacker, p(0));
        commander_combat(&mut game, attacker, p(1));
    }
    apply_state_based_actions(&mut game).unwrap();
    assert!(!game.player(p(1)).unwrap().has_lost, "CR 903.10a: damage must come from the same commander");
    let mut game = GameState::new(names(2), 40);
    let commander = game.create_object_from_definition(
        &hasty_creature("Burning commander", 1, true),
        p(0),
        Zone::Battlefield,
    );
    game.set_as_commander(commander, p(0));
    let mut ctx = EffectContext::new_default(commander, p(0));
    ironsmith::effects::DealDamageEffect::new(21, ironsmith::ChooseSpec::SpecificPlayer(p(1)))
        .execute(&mut game, &mut ctx)
        .unwrap();
    apply_state_based_actions(&mut game).unwrap();
    assert!(!game.player(p(1)).unwrap().has_lost, "CR 903.10a: only combat damage counts");
}

#[test]
fn cr_903_12h_brawl_profile_skips_the_commander_damage_loss() {
    let mut game = GameState::new(names(2), 30);
    game.set_commander_damage_loss_enabled(false);
    let attacker = game.create_object_from_definition(
        &hasty_creature("Brawl commander", 21, true),
        p(0),
        Zone::Battlefield,
    );
    game.set_as_commander(attacker, p(0));
    commander_combat(&mut game, attacker, p(1));
    assert_eq!(game.player(p(1)).unwrap().commander_damage_from(attacker), 21);
    apply_state_based_actions(&mut game).unwrap();
    assert!(
        !game.player(p(1)).unwrap().has_lost,
        "CR 903.12h: Brawl does not use the 21-commander-damage state-based action"
    );
}

// ---------------------------------------------------------------------------
// CR 904 Archenemy
// ---------------------------------------------------------------------------

fn scheme(name: &str, ongoing: bool) -> CardDefinition {
    let mut builder = CardBuilder::new(CardId::new(), name).card_types(vec![CardType::Scheme]);
    if ongoing {
        builder = builder.supertypes(vec![Supertype::Ongoing]);
    }
    CardDefinition::new(builder.build())
}

fn scheme_deck(prefix: &str, size: usize) -> Vec<CardDefinition> {
    (0..size)
        .map(|index| scheme(&format!("{prefix} Scheme {index}"), false))
        .collect()
}

fn scheme_named(game: &GameState, name: &str) -> ObjectId {
    game.archenemy
        .as_ref()
        .unwrap()
        .scheme_decks
        .values()
        .flatten()
        .chain(game.face_up_schemes())
        .copied()
        .find(|object| game.object(*object).is_some_and(|card| card.name == name))
        .unwrap()
}

fn put_scheme_on_top(game: &mut GameState, player: PlayerId, object: ObjectId) {
    let deck = game.archenemy.as_mut().unwrap().scheme_decks.get_mut(&player).unwrap();
    let index = deck.iter().position(|candidate| *candidate == object).unwrap();
    deck.remove(index);
    deck.push(object);
}

#[test]
fn cr_904_2_default_archenemy_is_two_teams_with_shared_team_turns() {
    let mut game = GameState::new(names(4), 20);
    game.enable_archenemy(ArchenemyVariant::Default, vec![(p(0), scheme_deck("Arch", 20))])
        .unwrap();
    let observed = (
        game.are_teammates(p(1), p(2)) && game.are_teammates(p(2), p(3)),
        game.are_opponents(p(0), p(1)) && !game.are_opponents(p(1), p(2)),
        game.shared_team_turns_enabled(),
    );
    assert_eq!(
        observed,
        (true, true, true),
        "CR 904.2: the default Archenemy setup is Team vs. Team (archenemy alone vs. everyone else) with shared team turns; observed (opponents are teammates, team opponency, shared turns) = {observed:?}"
    );
}

#[test]
fn cr_904_2a_b_exactly_one_archenemy_against_any_number_of_players() {
    for players in [2usize, 3, 5] {
        let mut game = GameState::new(names(players), 20);
        game.enable_archenemy(ArchenemyVariant::Default, vec![(p(1), scheme_deck("Arch", 20))])
            .expect("CR 904.2b: the opposing side may have any number of players");
        let archenemies = (0..players).filter(|index| game.is_archenemy(p(*index as u8))).count();
        assert_eq!(archenemies, 1, "CR 904.2a: exactly one archenemy");
    }
    let mut two = GameState::new(names(4), 20);
    assert!(
        two.enable_archenemy(
            ArchenemyVariant::Default,
            vec![(p(0), scheme_deck("A", 20)), (p(1), scheme_deck("B", 20))]
        )
        .is_err(),
        "CR 904.2a: the archenemy's team consists of exactly one player"
    );
}

#[test]
fn cr_904_3_904_13d_scheme_deck_size_and_name_limits() {
    let try_deck = |variant: ArchenemyVariant, deck: Vec<CardDefinition>| {
        GameState::new(names(3), 20).enable_archenemy(variant, vec![(p(0), deck)])
    };
    assert!(try_deck(ArchenemyVariant::Default, scheme_deck("Twenty", 20)).is_ok());
    assert!(
        try_deck(ArchenemyVariant::Default, scheme_deck("Nineteen", 19)).is_err(),
        "CR 904.3: at least twenty scheme cards"
    );
    let mut two_copies = scheme_deck("Copies", 20);
    two_copies[1] = scheme("Copies Scheme 0", false);
    assert!(try_deck(ArchenemyVariant::Default, two_copies.clone()).is_ok(), "CR 904.3: two copies allowed");
    two_copies[2] = scheme("Copies Scheme 0", false);
    assert!(
        try_deck(ArchenemyVariant::Default, two_copies).is_err(),
        "CR 904.3: no more than two of any English name"
    );
    assert!(
        try_deck(ArchenemyVariant::Commander, scheme_deck("Ten", 10)).is_ok(),
        "CR 904.13d: Archenemy Commander needs at least ten schemes"
    );
    assert!(try_deck(ArchenemyVariant::Commander, scheme_deck("Nine", 9)).is_err());
    let mut duplicate = scheme_deck("Singleton", 10);
    duplicate[1] = scheme("Singleton Scheme 0", false);
    assert!(
        try_deck(ArchenemyVariant::Commander, duplicate).is_err(),
        "CR 904.13d: each card must have a different English name"
    );
}

#[test]
fn cr_904_4_904_7_schemes_stay_in_command_owned_and_controlled_by_archenemy() {
    let mut game = GameState::new(names(3), 20);
    game.enable_archenemy(ArchenemyVariant::Default, vec![(p(0), scheme_deck("Arch", 20))])
        .unwrap();
    let face_down = game.scheme_deck(p(0)).unwrap()[0];
    let face_up = game.set_scheme_in_motion(p(0)).unwrap();
    for object in [face_down, face_up] {
        assert_eq!(game.object(object).unwrap().owner, p(0), "CR 904.7: owner is the archenemy");
        for zone in [Zone::Graveyard, Zone::Hand, Zone::Battlefield, Zone::Exile] {
            game.move_object_by_effect(object, zone);
            assert_eq!(
                game.object(object).map(|card| card.zone),
                Some(Zone::Command),
                "CR 904.4: scheme cards remain in the command zone ({zone:?})"
            );
        }
    }
    let _ = game.set_current_controller(face_up, p(1));
    assert_eq!(
        game.controller_of_id(face_up),
        Some(p(0)),
        "CR 904.7: the controller of a face-up scheme is its owner"
    );
}

#[test]
fn cr_904_6_archenemy_takes_the_first_turn() {
    for seat in 0..4u8 {
        let mut game = GameState::new(names(4), 20);
        game.enable_archenemy(ArchenemyVariant::Default, vec![(p(seat), scheme_deck("Arch", 20))])
            .unwrap();
        assert_eq!(game.turn.active_player, p(seat), "CR 904.6: the archenemy takes the first turn");
    }
}

#[test]
fn cr_904_8_face_up_scheme_abilities_function_from_command() {
    let mut cards = scheme_deck("Arch", 20);
    cards[19] = scheme("Triggered Scheme", false);
    cards[19].abilities.push(gain_life_trigger_on(KeywordActionKind::SetSchemeInMotion));
    cards[19].abilities.push(Ability::activated(TotalCost::free(), vec![Effect::gain_life(1)]));
    let mut game = GameState::new(names(2), 20);
    game.enable_archenemy(ArchenemyVariant::Default, vec![(p(0), cards)]).unwrap();
    set_main_phase(&mut game, p(0));
    let source = scheme_named(&game, "Triggered Scheme");
    assert!(
        !ironsmith::decision::compute_legal_actions(&game, p(0))
            .unwrap()
            .iter()
            .any(|action| matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source)),
        "control: face-down scheme abilities do not function"
    );
    put_scheme_on_top(&mut game, p(0), source);
    game.set_scheme_in_motion(p(0)).unwrap();
    assert!(
        ironsmith::decision::compute_legal_actions(&game, p(0))
            .unwrap()
            .iter()
            .any(|action| matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source)),
        "CR 904.8: a face-up scheme's activated ability may be activated"
    );
    let events = game.take_pending_trigger_events();
    let motion = keyword_events(&events, KeywordActionKind::SetSchemeInMotion);
    assert_eq!(check_triggers(&game, motion[0]).len(), 1, "CR 904.8: triggered abilities may trigger");
}

#[test]
fn cr_904_9_precombat_main_turn_action_sets_a_scheme_in_motion_without_the_stack() {
    use ironsmith::{TurnAction, TurnRunner, TurnRunnerState};
    let mut game = GameState::new(names(2), 20);
    let mut cards = scheme_deck("Turn", 20);
    cards[19].abilities.push(gain_life_trigger_on(KeywordActionKind::SetSchemeInMotion));
    game.enable_archenemy(ArchenemyVariant::Default, vec![(p(0), cards)]).unwrap();
    let triggered = scheme_named(&game, "Turn Scheme 19");
    put_scheme_on_top(&mut game, p(0), triggered);
    let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::FirstMain);
    let mut queue = TriggerQueue::new();
    let action = runner.advance(&mut game, &mut queue).unwrap();
    assert!(matches!(action, TurnAction::RunPriority));
    assert_eq!(
        game.face_up_schemes(),
        &[triggered],
        "CR 904.9: the top scheme is set in motion as the precombat main phase begins"
    );
    assert!(
        !game.stack.iter().any(|entry| entry.object_id == triggered && !entry.is_ability),
        "CR 904.9: the turn-based action doesn't use the stack"
    );
    let pending = game.take_pending_trigger_events();
    let motion = keyword_events(&pending, KeywordActionKind::SetSchemeInMotion);
    let triggered_count = motion.iter().map(|event| check_triggers(&game, event).len()).sum::<usize>()
        + queue.entries.len()
        + game.stack.len();
    assert!(
        triggered_count >= 1,
        "CR 904.9: 'When you set this scheme in motion' abilities trigger"
    );
}

#[test]
fn cr_904_11_ongoing_scheme_stays_face_up_until_abandoned() {
    let mut cards = scheme_deck("Arch", 20);
    cards[18] = scheme("Plain scheme", false);
    cards[19] = scheme("Ongoing scheme", true);
    let mut game = GameState::new(names(2), 20);
    game.enable_archenemy(ArchenemyVariant::Default, vec![(p(0), cards)]).unwrap();
    let plain = scheme_named(&game, "Plain scheme");
    let ongoing = scheme_named(&game, "Ongoing scheme");
    put_scheme_on_top(&mut game, p(0), plain);
    put_scheme_on_top(&mut game, p(0), ongoing);
    game.set_scheme_in_motion(p(0)).unwrap();
    game.set_scheme_in_motion(p(0)).unwrap();
    game.take_pending_trigger_events();
    apply_state_based_actions(&mut game).unwrap();
    apply_state_based_actions(&mut game).unwrap();
    assert_eq!(
        game.face_up_schemes(),
        &[ongoing],
        "CR 904.11: an ongoing scheme remains face up while a non-ongoing one is turned down"
    );
    game.next_turn();
    game.next_turn();
    apply_state_based_actions(&mut game).unwrap();
    assert_eq!(game.face_up_schemes(), &[ongoing], "CR 904.11: still face up turns later");
    game.abandon_scheme(ongoing).unwrap();
    assert!(game.face_up_schemes().is_empty(), "CR 904.11: abandoning ends it");
}

#[test]
fn cr_904_13c_archenemy_commander_poison_is_individual() {
    for poisoned in [0u8, 2u8] {
        let mut game = GameState::new(names(4), 40);
        game.enable_archenemy(ArchenemyVariant::Commander, vec![(p(0), scheme_deck("Arch", 10))])
            .unwrap();
        game.player_mut(p(poisoned)).unwrap().poison_counters = 10;
        game.player_mut(p(1)).unwrap().poison_counters = 9;
        game.player_mut(p(3)).unwrap().poison_counters = 6;
        apply_state_based_actions(&mut game).unwrap();
        let lost = (0..4).filter(|index| game.player(p(*index)).unwrap().has_lost).collect::<Vec<_>>();
        assert_eq!(
            lost,
            vec![poisoned],
            "CR 904.13c: only the individual with ten or more poison counters loses"
        );
    }
}

// ---------------------------------------------------------------------------
// CR 905 Conspiracy Draft
// ---------------------------------------------------------------------------

fn plain_card(name: &str) -> CardDefinition {
    CardDefinition::new(CardBuilder::new(CardId::new(), name).build())
}

fn draft_packs(prefix: &str, cards_per_pack: usize) -> Vec<Vec<CardDefinition>> {
    (1..=3)
        .map(|round| {
            (0..cards_per_pack)
                .map(|index| plain_card(&format!("{prefix} R{round} C{index}")))
                .collect()
        })
        .collect()
}

fn first_pick(draft: &ironsmith::ConspiracyDraftState, player: PlayerId) -> ironsmith::DraftSelection {
    ironsmith::DraftSelection {
        player,
        card_ids: vec![draft.current_pack_view(player, player)[0].id],
        exchange_face_up: None,
        public_notes: std::collections::HashMap::new(),
    }
}

fn three_seat_draft(cards_per_pack: usize) -> ironsmith::ConspiracyDraftState {
    ironsmith::ConspiracyDraftState::new(
        vec![p(0), p(1), p(2)],
        vec![
            (p(0), draft_packs("Alice", cards_per_pack)),
            (p(1), draft_packs("Bob", cards_per_pack)),
            (p(2), draft_packs("Cara", cards_per_pack)),
        ],
    )
    .unwrap()
}

fn all_first_picks(draft: &ironsmith::ConspiracyDraftState, seats: &[PlayerId]) -> Vec<ironsmith::DraftSelection> {
    seats.iter().map(|player| first_pick(draft, *player)).collect()
}

#[test]
fn cr_905_1a_b_one_card_picks_pass_left_right_left() {
    let seats = [p(0), p(1), p(2)];
    let mut draft = three_seat_draft(2);
    let doubled = seats
        .iter()
        .map(|player| {
            let pack = draft.current_pack_view(*player, *player);
            ironsmith::DraftSelection { card_ids: vec![pack[0].id, pack[1].id], ..first_pick(&draft, *player) }
        })
        .collect();
    assert!(draft.draft_step(doubled).is_err(), "CR 905.1a: each pick drafts exactly one card");
    draft.draft_step(all_first_picks(&draft, &seats)).unwrap();
    assert_eq!(
        draft.current_pack_view(p(1), p(1))[0].name.as_deref(),
        Some("Alice R1 C1"),
        "CR 905.1b: round one passes to the left"
    );
    draft.draft_step(all_first_picks(&draft, &seats)).unwrap();
    assert_eq!(draft.round(), 2);
    draft.draft_step(all_first_picks(&draft, &seats)).unwrap();
    assert_eq!(
        draft.current_pack_view(p(2), p(2))[0].name.as_deref(),
        Some("Alice R2 C1"),
        "CR 905.1b: round two passes to the right"
    );
    draft.draft_step(all_first_picks(&draft, &seats)).unwrap();
    draft.draft_step(all_first_picks(&draft, &seats)).unwrap();
    assert_eq!(
        draft.current_pack_view(p(1), p(1))[0].name.as_deref(),
        Some("Alice R3 C1"),
        "CR 905.1b: round three passes to the left"
    );
    draft.draft_step(all_first_picks(&draft, &seats)).unwrap();
    assert!(draft.is_complete(), "CR 905.1a: three draft rounds");
    for player in seats {
        assert_eq!(draft.card_pool(player).unwrap().len(), 6);
    }
}

#[test]
fn cr_905_1c_players_see_only_their_pack_and_their_drafted_cards() {
    let seats = [p(0), p(1), p(2)];
    let mut draft = three_seat_draft(3);
    assert!(draft.current_pack_view(p(0), p(0)).iter().all(|card| card.name.is_some()));
    assert!(
        draft.current_pack_view(p(1), p(0)).iter().all(|card| card.name.is_none()),
        "CR 905.1c: another player's current pack is hidden"
    );
    draft.draft_step(all_first_picks(&draft, &seats)).unwrap();
    assert!(draft.drafted_view(p(0), p(0)).iter().all(|card| card.name.is_some()));
    assert!(
        draft.drafted_view(p(1), p(0)).iter().all(|card| card.name.is_none()),
        "CR 905.1c: drafted cards are not revealed to other players"
    );
}

#[test]
fn cr_905_1d_deck_comes_from_the_pool_plus_basics_with_forty_card_minimum() {
    let seats = [p(0), p(1)];
    let mut draft = ironsmith::ConspiracyDraftState::new(
        vec![p(0), p(1)],
        vec![(p(0), draft_packs("Alice", 14)), (p(1), draft_packs("Bob", 14))],
    )
    .unwrap();
    while !draft.is_complete() {
        draft.draft_step(all_first_picks(&draft, &seats)).unwrap();
    }
    let pool = draft.card_pool(p(0)).unwrap();
    let basic = CardDefinition::new(
        CardBuilder::new(CardId::new(), "Island")
            .supertypes(vec![Supertype::Basic])
            .card_types(vec![CardType::Land])
            .build(),
    );
    let mut legal = pool.iter().take(20).cloned().collect::<Vec<_>>();
    legal.extend(std::iter::repeat_n(basic, 20));
    draft.validate_deck(p(0), &legal).expect("CR 905.1d: pool cards plus any number of basics");
    assert!(draft.validate_deck(p(0), &legal[..39]).is_err(), "CR 905.1d/100.2b: at least forty cards");
    let mut foreign = legal.clone();
    foreign[0] = plain_card("Undrafted card");
    assert!(draft.validate_deck(p(0), &foreign).is_err(), "CR 905.1d: only drafted cards");
    let mut bob_card = legal;
    bob_card[0] = draft.card_pool(p(1)).unwrap()[0].clone();
    assert!(
        draft.validate_deck(p(0), &bob_card).is_err(),
        "CR 905.1d: another player's drafted card is not in this pool"
    );
}

#[test]
fn cr_905_2_905_2b_905_2c_draft_abilities_face_up_and_noted_information() {
    let cogwork = compile(
        "Cogwork Librarian",
        "Mana cost: {4}\nType: Artifact Creature — Construct\nPower/Toughness: 3/3\nDraft this card face up.\nAs you draft a card, you may draft an additional card from that booster pack. If you do, put this card into that booster pack.",
    );
    let pyretic = compile(
        "Pyretic Hunter",
        "Mana cost: {4}{R}\nType: Creature — Elemental Cat\nPower/Toughness: 1/1\nReveal this card as you draft it and note how many cards you've drafted this draft round, including this card.\nMenace (This creature can't be blocked except by two or more creatures.)\nThis creature enters with X +1/+1 counters on it, where X is the highest number you noted for cards named Pyretic Hunter.",
    );
    let mut alice_packs = draft_packs("Alice", 3);
    alice_packs[0][0] = cogwork;
    let mut bob_packs = draft_packs("Bob", 3);
    bob_packs[0][0] = pyretic;
    let mut draft = ironsmith::ConspiracyDraftState::new(
        vec![p(0), p(1)],
        vec![(p(0), alice_packs), (p(1), bob_packs)],
    )
    .unwrap();
    let cogwork_id = draft.current_pack_view(p(0), p(0))[0].id;
    let bob_other = draft.current_pack_view(p(1), p(1))[1].id;
    draft
        .draft_step(vec![
            ironsmith::DraftSelection { card_ids: vec![cogwork_id], ..first_pick(&draft, p(0)) },
            ironsmith::DraftSelection { card_ids: vec![bob_other], ..first_pick(&draft, p(1)) },
        ])
        .unwrap();
    assert_eq!(
        draft.drafted_view(p(1), p(0))[0].name.as_deref(),
        Some("Cogwork Librarian"),
        "CR 905.2c: a card drafted face up may be looked at by all players"
    );
    let pack = draft.current_pack_view(p(0), p(0));
    let pyretic_id = pack.iter().find(|card| card.name.as_deref() == Some("Pyretic Hunter")).unwrap().id;
    let other = pack.iter().find(|card| card.id != pyretic_id).unwrap().id;
    draft
        .draft_step(vec![
            ironsmith::DraftSelection {
                player: p(0),
                card_ids: vec![pyretic_id, other],
                exchange_face_up: Some(cogwork_id),
                public_notes: std::collections::HashMap::from([(pyretic_id, "2".to_string())]),
            },
            first_pick(&draft, p(1)),
        ])
        .expect("CR 905.2: Cogwork Librarian's draft ability functions during the draft");
    let view = draft.drafted_view(p(1), p(0));
    let noted = view.iter().find(|card| card.id == pyretic_id).unwrap();
    assert_eq!(
        noted.public_note.as_deref(),
        Some("2"),
        "CR 905.2b: noted information can be looked at by any player"
    );
    assert!(
        !view.iter().any(|card| card.id == cogwork_id),
        "CR 905.2c: the face-up card left the drafted pile when exchanged"
    );
}

#[test]
fn cr_905_4_905_5_905_6_conspiracies_start_in_command_owned_by_their_player() {
    let public = CardDefinitionBuilder::new(CardId::new(), "Public Conspiracy")
        .card_types(vec![CardType::Conspiracy])
        .build();
    let mut game = GameState::new(names(3), 99);
    game.enable_conspiracy(vec![(
        p(1),
        vec![ironsmith::ConspiracySetupCard { definition: public, agenda_names: vec![] }],
    )])
    .expect("CR 905.4: conspiracy cards may be put into the command zone");
    let cards = game.conspiracy_cards();
    assert_eq!(cards.len(), 1);
    let card = cards[0];
    assert_eq!(game.object(card).unwrap().zone, Zone::Command, "CR 905.4: into the command zone");
    assert_eq!(game.object(card).unwrap().owner, p(1), "CR 905.5: owner is the player who put it there");
    let _ = game.set_current_controller(card, p(2));
    assert_eq!(game.controller_of_id(card), Some(p(1)), "CR 905.5: controller is its owner");
    for index in 0..3 {
        assert_eq!(game.player(p(index)).unwrap().life, 20, "CR 905.6: each player's life total is 20");
    }
}
