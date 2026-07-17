use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::effects::{ExchangeLifeTotalsEffect, ForPlayersEffect, SetLifeTotalEffect};
use ironsmith::rules::state_based::LoseReason;
use ironsmith::{
    Ability, CardId, CardType, CounterType, DecisionMaker, Effect, EffectContext, EffectExecutor,
    GameState, LifeLossEvent, PlayerFilter, PlayerId, ResolvedTarget, SelectOptionsContext,
    StaticAbility, Value, Zone, apply_state_based_actions, check_state_based_actions,
};

struct ChooseLowestHead;

impl DecisionMaker for ChooseLowestHead {
    fn decide_options(&mut self, _game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        ctx.options
            .iter()
            .filter(|option| option.legal)
            .min_by_key(|option| &option.description)
            .map(|option| vec![option.index])
            .unwrap_or_default()
    }
}

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

fn enable(game: &mut GameState, seats: &[PlayerId], team_size: usize) {
    game.set_random_seed(810);
    game.enable_two_headed_giant(vec![
        seats[..team_size].to_vec(),
        seats[team_size..].to_vec(),
    ])
    .unwrap();
}

fn team_pool(game: &GameState, team: &[PlayerId]) -> Vec<(i32, u32)> {
    team.iter()
        .map(|player| {
            let player = game.player(*player).unwrap();
            (player.life, player.poison_counters)
        })
        .collect()
}

#[test]
fn u075_profile_builds_shared_turns_seats_life_draw_skip_and_larger_thresholds() {
    let (mut game, seats) = players(4);
    let teams = vec![seats[0..2].to_vec(), seats[2..4].to_vec()];
    game.set_random_seed(810);
    game.enable_two_headed_giant(teams.clone()).unwrap();

    let profile = game.two_headed_giant().expect("Two-Headed Giant profile");
    assert_eq!(profile.teams(), teams);
    assert_eq!(profile.seats(), seats);
    assert_eq!(profile.starting_life(), 30);
    assert_eq!(profile.poison_threshold(), 15);
    assert_eq!(
        profile.starting_player(),
        *profile.teams()[profile.starting_team()].last().unwrap()
    );
    assert_eq!(game.turn.active_player, profile.starting_player());
    assert!(game.shared_team_turns_enabled());
    assert!(game.limited_range_of_influence().is_none());
    assert!(!game.deploy_creatures_enabled());
    assert!(game.can_review_teammate_hand(seats[0], seats[1]));
    assert!(!game.can_review_teammate_hand(seats[0], seats[2]));
    assert!(
        profile.teams()[profile.starting_team()]
            .iter()
            .all(|player| game.should_skip_first_turn_draw(*player))
    );
    assert!(
        profile.teams()[1 - profile.starting_team()]
            .iter()
            .all(|player| !game.should_skip_first_turn_draw(*player))
    );
    assert!(game.players.iter().all(|player| player.life == 30));

    game.disable_shared_team_turns();
    game.set_deploy_creatures(true);
    assert!(game.set_teams(teams.into_iter().rev().collect()).is_err());
    assert!(game.shared_team_turns_enabled());
    assert!(!game.deploy_creatures_enabled());

    let (mut larger, larger_seats) = players(6);
    enable(&mut larger, &larger_seats, 3);
    let larger_profile = larger.two_headed_giant().unwrap();
    assert_eq!(larger_profile.starting_life(), 45);
    assert_eq!(larger_profile.poison_threshold(), 20);
    assert!(larger.players.iter().all(|player| player.life == 45));

    let (mut invalid, invalid_seats) = players(5);
    assert!(
        invalid
            .enable_two_headed_giant(vec![
                invalid_seats[0..2].to_vec(),
                invalid_seats[2..5].to_vec(),
            ])
            .is_err()
    );
    assert!(invalid.two_headed_giant().is_none());
    assert!(invalid.players.iter().all(|player| player.life == 20));
}

#[test]
fn u075_individual_life_events_change_one_shared_pool_and_each_player_sets_once_per_team() {
    let (mut game, seats) = players(4);
    enable(&mut game, &seats, 2);
    let source = game.new_object_id();
    let mut choices = ChooseLowestHead;
    let mut ctx = EffectContext::new_default(source, seats[0]).with_decision_maker(&mut choices);

    ForPlayersEffect::new(
        PlayerFilter::Any,
        vec![Effect::lose_life_player(4, PlayerFilter::IteratedPlayer)],
    )
    .execute(&mut game, &mut ctx)
    .unwrap();
    assert_eq!(team_pool(&game, &seats[0..2]), vec![(22, 0), (22, 0)]);
    assert_eq!(team_pool(&game, &seats[2..4]), vec![(22, 0), (22, 0)]);

    let outcome = ForPlayersEffect::new(
        PlayerFilter::Any,
        vec![Effect::new(SetLifeTotalEffect::new(
            10,
            PlayerFilter::IteratedPlayer,
        ))],
    )
    .execute(&mut game, &mut ctx)
    .unwrap();
    let affected = outcome
        .events
        .iter()
        .filter_map(|event| event.downcast::<LifeLossEvent>())
        .map(|event| event.player)
        .collect::<Vec<_>>();
    assert_eq!(affected, vec![seats[0], seats[2]]);
    assert_eq!(team_pool(&game, &seats[0..2]), vec![(10, 0), (10, 0)]);
    assert_eq!(team_pool(&game, &seats[2..4]), vec![(10, 0), (10, 0)]);

    game.gain_life(seats[1], 7);
    assert_eq!(team_pool(&game, &seats[0..2]), vec![(17, 0), (17, 0)]);
    game.lose_life(seats[0], 2);
    assert_eq!(team_pool(&game, &seats[0..2]), vec![(15, 0), (15, 0)]);

    ctx.targets = vec![ResolvedTarget::Player(seats[1])];
    let prevented = ExchangeLifeTotalsEffect::with_target()
        .execute(&mut game, &mut ctx)
        .unwrap();
    assert_eq!(
        prevented.status,
        ironsmith::effect::OutcomeStatus::Prevented
    );

    game.write_life_total(seats[2], 5);
    ctx.targets = vec![ResolvedTarget::Player(seats[2])];
    ExchangeLifeTotalsEffect::with_target()
        .execute(&mut game, &mut ctx)
        .unwrap();
    assert_eq!(team_pool(&game, &seats[0..2]), vec![(5, 0), (5, 0)]);
    assert_eq!(team_pool(&game, &seats[2..4]), vec![(15, 0), (15, 0)]);
}

#[test]
fn u075_each_player_damage_is_individual_and_changes_each_shared_pool_twice() {
    let (mut game, seats) = players(4);
    enable(&mut game, &seats, 2);
    let source = game.new_object_id();
    let mut ctx = EffectContext::new_default(source, seats[0]);

    Effect::deal_damage(4, ironsmith::ChooseSpec::EachPlayer(PlayerFilter::Any))
        .0
        .execute(&mut game, &mut ctx)
        .unwrap();

    assert_eq!(team_pool(&game, &seats[0..2]), vec![(22, 0), (22, 0)]);
    assert_eq!(team_pool(&game, &seats[2..4]), vec![(22, 0), (22, 0)]);
}

#[test]
fn u075_payments_redistribution_and_opposing_poison_queries_use_team_pools() {
    let (mut game, seats) = players(4);
    enable(&mut game, &seats, 2);

    assert!(!game.pay_life_simultaneously(&[(seats[0], 20), (seats[1], 11)]));
    assert_eq!(team_pool(&game, &seats[0..2]), vec![(30, 0), (30, 0)]);
    assert!(game.pay_life_simultaneously(&[(seats[0], 8), (seats[1], 4)]));
    assert_eq!(team_pool(&game, &seats[0..2]), vec![(18, 0), (18, 0)]);
    assert!(game.pay_life_simultaneously(&[(seats[0], 0), (seats[1], 0)]));

    assert!(!game.redistribute_life_totals(&[(seats[0], 18), (seats[1], 18)]));
    assert!(game.redistribute_life_totals(&[(seats[0], 30), (seats[2], 18)]));
    assert_eq!(team_pool(&game, &seats[0..2]), vec![(30, 0), (30, 0)]);
    assert_eq!(team_pool(&game, &seats[2..4]), vec![(18, 0), (18, 0)]);

    game.add_player_counters_with_source(seats[2], CounterType::Poison, 4, None, None);
    let source = game.new_object_id();
    let mut ctx = EffectContext::new_default(source, seats[0]);
    Effect::gain_life_player(
        Value::PlayerCounters(PlayerFilter::Opponent, CounterType::Poison),
        ironsmith::ChooseSpec::Player(PlayerFilter::You),
    )
    .0
    .execute(&mut game, &mut ctx)
    .unwrap();
    assert_eq!(team_pool(&game, &seats[0..2]), vec![(34, 0), (34, 0)]);
}

#[test]
fn u075_life_restrictions_on_one_head_protect_the_whole_team() {
    let (mut game, seats) = players(4);
    enable(&mut game, &seats, 2);
    let lock = CardDefinitionBuilder::new(CardId::new(), "Shared Life Lock")
        .card_types(vec![CardType::Artifact])
        .with_ability(Ability::static_ability(
            StaticAbility::your_life_total_cant_change(),
        ))
        .build();
    game.create_object_from_definition(&lock, seats[0], Zone::Battlefield);
    game.refresh_continuous_state();

    assert!(!game.can_gain_life(seats[0]));
    assert!(!game.can_gain_life(seats[1]));
    assert!(!game.can_lose_life(seats[0]));
    assert!(!game.can_lose_life(seats[1]));
    assert!(game.can_gain_life(seats[2]));
    assert_eq!(game.lose_life(seats[1], 4), 0);
    assert_eq!(team_pool(&game, &seats[0..2]), vec![(30, 0), (30, 0)]);
}

#[test]
fn u075_poison_is_shared_uses_the_team_threshold_and_any_loss_propagates() {
    let (mut game, seats) = players(4);
    enable(&mut game, &seats, 2);
    game.add_player_counters_with_source(seats[0], CounterType::Poison, 7, None, None);
    assert_eq!(team_pool(&game, &seats[0..2]), vec![(30, 7), (30, 7)]);
    game.remove_player_counters_with_source(seats[1], CounterType::Poison, 2, None, None);
    assert_eq!(team_pool(&game, &seats[0..2]), vec![(30, 5), (30, 5)]);
    game.add_player_counters_with_source(seats[1], CounterType::Poison, 10, None, None);

    let actions = check_state_based_actions(&game);
    assert_eq!(
        actions
            .iter()
            .filter(|action| matches!(
                action,
                ironsmith::StateBasedAction::PlayerLoses {
                    reason: LoseReason::Poison,
                    ..
                }
            ))
            .count(),
        1
    );
    assert!(apply_state_based_actions(&mut game));
    assert!(
        seats[0..2]
            .iter()
            .all(|player| game.player(*player).unwrap().has_left_game)
    );
    assert!(
        seats[2..4]
            .iter()
            .all(|player| game.player(*player).unwrap().is_in_game())
    );
}

#[test]
fn u075_win_restart_and_subgame_operate_on_complete_shared_turn_teams() {
    let (mut game, seats) = players(4);
    enable(&mut game, &seats, 2);
    let source = game.new_object_id();
    let mut ctx = EffectContext::new_default(source, seats[0]);
    Effect::win_the_game()
        .0
        .execute(&mut game, &mut ctx)
        .unwrap();
    assert!(game.player(seats[0]).unwrap().is_in_game());
    assert!(game.player(seats[1]).unwrap().is_in_game());
    assert!(game.player(seats[2]).unwrap().has_left_game);
    assert!(game.player(seats[3]).unwrap().has_left_game);

    let (mut lifecycle, lifecycle_seats) = players(4);
    enable(&mut lifecycle, &lifecycle_seats, 2);
    let teams = lifecycle.two_headed_giant().unwrap().teams().to_vec();
    lifecycle.lose_life(lifecycle_seats[0], 12);
    lifecycle.restart_game(lifecycle_seats[2], &[]);
    assert_eq!(lifecycle.two_headed_giant().unwrap().teams(), teams);
    assert!(lifecycle.shared_team_turns_enabled());
    assert!(lifecycle.players.iter().all(|player| player.life == 30));

    lifecycle
        .begin_subgame(None, lifecycle_seats[2], Vec::new())
        .unwrap();
    assert_eq!(lifecycle.two_headed_giant().unwrap().teams(), teams);
    assert!(lifecycle.shared_team_turns_enabled());
    assert!(lifecycle.players.iter().all(|player| player.life == 30));
}

#[test]
fn u075_cant_win_cant_lose_and_concession_apply_to_the_complete_team() {
    let (mut game, seats) = players(4);
    enable(&mut game, &seats, 2);
    let angel = CardDefinitionBuilder::new(CardId::new(), "Shared Platinum Angel")
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .with_ability(Ability::static_ability(StaticAbility::you_cant_lose_game()))
        .with_ability(Ability::static_ability(
            StaticAbility::opponents_cant_win_game(),
        ))
        .build();
    game.create_object_from_definition(&angel, seats[0], Zone::Battlefield);
    game.refresh_continuous_state();

    assert!(!game.can_lose_game(seats[0]));
    assert!(!game.can_lose_game(seats[1]));
    assert!(!game.can_win_game(seats[2]));
    assert!(!game.can_win_game(seats[3]));

    game.write_life_total(seats[1], 0);
    assert!(
        check_state_based_actions(&game)
            .iter()
            .all(|action| !matches!(
                action,
                ironsmith::StateBasedAction::PlayerLoses { player, .. }
                    if seats[0..2].contains(player)
            ))
    );

    assert!(game.concede_game(seats[1]));
    assert!(
        seats[0..2]
            .iter()
            .all(|player| game.player(*player).unwrap().has_left_game)
    );
}
