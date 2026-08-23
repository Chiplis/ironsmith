use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{GameProgress, GameResult};
use ironsmith::game_loop::advance_priority;
use ironsmith::{
    AttackDirection, AttackTarget, CardId, CardType, CombatState, GameState, ManaSymbol, PlayerId,
    PowerToughness, Subtype, TriggerQueue, Zone, compute_legal_attackers,
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

fn creature(game: &mut GameState, controller: PlayerId, name: &str) -> ironsmith::ObjectId {
    let definition = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let object = game.create_object_from_definition(&definition, controller, Zone::Battlefield);
    game.remove_summoning_sickness(object);
    object
}

fn planeswalker(game: &mut GameState, controller: PlayerId, name: &str) -> ironsmith::ObjectId {
    let definition = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Planeswalker])
        .loyalty(4)
        .build();
    game.create_object_from_definition(&definition, controller, Zone::Battlefield)
}

fn siege(game: &mut GameState, controller: PlayerId, protector: PlayerId) -> ironsmith::ObjectId {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Emperor Siege")
        .card_types(vec![CardType::Battle])
        .subtypes(vec![Subtype::Siege])
        .defense(4)
        .build();
    let battle = game.create_object_from_definition(&definition, controller, Zone::Battlefield);
    assert!(game.set_battle_protector(battle, protector));
    battle
}

#[test]
fn u074_default_profile_derives_roles_ranges_deploy_and_starting_emperor() {
    let (mut game, seats) = players(6);
    let teams = vec![seats[0..3].to_vec(), seats[3..6].to_vec()];
    game.set_random_seed(809);
    game.enable_emperor(teams.clone()).unwrap();

    let profile = game.emperor().expect("Emperor profile");
    assert_eq!(profile.teams(), teams);
    assert_eq!(profile.seats(), seats);
    assert_eq!(profile.emperors(), &[seats[1], seats[4]]);
    assert_eq!(profile.ranges(), &[1, 2, 1, 1, 2, 1]);
    assert_eq!(
        profile.starting_emperor(),
        profile.emperors()[profile.starting_team()]
    );
    assert_eq!(game.turn.active_player, profile.starting_emperor());
    assert_eq!(game.turn_store.turn_order[0], profile.starting_emperor());
    assert!(game.deploy_creatures_enabled());
    assert!(!game.shared_team_turns_enabled());
    assert!(game.can_review_teammate_hand(seats[0], seats[1]));
    let deployed = creature(&mut game, seats[0], "Deployed General");
    assert_eq!(game.current_abilities(deployed).unwrap().len(), 1);

    game.set_deploy_creatures(false);
    game.disable_limited_range_of_influence();
    game.set_attack_direction(Some(AttackDirection::Left));
    assert!(game.set_teams(teams.into_iter().rev().collect()).is_err());
    assert!(game.deploy_creatures_enabled());
    assert!(game.limited_range_of_influence().is_some());
    assert_eq!(game.attack_direction(), None);

    game.player_mut(seats[0])
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Colorless, 2);
    assert_eq!(game.player(seats[0]).unwrap().mana_pool.total(), 2);
    assert_eq!(game.player(seats[1]).unwrap().mana_pool.total(), 0);

    let (mut invalid, invalid_seats) = players(7);
    assert!(
        invalid
            .enable_emperor(vec![
                invalid_seats[0..3].to_vec(),
                invalid_seats[3..7].to_vec(),
            ])
            .is_err()
    );
    assert!(invalid.emperor().is_none());
}

#[test]
fn u074_larger_teams_use_minimum_role_ranges_from_their_seats() {
    let (mut game, seats) = players(8);
    game.restore_emperor(
        vec![seats[0..4].to_vec(), seats[4..8].to_vec()],
        seats.clone(),
        0,
        seats[1],
        vec![1, 3, 2, 1, 1, 3, 2, 1],
    )
    .unwrap();

    let profile = game.emperor().unwrap();
    assert_eq!(profile.emperors(), &[seats[1], seats[5]]);
    for (player, expected) in seats.iter().zip([1, 3, 2, 1, 1, 3, 2, 1]) {
        assert_eq!(
            game.limited_range_of_influence()
                .unwrap()
                .configured_range(*player),
            Some(expected)
        );
    }
}

#[test]
fn u074_attacks_require_an_immediately_adjacent_opponent_for_every_target_kind() {
    let (mut game, seats) = players(6);
    game.restore_emperor(
        vec![seats[0..3].to_vec(), seats[3..6].to_vec()],
        seats.clone(),
        0,
        seats[1],
        vec![1, 2, 1, 1, 2, 1],
    )
    .unwrap();
    let general_attacker = creature(&mut game, seats[2], "General Attacker");
    let emperor_attacker = creature(&mut game, seats[1], "Emperor Attacker");
    let adjacent_walker = planeswalker(&mut game, seats[3], "Adjacent Walker");
    let distant_walker = planeswalker(&mut game, seats[4], "Distant Walker");
    let adjacent_battle = siege(&mut game, seats[2], seats[3]);
    game.turn.active_player = seats[2];
    game.turn.priority_player = Some(seats[2]);

    let options = compute_legal_attackers(&game, &CombatState::default());
    let general = options
        .iter()
        .find(|option| option.creature == general_attacker)
        .expect("outer general can attack the adjacent opposing general");
    assert!(
        general
            .valid_targets
            .contains(&AttackTarget::Player(seats[3]))
    );
    assert!(
        general
            .valid_targets
            .contains(&AttackTarget::Planeswalker(adjacent_walker))
    );
    assert!(
        general
            .valid_targets
            .contains(&AttackTarget::Battle(adjacent_battle))
    );
    assert!(
        !general
            .valid_targets
            .contains(&AttackTarget::Player(seats[4]))
    );
    assert!(
        !general
            .valid_targets
            .contains(&AttackTarget::Planeswalker(distant_walker))
    );
    assert!(
        options
            .iter()
            .all(|option| option.creature != emperor_attacker),
        "the emperor's adjacent seats are teammates"
    );
}

#[test]
fn u074_emperor_loss_or_draw_propagates_to_the_team_and_the_other_team_wins() {
    let (mut loss_game, seats) = players(6);
    loss_game
        .restore_emperor(
            vec![seats[0..3].to_vec(), seats[3..6].to_vec()],
            seats.clone(),
            0,
            seats[1],
            vec![1, 2, 1, 1, 2, 1],
        )
        .unwrap();
    assert!(loss_game.mark_player_lost(seats[0]));
    assert!(loss_game.player(seats[1]).unwrap().is_in_game());
    assert!(loss_game.mark_player_lost(seats[1]));
    assert!(
        seats[0..3]
            .iter()
            .all(|player| loss_game.player(*player).unwrap().has_left_game)
    );
    let progress = advance_priority(&mut loss_game, &mut TriggerQueue::new()).unwrap();
    let GameProgress::GameOver(GameResult::Remaining(winners)) = progress else {
        panic!("surviving Emperor team should win: {progress:?}");
    };
    assert_eq!(winners, seats[3..6]);

    let (mut draw_game, draw_seats) = players(6);
    draw_game
        .restore_emperor(
            vec![draw_seats[0..3].to_vec(), draw_seats[3..6].to_vec()],
            draw_seats.clone(),
            0,
            draw_seats[1],
            vec![1, 2, 1, 1, 2, 1],
        )
        .unwrap();
    let drawn = draw_game.draw_game_for_players([draw_seats[1]]);
    assert_eq!(drawn, draw_seats[0..3]);
    assert!(draw_seats[0..3].iter().all(|player| {
        let player = draw_game.player(*player).unwrap();
        player.has_left_game && !player.has_lost
    }));
}

#[test]
fn u074_restart_preserves_the_profile_and_subgames_choose_a_fresh_emperor() {
    let (mut game, seats) = players(6);
    let teams = vec![seats[0..3].to_vec(), seats[3..6].to_vec()];
    game.restore_emperor(
        teams.clone(),
        seats.clone(),
        0,
        seats[1],
        vec![1, 2, 1, 1, 2, 1],
    )
    .unwrap();

    game.restart_game(seats[4], &[]);
    assert_eq!(game.emperor().unwrap().teams(), teams);
    assert_eq!(game.turn.active_player, seats[4]);
    assert!(game.deploy_creatures_enabled());

    game.begin_subgame(None, seats[4], Vec::new()).unwrap();
    let child = game.emperor().expect("subgame Emperor profile");
    assert_eq!(child.teams(), teams);
    assert_eq!(child.seats(), seats);
    assert_eq!(
        child.starting_emperor(),
        child.emperors()[child.starting_team()]
    );
    assert_eq!(game.turn.active_player, child.starting_emperor());
    assert_eq!(game.turn_store.turn_order[0], child.starting_emperor());
    assert!(game.deploy_creatures_enabled());
}
