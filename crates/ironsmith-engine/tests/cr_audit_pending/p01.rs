//! CR audit (pending-entry pass, package p01): assertions express the rules, so confirmed defects fail.
//! Do not use `should_panic`, `ignore`, or assertions of current broken behavior.

#![allow(unused_imports, dead_code)]

use ironsmith::{EffectContext, GameState, PlayerId};

use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::{
    CardId, CardType, ChooseSpec, Effect, EffectExecutor, GameProgress, GameResult, ObjectFilter,
    ObjectId, PlayerFilter, PowerToughness, Zone, execute_effect,
};

struct NoChoices;
impl ironsmith::DecisionMaker for NoChoices {}

fn creature_def(name: &str) -> ironsmith::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}

fn creature(game: &mut GameState, owner: PlayerId, name: &str) -> ObjectId {
    game.create_object_from_definition(&creature_def(name), owner, Zone::Battlefield)
}

fn named_game(count: usize) -> GameState {
    GameState::new((0..count).map(|i| format!("Player {i}")).collect(), 20)
}

fn advance(game: &mut GameState) -> GameProgress {
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    ironsmith::game_loop::advance_priority(game, &mut queue).unwrap()
}

// ---------------------------------------------------------------- 101.2a

#[test]
fn cr_101_2a_adding_and_removing_abilities_follow_timestamps_not_cant_beats_can() {
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::{StaticAbility, Until};
    let alice = PlayerId::from_index(0);
    let mut game = named_game(2);
    let first = creature(&mut game, alice, "Remove then add");
    let second = creature(&mut game, alice, "Add then remove");
    let apply = |game: &mut GameState, object: ObjectId, modification: Modification| {
        ApplyContinuousEffect::new(EffectTarget::Specific(object), modification, Until::EndOfTurn)
            .execute(game, &mut EffectContext::new_default(object, alice))
            .unwrap();
    };
    apply(&mut game, first, Modification::RemoveAllAbilities);
    apply(&mut game, first, Modification::AddAbility(StaticAbility::flying()));
    apply(&mut game, second, Modification::AddAbility(StaticAbility::flying()));
    apply(&mut game, second, Modification::RemoveAllAbilities);
    assert!(
        game.object_has_ability(first, &StaticAbility::flying()),
        "CR 101.2a/613.7: a later 'gains flying' applies after an earlier 'loses all abilities'"
    );
    assert!(
        !game.object_has_ability(second, &StaticAbility::flying()),
        "CR 101.2a/613.7: a later 'loses all abilities' removes an earlier granted flying"
    );
}

// ---------------------------------------------------------------- 101.4 / 101.4c / 101.4e

struct RecordChoices {
    calls: Vec<(PlayerId, &'static str)>,
}
impl ironsmith::DecisionMaker for RecordChoices {
    fn decide_boolean(
        &mut self,
        _game: &GameState,
        choice: &ironsmith::decisions::context::BooleanContext,
    ) -> bool {
        self.calls.push((choice.player, "accept"));
        true
    }
    fn decide_objects(
        &mut self,
        _game: &GameState,
        choice: &ironsmith::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        self.calls.push((choice.player, "sacrifice"));
        vec![choice.candidates[0].id]
    }
}

#[test]
fn cr_101_4c_101_4e_starting_player_is_active_and_each_player_makes_all_choices_in_order() {
    use ironsmith::effects::{ForPlayersEffect, SacrificeEffect};
    let p: Vec<_> = (0..3).map(PlayerId::from_index).collect();
    let mut game = named_game(3);
    // CR 101.4e: the starting player is the active player for start-of-game choices.
    game.set_starting_player(p[1]);
    assert_eq!(game.turn.active_player, p[1], "CR 101.4e: starting player is active");
    for owner in &p {
        for i in 0..2 {
            creature(&mut game, *owner, &format!("Candidate {i}"));
        }
    }
    let source = creature(&mut game, p[1], "Source");
    let mut dm = RecordChoices { calls: vec![] };
    let mut ctx = EffectContext::new_default(source, p[1]).with_decision_maker(&mut dm);
    ForPlayersEffect::new(
        PlayerFilter::Any,
        vec![Effect::may(vec![Effect::new(SacrificeEffect::player(
            ObjectFilter::creature(),
            1,
            PlayerFilter::IteratedPlayer,
        ))])],
    )
    .execute(&mut game, &mut ctx)
    .unwrap();
    drop(ctx);
    assert_eq!(
        dm.calls,
        vec![
            (p[1], "accept"),
            (p[1], "sacrifice"),
            (p[2], "accept"),
            (p[2], "sacrifice"),
            (p[0], "accept"),
            (p[0], "sacrifice"),
        ],
        "CR 101.4/101.4c/101.4e: APNAP from the starting player; each player's choices in the specified order"
    );
}

// ---------------------------------------------------------------- 102.4

#[test]
fn cr_102_4_your_team_means_you_outside_team_games_and_teammates_inside() {
    use ironsmith::{StaticAbility, Subtype};
    let definition = ironsmith_compiler_runtime::compile_to_runtime_definition(
        "Rushblade Commander",
        "Mana cost: {B}{R}\nType: Creature — Azra Warrior\nPower/Toughness: 2/2\nWarriors your team controls have haste.",
        false,
    )
    .expect("strict compile of Rushblade Commander");
    let warrior = CardDefinitionBuilder::new(CardId::new(), "Warrior body")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Warrior])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    // Two-player game: "your team" is "you".
    let mut game = named_game(2);
    let (alice, bob) = (PlayerId::from_index(0), PlayerId::from_index(1));
    game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let mine = game.create_object_from_definition(&warrior, alice, Zone::Battlefield);
    let theirs = game.create_object_from_definition(&warrior, bob, Zone::Battlefield);
    assert!(
        game.object_has_ability(mine, &StaticAbility::haste()),
        "CR 102.4: 'your team' includes you"
    );
    assert!(
        !game.object_has_ability(theirs, &StaticAbility::haste()),
        "CR 102.4: outside a team game 'your team' means only you"
    );
    // Team game: teammates are included, opponents are not.
    let mut game = named_game(4);
    let p: Vec<_> = (0..4).map(PlayerId::from_index).collect();
    game.set_teams(vec![vec![p[0], p[2]], vec![p[1], p[3]]]).unwrap();
    game.create_object_from_definition(&definition, p[0], Zone::Battlefield);
    let bodies: Vec<_> = p
        .iter()
        .map(|o| game.create_object_from_definition(&warrior, *o, Zone::Battlefield))
        .collect();
    let haste: Vec<_> = bodies
        .iter()
        .map(|c| game.object_has_ability(*c, &StaticAbility::haste()))
        .collect();
    assert_eq!(
        haste,
        vec![true, false, true, false],
        "CR 102.4: 'your team' is you and/or your teammates"
    );
}

// ---------------------------------------------------------------- 103.1c

#[test]
fn cr_103_1c_power_play_makes_its_controller_the_starting_player() {
    let text = "Type: Conspiracy\n(Start the game with this conspiracy face up in the command zone.)\nYou are the starting player. If multiple players would be the starting player, one of those players is chosen at random.";
    let compiled =
        ironsmith_compiler_runtime::compile_to_runtime_definition("Power Play", text, false);
    assert!(
        compiled.is_ok(),
        "CR 103.1c: Power Play's starting-player effect must compile: {:?}",
        compiled.as_ref().err()
    );
    let definition = compiled.unwrap();
    assert!(
        !ironsmith::cards::generated_definition_has_unimplemented_content(&definition),
        "CR 103.1c: Power Play must not compile to unimplemented content"
    );
    let bob = PlayerId::from_index(1);
    let mut wrong = Vec::new();
    for seed in 0..16u64 {
        let mut game = named_game(4);
        game.enable_conspiracy(vec![(
            bob,
            vec![ironsmith::game_state::ConspiracySetupCard {
                definition: definition.clone(),
                agenda_names: vec![],
            }],
        )])
        .unwrap();
        game.set_random_seed(seed + 1);
        let chosen = game.randomize_starting_player();
        if chosen != Some(bob) || game.turn.active_player != bob {
            wrong.push((seed, chosen));
        }
    }
    assert!(
        wrong.is_empty(),
        "CR 103.1c: Power Play supersedes the random starting-player determination; got {wrong:?}"
    );
    let charlie = PlayerId::from_index(2);
    let mut observed = std::collections::HashSet::new();
    for seed in 1..=32 {
        let choose = || {
            let mut game = named_game(4);
            game.enable_conspiracy(vec![
                (bob, vec![
                    ironsmith::game_state::ConspiracySetupCard { definition: definition.clone(), agenda_names: vec![] },
                    ironsmith::game_state::ConspiracySetupCard { definition: definition.clone(), agenda_names: vec![] },
                ]),
                (charlie, vec![ironsmith::game_state::ConspiracySetupCard {
                    definition: definition.clone(), agenda_names: vec![],
                }]),
            ]).unwrap();
            game.set_random_seed(seed);
            game.randomize_starting_player().unwrap()
        };
        let chosen = choose();
        assert!([bob, charlie].contains(&chosen), "only eligible players may start");
        assert_eq!(chosen, choose(), "the match seed reproduces the selection");
        observed.insert(chosen);
    }
    assert_eq!(observed.len(), 2, "random selection reaches both eligible players");

}

// ---------------------------------------------------------------- 104.1 / 104.2b / 104.3e / 104.4c

#[test]
fn cr_104_1_104_2b_win_effect_ends_the_game_immediately_before_state_based_actions() {
    let alice = PlayerId::from_index(0);
    let mut game = named_game(2);
    game.player_mut(alice).unwrap().life = 1;
    let source = creature(&mut game, alice, "Win source");
    let mut dm = NoChoices;
    let mut ctx = EffectContext::new_default(source, alice).with_decision_maker(&mut dm);
    // "You lose 1 life. You win the game." resolving as one spell.
    execute_effect(&mut game, &Effect::lose_life(1), &mut ctx).unwrap();
    execute_effect(&mut game, &Effect::win_the_game(), &mut ctx).unwrap();
    drop(ctx);
    let progress = advance(&mut game);
    assert!(
        matches!(progress, GameProgress::GameOver(GameResult::Winner(id)) if id == alice),
        "CR 104.1/104.2b: the game ends the moment Alice wins; a later SBA cannot turn it into {progress:?}"
    );
}

#[test]
fn cr_104_2b_104_3e_104_4c_win_lose_and_draw_effects() {
    let p: Vec<_> = (0..3).map(PlayerId::from_index).collect();
    // Win effect in a three-player game.
    let mut game = named_game(3);
    let source = creature(&mut game, p[0], "Win");
    let mut dm = NoChoices;
    let mut ctx = EffectContext::new_default(source, p[0]).with_decision_maker(&mut dm);
    execute_effect(&mut game, &Effect::win_the_game(), &mut ctx).unwrap();
    drop(ctx);
    let progress = advance(&mut game);
    assert!(
        matches!(progress, GameProgress::GameOver(GameResult::Winner(id)) if id == p[0]),
        "CR 104.2b: an effect may state that a player wins: {progress:?}"
    );
    assert!(game.player(p[1]).unwrap().has_lost && game.player(p[2]).unwrap().has_lost);

    // Lose effect: the named player loses; two players remain.
    let mut game = named_game(3);
    let source = creature(&mut game, p[0], "Lose");
    let mut dm = NoChoices;
    let mut ctx = EffectContext::new_default(source, p[0]).with_decision_maker(&mut dm);
    execute_effect(
        &mut game,
        &Effect::lose_the_game_player(PlayerFilter::Specific(p[1])),
        &mut ctx,
    )
    .unwrap();
    drop(ctx);
    assert!(game.player(p[1]).unwrap().has_lost, "CR 104.3e: effect loss");
    assert!(!matches!(advance(&mut game), GameProgress::GameOver(_)));

    // Draw effect in an ordinary game: the whole game is a draw.
    let mut game = named_game(2);
    let source = creature(&mut game, p[0], "Draw");
    let mut dm = NoChoices;
    let mut ctx = EffectContext::new_default(source, p[0]).with_decision_maker(&mut dm);
    execute_effect(&mut game, &Effect::draw_the_game(), &mut ctx).unwrap();
    drop(ctx);
    let progress = advance(&mut game);
    assert!(
        matches!(progress, GameProgress::GameOver(GameResult::Draw)),
        "CR 104.4c: an effect may state the game is a draw: {progress:?}"
    );
    assert!(game.players.iter().all(|player| !player.has_lost && !player.has_won));
}

// ---------------------------------------------------------------- team results

#[test]
fn cr_104_2c_104_3g_104_4d_104_4g_team_results() {
    let p: Vec<_> = (0..4).map(PlayerId::from_index).collect();
    let teams = vec![vec![p[0], p[1]], vec![p[2], p[3]]];

    // 104.3g + 104.2c: a team with every player lost loses; the other team wins
    // even its previously lost member.
    let mut game = named_game(4);
    game.set_teams(teams.clone()).unwrap();
    game.mark_player_lost(p[0]).unwrap();
    assert!(!matches!(advance(&mut game), GameProgress::GameOver(_)), "a live teammate keeps the team in");
    game.mark_player_lost(p[2]).unwrap();
    game.mark_player_lost(p[3]).unwrap();
    let progress = advance(&mut game);
    assert!(
        matches!(&progress, GameProgress::GameOver(GameResult::Remaining(w)) if *w == vec![p[0], p[1]]),
        "CR 104.2c/104.3g: the surviving team wins together: {progress:?}"
    );
    assert!(game.player(p[0]).unwrap().has_won, "CR 104.2c: previously lost teammate also wins");
    assert!(!game.player(p[2]).unwrap().has_won && !game.player(p[3]).unwrap().has_won);

    // 104.4d: every remaining team loses simultaneously -> draw.
    let mut game = named_game(4);
    game.set_teams(teams.clone()).unwrap();
    for player in &p {
        game.player_mut(*player).unwrap().life = 0;
    }
    let progress = advance(&mut game);
    assert!(
        matches!(progress, GameProgress::GameOver(GameResult::Draw)),
        "CR 104.4d: all teams losing simultaneously is a draw: {progress:?}"
    );

    // 104.4g: the game is a draw for a team when it is a draw for all its remaining
    // players; that team leaves without losing, and the other team then wins.
    let mut game = named_game(4);
    game.set_teams(teams).unwrap();
    game.draw_game_for_players([p[0], p[1]]).unwrap();
    for player in &p[0..2] {
        let player = game.player(*player).unwrap();
        assert!(player.has_left_game && !player.has_lost, "CR 104.4g/104.5: drawn, not lost");
    }
    let progress = advance(&mut game);
    assert!(
        matches!(&progress, GameProgress::GameOver(GameResult::Remaining(w)) if *w == vec![p[2], p[3]]),
        "CR 104.2c: the remaining team wins once the drawn team has left: {progress:?}"
    );
    assert!(!game.player(p[0]).unwrap().has_won, "CR 104.4g: drawn team does not win");
}

// ---------------------------------------------------------------- 104.2d / 104.3i / 104.4h Emperor

#[test]
fn cr_104_2d_104_3i_104_4h_emperor_results_belong_to_the_team() {
    let seats: Vec<_> = (0..6).map(PlayerId::from_index).collect();
    let teams = vec![seats[0..3].to_vec(), seats[3..6].to_vec()];
    // 104.3i: the emperor losing makes the team lose; 104.2d: the other emperor's team wins.
    let mut game = named_game(6);
    game.restore_emperor(teams.clone(), seats.clone(), 0, seats[1], vec![1, 2, 1, 1, 2, 1])
        .unwrap();
    game.mark_player_lost(seats[4]).unwrap();
    for player in &seats[3..6] {
        assert!(game.player(*player).unwrap().has_lost, "CR 104.3i: emperor's team loses");
    }
    let progress = advance(&mut game);
    assert!(
        matches!(&progress, GameProgress::GameOver(GameResult::Remaining(w)) if *w == seats[0..3]),
        "CR 104.2d: the emperor's team wins: {progress:?}"
    );
    assert!(seats[0..3].iter().all(|p| game.player(*p).unwrap().has_won));

    // 104.3i: a general losing alone does not take the team out.
    let mut game = named_game(6);
    game.restore_emperor(teams.clone(), seats.clone(), 0, seats[1], vec![1, 2, 1, 1, 2, 1])
        .unwrap();
    game.mark_player_lost(seats[3]).unwrap();
    assert!(game.player(seats[4]).unwrap().is_in_game() && game.player(seats[5]).unwrap().is_in_game());

    // 104.4h: a draw for the emperor is a draw for the team.
    let mut game = named_game(6);
    game.restore_emperor(teams, seats.clone(), 0, seats[1], vec![1, 2, 1, 1, 2, 1])
        .unwrap();
    let drawn = game.draw_game_for_players([seats[1]]).unwrap();
    assert_eq!(drawn, seats[0..3].to_vec(), "CR 104.4h: emperor draw is the team's draw");
}

// ---------------------------------------------------------------- 104.3h / 104.4e / 104.4f range of influence

fn limited_five(range: u8) -> (GameState, Vec<PlayerId>) {
    let p: Vec<_> = (0..5).map(PlayerId::from_index).collect();
    let mut game = named_game(5);
    game.enable_limited_range_of_influence(p.clone(), vec![range; 5]).unwrap();
    (game, p)
}

#[test]
fn cr_104_3h_104_4e_limited_range_win_and_draw_affect_only_players_in_range() {
    let (mut game, p) = limited_five(1);
    let source = creature(&mut game, p[0], "Win");
    let mut dm = NoChoices;
    let mut ctx = EffectContext::new_default(source, p[0]).with_decision_maker(&mut dm);
    execute_effect(&mut game, &Effect::win_the_game(), &mut ctx).unwrap();
    drop(ctx);
    let lost: Vec<_> = p.iter().map(|x| game.player(*x).unwrap().has_lost).collect();
    assert_eq!(lost, vec![false, true, false, false, true], "CR 104.3h: in-range opponents lose");
    assert!(
        !matches!(advance(&mut game), GameProgress::GameOver(_)),
        "CR 104.3h: this may not cause the game to end"
    );

    let (mut game, p) = limited_five(1);
    let source = creature(&mut game, p[0], "Draw");
    let mut dm = NoChoices;
    let mut ctx = EffectContext::new_default(source, p[0]).with_decision_maker(&mut dm);
    execute_effect(&mut game, &Effect::draw_the_game(), &mut ctx).unwrap();
    drop(ctx);
    let left: Vec<_> = p.iter().map(|x| game.player(*x).unwrap().has_left_game).collect();
    assert_eq!(left, vec![true, true, false, false, true], "CR 104.4e: controller + range draw");
    assert!(p.iter().all(|x| !game.player(*x).unwrap().has_lost));
    assert!(!matches!(advance(&mut game), GameProgress::GameOver(_)), "CR 104.4e: game continues");
}

#[test]
fn cr_104_4f_limited_range_mandatory_loop_draws_only_involved_controllers_and_their_range() {
    use ironsmith::triggers::{Trigger, TriggerQueue};
    use ironsmith::{
        Ability, LegalAction, Phase, PriorityLoopState, PriorityResponse, StackEntry,
    };
    let (mut game, p) = limited_five(1);
    game.turn.active_player = p[0];
    game.turn.phase = Phase::FirstMain;
    game.turn.priority_player = Some(p[0]);
    let definition = CardDefinitionBuilder::new(CardId::new(), "Endless gain")
        .card_types(vec![CardType::Enchantment])
        .with_ability(Ability::triggered(Trigger::you_gain_life(), vec![Effect::gain_life(1)]))
        .build();
    let source = game.create_object_from_definition(&definition, p[0], Zone::Battlefield);
    game.stack.push(StackEntry::ability(source, p[0], vec![Effect::gain_life(1)]));
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    let mut terminal = None;
    for _ in 0..1024 {
        if !game.player(p[0]).unwrap().is_in_game() {
            break;
        }
        match ironsmith::game_loop::apply_priority_response_with_dm(
            &mut game,
            &mut queue,
            &mut state,
            &PriorityResponse::PriorityAction(LegalAction::PassPriority),
            &mut dm,
        )
        .unwrap()
        {
            GameProgress::GameOver(result) => {
                terminal = Some(result);
                break;
            }
            GameProgress::Continue => break,
            _ => {}
        }
    }
    assert!(terminal.is_none(), "CR 104.4f: the game continues for others: {terminal:?}");
    let left: Vec<_> = p.iter().map(|x| game.player(*x).unwrap().has_left_game).collect();
    assert_eq!(
        left,
        vec![true, true, false, false, true],
        "CR 104.4f: loop controller and players in its range draw; others stay"
    );
    assert!(p.iter().all(|x| !game.player(*x).unwrap().has_lost));
}

// ---------------------------------------------------------------- 104.3j

#[test]
fn cr_104_3j_twenty_one_combat_damage_from_one_commander_loses() {
    let (alice, bob) = (PlayerId::from_index(0), PlayerId::from_index(1));
    let mut game = named_game(3);
    let first = creature(&mut game, alice, "Commander A");
    let second = creature(&mut game, alice, "Commander B");
    game.set_as_commander(first, alice);
    game.set_as_commander(second, alice);
    game.record_commander_damage(bob, first, 11);
    game.record_commander_damage(bob, second, 10);
    game.record_commander_damage(bob, first, 9);
    advance(&mut game);
    assert!(
        !game.player(bob).unwrap().has_lost,
        "CR 104.3j: 20 from one commander plus 10 from another is not lethal"
    );
    game.record_commander_damage(bob, first, 1);
    advance(&mut game);
    assert!(game.player(bob).unwrap().has_lost, "CR 104.3j: 21 from the same commander loses");
}

// ---------------------------------------------------------------- 104.6

#[test]
fn cr_104_6_restart_begins_a_new_game_for_players_still_in_the_game() {
    use ironsmith::effects::RestartGameEffect;
    let p: Vec<_> = (0..3).map(PlayerId::from_index).collect();
    let mut game = named_game(3);
    for owner in &p {
        for i in 0..10 {
            game.create_object_from_definition(&creature_def(&format!("Card {i}")), *owner, Zone::Library);
        }
    }
    game.mark_player_lost(p[2]).unwrap();
    game.player_mut(p[0]).unwrap().life = 3;
    let source = creature(&mut game, p[0], "Karn stand-in");
    let mut ctx = EffectContext::new_default(source, p[0]);
    RestartGameEffect::new(None).execute(&mut game, &mut ctx).unwrap();
    assert!(game.restart_starting_procedure_pending(), "CR 104.6/727: a new game begins");
    assert!(game.player(p[0]).unwrap().is_in_game() && game.player(p[1]).unwrap().is_in_game());
    assert!(!game.player(p[2]).unwrap().is_in_game(), "CR 104.6: only players still in the game restart");
    assert_eq!(game.player(p[0]).unwrap().life, 20, "CR 104.6: new game starting life");
    assert!(game.battlefield.is_empty(), "CR 104.6: nothing carries over on the battlefield");
}

// ---------------------------------------------------------------- 105

#[test]
fn cr_105_1_105_2_colors_are_five_and_objects_take_mana_symbol_colors() {
    use ironsmith::{Color, ColorSet, ManaCost, ManaSymbol};
    assert_eq!(Color::ALL.len(), 5, "CR 105.1: five colors");
    let all: ColorSet = Color::ALL.into_iter().collect();
    assert_eq!(all.count(), 5);
    let mut game = named_game(2);
    let alice = PlayerId::from_index(0);
    let def = CardDefinitionBuilder::new(CardId::new(), "Gold")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(1), ManaSymbol::White, ManaSymbol::Blue]))
        .build();
    let gold = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    assert_eq!(
        game.current_colors(gold),
        Some(ColorSet::WHITE.union(ColorSet::BLUE)),
        "CR 105.2: an object is the colors of the mana symbols in its cost"
    );
}

fn colored(game: &mut GameState, owner: PlayerId, symbols: Vec<ironsmith::ManaSymbol>) -> ObjectId {
    let def = CardDefinitionBuilder::new(CardId::new(), "Colored")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .mana_cost(ironsmith::ManaCost::from_symbols(symbols))
        .build();
    game.create_object_from_definition(&def, owner, Zone::Battlefield)
}

#[test]
fn cr_105_2a_2b_2c_5_monocolored_multicolored_colorless_and_color_pairs() {
    use ironsmith::ManaSymbol::{Black, Blue, Generic, Green, Red, White};
    let alice = PlayerId::from_index(0);
    let mut game = named_game(2);
    colored(&mut game, alice, vec![Generic(3)]); // colorless
    colored(&mut game, alice, vec![Red]); // mono
    colored(&mut game, alice, vec![Green, Green]); // mono
    colored(&mut game, alice, vec![White, Blue]); // pair WU
    colored(&mut game, alice, vec![Blue, White]); // same pair
    colored(&mut game, alice, vec![Black, Red]); // pair BR
    colored(&mut game, alice, vec![White, Blue, Black]); // three colors, not a pair
    let source = creature(&mut game, alice, "Probe"); // colorless too
    let ctx = EffectContext::new_default(source, alice);
    let count = |filter: ObjectFilter| {
        ironsmith::effects::helpers::resolve_value(&game, &ironsmith::Value::Count(filter), &ctx)
            .unwrap()
    };
    let mono = ObjectFilter { monocolored: true, ..ObjectFilter::creature() };
    let multi = ObjectFilter { multicolored: true, ..ObjectFilter::creature() };
    let colorless = ObjectFilter { colorless: true, ..ObjectFilter::creature() };
    assert_eq!(count(mono), 2, "CR 105.2a: monocolored is exactly one color");
    assert_eq!(count(multi), 4, "CR 105.2b: multicolored is two or more colors");
    assert_eq!(count(colorless), 2, "CR 105.2c: colorless has no color");
    let pairs = ironsmith::effects::helpers::resolve_value(
        &game,
        &ironsmith::Value::ColorPairsAmong(ObjectFilter::creature()),
        &ctx,
    )
    .unwrap();
    assert_eq!(pairs, 2, "CR 105.5: a color pair is exactly two of the five colors");
}

struct RecordColorOptions {
    seen: Vec<String>,
}
impl ironsmith::DecisionMaker for RecordColorOptions {
    fn decide_options(
        &mut self,
        _game: &GameState,
        ctx: &ironsmith::SelectOptionsContext,
    ) -> Vec<usize> {
        self.seen = ctx.options.iter().map(|o| o.description.to_lowercase()).collect();
        vec![ctx.options.last().unwrap().index]
    }
}

#[test]
fn cr_105_4_choosing_a_color_offers_exactly_the_five_colors() {
    use ironsmith::effects::ChooseColorEffect;
    let alice = PlayerId::from_index(0);
    let mut game = named_game(2);
    let source = creature(&mut game, alice, "Chooser");
    let mut dm = RecordColorOptions { seen: vec![] };
    let mut ctx = EffectContext::new_default(source, alice).with_decision_maker(&mut dm);
    ChooseColorEffect::new(PlayerFilter::You).execute(&mut game, &mut ctx).unwrap();
    drop(ctx);
    assert_eq!(
        dm.seen,
        vec!["white", "blue", "black", "red", "green"],
        "CR 105.4: one of the five colors; not multicolored or colorless"
    );
    assert_eq!(game.chosen_color(source), Some(ironsmith::Color::Green));
}
