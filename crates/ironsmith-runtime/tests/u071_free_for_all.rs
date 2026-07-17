use std::collections::HashSet;

use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::{
    AttackDirection, AttackTarget, CardId, CardType, CombatState, FreeForAllAttackOption,
    GameState, PlayerId, PowerToughness, Zone, compute_legal_attackers, declare_attackers,
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
    let definition = CardDefinitionBuilder::new(CardId::new(), "Free-for-All Attacker")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let attacker = game.create_object_from_definition(&definition, controller, Zone::Battlefield);
    game.remove_summoning_sickness(attacker);
    attacker
}

#[test]
fn u071_default_profile_randomizes_and_records_individual_unrestricted_seating() {
    let (mut game, players) = four_player_game();
    game.set_random_seed(0x806);
    game.enable_free_for_all(FreeForAllAttackOption::MultiplePlayers, None)
        .expect("valid Free-for-All profile");

    let state = game.free_for_all().expect("profile recorded");
    assert_eq!(
        state.seats().iter().copied().collect::<HashSet<_>>(),
        players.into_iter().collect()
    );
    assert_eq!(state.seats(), game.turn_store.turn_order);
    assert_eq!(game.turn.active_player, state.seats()[0]);
    assert_eq!(
        state.attack_option(),
        FreeForAllAttackOption::MultiplePlayers
    );
    assert_eq!(state.range_of_influence(), None);
    assert_eq!(game.attack_direction(), None);
    assert!(game.limited_range_of_influence().is_none());
    assert!(game.team_state().is_none());
    assert!(!game.shared_team_turns_enabled());
    assert!(!game.deploy_creatures_enabled());
    for first in players {
        for second in players {
            assert_eq!(game.are_opponents(first, second), first != second);
        }
    }
    assert!(
        game.set_teams(vec![vec![players[0]], players[1..].to_vec()])
            .is_err()
    );
    game.set_deploy_creatures(true);
    assert!(!game.deploy_creatures_enabled());

    let active_player = game.turn.active_player;
    let creature = attacker(&mut game, active_player);
    let legal = compute_legal_attackers(&game, &CombatState::default())
        .into_iter()
        .find(|option| option.creature == creature)
        .expect("the active player may attack");
    assert_eq!(legal.valid_targets.len(), 3);
}

#[test]
fn u071_direction_and_uniform_range_are_one_atomic_profile() {
    let (mut game, [alice, bob, charlie, diana]) = four_player_game();
    let seats = vec![alice, charlie, bob, diana];
    game.restore_free_for_all(seats.clone(), FreeForAllAttackOption::Left, Some(1))
        .expect("explicit synchronized profile");
    let creature = attacker(&mut game, alice);

    let state = game.free_for_all().expect("profile recorded");
    assert_eq!(state.seats(), seats);
    assert_eq!(state.attack_option(), FreeForAllAttackOption::Left);
    assert_eq!(state.range_of_influence(), Some(1));
    assert_eq!(game.attack_direction(), Some(AttackDirection::Left));
    let range = game
        .limited_range_of_influence()
        .expect("uniform range enabled");
    for player in [alice, bob, charlie, diana] {
        assert_eq!(range.configured_range(player), Some(1));
    }

    let option = compute_legal_attackers(&game, &CombatState::default())
        .into_iter()
        .find(|option| option.creature == creature)
        .expect("left-adjacent player is in range");
    assert_eq!(option.valid_targets, vec![AttackTarget::Player(charlie)]);
    assert!(
        declare_attackers(
            &mut game,
            &mut CombatState::default(),
            vec![(creature, AttackTarget::Player(bob))],
        )
        .is_err()
    );

    let before = game.free_for_all().cloned();
    let before_range = game.limited_range_of_influence().cloned();
    assert!(
        game.restore_free_for_all(
            vec![alice, alice, bob, diana],
            FreeForAllAttackOption::Right,
            None,
        )
        .is_err()
    );
    assert_eq!(game.free_for_all(), before.as_ref());
    assert_eq!(game.limited_range_of_influence(), before_range.as_ref());
    assert!(
        game.enable_limited_range_of_influence(seats, vec![1, 2, 1, 1])
            .is_err(),
        "Free-for-All rejects heterogeneous ranges"
    );
    game.set_attack_direction(Some(AttackDirection::Right));
    assert_eq!(game.attack_direction(), Some(AttackDirection::Left));
    assert!(
        game.enable_limited_range_of_influence(vec![alice, charlie, bob, diana], vec![2, 2, 2, 2],)
            .is_err(),
        "Free-for-All range is fixed before play begins"
    );
    game.disable_limited_range_of_influence();
    assert!(game.limited_range_of_influence().is_some());
    assert_eq!(game.free_for_all().unwrap().range_of_influence(), Some(1));
}

#[test]
fn u071_restart_and_subgame_preserve_physical_seats_without_rerandomizing() {
    let (mut game, [alice, bob, charlie, diana]) = four_player_game();
    let seats = vec![diana, bob, alice, charlie];
    game.restore_free_for_all(seats.clone(), FreeForAllAttackOption::Right, None)
        .expect("explicit synchronized profile");

    game.restart_game(charlie, &[]);
    assert_eq!(game.free_for_all().unwrap().seats(), seats);
    assert_eq!(game.physical_seats(), seats);
    assert_eq!(game.turn.active_player, charlie);
    assert_eq!(game.attack_direction(), Some(AttackDirection::Right));

    game.begin_subgame(None, charlie, Vec::new())
        .expect("Free-for-All subgame");
    assert_eq!(game.free_for_all().unwrap().seats(), seats);
    assert_eq!(game.physical_seats(), seats);
    assert_eq!(game.attack_direction(), Some(AttackDirection::Right));
}
