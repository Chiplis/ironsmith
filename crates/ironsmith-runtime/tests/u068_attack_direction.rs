use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::{
    AttackDirection, AttackTarget, CardId, CardType, CombatState, GameState, PlayerId,
    PowerToughness, Subtype, Zone, compute_legal_attackers, declare_attackers,
};

fn four_player_game() -> (GameState, [PlayerId; 4]) {
    let game = GameState::new(
        vec![
            "Alice".into(),
            "Bob".into(),
            "Charlie".into(),
            "Diana".into(),
        ],
        20,
    );
    (
        game,
        [
            PlayerId::from_index(0),
            PlayerId::from_index(1),
            PlayerId::from_index(2),
            PlayerId::from_index(3),
        ],
    )
}

fn attacker(game: &mut GameState, controller: PlayerId) -> ironsmith::ObjectId {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Directional Attacker")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let attacker = game.create_object_from_definition(&definition, controller, Zone::Battlefield);
    game.remove_summoning_sickness(attacker);
    attacker
}

fn planeswalker(game: &mut GameState, controller: PlayerId, name: &str) -> ironsmith::ObjectId {
    let definition = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Planeswalker])
        .loyalty(4)
        .build();
    game.create_object_from_definition(&definition, controller, Zone::Battlefield)
}

fn siege(
    game: &mut GameState,
    controller: PlayerId,
    protector: PlayerId,
    name: &str,
) -> ironsmith::ObjectId {
    let definition = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Battle])
        .subtypes(vec![Subtype::Siege])
        .defense(4)
        .build();
    let battle = game.create_object_from_definition(&definition, controller, Zone::Battlefield);
    assert!(game.set_battle_protector(battle, protector));
    battle
}

#[test]
fn u068_left_and_right_limit_every_attack_target_to_the_adjacent_defender() {
    let (mut game, [alice, bob, charlie, diana]) = four_player_game();
    let creature = attacker(&mut game, alice);
    let bob_walker = planeswalker(&mut game, bob, "Bob Walker");
    let charlie_walker = planeswalker(&mut game, charlie, "Charlie Walker");
    let bob_battle = siege(&mut game, alice, bob, "Bob-protected Siege");
    let charlie_battle = siege(&mut game, alice, charlie, "Charlie-protected Siege");

    let unrestricted = compute_legal_attackers(&game, &CombatState::default())
        .into_iter()
        .find(|option| option.creature == creature)
        .expect("ordinary multiplayer attacks are unrestricted by direction");
    assert!(
        unrestricted
            .valid_targets
            .contains(&AttackTarget::Player(bob))
    );
    assert!(
        unrestricted
            .valid_targets
            .contains(&AttackTarget::Player(charlie))
    );
    assert!(
        unrestricted
            .valid_targets
            .contains(&AttackTarget::Player(diana))
    );

    game.set_attack_direction(Some(AttackDirection::Left));
    let left = compute_legal_attackers(&game, &CombatState::default())
        .into_iter()
        .find(|option| option.creature == creature)
        .expect("the left-adjacent opponent is attackable");
    assert!(left.valid_targets.contains(&AttackTarget::Player(bob)));
    assert!(
        left.valid_targets
            .contains(&AttackTarget::Planeswalker(bob_walker))
    );
    assert!(
        left.valid_targets
            .contains(&AttackTarget::Battle(bob_battle))
    );
    assert!(!left.valid_targets.contains(&AttackTarget::Player(charlie)));
    assert!(
        !left
            .valid_targets
            .contains(&AttackTarget::Planeswalker(charlie_walker))
    );
    assert!(
        !left
            .valid_targets
            .contains(&AttackTarget::Battle(charlie_battle))
    );
    assert!(!left.valid_targets.contains(&AttackTarget::Player(diana)));

    game.set_attack_direction(Some(AttackDirection::Right));
    let right = compute_legal_attackers(&game, &CombatState::default())
        .into_iter()
        .find(|option| option.creature == creature)
        .expect("the right-adjacent opponent is attackable");
    assert_eq!(right.valid_targets, vec![AttackTarget::Player(diana)]);

    game.set_attack_direction(Some(AttackDirection::Left));
    assert!(
        declare_attackers(
            &mut game,
            &mut CombatState::default(),
            vec![(creature, AttackTarget::Player(charlie))],
        )
        .is_err(),
        "direct declarations cannot bypass the directional option"
    );
    declare_attackers(
        &mut game,
        &mut CombatState::default(),
        vec![(creature, AttackTarget::Player(bob))],
    )
    .expect("the immediately-left opponent is a legal declaration");
}

#[test]
fn u068_an_empty_adjacent_seat_does_not_make_the_next_opponent_attackable() {
    let (mut game, [alice, bob, charlie, _diana]) = four_player_game();
    let creature = attacker(&mut game, alice);
    game.set_attack_direction(Some(AttackDirection::Left));
    assert_eq!(game.adjacent_player_in_attack_direction(alice), Some(bob));

    assert!(game.leave_game(bob));
    assert_eq!(game.adjacent_player_in_attack_direction(alice), None);
    assert!(
        compute_legal_attackers(&game, &CombatState::default())
            .into_iter()
            .all(|option| option.creature != creature),
        "Charlie remains two physical seats away"
    );
    assert!(
        declare_attackers(
            &mut game,
            &mut CombatState::default(),
            vec![(creature, AttackTarget::Player(charlie))],
        )
        .is_err()
    );
}

#[test]
fn u068_direction_and_limited_range_are_both_required_when_combined() {
    let (mut game, [alice, bob, _charlie, _diana]) = four_player_game();
    let creature = attacker(&mut game, alice);
    game.set_attack_direction(Some(AttackDirection::Left));
    game.enable_limited_range_of_influence(
        vec![
            PlayerId::from_index(0),
            PlayerId::from_index(1),
            PlayerId::from_index(2),
            PlayerId::from_index(3),
        ],
        vec![0, 1, 1, 1],
    )
    .expect("valid ranges");
    assert_eq!(game.adjacent_player_in_attack_direction(alice), Some(bob));
    assert!(
        compute_legal_attackers(&game, &CombatState::default())
            .into_iter()
            .all(|option| option.creature != creature),
        "the adjacent opponent is still outside Alice's range"
    );
}
