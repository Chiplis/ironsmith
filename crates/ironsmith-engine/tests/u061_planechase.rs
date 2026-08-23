use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::events::{KeywordActionEvent, KeywordActionKind};
use ironsmith::special_actions;
use ironsmith::triggers::Trigger;
use ironsmith::{
    Ability, CardBuilder, CardDefinition, CardId, CardType, CastingMethod, Effect, GameState,
    LegalAction, ManaCost, ObjectId, Phase, PlanarCardKind, PlanarDieFace, PlayerFilter, PlayerId,
    SpecialAction, StackEntry, StateBasedAction, TotalCost, TriggerEvent, TriggerQueue, Zone,
    apply_state_based_actions, check_state_based_actions, check_triggers, put_triggers_on_stack,
    resolve_stack_entry,
};

fn planar_card(name: &str, trigger: Option<KeywordActionKind>) -> CardDefinition {
    let mut definition = CardDefinition::new(
        CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Sorcery])
            .mana_cost(ManaCost::from_symbols(vec![]))
            .build(),
    );
    if let Some(action) = trigger {
        definition.abilities.push(Ability::triggered(
            Trigger::keyword_action_from_source(action, PlayerFilter::You),
            vec![Effect::gain_life(1)],
        ));
    }
    definition
}

fn individual_deck(prefix: &str) -> Vec<(CardDefinition, PlanarCardKind)> {
    (0..10)
        .map(|index| {
            (
                planar_card(&format!("{prefix} Plane {index}"), None),
                PlanarCardKind::Plane,
            )
        })
        .collect()
}

fn enabled_game() -> (GameState, PlayerId, PlayerId) {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.enable_planechase(vec![
        (alice, individual_deck("Alice")),
        (bob, individual_deck("Bob")),
    ])
    .expect("valid individual planar decks");
    (game, alice, bob)
}

fn enabled_game_with_alice_trigger(
    card_index: usize,
    trigger: KeywordActionKind,
) -> (GameState, PlayerId, PlayerId) {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut alice_deck = individual_deck("Alice");
    alice_deck[card_index].0 = planar_card(&format!("Alice Plane {card_index}"), Some(trigger));
    game.enable_planechase(vec![(alice, alice_deck), (bob, individual_deck("Bob"))])
        .expect("valid individual planar decks");
    (game, alice, bob)
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

fn put_on_top(game: &mut GameState, player: PlayerId, object: ObjectId) {
    let state = game.planechase.as_mut().expect("Planechase enabled");
    let deck = state.decks.get_mut(&player).expect("individual deck");
    let index = deck
        .iter()
        .position(|candidate| *candidate == object)
        .expect("card in planar deck");
    deck.remove(index);
    deck.push(object);
}

fn keyword_event(events: &[TriggerEvent], action: KeywordActionKind) -> Option<&TriggerEvent> {
    events.iter().find(|event| {
        event
            .downcast::<KeywordActionEvent>()
            .is_some_and(|event| event.action == action)
    })
}

#[test]
fn u061_plane_and_phenomenon_type_lines_and_triggers_compile() {
    let plane = CardDefinitionBuilder::new(CardId::new(), "Test Plane")
        .card_types(vec![CardType::Plane])
        .parse_text("Whenever chaos ensues, you gain 1 life.")
        .expect("plane type and chaos trigger should compile");
    assert_eq!(plane.card.card_types, vec![CardType::Plane]);
    assert!(matches!(
        plane.abilities.first().map(|ability| &ability.kind),
        Some(ironsmith::AbilityKind::Triggered(_))
    ));

    let phenomenon = CardDefinitionBuilder::new(CardId::new(), "Test Phenomenon")
        .card_types(vec![CardType::Phenomenon])
        .parse_text("When you encounter Test Phenomenon, you gain 1 life.")
        .expect("phenomenon type and encounter trigger should compile");
    assert_eq!(phenomenon.card.card_types, vec![CardType::Phenomenon]);
    assert!(matches!(
        phenomenon.abilities.first().map(|ability| &ability.kind),
        Some(ironsmith::AbilityKind::Triggered(_))
    ));
}

#[test]
fn u061_planar_decks_validate_and_starting_reveal_skips_phenomena_without_triggers() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    assert!(
        game.enable_planechase(vec![
            (alice, individual_deck("Only Alice")),
            (bob, individual_deck("Only Alice")),
        ])
        .is_ok(),
        "names need be unique within each planar deck, not across players"
    );

    let mut invalid = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    assert!(
        invalid
            .enable_planechase(vec![(alice, individual_deck("Alice"))])
            .unwrap_err()
            .contains("each player")
    );
    let mut duplicate_deck = individual_deck("Duplicate");
    duplicate_deck[1].0 = duplicate_deck[0].0.clone();
    assert!(
        GameState::new(vec!["Alice".into(), "Bob".into()], 20)
            .enable_planechase(vec![(alice, duplicate_deck), (bob, individual_deck("Bob")),])
            .unwrap_err()
            .contains("may not contain two cards")
    );

    let (mut game, alice, _) = enabled_game();
    let phenomenon = planar_object_named(&game, "Alice Plane 8");
    let plane = planar_object_named(&game, "Alice Plane 9");
    game.planechase
        .as_mut()
        .unwrap()
        .card_kinds
        .insert(phenomenon, PlanarCardKind::Phenomenon);
    let phenomenon_stable = game.object(phenomenon).unwrap().stable_id;
    put_on_top(&mut game, alice, plane);
    put_on_top(&mut game, alice, phenomenon);

    let revealed = game.reveal_starting_plane().expect("starting plane");
    assert_eq!(revealed, plane);
    assert!(game.take_pending_trigger_events().is_empty());
    assert!(
        game.object(phenomenon).is_none(),
        "face-down is a new object"
    );
    let bottom = game.planar_deck(alice).unwrap()[0];
    assert_eq!(game.object(bottom).unwrap().stable_id, phenomenon_stable);
    assert_eq!(
        game.planar_card_kind(bottom),
        Some(PlanarCardKind::Phenomenon)
    );
}

#[test]
fn u061_planeswalk_recycles_new_objects_and_only_face_up_plane_abilities_function() {
    let (mut game, alice, _) = enabled_game_with_alice_trigger(9, KeywordActionKind::ChaosEnsues);
    let active = planar_object_named(&game, "Alice Plane 9");
    let destination = planar_object_named(&game, "Alice Plane 8");
    let old_stable = game.object(active).unwrap().stable_id;
    put_on_top(&mut game, alice, active);
    game.reveal_starting_plane().unwrap();

    game.chaos_ensues(alice, active).unwrap();
    let chaos_events = game.take_pending_trigger_events();
    let chaos = keyword_event(&chaos_events, KeywordActionKind::ChaosEnsues).unwrap();
    assert_eq!(check_triggers(&game, chaos).len(), 1);

    put_on_top(&mut game, alice, destination);
    assert_eq!(game.planeswalk(alice, active).unwrap(), destination);
    assert!(game.object(active).is_none());
    let recycled = game.planar_deck(alice).unwrap()[0];
    assert_ne!(recycled, active);
    assert_eq!(game.object(recycled).unwrap().stable_id, old_stable);
    assert_eq!(game.face_up_planar_objects(), &[destination]);
    assert_eq!(game.object(destination).unwrap().zone, Zone::Command);
    assert_eq!(
        game.move_object_by_effect(destination, Zone::Graveyard),
        Some(destination)
    );
    assert_eq!(game.object(destination).unwrap().zone, Zone::Command);
    assert!(!ironsmith::decision::can_cast_spell(
        &game,
        alice,
        game.object(destination).unwrap(),
        &CastingMethod::Normal,
    ));

    game.take_pending_trigger_events();
    game.chaos_ensues(alice, destination).unwrap();
    let chaos_events = game.take_pending_trigger_events();
    let chaos = keyword_event(&chaos_events, KeywordActionKind::ChaosEnsues).unwrap();
    assert!(check_triggers(&game, chaos).is_empty());
}

#[test]
fn u061_face_down_planar_abilities_are_dormant_and_planeswalk_away_uses_lki() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut alice_deck = individual_deck("Alice");
    alice_deck[9].0.abilities.extend([
        Ability::triggered(
            Trigger::keyword_action(KeywordActionKind::Planeswalk, PlayerFilter::You),
            vec![Effect::gain_life(1)],
        ),
        Ability::activated(TotalCost::free(), vec![Effect::gain_life(1)]),
    ]);
    game.enable_planechase(vec![(alice, alice_deck), (bob, individual_deck("Bob"))])
        .expect("valid individual planar decks");
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;

    let active = planar_object_named(&game, "Alice Plane 9");
    let destination = planar_object_named(&game, "Alice Plane 8");
    let active_stable = game.object(active).unwrap().stable_id;
    assert!(
        game.object(active)
            .unwrap()
            .abilities
            .iter()
            .all(|ability| ability.functional_zones.is_empty())
    );
    assert!(
        !ironsmith::decision::compute_legal_actions(&game, alice)
            .iter()
            .any(|action| matches!(action, LegalAction::ActivateAbility { source, .. } if *source == active))
    );

    put_on_top(&mut game, alice, active);
    game.reveal_starting_plane().unwrap();
    assert!(
        ironsmith::decision::compute_legal_actions(&game, alice)
            .iter()
            .any(|action| matches!(action, LegalAction::ActivateAbility { source, .. } if *source == active))
    );

    put_on_top(&mut game, alice, destination);
    game.planeswalk(alice, active).unwrap();
    let events = game.take_pending_trigger_events();
    let planeswalk = keyword_event(&events, KeywordActionKind::Planeswalk).unwrap();
    let entries = check_triggers(&game, planeswalk);
    assert_eq!(
        entries.len(),
        1,
        "the departed plane should trigger from LKI"
    );
    assert_eq!(entries[0].source_stable_id, active_stable);

    let recycled = game.planar_deck(alice).unwrap()[0];
    assert!(
        game.object(recycled)
            .unwrap()
            .abilities
            .iter()
            .all(|ability| ability.functional_zones.is_empty())
    );
    assert!(
        !ironsmith::decision::compute_legal_actions(&game, alice)
            .iter()
            .any(|action| matches!(action, LegalAction::ActivateAbility { source, .. } if *source == recycled))
    );
}

#[test]
fn u061_planar_die_is_a_costed_special_action_and_planeswalking_is_not_immediate() {
    let (mut game, alice, _) = enabled_game();
    let active = planar_object_named(&game, "Alice Plane 9");
    put_on_top(&mut game, alice, active);
    game.reveal_starting_plane().unwrap();
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    let mut decisions = ironsmith::decision::SelectFirstDecisionMaker;

    game.force_next_die_roll(6);
    special_actions::perform(
        SpecialAction::RollPlanarDie,
        &mut game,
        alice,
        &mut decisions,
    )
    .expect("first planar roll is free");
    assert_eq!(game.planar_die_roll_cost(alice), Some(1));
    let events = game.take_pending_trigger_events();
    assert!(events.iter().any(|event| {
        event
            .downcast::<ironsmith::events::other::DieRolledEvent>()
            .is_some_and(|event| event.is_planar)
    }));

    game.player_mut(alice).unwrap().mana_pool.colorless = 1;
    game.force_next_die_roll(6);
    special_actions::perform(
        SpecialAction::RollPlanarDie,
        &mut game,
        alice,
        &mut decisions,
    )
    .expect("second roll costs one generic mana");
    assert_eq!(game.player(alice).unwrap().mana_pool.colorless, 0);
    assert_eq!(game.planar_die_roll_cost(alice), Some(2));
    game.take_pending_trigger_events();

    game.force_next_die_roll(2);
    assert_eq!(
        game.roll_planar_die(alice, false).unwrap(),
        PlanarDieFace::Chaos
    );
    assert_eq!(game.planar_die_roll_cost(alice), Some(2));
    let events = game.take_pending_trigger_events();
    assert!(keyword_event(&events, KeywordActionKind::ChaosEnsues).is_some());

    let planeswalks_before = game.planechase.as_ref().unwrap().planeswalk_count;
    game.force_next_die_roll(1);
    assert_eq!(
        game.roll_planar_die(alice, false).unwrap(),
        PlanarDieFace::Planeswalker
    );
    assert_eq!(
        game.planechase.as_ref().unwrap().planeswalk_count,
        planeswalks_before,
        "the planeswalker face creates a triggered ability instead of planeswalking immediately"
    );
    let mut trigger_queue = TriggerQueue::new();
    put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("the game-rule planeswalking ability should use the stack");
    assert_eq!(game.stack.len(), 1);
    assert_eq!(
        game.planechase.as_ref().unwrap().planeswalk_count,
        planeswalks_before
    );
    resolve_stack_entry(&mut game).expect("the planeswalking ability should resolve");
    assert_eq!(
        game.planechase.as_ref().unwrap().planeswalk_count,
        planeswalks_before + 1
    );

    let current_plane = game.face_up_planar_objects()[0];
    game.force_next_die_roll(1);
    assert_eq!(
        game.roll_planar_die(alice, false).unwrap(),
        PlanarDieFace::Planeswalker
    );
    let before_departure = game.planechase.as_ref().unwrap().planeswalk_count;
    game.planeswalk(alice, current_plane).unwrap();
    let after_departure = game.planechase.as_ref().unwrap().planeswalk_count;
    assert_eq!(after_departure, before_departure + 1);
    put_triggers_on_stack(&mut game, &mut trigger_queue).unwrap();
    assert_eq!(game.stack.len(), 1);
    resolve_stack_entry(&mut game).unwrap();
    assert_eq!(
        game.planechase.as_ref().unwrap().planeswalk_count,
        after_departure,
        "a planar-die ability must do nothing if its original plane departed"
    );
}

#[test]
fn u061_planar_rolls_are_observed_as_die_rolls_but_not_as_numerical_results() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut alice_deck = individual_deck("Alice");
    alice_deck[9].0.abilities.extend([
        Ability::triggered(
            Trigger::player_rolls_die(PlayerFilter::You),
            vec![Effect::gain_life(1)],
        ),
        Ability::triggered(
            Trigger::player_rolls_result(PlayerFilter::You, 6),
            vec![Effect::gain_life(1)],
        ),
    ]);
    game.enable_planechase(vec![(alice, alice_deck), (bob, individual_deck("Bob"))])
        .expect("valid individual planar decks");
    let active = planar_object_named(&game, "Alice Plane 9");
    put_on_top(&mut game, alice, active);
    game.reveal_starting_plane().unwrap();
    game.force_next_die_roll(6);
    assert_eq!(
        game.roll_planar_die(alice, false).unwrap(),
        PlanarDieFace::Blank
    );

    let events = game.take_pending_trigger_events();
    let die_event = events
        .iter()
        .find(|event| {
            event
                .downcast::<ironsmith::events::other::DieRolledEvent>()
                .is_some()
        })
        .expect("planar roll should be observable as a die roll");
    let entries = check_triggers(&game, die_event);
    assert_eq!(
        entries.len(),
        1,
        "the generic die-roll trigger should match while the numerical result trigger does not"
    );
}

#[test]
fn u061_phenomenon_waits_for_its_encounter_trigger_then_planeswalks_as_an_sba() {
    let (mut game, alice, _) =
        enabled_game_with_alice_trigger(8, KeywordActionKind::EncounterPhenomenon);
    let starting = planar_object_named(&game, "Alice Plane 9");
    let phenomenon = planar_object_named(&game, "Alice Plane 8");
    let next_plane = planar_object_named(&game, "Alice Plane 7");
    game.planechase
        .as_mut()
        .unwrap()
        .card_kinds
        .insert(phenomenon, PlanarCardKind::Phenomenon);
    put_on_top(&mut game, alice, starting);
    game.reveal_starting_plane().unwrap();
    put_on_top(&mut game, alice, phenomenon);
    assert_eq!(game.planeswalk(alice, starting).unwrap(), phenomenon);
    assert!(
        !check_state_based_actions(&game)
            .contains(&StateBasedAction::PlaneswalkFromPhenomenon(phenomenon)),
        "the pending encounter event protects the phenomenon"
    );

    let events = game.take_pending_trigger_events();
    let encounter = keyword_event(&events, KeywordActionKind::EncounterPhenomenon).unwrap();
    let entries = check_triggers(&game, encounter);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].source, phenomenon);

    let mut stack_entry = StackEntry::new(phenomenon, alice);
    stack_entry.is_ability = true;
    game.stack.push(stack_entry);
    assert!(
        !check_state_based_actions(&game)
            .contains(&StateBasedAction::PlaneswalkFromPhenomenon(phenomenon))
    );
    game.stack.clear();
    assert!(
        check_state_based_actions(&game)
            .contains(&StateBasedAction::PlaneswalkFromPhenomenon(phenomenon))
    );

    put_on_top(&mut game, alice, next_plane);
    assert!(apply_state_based_actions(&mut game));
    assert_eq!(game.face_up_planar_objects(), &[next_plane]);
}

#[test]
fn u061_controller_rotation_departure_and_communal_ownership_are_preserved() {
    let (mut game, alice, bob) = enabled_game_with_alice_trigger(9, KeywordActionKind::ChaosEnsues);
    let alice_plane = planar_object_named(&game, "Alice Plane 9");
    let bob_plane = planar_object_named(&game, "Bob Plane 9");
    put_on_top(&mut game, alice, alice_plane);
    put_on_top(&mut game, bob, bob_plane);
    game.reveal_starting_plane().unwrap();
    game.chaos_ensues(alice, alice_plane).unwrap();
    let mut trigger_queue = TriggerQueue::new();
    put_triggers_on_stack(&mut game, &mut trigger_queue).unwrap();
    assert_eq!(game.stack.len(), 1);
    assert_eq!(game.stack[0].controller, alice);
    assert!(game.leave_game(alice));
    assert_eq!(game.planar_controller(), Some(bob));
    assert_eq!(game.face_up_planar_objects(), &[bob_plane]);
    assert_eq!(game.stack.len(), 1, "a planar-card ability should survive");
    assert_eq!(game.stack[0].controller, bob);

    let mut communal = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let cards = (0..20)
        .map(|index| {
            (
                planar_card(&format!("Communal Plane {index}"), None),
                PlanarCardKind::Plane,
            )
        })
        .collect();
    communal
        .enable_planechase_communal(cards)
        .expect("two-player communal deck needs twenty cards");
    communal.reveal_starting_plane().unwrap();
    assert!(
        communal
            .planechase
            .as_ref()
            .unwrap()
            .card_kinds
            .keys()
            .all(|id| communal.object(*id).unwrap().owner == alice)
    );
    communal.next_turn();
    assert_eq!(communal.planar_controller(), Some(bob));
    assert!(
        communal
            .planechase
            .as_ref()
            .unwrap()
            .card_kinds
            .keys()
            .all(|id| communal.object(*id).unwrap().owner == bob)
    );
    assert!(communal.leave_game(bob));
    assert_eq!(communal.planar_controller(), Some(alice));
    assert_eq!(communal.planechase.as_ref().unwrap().card_kinds.len(), 20);
}
