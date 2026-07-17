use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::events::{KeywordActionEvent, KeywordActionKind};
use ironsmith::triggers::Trigger;
use ironsmith::{
    Ability, ArchenemyVariant, CardBuilder, CardDefinition, CardId, CardType, CastingMethod,
    Effect, GameState, LegalAction, PlayerFilter, PlayerId, StateBasedAction, Supertype, TotalCost,
    TriggerQueue, TurnAction, TurnRunner, TurnRunnerState, Zone, apply_state_based_actions,
    check_state_based_actions, check_triggers,
};

fn scheme(name: &str, ongoing: bool) -> CardDefinition {
    let mut builder = CardBuilder::new(CardId::new(), name).card_types(vec![CardType::Scheme]);
    if ongoing {
        builder = builder.supertypes(vec![Supertype::Ongoing]);
    }
    CardDefinition::new(builder.build())
}

fn deck(prefix: &str, size: usize) -> Vec<CardDefinition> {
    (0..size)
        .map(|index| scheme(&format!("{prefix} Scheme {index}"), false))
        .collect()
}

fn object_named(game: &GameState, name: &str) -> ironsmith::ObjectId {
    game.archenemy
        .as_ref()
        .expect("Archenemy enabled")
        .scheme_decks
        .values()
        .flatten()
        .chain(game.face_up_schemes())
        .copied()
        .find(|object| game.object(*object).is_some_and(|card| card.name == name))
        .unwrap_or_else(|| panic!("missing scheme {name}"))
}

fn put_on_top(game: &mut GameState, player: PlayerId, object: ironsmith::ObjectId) {
    let deck = game
        .archenemy
        .as_mut()
        .unwrap()
        .scheme_decks
        .get_mut(&player)
        .unwrap();
    let index = deck
        .iter()
        .position(|candidate| *candidate == object)
        .unwrap();
    deck.remove(index);
    deck.push(object);
}

#[test]
fn u063_ongoing_scheme_type_line_motion_trigger_and_abandon_compile() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Test Ongoing Scheme")
        .supertypes(vec![Supertype::Ongoing])
        .card_types(vec![CardType::Scheme])
        .parse_text(
            "When you set this scheme in motion, you gain 1 life.\nAt the beginning of your upkeep, abandon this scheme.",
        )
        .expect("canonical scheme trigger and abandon action should compile");
    assert_eq!(definition.card.card_types, vec![CardType::Scheme]);
    assert_eq!(definition.card.supertypes, vec![Supertype::Ongoing]);
    assert_eq!(definition.abilities.len(), 2);

    let real_scheme = CardDefinitionBuilder::new(CardId::new(), "Look Skyward and Despair")
        .card_types(vec![CardType::Scheme])
        .parse_text(
            "When you set this scheme in motion, create a 5/5 red Dragon creature token with flying.",
        )
        .expect("a real printed Scheme oracle line should compile through generic mechanics");
    assert_eq!(real_scheme.card.card_types, vec![CardType::Scheme]);
    assert_eq!(real_scheme.abilities.len(), 1);
}

#[test]
fn u063_decks_validate_transactionally_and_variants_set_starting_state() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut invalid_game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    assert!(
        invalid_game
            .enable_archenemy(ArchenemyVariant::Default, vec![(alice, deck("Short", 19))])
            .unwrap_err()
            .contains("at least 20")
    );
    assert!(invalid_game.archenemy.is_none());
    assert!(invalid_game.command_zone.is_empty());

    let mut duplicate = deck("Duplicate", 20);
    duplicate[0] = scheme("Repeated", false);
    duplicate[1] = scheme("Repeated", false);
    duplicate[2] = scheme("Repeated", false);
    assert!(
        invalid_game
            .enable_archenemy(ArchenemyVariant::Default, vec![(alice, duplicate)])
            .unwrap_err()
            .contains("no more than 2")
    );
    assert!(invalid_game.command_zone.is_empty());

    let mut default_game = GameState::new(vec!["Alice".into(), "Bob".into()], 99);
    default_game
        .enable_archenemy(ArchenemyVariant::Default, vec![(bob, deck("Bob", 20))])
        .unwrap();
    assert_eq!(default_game.player(alice).unwrap().life, 20);
    assert_eq!(default_game.player(bob).unwrap().life, 40);
    assert_eq!(default_game.turn.active_player, bob);
    assert_eq!(default_game.turn_store.turn_order[0], bob);

    let mut rumble = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let original_first = rumble.turn.active_player;
    rumble
        .enable_archenemy(
            ArchenemyVariant::SupervillainRumble,
            vec![(alice, deck("Alice", 20)), (bob, deck("Bob", 20))],
        )
        .unwrap();
    assert!(rumble.is_archenemy(alice) && rumble.is_archenemy(bob));
    assert_eq!(rumble.player(alice).unwrap().life, 40);
    assert_eq!(rumble.player(bob).unwrap().life, 40);
    assert_eq!(rumble.turn.active_player, original_first);

    let mut commander = GameState::new(vec!["Alice".into(), "Bob".into()], 40);
    commander
        .enable_archenemy(
            ArchenemyVariant::Commander,
            vec![(alice, deck("Commander", 10))],
        )
        .unwrap();
    assert_eq!(commander.player(alice).unwrap().life, 60);
    assert_eq!(commander.turn.active_player, alice);
}

#[test]
fn u063_motion_enables_only_face_up_abilities_and_emits_typed_source_event() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut cards = deck("Alice", 20);
    cards[19] = scheme("Triggered Scheme", false);
    cards[19].abilities.extend([
        Ability::triggered(
            Trigger::keyword_action_from_source(
                KeywordActionKind::SetSchemeInMotion,
                PlayerFilter::You,
            ),
            vec![Effect::gain_life(1)],
        ),
        Ability::activated(TotalCost::free(), vec![Effect::gain_life(1)]),
    ]);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.enable_archenemy(ArchenemyVariant::Default, vec![(alice, cards)])
        .unwrap();
    let source = object_named(&game, "Triggered Scheme");
    put_on_top(&mut game, alice, source);
    assert!(
        game.object(source)
            .unwrap()
            .abilities
            .iter()
            .all(|ability| ability.functional_zones.is_empty())
    );

    assert_eq!(game.set_scheme_in_motion(alice).unwrap(), source);
    assert_eq!(game.object(source).unwrap().zone, Zone::Command);
    assert_eq!(game.controller_of_id(source), Some(alice));
    assert!(
        game.object(source)
            .unwrap()
            .abilities
            .iter()
            .all(|ability| ability.functional_zones == [Zone::Command])
    );
    assert!(
        ironsmith::decision::compute_legal_actions(&game, alice)
            .iter()
            .any(|action| matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source))
    );
    assert!(!ironsmith::decision::can_cast_spell(
        &game,
        alice,
        game.object(source).unwrap(),
        &CastingMethod::Normal,
    ));
    assert_eq!(
        game.move_object_by_effect(source, Zone::Graveyard),
        Some(source)
    );
    game.set_current_controller(source, bob);
    assert_eq!(game.controller_of_id(source), Some(alice));

    let events = game.take_pending_trigger_events();
    let event = events
        .iter()
        .find(|event| {
            event.downcast::<KeywordActionEvent>().is_some_and(|event| {
                event.action == KeywordActionKind::SetSchemeInMotion && event.source == source
            })
        })
        .expect("typed set-in-motion event");
    let entries = check_triggers(&game, event);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].source, source);
}

#[test]
fn u063_sba_waits_for_every_scheme_trigger_and_abandon_recycles_ongoing_scheme() {
    let alice = PlayerId::from_index(0);
    let mut cards = deck("Alice", 20);
    cards[18] = scheme("Ordinary", false);
    cards[19] = scheme("Ongoing", true);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.enable_archenemy(ArchenemyVariant::Default, vec![(alice, cards)])
        .unwrap();
    let ordinary = object_named(&game, "Ordinary");
    let ongoing = object_named(&game, "Ongoing");
    put_on_top(&mut game, alice, ordinary);
    put_on_top(&mut game, alice, ongoing);
    assert_eq!(game.set_scheme_in_motion(alice).unwrap(), ongoing);
    assert_eq!(game.set_scheme_in_motion(alice).unwrap(), ordinary);

    assert!(
        !check_state_based_actions(&game).contains(&StateBasedAction::RecycleScheme(ordinary)),
        "the pending set-in-motion event from either scheme protects all face-up schemes"
    );
    game.take_pending_trigger_events();
    assert!(check_state_based_actions(&game).contains(&StateBasedAction::RecycleScheme(ordinary)));
    assert!(!check_state_based_actions(&game).contains(&StateBasedAction::RecycleScheme(ongoing)));
    let ordinary_stable = game.object(ordinary).unwrap().stable_id;
    assert!(apply_state_based_actions(&mut game));
    let recycled_ordinary = game.scheme_deck(alice).unwrap()[0];
    assert_ne!(recycled_ordinary, ordinary);
    assert_eq!(
        game.object(recycled_ordinary).unwrap().stable_id,
        ordinary_stable
    );
    assert_eq!(game.face_up_schemes(), &[ongoing]);

    let ongoing_stable = game.object(ongoing).unwrap().stable_id;
    let recycled_ongoing = game.abandon_scheme(ongoing).unwrap();
    assert_ne!(recycled_ongoing, ongoing);
    assert_eq!(
        game.object(recycled_ongoing).unwrap().stable_id,
        ongoing_stable
    );
    assert_eq!(game.scheme_deck(alice).unwrap()[0], recycled_ongoing);
    assert!(game.face_up_schemes().is_empty());
}

#[test]
fn u063_precombat_turn_action_sets_scheme_in_motion_before_priority() {
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.enable_archenemy(
        ArchenemyVariant::Default,
        vec![(alice, deck("Turn Action", 20))],
    )
    .unwrap();
    let top = *game.scheme_deck(alice).unwrap().last().unwrap();
    let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::FirstMain);
    let mut queue = TriggerQueue::new();
    assert!(matches!(
        runner.advance(&mut game, &mut queue).unwrap(),
        TurnAction::RunPriority
    ));
    assert_eq!(game.face_up_schemes(), &[top]);
    assert!(
        game.effect_store
            .pending_trigger_events
            .iter()
            .any(|event| {
                event.downcast::<KeywordActionEvent>().is_some_and(|event| {
                    event.action == KeywordActionKind::SetSchemeInMotion && event.source == top
                })
            })
    );
}

#[test]
fn u063_restart_rebuilds_face_down_deck_and_departure_removes_owned_schemes() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut cards = deck("Restart", 20);
    cards[19] = scheme("Restart Ongoing", true);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.enable_archenemy(ArchenemyVariant::Default, vec![(alice, cards)])
        .unwrap();
    let ongoing = object_named(&game, "Restart Ongoing");
    put_on_top(&mut game, alice, ongoing);
    game.set_scheme_in_motion(alice).unwrap();
    game.restart_game(bob, &[]);

    assert_eq!(
        game.turn.active_player, bob,
        "the restart effect chooses the starter"
    );
    assert_eq!(game.player(alice).unwrap().life, 40);
    assert_eq!(game.player(bob).unwrap().life, 20);
    assert!(game.face_up_schemes().is_empty());
    assert_eq!(game.scheme_deck(alice).unwrap().len(), 20);
    assert!(game.scheme_deck(alice).unwrap().iter().all(|object| {
        game.object(*object).is_some_and(|card| {
            card.zone == Zone::Command
                && card
                    .abilities
                    .iter()
                    .all(|ability| ability.functional_zones.is_empty())
        })
    }));

    let departing_top = *game.scheme_deck(alice).unwrap().last().unwrap();
    game.set_scheme_in_motion(alice).unwrap();
    assert!(game.leave_game(alice));
    assert!(game.archenemy.as_ref().unwrap().archenemies.is_empty());
    assert!(game.scheme_deck(alice).is_none());
    assert!(game.face_up_schemes().is_empty());
    assert!(game.object(departing_top).is_none());
}
