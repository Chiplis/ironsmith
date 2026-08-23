use ironsmith::{
    CardBuilder, CardDefinition, CardId, CardType, GameProgress, GameResult, GameState, PlayerId,
    TriggerQueue, Zone, advance_priority,
};

fn card(name: &str) -> CardDefinition {
    CardDefinition::new(
        CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Sorcery])
            .build(),
    )
}

#[test]
fn u060_ante_is_owner_only_public_and_preserved_when_its_owner_leaves() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let alice_card = game.create_object_from_definition(&card("Alice Card"), alice, Zone::Library);
    let bob_card = game.create_object_from_definition(&card("Bob Card"), bob, Zone::Library);

    assert!(game.ante_owned_object(alice, bob_card).is_err());
    let ante_id = game
        .ante_owned_object(alice, alice_card)
        .expect("an owner may ante their own card");
    assert_eq!(game.object(ante_id).expect("ante card").zone, Zone::Ante);
    assert!(Zone::Ante.is_public());
    assert!(!Zone::Ante.is_ordered());

    game.mark_player_lost(alice);
    let retained = game.object(ante_id).expect("CR 800.4n retains ante cards");
    assert_eq!(retained.owner, alice);
    assert_eq!(retained.zone, Zone::Ante);
}

#[test]
fn u060_terminal_winner_receives_every_ante_card_exactly_once() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let alice_card = game.create_object_from_definition(&card("Alice Stake"), alice, Zone::Library);
    let bob_card = game.create_object_from_definition(&card("Bob Stake"), bob, Zone::Library);
    let alice_ante = game.ante_owned_object(alice, alice_card).unwrap();
    let bob_ante = game.ante_owned_object(bob, bob_card).unwrap();

    game.mark_player_lost(alice);
    let mut triggers = TriggerQueue::new();
    let progress =
        advance_priority(&mut game, &mut triggers).expect("priority reaches game result");
    assert!(matches!(progress, GameProgress::GameOver(GameResult::Winner(id)) if id == bob));
    assert_eq!(game.object(alice_ante).expect("alice stake").owner, bob);
    assert_eq!(game.object(bob_ante).expect("bob stake").owner, bob);
    assert_eq!(
        game.finalize_ante_ownership(bob),
        0,
        "transfer is idempotent"
    );
}

#[test]
fn u060_random_ante_selects_exactly_one_card_and_decrements_the_library() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    game.set_random_seed(4072);
    for index in 0..5 {
        game.create_object_from_definition(
            &card(&format!("Candidate {index}")),
            alice,
            Zone::Library,
        );
    }

    let selected = game
        .ante_random_library_card(alice)
        .expect("nonempty library can supply ante");
    assert_eq!(game.ante, vec![selected]);
    assert_eq!(game.player(alice).unwrap().library.len(), 4);
    assert_eq!(game.object(selected).unwrap().owner, alice);
}
