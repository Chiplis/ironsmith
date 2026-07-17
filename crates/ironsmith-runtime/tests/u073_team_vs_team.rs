use ironsmith::decision::{GameProgress, GameResult};
use ironsmith::game_loop::advance_priority;
use ironsmith::{AttackDirection, GameState, ManaSymbol, PlayerId, TriggerQueue};

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

#[test]
fn u073_seeded_starting_team_uses_its_center_or_left_midpoint_seat() {
    let (mut first, seats) = players(7);
    let teams = vec![seats[0..3].to_vec(), seats[3..7].to_vec()];
    first.set_random_seed(808);
    first.enable_team_vs_team(teams.clone()).unwrap();

    let state = first.team_vs_team().expect("Team vs. Team profile");
    let selected = &teams[state.starting_team()];
    assert_eq!(state.starting_player(), selected[(selected.len() - 1) / 2]);
    assert_eq!(state.seats(), seats);
    assert_eq!(first.turn.active_player, state.starting_player());
    assert_eq!(first.turn_store.turn_order[0], state.starting_player());
    assert_eq!(
        first.turn_store.turn_order,
        state
            .seats()
            .iter()
            .cycle()
            .skip_while(|player| **player != state.starting_player())
            .take(state.seats().len())
            .copied()
            .collect::<Vec<_>>()
    );

    let (mut replay, _) = players(7);
    replay.set_random_seed(808);
    replay.enable_team_vs_team(teams).unwrap();
    assert_eq!(
        replay.team_vs_team().unwrap().starting_team(),
        state.starting_team(),
        "the starting-team choice comes from the deterministic match RNG",
    );
}

#[test]
fn u073_profile_fixes_options_but_keeps_resources_and_control_individual() {
    let (mut game, seats) = players(6);
    let teams = vec![seats[0..2].to_vec(), seats[2..6].to_vec()];
    game.restore_team_vs_team(teams.clone(), seats.clone(), 0, seats[0])
        .unwrap();

    assert!(game.are_teammates(seats[0], seats[1]));
    assert!(!game.are_opponents(seats[0], seats[1]));
    assert!(game.are_opponents(seats[0], seats[2]));
    assert!(!game.shared_team_turns_enabled());
    assert!(!game.deploy_creatures_enabled());
    assert!(game.limited_range_of_influence().is_none());
    assert_eq!(game.attack_direction(), None);

    game.set_deploy_creatures(true);
    game.set_attack_direction(Some(AttackDirection::Left));
    assert!(
        game.enable_limited_range_of_influence(seats.clone(), vec![1; seats.len()])
            .is_err()
    );
    assert!(game.enable_shared_team_turns().is_err());
    assert!(game.set_teams(teams.into_iter().rev().collect()).is_err());
    assert!(!game.deploy_creatures_enabled());
    assert_eq!(game.attack_direction(), None);
    assert!(game.limited_range_of_influence().is_none());

    game.player_mut(seats[0])
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Colorless, 3);
    assert_eq!(game.player(seats[0]).unwrap().mana_pool.total(), 3);
    assert_eq!(game.player(seats[1]).unwrap().mana_pool.total(), 0);
}

#[test]
fn u073_individuals_leave_but_the_last_team_makes_every_teammate_a_winner() {
    let (mut game, seats) = players(4);
    game.restore_team_vs_team(
        vec![seats[0..2].to_vec(), seats[2..4].to_vec()],
        seats.clone(),
        0,
        seats[0],
    )
    .unwrap();

    assert!(game.mark_player_lost(seats[0]));
    assert!(game.mark_player_lost(seats[2]));
    assert!(game.mark_player_lost(seats[3]));
    assert!(game.player(seats[1]).unwrap().is_in_game());

    let mut queue = TriggerQueue::new();
    let result = advance_priority(&mut game, &mut queue).unwrap();
    let GameProgress::GameOver(GameResult::Remaining(winners)) = result else {
        panic!("the sole surviving team should win together: {result:?}");
    };
    assert_eq!(winners, vec![seats[0], seats[1]]);
    assert!(game.player(seats[0]).unwrap().has_won);
    assert!(game.player(seats[1]).unwrap().has_won);
    assert!(game.player(seats[0]).unwrap().has_left_game);
}

#[test]
fn u073_restart_and_subgame_preserve_team_blocks_and_individual_turns() {
    let (mut game, seats) = players(6);
    let teams = vec![seats[0..3].to_vec(), seats[3..6].to_vec()];
    game.restore_team_vs_team(teams.clone(), seats.clone(), 0, seats[1])
        .unwrap();

    game.restart_game(seats[4], &[]);
    let restarted = game.team_vs_team().expect("restart profile");
    assert_eq!(restarted.teams(), teams);
    assert_eq!(restarted.seats(), seats);
    assert_eq!(game.turn.active_player, seats[4]);
    assert!(!game.shared_team_turns_enabled());

    game.begin_subgame(None, seats[4], Vec::new()).unwrap();
    let child = game.team_vs_team().expect("subgame profile");
    assert_eq!(child.teams(), teams);
    assert_eq!(child.seats(), seats);
    let selected_team = &teams[child.starting_team()];
    assert_eq!(
        child.starting_player(),
        selected_team[(selected_team.len() - 1) / 2]
    );
    assert_eq!(game.turn.active_player, child.starting_player());
    assert_eq!(game.turn_store.turn_order[0], child.starting_player());
    assert!(!game.shared_team_turns_enabled());
}
