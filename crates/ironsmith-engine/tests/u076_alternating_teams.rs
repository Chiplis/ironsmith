use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{GameProgress, GameResult};
use ironsmith::game_loop::advance_priority;
use ironsmith::{
    AttackDirection, AttackTarget, CardId, CardType, CombatState, FreeForAllAttackOption,
    GameState, ManaSymbol, PlayerId, PowerToughness, Subtype, TriggerQueue, Zone,
    compute_legal_attackers,
};

fn players(count: usize) -> (GameState, Vec<PlayerId>) {
    (
        GameState::new(
            (0..count).map(|index| format!("Player {index}")).collect(),
            20,
        ),
        (0..count)
            .map(|index| PlayerId::from_index(index as u8))
            .collect(),
    )
}

fn teams(seats: &[PlayerId]) -> Vec<Vec<PlayerId>> {
    vec![
        vec![seats[0], seats[1]],
        vec![seats[2], seats[3]],
        vec![seats[4], seats[5]],
    ]
}

fn round_robin_seats(seats: &[PlayerId]) -> Vec<PlayerId> {
    vec![seats[0], seats[2], seats[4], seats[1], seats[3], seats[5]]
}

fn creature(game: &mut GameState, controller: PlayerId) -> ironsmith::ObjectId {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Alternating Attacker")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let object = game.create_object_from_definition(&definition, controller, Zone::Battlefield);
    game.remove_summoning_sickness(object);
    object
}

fn planeswalker(game: &mut GameState, controller: PlayerId) -> ironsmith::ObjectId {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Adjacent Walker")
        .card_types(vec![CardType::Planeswalker])
        .loyalty(4)
        .build();
    game.create_object_from_definition(&definition, controller, Zone::Battlefield)
}

fn siege(game: &mut GameState, controller: PlayerId, protector: PlayerId) -> ironsmith::ObjectId {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Adjacent Siege")
        .card_types(vec![CardType::Battle])
        .subtypes(vec![Subtype::Siege])
        .defense(4)
        .build();
    let battle = game.create_object_from_definition(&definition, controller, Zone::Battlefield);
    assert!(game.set_battle_protector(battle, protector));
    battle
}

#[test]
fn u076_profile_uses_round_robin_seats_seeded_start_and_fixed_options() {
    let (mut game, seats) = players(6);
    let configured_teams = teams(&seats);
    let physical_seats = round_robin_seats(&seats);
    game.set_random_seed(811);
    game.enable_alternating_teams(
        configured_teams.clone(),
        FreeForAllAttackOption::MultiplePlayers,
        Some(2),
        false,
    )
    .unwrap();

    let profile = game.alternating_teams().expect("Alternating Teams profile");
    assert_eq!(profile.teams(), configured_teams);
    assert_eq!(profile.seats(), physical_seats);
    assert_eq!(
        profile.attack_option(),
        FreeForAllAttackOption::MultiplePlayers
    );
    assert_eq!(profile.range_of_influence(), Some(2));
    assert!(!profile.deploy_creatures());
    assert!(physical_seats.contains(&profile.starting_player()));
    assert_eq!(game.turn.active_player, profile.starting_player());
    assert_eq!(game.turn_store.turn_order[0], profile.starting_player());
    let starting_player = profile.starting_player();
    assert!(!game.shared_team_turns_enabled());
    assert!(!game.deploy_creatures_enabled());
    assert_eq!(game.attack_direction(), None);
    assert!(seats.iter().all(|player| {
        game.limited_range_of_influence()
            .unwrap()
            .configured_range(*player)
            == Some(2)
    }));

    game.set_deploy_creatures(true);
    game.set_attack_direction(Some(AttackDirection::Left));
    game.disable_limited_range_of_influence();
    assert!(game.enable_shared_team_turns().is_err());
    assert!(
        game.set_teams(configured_teams.into_iter().rev().collect())
            .is_err()
    );
    assert!(!game.deploy_creatures_enabled());
    assert_eq!(game.attack_direction(), None);
    assert!(game.limited_range_of_influence().is_some());

    let (mut replay, replay_seats) = players(6);
    replay.set_random_seed(811);
    replay
        .enable_alternating_teams(
            teams(&replay_seats),
            FreeForAllAttackOption::MultiplePlayers,
            Some(2),
            false,
        )
        .unwrap();
    assert_eq!(
        replay.alternating_teams().unwrap().starting_player(),
        starting_player,
        "the starting-player choice comes from the deterministic match RNG",
    );

    let (mut invalid, invalid_seats) = players(5);
    assert!(
        invalid
            .enable_alternating_teams(
                vec![invalid_seats[0..2].to_vec(), invalid_seats[2..5].to_vec()],
                FreeForAllAttackOption::MultiplePlayers,
                Some(2),
                false,
            )
            .is_err()
    );
    assert!(invalid.alternating_teams().is_none());
}

#[test]
fn u076_multiple_players_attacks_only_both_adjacent_opponents_and_their_objects() {
    let (mut game, seats) = players(6);
    let physical_seats = round_robin_seats(&seats);
    game.restore_alternating_teams(
        teams(&seats),
        physical_seats,
        seats[0],
        FreeForAllAttackOption::MultiplePlayers,
        Some(2),
        false,
    )
    .unwrap();
    let attacker = creature(&mut game, seats[0]);
    let left_walker = planeswalker(&mut game, seats[2]);
    let right_battle = siege(&mut game, seats[4], seats[5]);
    let distant_walker = planeswalker(&mut game, seats[3]);
    game.turn.active_player = seats[0];
    game.turn.priority_player = Some(seats[0]);

    let options = compute_legal_attackers(&game, &CombatState::default());
    let option = options
        .iter()
        .find(|option| option.creature == attacker)
        .expect("the creature has adjacent opposing defenders");
    assert!(
        option
            .valid_targets
            .contains(&AttackTarget::Player(seats[2]))
    );
    assert!(
        option
            .valid_targets
            .contains(&AttackTarget::Player(seats[5]))
    );
    assert!(
        option
            .valid_targets
            .contains(&AttackTarget::Planeswalker(left_walker))
    );
    assert!(
        option
            .valid_targets
            .contains(&AttackTarget::Battle(right_battle))
    );
    assert!(
        !option
            .valid_targets
            .contains(&AttackTarget::Player(seats[3]))
    );
    assert!(
        !option
            .valid_targets
            .contains(&AttackTarget::Planeswalker(distant_walker))
    );
}

#[test]
fn u076_left_and_right_options_do_not_skip_an_empty_physical_seat() {
    for (attack, expected, other) in [
        (FreeForAllAttackOption::Left, 2usize, 5usize),
        (FreeForAllAttackOption::Right, 5usize, 2usize),
    ] {
        let (mut game, seats) = players(6);
        game.restore_alternating_teams(
            teams(&seats),
            round_robin_seats(&seats),
            seats[0],
            attack,
            None,
            true,
        )
        .unwrap();
        assert!(game.deploy_creatures_enabled());
        assert!(game.limited_range_of_influence().is_none());
        assert!(game.attack_direction_allows_defender(seats[0], seats[expected]));
        assert!(!game.attack_direction_allows_defender(seats[0], seats[other]));

        assert!(game.mark_player_lost(seats[expected]));
        assert!(
            !game.attack_direction_allows_defender(seats[0], seats[other]),
            "a vacated adjacent seat remains part of the physical seat map",
        );
    }
}

#[test]
fn u076_resources_turns_and_information_remain_individual_but_victory_is_by_team() {
    let (mut game, seats) = players(6);
    game.restore_alternating_teams(
        teams(&seats),
        round_robin_seats(&seats),
        seats[0],
        FreeForAllAttackOption::MultiplePlayers,
        Some(2),
        false,
    )
    .unwrap();

    game.player_mut(seats[0]).unwrap().life = 7;
    game.player_mut(seats[0])
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Colorless, 3);
    assert_eq!(game.player(seats[0]).unwrap().life, 7);
    assert_eq!(game.player(seats[1]).unwrap().life, 20);
    assert_eq!(game.player(seats[0]).unwrap().mana_pool.total(), 3);
    assert_eq!(game.player(seats[1]).unwrap().mana_pool.total(), 0);
    assert!(!game.can_review_teammate_hand(seats[0], seats[1]));
    assert_eq!(game.turn_store.turn_order.len(), 6);

    for loser in [seats[0], seats[2], seats[3], seats[4], seats[5]] {
        assert!(game.mark_player_lost(loser));
    }
    let result = advance_priority(&mut game, &mut TriggerQueue::new()).unwrap();
    let GameProgress::GameOver(GameResult::Remaining(winners)) = result else {
        panic!("the sole surviving team should win together: {result:?}");
    };
    assert_eq!(winners, vec![seats[0], seats[1]]);
}

#[test]
fn u076_restart_and_subgame_preserve_seats_and_options_with_new_starters() {
    let (mut game, seats) = players(6);
    let configured_teams = teams(&seats);
    let physical_seats = round_robin_seats(&seats);
    game.restore_alternating_teams(
        configured_teams.clone(),
        physical_seats.clone(),
        seats[0],
        FreeForAllAttackOption::Right,
        Some(2),
        true,
    )
    .unwrap();

    game.restart_game(seats[4], &[]);
    let restarted = game.alternating_teams().expect("restart profile");
    assert_eq!(restarted.teams(), configured_teams);
    assert_eq!(restarted.seats(), physical_seats);
    assert_eq!(restarted.starting_player(), seats[4]);
    assert_eq!(restarted.attack_option(), FreeForAllAttackOption::Right);
    assert_eq!(restarted.range_of_influence(), Some(2));
    assert!(restarted.deploy_creatures());
    assert_eq!(game.turn.active_player, seats[4]);

    game.begin_subgame(None, seats[4], Vec::new()).unwrap();
    let child = game.alternating_teams().expect("subgame profile");
    assert_eq!(child.teams(), configured_teams);
    assert_eq!(child.seats(), physical_seats);
    assert_eq!(child.starting_player(), game.turn.active_player);
    assert_eq!(game.turn_store.turn_order[0], child.starting_player());
    assert_eq!(child.attack_option(), FreeForAllAttackOption::Right);
    assert_eq!(child.range_of_influence(), Some(2));
    assert!(child.deploy_creatures());
    assert!(child.seats().contains(&child.starting_player()));
}
