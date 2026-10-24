//! CR audit (pending-entry pass, package p16): assertions express the rules, so confirmed defects fail.
//! Do not use `should_panic`, `ignore`, or assertions of current broken behavior.

#![allow(unused_imports, dead_code)]

use ironsmith::{EffectContext, GameState, PlayerId};

use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::DecisionMaker;
use ironsmith::triggers::{
    Trigger, TriggerEvent, TriggerQueue, TriggeredAbilityEntry, TriggeredAbilitySourceKind,
    compute_trigger_identity,
};
use ironsmith::{
    Ability, AttackTarget, AttackerDeclaration, BlockerDeclaration, CardDefinition, CardId,
    CardType, ChooseSpec, CombatState, Effect, ObjectFilter, ObjectId, Phase, PlayerFilter,
    PowerToughness, ResolutionProgram, StaticAbility, TriggeredAbility, Value, Zone,
};

struct P16Dm;
impl DecisionMaker for P16Dm {}

fn p16_game(count: usize) -> (GameState, Vec<PlayerId>) {
    (
        GameState::new((0..count).map(|i| format!("Player {i}")).collect(), 20),
        (0..count).map(|i| PlayerId::from_index(i as u8)).collect(),
    )
}

fn p16_creature_def(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}

fn p16_creature(game: &mut GameState, controller: PlayerId, name: &str) -> ObjectId {
    let id = game.create_object_from_definition(&p16_creature_def(name), controller, Zone::Battlefield);
    game.remove_summoning_sickness(id);
    id
}

fn p16_creature_with(
    game: &mut GameState,
    controller: PlayerId,
    name: &str,
    abilities: Vec<Ability>,
) -> ObjectId {
    let mut builder = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2));
    for ability in abilities {
        builder = builder.with_ability(ability);
    }
    let id = game.create_object_from_definition(&builder.build(), controller, Zone::Battlefield);
    game.remove_summoning_sickness(id);
    id
}

fn p16_compile(name: &str, text: &str) -> CardDefinition {
    let def = ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false)
        .unwrap_or_else(|e| panic!("fixture card {name} must compile: {e:?}"));
    assert!(
        !ironsmith::cards::generated_definition_has_unimplemented_content(&def),
        "fixture card {name} must be fully supported"
    );
    def
}

fn p16_library_card(game: &mut GameState, owner: PlayerId) -> ObjectId {
    let def = CardDefinitionBuilder::new(CardId::new(), "Library Card")
        .card_types(vec![CardType::Artifact])
        .build();
    game.create_object_from_definition(&def, owner, Zone::Library)
}

fn p16_trigger_entry(
    game: &GameState,
    source: ObjectId,
    controller: PlayerId,
    event: TriggerEvent,
) -> TriggeredAbilityEntry {
    let ability = TriggeredAbility {
        trigger: Trigger::beginning_of_upkeep(PlayerFilter::You),
        effects: ResolutionProgram::default(),
        choices: Vec::new(),
        intervening_if: None,
        presentation_label: None,
    };
    TriggeredAbilityEntry {
        linked_exile_owner: None,
        source_number_owner: None,
        source,
        controller,
        x_value: None,
        event_value_amount: None,
        ability: ability.clone(),
        triggering_event: event,
        source_stable_id: game.object(source).expect("trigger source").stable_id,
        source_name: "P16 Trigger".to_string(),
        source_snapshot: None,
        tagged_objects: std::collections::HashMap::new(),
        source_kind: TriggeredAbilitySourceKind::Object,
        trigger_identity: compute_trigger_identity(&ability),
    }
}

fn p16_attack_options(game: &GameState, creature: ObjectId) -> Option<Vec<AttackTarget>> {
    ironsmith::compute_legal_attackers(game, &CombatState::default())
        .into_iter()
        .find(|option| option.creature == creature)
        .map(|option| option.valid_targets)
}

// ---------------------------------------------------------------------------
// CR 800: general multiplayer rules
// ---------------------------------------------------------------------------

#[test]
fn cr_800_1_800_4_multiplayer_game_continues_after_a_player_leaves() {
    use ironsmith::decision::GameProgress;
    let (mut game, p) = p16_game(3);
    assert!(game.mark_player_lost(p[1]).expect("departure fixture"));
    let progress = ironsmith::game_loop::advance_priority(&mut game, &mut TriggerQueue::new())
        .expect("priority advances");
    assert!(
        !matches!(progress, GameProgress::GameOver(_)),
        "CR 800.4: a multiplayer game continues after one player leaves: {progress:?}"
    );
    assert!(game.player(p[0]).unwrap().is_in_game());
    assert!(game.player(p[2]).unwrap().is_in_game());
    assert_eq!(game.players_in_game(), 2);

    let (mut two, q) = p16_game(2);
    assert!(two.mark_player_lost(q[1]).expect("departure fixture"));
    let progress = ironsmith::game_loop::advance_priority(&mut two, &mut TriggerQueue::new())
        .expect("priority advances");
    assert!(
        matches!(progress, GameProgress::GameOver(_)),
        "control: a two-player game ends when one player leaves"
    );
}

#[test]
fn cr_800_2_a_game_uses_at_most_one_variant() {
    let (mut game, p) = p16_game(4);
    game.set_random_seed(16);
    game.enable_two_headed_giant(vec![vec![p[0], p[1]], vec![p[2], p[3]]])
        .expect("Two-Headed Giant profile");
    let _ = game.enable_team_vs_team(vec![vec![p[0], p[1]], vec![p[2], p[3]]]);
    let _ = game.enable_free_for_all(ironsmith::FreeForAllAttackOption::MultiplePlayers, None);
    let active = [
        ("two_headed_giant", game.two_headed_giant().is_some()),
        ("team_vs_team", game.team_vs_team().is_some()),
        ("free_for_all", game.free_for_all().is_some()),
    ];
    let count = active.iter().filter(|(_, on)| *on).count();
    assert_eq!(
        count,
        1,
        "CR 800.2: a single game may use only one variant at a time: {active:?}"
    );
}

#[test]
fn cr_800_4h_rule_choice_for_a_departed_player_passes_to_next_player_in_turn_order() {
    let (mut game, p) = p16_game(4);
    assert_eq!(game.controlling_player_for(p[1]), p[1]);
    assert!(game.leave_game(p[1]).expect("departure fixture"));
    assert_eq!(
        game.controlling_player_for(p[1]),
        p[2],
        "CR 800.4h: the next player in turn order makes a choice the departed player would make"
    );
    assert!(game.leave_game(p[2]).expect("departure fixture"));
    assert_eq!(
        game.controlling_player_for(p[1]),
        p[3],
        "CR 800.4h: skips players who have also left"
    );
}

fn p16_planar_deck(owner: usize) -> Vec<(CardDefinition, ironsmith::PlanarCardKind)> {
    (0..10)
        .map(|index| {
            (
                CardDefinitionBuilder::new(CardId::new(), format!("P16 Plane {owner}-{index}"))
                    .card_types(vec![CardType::Plane])
                    .build(),
                ironsmith::PlanarCardKind::Plane,
            )
        })
        .collect()
}

#[test]
fn cr_800_4p_departing_planar_controller_hands_off_to_next_staying_player() {
    let (mut game, p) = p16_game(4);
    game.enable_planechase(
        p.iter()
            .enumerate()
            .map(|(i, player)| (*player, p16_planar_deck(i)))
            .collect(),
    )
    .expect("Planechase enabled");
    assert_eq!(game.planar_controller(), Some(p[0]));
    assert!(game.leave_game(p[0]).expect("departure fixture"));
    assert_eq!(
        game.planar_controller(),
        Some(p[1]),
        "CR 800.4p: the next player in turn order becomes planar controller"
    );

    let (mut simultaneous, q) = p16_game(4);
    simultaneous
        .enable_planechase(
            q.iter()
                .enumerate()
                .map(|(i, player)| (*player, p16_planar_deck(i)))
                .collect(),
        )
        .expect("Planechase enabled");
    simultaneous
        .mark_players_lost_simultaneously(&[q[0], q[1]])
        .expect("simultaneous departure fixture");
    assert_eq!(
        simultaneous.planar_controller(),
        Some(q[2]),
        "CR 800.4p: the next player in turn order that wouldn't leave the game becomes planar controller"
    );
}

#[test]
fn cr_800_7_810_6_multiplayer_starting_player_draws_but_2hg_starting_team_skips() {
    use ironsmith::Step;
    // Ordinary three-player game: starting player draws on turn 1.
    let (mut game, p) = p16_game(3);
    p16_library_card(&mut game, p[0]);
    game.turn.active_player = p[0];
    game.turn.phase = Phase::Beginning;
    game.turn.step = Some(Step::Draw);
    ironsmith::turn::execute_draw_step(&mut game).expect("draw step");
    assert_eq!(
        game.player(p[0]).unwrap().hand.len(),
        1,
        "CR 800.7: the starting player of a non-2HG multiplayer game doesn't skip the first draw"
    );

    // Control: two-player starting player skips.
    let (mut two, q) = p16_game(2);
    p16_library_card(&mut two, q[0]);
    two.turn.active_player = q[0];
    two.turn.phase = Phase::Beginning;
    two.turn.step = Some(Step::Draw);
    ironsmith::turn::execute_draw_step(&mut two).expect("draw step");
    assert_eq!(two.player(q[0]).unwrap().hand.len(), 0);

    // Two-Headed Giant: each member of the starting team skips the first draw.
    let (mut thg, r) = p16_game(4);
    thg.set_random_seed(7);
    thg.enable_two_headed_giant(vec![vec![r[0], r[1]], vec![r[2], r[3]]])
        .expect("2HG");
    for player in &r {
        p16_library_card(&mut thg, *player);
    }
    let starting_team = thg.two_headed_giant().unwrap().starting_team();
    let team = thg.two_headed_giant().unwrap().teams()[starting_team].clone();
    thg.turn.phase = Phase::Beginning;
    thg.turn.step = Some(Step::Draw);
    ironsmith::turn::execute_draw_step(&mut thg).expect("draw step");
    for member in &team {
        assert_eq!(
            thg.player(*member).unwrap().hand.len(),
            0,
            "CR 800.7/810.6: the 2HG team who plays first skips its first draw"
        );
    }
}

// ---------------------------------------------------------------------------
// CR 801: limited range of influence
// ---------------------------------------------------------------------------

fn p16_roi_game(count: usize, range: u8) -> (GameState, Vec<PlayerId>) {
    let (mut game, p) = p16_game(count);
    game.enable_limited_range_of_influence(p.clone(), vec![range; count])
        .expect("valid range");
    (game, p)
}

#[test]
fn cr_801_3_no_opponent_in_range_means_creatures_cannot_attack() {
    let (mut game, p) = p16_game(5);
    game.enable_limited_range_of_influence(p.clone(), vec![0, 1, 1, 1, 1])
        .expect("valid range");
    let attacker = p16_creature(&mut game, p[0], "Isolated Attacker");
    assert!(
        p16_attack_options(&game, attacker).is_none(),
        "CR 801.3: with no opponent in range, the player's creatures can't attack"
    );
    assert!(
        ironsmith::declare_attackers(
            &mut game,
            &mut CombatState::default(),
            vec![(attacker, AttackTarget::Player(p[1]))],
        )
        .is_err(),
        "CR 801.3: direct declaration against an out-of-range opponent is illegal"
    );

    let (mut ranged, q) = p16_roi_game(5, 1);
    let attacker = p16_creature(&mut ranged, q[0], "Ranged Attacker");
    let targets = p16_attack_options(&ranged, attacker).expect("in-range opponents exist");
    assert!(targets.contains(&AttackTarget::Player(q[1])));
    assert!(targets.contains(&AttackTarget::Player(q[4])));
    assert!(
        !targets.contains(&AttackTarget::Player(q[2])),
        "CR 801.3: out-of-range opponents can't be attacked"
    );
}

#[test]
fn cr_801_6_cannot_activate_abilities_of_out_of_range_objects() {
    let (mut game, p) = p16_roi_game(5, 1);
    let storm = p16_compile(
        "Aether Storm",
        "Mana cost: {3}{U}\nType: Enchantment\nCreature spells can't be cast.\nPay 4 life: Destroy this enchantment. It can't be regenerated. Any player may activate this ability.",
    );
    let storm = game.create_object_from_definition(&storm, p[2], Zone::Battlefield);
    let can_activate = |game: &mut GameState, player: PlayerId| {
        game.turn.active_player = player;
        game.turn.priority_player = Some(player);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        ironsmith::compute_legal_actions(game, player)
            .expect("legal actions")
            .into_iter()
            .any(|action| {
                matches!(action, ironsmith::LegalAction::ActivateAbility { source, .. } if source == storm)
            })
    };
    assert!(
        can_activate(&mut game, p[1]),
        "fixture: an in-range opponent may activate the any-player ability"
    );
    assert!(
        !can_activate(&mut game, p[0]),
        "CR 801.6: a player can't activate abilities of an object outside their range"
    );
}

#[test]
fn cr_801_10_801_11_effects_and_information_are_limited_to_controllers_range() {
    let (mut game, p) = p16_roi_game(6, 1);
    let creatures: Vec<_> = p
        .iter()
        .map(|player| p16_creature(&mut game, *player, "Range Creature"))
        .collect();
    let source = creatures[0];
    let mut dm = P16Dm;
    let mut ctx = EffectContext::new_default(source, p[0]).with_decision_maker(&mut dm);
    ironsmith::execute_effect(
        &mut game,
        &Effect::deal_damage(2, ChooseSpec::All(ObjectFilter::creature())),
        &mut ctx,
    )
    .expect("Pyroclasm-style effect resolves");
    for (index, creature) in creatures.iter().enumerate() {
        let in_range = matches!(index, 0 | 1 | 5);
        assert_eq!(
            game.damage_on(*creature),
            if in_range { 2 } else { 0 },
            "CR 801.10: each-creature damage affects only creatures in range (seat {index})"
        );
    }

    let (mut info, q) = p16_roi_game(6, 1);
    let first = {
        let mut first = None;
        for player in &q {
            let id = p16_creature(&mut info, *player, "Counted Creature");
            first.get_or_insert(id);
        }
        first.unwrap()
    };
    let mut dm = P16Dm;
    let mut ctx = EffectContext::new_default(first, q[0]).with_decision_maker(&mut dm);
    ironsmith::execute_effect(
        &mut info,
        &Effect::gain_life(Value::Count(ObjectFilter::creature())),
        &mut ctx,
    )
    .expect("count effect resolves");
    assert_eq!(
        info.player(q[0]).unwrap().life,
        23,
        "CR 801.11: the count sees only the three creatures within the controller's range"
    );
}

#[test]
fn cr_801_13a_redirect_to_out_of_range_player_does_nothing_for_that_portion() {
    use ironsmith::effects::{RedirectNextDamageDestination, RedirectNextDamageToTargetEffect};
    let (mut game, p) = p16_roi_game(5, 1);
    let alice_source = p16_creature(&mut game, p[0], "Lava Axe Source");
    let bob_source = p16_creature(&mut game, p[1], "Captain's Maneuver Source");
    let redirect = RedirectNextDamageToTargetEffect {
        amount: Value::Fixed(3),
        protected_target: Some(ChooseSpec::SpecificPlayer(p[1])),
        destination: RedirectNextDamageDestination::TargetObject,
        destination_target: Some(ChooseSpec::SpecificPlayer(p[2])),
        source_of_your_choice: false,
        protect_you_and_permanents: None,
    };
    {
        let mut dm = P16Dm;
        let mut ctx = EffectContext::new_default(bob_source, p[1]).with_decision_maker(&mut dm);
        ironsmith::execute_effect(&mut game, &Effect::new(redirect), &mut ctx)
            .expect("redirect shield created");
    }
    let mut dm = P16Dm;
    let mut ctx = EffectContext::new_default(alice_source, p[0]).with_decision_maker(&mut dm);
    ironsmith::execute_effect(
        &mut game,
        &Effect::deal_damage(5, ChooseSpec::SpecificPlayer(p[1])),
        &mut ctx,
    )
    .expect("damage resolves");
    assert_eq!(
        (game.player(p[1]).unwrap().life, game.player(p[2]).unwrap().life),
        (18, 20),
        "CR 801.13a: Rob takes 2 and out-of-range Carissa takes no damage"
    );
}

#[test]
fn cr_801_16_mandatory_loop_draws_loop_controllers_and_players_in_their_range() {
    use ironsmith::decision::GameProgress;
    let (mut game, p) = p16_roi_game(5, 1);
    game.turn.active_player = p[0];
    game.turn.phase = Phase::FirstMain;
    game.turn.priority_player = Some(p[0]);
    let def = CardDefinitionBuilder::new(CardId::new(), "Mandatory Life Loop")
        .card_types(vec![CardType::Enchantment])
        .with_ability(Ability::triggered(
            Trigger::you_gain_life(),
            vec![Effect::gain_life(1)],
        ))
        .build();
    let source = game.create_object_from_definition(&def, p[0], Zone::Battlefield);
    game.stack.push(ironsmith::StackEntry::ability(
        source,
        p[0],
        vec![Effect::gain_life(1)],
    ));
    let mut queue = TriggerQueue::new();
    let mut state = ironsmith::PriorityLoopState::new(game.players_in_game());
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    let mut terminal = None;
    for _ in 0..256 {
        if !game.player(p[0]).unwrap().is_in_game() {
            break;
        }
        let progress = ironsmith::game_loop::apply_priority_response_with_dm(
            &mut game,
            &mut queue,
            &mut state,
            &ironsmith::PriorityResponse::PriorityAction(ironsmith::LegalAction::PassPriority),
            &mut dm,
        )
        .expect("priority loop step");
        if let GameProgress::GameOver(result) = progress {
            terminal = Some(result);
            break;
        }
    }
    let left: Vec<_> = p
        .iter()
        .map(|player| game.player(*player).unwrap().has_left_game)
        .collect();
    assert_eq!(
        left,
        vec![true, true, false, false, true],
        "CR 801.16: only the loop controller and players in its range draw and leave (terminal={terminal:?})"
    );
    assert!(
        p.iter().all(|player| !game.player(*player).unwrap().has_lost),
        "CR 801.16: departing players draw rather than lose"
    );
}

#[test]
fn cr_801_17_restart_involves_players_outside_range() {
    let (mut game, p) = p16_roi_game(5, 1);
    game.player_mut(p[2]).unwrap().life = 3;
    game.restart_game(p[0], &[]);
    assert!(
        p.iter().all(|player| game.player(*player).unwrap().is_in_game()),
        "CR 801.17: every player is involved in the restarted game"
    );
    assert_eq!(
        game.player(p[2]).unwrap().life,
        20,
        "CR 801.17: the out-of-range player's game state is restarted too"
    );
}

// ---------------------------------------------------------------------------
// CR 802: attack multiple players
// ---------------------------------------------------------------------------

#[test]
fn cr_802_1_2_3_attacks_may_target_different_or_single_defending_players() {
    let (mut game, p) = p16_game(4);
    game.turn.active_player = p[0];
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(ironsmith::Step::DeclareAttackers);
    let a1 = p16_creature(&mut game, p[0], "Attacker One");
    let a2 = p16_creature(&mut game, p[0], "Attacker Two");
    let walker = CardDefinitionBuilder::new(CardId::new(), "Diana Walker")
        .card_types(vec![CardType::Planeswalker])
        .loyalty(4)
        .build();
    let walker = game.create_object_from_definition(&walker, p[3], Zone::Battlefield);
    let a3 = p16_creature(&mut game, p[0], "Attacker Three");
    let mut combat = CombatState::default();
    ironsmith::game_loop::apply_attacker_declarations(
        &mut game,
        &mut combat,
        &mut TriggerQueue::new(),
        &[
            AttackerDeclaration { creature: a1, target: AttackTarget::Player(p[1]) },
            AttackerDeclaration { creature: a2, target: AttackTarget::Player(p[2]) },
            AttackerDeclaration { creature: a3, target: AttackTarget::Planeswalker(walker) },
        ],
    )
    .expect("CR 802.2/802.3: each attacker chooses its own defending player or planeswalker");
    assert_eq!(
        ironsmith::combat_state::get_attack_target(&combat, a2),
        Some(&AttackTarget::Player(p[2]))
    );

    let (mut single, q) = p16_game(4);
    single.turn.active_player = q[0];
    single.turn.phase = Phase::Combat;
    single.turn.step = Some(ironsmith::Step::DeclareAttackers);
    let b1 = p16_creature(&mut single, q[0], "Single One");
    let b2 = p16_creature(&mut single, q[0], "Single Two");
    let mut combat = CombatState::default();
    ironsmith::game_loop::apply_attacker_declarations(
        &mut single,
        &mut combat,
        &mut TriggerQueue::new(),
        &[
            AttackerDeclaration { creature: b1, target: AttackTarget::Player(q[2]) },
            AttackerDeclaration { creature: b2, target: AttackTarget::Player(q[2]) },
        ],
    )
    .expect("CR 802.1: the attacker may attack only one player");
}

#[test]
fn cr_802_3a_group_restrictions_use_all_attackers_but_player_restrictions_only_that_player() {
    let construct = p16_compile(
        "Bonded Construct",
        "Mana cost: {1}\nType: Artifact Creature — Construct\nPower/Toughness: 2/1\nThis creature can't attack alone.",
    );
    let archon = p16_compile(
        "Blazing Archon",
        "Mana cost: {6}{W}{W}{W}\nType: Creature — Archon\nPower/Toughness: 5/6\nFlying\nCreatures can't attack you.",
    );
    let setup = || {
        let (mut game, p) = p16_game(3);
        game.turn.active_player = p[0];
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(ironsmith::Step::DeclareAttackers);
        let alone = game.create_object_from_definition(&construct, p[0], Zone::Battlefield);
        game.remove_summoning_sickness(alone);
        let partner = p16_creature(&mut game, p[0], "Partner");
        game.create_object_from_definition(&archon, p[2], Zone::Battlefield);
        game.refresh_continuous_state().expect("refresh");
        (game, p, alone, partner)
    };

    let (mut game, p, alone, partner) = setup();
    let ok = ironsmith::game_loop::apply_attacker_declarations(
        &mut game,
        &mut CombatState::default(),
        &mut TriggerQueue::new(),
        &[
            AttackerDeclaration { creature: alone, target: AttackTarget::Player(p[1]) },
            AttackerDeclaration { creature: partner, target: AttackTarget::Player(p[1]) },
        ],
    );
    assert!(ok.is_ok(), "fixture: two attackers satisfy 'can't attack alone': {ok:?}");

    let (mut game, p, alone, partner) = setup();
    let archon_blocks = ironsmith::game_loop::apply_attacker_declarations(
        &mut game,
        &mut CombatState::default(),
        &mut TriggerQueue::new(),
        &[
            AttackerDeclaration { creature: alone, target: AttackTarget::Player(p[1]) },
            AttackerDeclaration { creature: partner, target: AttackTarget::Player(p[2]) },
        ],
    );
    assert!(
        archon_blocks.is_err(),
        "CR 802.3a: 'Creatures can't attack you' applies to the creature attacking that player"
    );

    let (mut game, p, alone, _partner) = setup();
    let lone = ironsmith::game_loop::apply_attacker_declarations(
        &mut game,
        &mut CombatState::default(),
        &mut TriggerQueue::new(),
        &[AttackerDeclaration { creature: alone, target: AttackTarget::Player(p[1]) }],
    );
    assert!(lone.is_err(), "CR 802.3a: the whole group must still be legal (alone is illegal)");

    // Group restriction evaluated across defenders: the partner attacks a
    // different (fourth) player and 'can't attack alone' is still satisfied.
    let (mut game4, q) = p16_game(4);
    game4.turn.active_player = q[0];
    game4.turn.phase = Phase::Combat;
    game4.turn.step = Some(ironsmith::Step::DeclareAttackers);
    let alone4 = game4.create_object_from_definition(&construct, q[0], Zone::Battlefield);
    game4.remove_summoning_sickness(alone4);
    let partner4 = p16_creature(&mut game4, q[0], "Partner");
    game4.refresh_continuous_state().expect("refresh");
    let split = ironsmith::game_loop::apply_attacker_declarations(
        &mut game4,
        &mut CombatState::default(),
        &mut TriggerQueue::new(),
        &[
            AttackerDeclaration { creature: alone4, target: AttackTarget::Player(q[1]) },
            AttackerDeclaration { creature: partner4, target: AttackTarget::Player(q[3]) },
        ],
    );
    assert!(
        split.is_ok(),
        "CR 802.3a: 'can't attack alone' is judged on the entire group of attackers: {split:?}"
    );
}

#[test]
fn cr_802_3b_a_band_cannot_attack_different_players() {
    let (mut game, p) = p16_game(3);
    let bander = p16_creature_with(
        &mut game,
        p[0],
        "Bander",
        vec![Ability::static_ability(StaticAbility::banding())],
    );
    let companion = p16_creature(&mut game, p[0], "Companion");
    let mut split = CombatState {
        attackers: vec![
            ironsmith::combat_state::AttackerInfo { creature: bander, target: AttackTarget::Player(p[1]) },
            ironsmith::combat_state::AttackerInfo { creature: companion, target: AttackTarget::Player(p[2]) },
        ],
        ..Default::default()
    };
    assert!(
        ironsmith::combat_state::set_attacking_band(&game, &mut split, vec![bander, companion])
            .is_err(),
        "CR 802.3b: creatures in a band can't attack different players"
    );
    let mut same = CombatState {
        attackers: vec![
            ironsmith::combat_state::AttackerInfo { creature: bander, target: AttackTarget::Player(p[1]) },
            ironsmith::combat_state::AttackerInfo { creature: companion, target: AttackTarget::Player(p[1]) },
        ],
        ..Default::default()
    };
    ironsmith::combat_state::set_attacking_band(&game, &mut same, vec![bander, companion])
        .expect("control: a band attacking one player is legal");
}

fn p16_three_way_combat() -> (GameState, Vec<PlayerId>, CombatState, ObjectId, ObjectId) {
    let (mut game, p) = p16_game(3);
    game.turn.active_player = p[0];
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(ironsmith::Step::DeclareAttackers);
    let at_bob = p16_creature(&mut game, p[0], "Attacks Bob");
    let at_charlie = p16_creature(&mut game, p[0], "Attacks Charlie");
    let mut combat = CombatState::default();
    ironsmith::game_loop::apply_attacker_declarations(
        &mut game,
        &mut combat,
        &mut TriggerQueue::new(),
        &[
            AttackerDeclaration { creature: at_bob, target: AttackTarget::Player(p[1]) },
            AttackerDeclaration { creature: at_charlie, target: AttackTarget::Player(p[2]) },
        ],
    )
    .expect("attacks declared");
    game.turn.step = Some(ironsmith::Step::DeclareBlockers);
    (game, p, combat, at_bob, at_charlie)
}

#[test]
fn cr_802_4a_defenders_block_only_attacks_on_them_with_their_own_creatures() {
    let (mut game, p, combat, at_bob, at_charlie) = p16_three_way_combat();
    let charlie_blocker = p16_creature(&mut game, p[2], "Charlie Blocker");
    let bob_blocker = p16_creature(&mut game, p[1], "Bob Blocker");

    let mut c1 = combat.clone();
    assert!(
        ironsmith::game_loop::apply_blocker_declarations(
            &mut game.clone(),
            &mut c1,
            &mut TriggerQueue::new(),
            &[BlockerDeclaration { blocker: charlie_blocker, blocking: at_bob }],
            p[2],
        )
        .is_err(),
        "CR 802.4a: Charlie can't block a creature attacking Bob"
    );
    let mut c2 = combat.clone();
    assert!(
        ironsmith::game_loop::apply_blocker_declarations(
            &mut game.clone(),
            &mut c2,
            &mut TriggerQueue::new(),
            &[BlockerDeclaration { blocker: charlie_blocker, blocking: at_bob }],
            p[1],
        )
        .is_err(),
        "CR 802.4a: Bob can't block with Charlie's creature"
    );
    let mut c3 = combat.clone();
    ironsmith::game_loop::apply_blocker_declarations(
        &mut game,
        &mut c3,
        &mut TriggerQueue::new(),
        &[BlockerDeclaration { blocker: bob_blocker, blocking: at_bob }],
        p[1],
    )
    .expect("control: Bob blocks the creature attacking him");
    let _ = at_charlie;
}

#[test]
fn cr_802_4b_blocking_legality_ignores_other_defenders_blockers() {
    let flunkies = p16_compile(
        "Mogg Flunkies",
        "Mana cost: {1}{R}\nType: Creature — Goblin\nPower/Toughness: 3/3\nThis creature can't attack or block alone.",
    );
    let (mut game, p, mut combat, at_bob, at_charlie) = p16_three_way_combat();
    let charlie_blocker = p16_creature(&mut game, p[2], "Charlie Blocker");
    let flunkies = game.create_object_from_definition(&flunkies, p[1], Zone::Battlefield);
    game.refresh_continuous_state().expect("refresh");
    ironsmith::game_loop::apply_blocker_declarations(
        &mut game,
        &mut combat,
        &mut TriggerQueue::new(),
        &[BlockerDeclaration { blocker: charlie_blocker, blocking: at_charlie }],
        p[2],
    )
    .expect("Charlie blocks");
    let result = ironsmith::game_loop::apply_blocker_declarations(
        &mut game,
        &mut combat,
        &mut TriggerQueue::new(),
        &[BlockerDeclaration { blocker: flunkies, blocking: at_bob }],
        p[1],
    );
    assert!(
        result.is_err(),
        "CR 802.4b: Charlie's blocker doesn't let Bob's Flunkies block 'not alone'"
    );
}

#[test]
fn cr_802_5_combat_damage_assignments_follow_apnap_order() {
    let (mut game, p) = p16_game(3);
    game.turn.active_player = p[0];
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(ironsmith::Step::DeclareAttackers);
    let multi = || vec![Ability::static_ability(StaticAbility::can_block_any_number())];
    // Charlie's multi-blocker is created first so object-id order would put it first.
    let c1 = p16_creature_with(&mut game, p[2], "Charlie Multi", multi());
    let b1 = p16_creature_with(&mut game, p[1], "Bob Multi", multi());
    let b2 = p16_creature(&mut game, p[1], "Bob Plain");
    let a: Vec<_> = (0..4).map(|i| p16_creature(&mut game, p[0], &format!("Attacker {i}"))).collect();
    let mut combat = CombatState::default();
    ironsmith::game_loop::apply_attacker_declarations(
        &mut game,
        &mut combat,
        &mut TriggerQueue::new(),
        &[
            AttackerDeclaration { creature: a[2], target: AttackTarget::Player(p[2]) },
            AttackerDeclaration { creature: a[3], target: AttackTarget::Player(p[2]) },
            AttackerDeclaration { creature: a[0], target: AttackTarget::Player(p[1]) },
            AttackerDeclaration { creature: a[1], target: AttackTarget::Player(p[1]) },
        ],
    )
    .expect("attacks");
    game.turn.step = Some(ironsmith::Step::DeclareBlockers);
    ironsmith::game_loop::apply_blocker_declarations(
        &mut game,
        &mut combat,
        &mut TriggerQueue::new(),
        &[
            BlockerDeclaration { blocker: b1, blocking: a[0] },
            BlockerDeclaration { blocker: b1, blocking: a[1] },
            BlockerDeclaration { blocker: b2, blocking: a[0] },
        ],
        p[1],
    )
    .expect("Bob blocks");
    ironsmith::game_loop::apply_blocker_declarations(
        &mut game,
        &mut combat,
        &mut TriggerQueue::new(),
        &[
            BlockerDeclaration { blocker: c1, blocking: a[2] },
            BlockerDeclaration { blocker: c1, blocking: a[3] },
        ],
        p[2],
    )
    .expect("Charlie blocks");
    game.turn.step = Some(ironsmith::Step::CombatDamage);
    let mut order = Vec::new();
    for _ in 0..8 {
        let Some(prompt) =
            ironsmith::game_loop::next_combat_damage_assignment_prompt(&game, &combat, false, None)
        else {
            break;
        };
        order.push((prompt.player, prompt.source));
        game.turn_store
            .combat_damage_assignments
            .insert(prompt.source, std::collections::HashMap::new());
    }
    let players: Vec<_> = order.iter().map(|(player, _)| *player).collect();
    assert_eq!(
        players,
        vec![p[0], p[1], p[2]],
        "CR 802.5: combat damage is assigned in APNAP order: {order:?}"
    );
}

// ---------------------------------------------------------------------------
// CR 805: shared team turns
// ---------------------------------------------------------------------------

fn p16_shared(game: &mut GameState, p: &[PlayerId]) {
    game.set_teams(vec![vec![p[0], p[1]], vec![p[2], p[3]]])
        .expect("teams");
    game.enable_shared_team_turns().expect("shared turns");
}

#[test]
fn cr_805_3a_3b_team_pregame_order_puts_starting_team_first_then_turn_order() {
    let (mut game, p) = p16_game(6);
    game.set_teams(vec![vec![p[0], p[1]], vec![p[2], p[3]], vec![p[4], p[5]]])
        .expect("teams");
    game.enable_shared_team_turns().expect("shared turns");
    game.set_shared_team_member_order(0, vec![p[0], p[1]]).unwrap();
    game.set_shared_team_member_order(1, vec![p[3], p[2]]).unwrap();
    let order = game.team_apnap_player_order();
    assert_eq!(
        order,
        vec![p[0], p[1], p[3], p[2], p[4], p[5]],
        "CR 805.3a/b: starting team (in its chosen order), then each other team in turn order"
    );
    game.next_turn();
    assert_eq!(
        game.team_apnap_player_order(),
        vec![p[3], p[2], p[4], p[5], p[0], p[1]],
        "CR 805.6: team APNAP order starts from the active team"
    );
}

#[test]
fn cr_805_4d_each_players_upkeep_triggers_once_per_active_teammate_when_referring_to_that_player() {
    let (mut game, p) = p16_game(4);
    p16_shared(&mut game, &p);
    let vortex = p16_compile(
        "Sulfuric Vortex",
        "Mana cost: {1}{R}{R}\nType: Enchantment\nAt the beginning of each player's upkeep, this enchantment deals 2 damage to that player.\nIf a player would gain life, that player gains no life instead.",
    );
    let plain = p16_compile(
        "Upkeep Observer",
        "Mana cost: {1}\nType: Artifact\nAt the beginning of each upkeep, you gain 1 life.",
    );
    let vortex = game.create_object_from_definition(&vortex, p[2], Zone::Battlefield);
    let plain = game.create_object_from_definition(&plain, p[2], Zone::Battlefield);
    game.refresh_continuous_state().expect("refresh");
    game.turn.phase = Phase::Beginning;
    game.turn.step = Some(ironsmith::Step::Upkeep);
    let mut queue = TriggerQueue::new();
    ironsmith::game_loop::generate_and_queue_step_triggers(&mut game, &mut queue);
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm)
        .expect("upkeep triggers stacked");
    let stacked_from = |source: ObjectId| {
        game.stack
            .iter()
            .filter(|entry| entry.source_stable_id == game.object(source).map(|o| o.stable_id))
            .count()
    };
    let vortex_count = stacked_from(vortex);
    let plain_count = stacked_from(plain);
    assert_eq!(
        (vortex_count, plain_count),
        (2, 1),
        "CR 805.4d: 'each player's upkeep ... that player' triggers once per active teammate; a plain 'each upkeep' trigger once"
    );
}

#[test]
fn cr_805_6_simultaneous_draws_follow_team_apnap_and_chosen_member_order() {
    use ironsmith::effects::{EffectExecutor, ForPlayersEffect};
    use ironsmith::events::CardsDrawnEvent;
    let (mut game, p) = p16_game(4);
    p16_shared(&mut game, &p);
    game.set_shared_team_member_order(0, vec![p[1], p[0]]).unwrap();
    game.set_shared_team_member_order(1, vec![p[3], p[2]]).unwrap();
    let source = p16_creature(&mut game, p[0], "Draw Source");
    for player in &p {
        for _ in 0..2 {
            p16_library_card(&mut game, *player);
        }
    }
    let outcome = ForPlayersEffect::new(
        PlayerFilter::Any,
        vec![Effect::target_draws(2, PlayerFilter::IteratedPlayer)],
    )
    .execute(&mut game, &mut EffectContext::new_default(source, p[0]))
    .expect("each player draws two");
    let order: Vec<_> = outcome
        .events_of_type::<CardsDrawnEvent>()
        .flat_map(|e| std::iter::repeat_n(e.player, e.amount() as usize))
        .collect();
    assert_eq!(
        order,
        vec![p[1], p[1], p[0], p[0], p[3], p[3], p[2], p[2]],
        "CR 805.6: active team members act in their chosen order, then nonactive team members"
    );
}

#[test]
fn cr_805_7_active_team_puts_all_its_triggers_on_the_stack_before_nonactive_team() {
    use ironsmith::events::BeginningOfUpkeepEvent;
    let (mut game, p) = p16_game(4);
    p16_shared(&mut game, &p);
    game.next_turn();
    assert!(game.is_active_player(p[2]) && game.is_active_player(p[3]));
    let sources: Vec<_> = p
        .iter()
        .map(|player| {
            let def = CardDefinitionBuilder::new(CardId::new(), "Trigger Source")
                .card_types(vec![CardType::Enchantment])
                .build();
            game.create_object_from_definition(&def, *player, Zone::Battlefield)
        })
        .collect();
    let mut queue = TriggerQueue::new();
    for index in [0usize, 1, 2, 3] {
        queue.add(p16_trigger_entry(
            &game,
            sources[index],
            p[index],
            TriggerEvent::new_with_provenance(
                BeginningOfUpkeepEvent::new(p[3]),
                ironsmith::ProvNodeId::default(),
            ),
        ));
    }
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm)
        .expect("triggers stacked");
    let controllers: Vec<_> = game.stack.iter().map(|entry| entry.controller).collect();
    assert_eq!(controllers.len(), 4);
    let first_two: std::collections::HashSet<_> = controllers[..2].iter().copied().collect();
    assert_eq!(
        first_two,
        std::collections::HashSet::from([p[2], p[3]]),
        "CR 805.7: the active team's triggers go on the stack first: {controllers:?}"
    );
}

#[test]
fn cr_805_10c_blockers_attacking_player_is_controller_of_the_blocked_attacker() {
    let (mut game, p) = p16_game(4);
    p16_shared(&mut game, &p);
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(ironsmith::Step::DeclareAttackers);
    let alice_attacker = p16_creature(&mut game, p[0], "Alice Attacker");
    let bob_attacker = p16_creature(&mut game, p[1], "Bob Attacker");
    let blocker = p16_creature_with(
        &mut game,
        p[2],
        "Punishing Blocker",
        vec![Ability::triggered(
            Trigger::this_blocks(),
            vec![Effect::lose_life_player(2, PlayerFilter::Attacking)],
        )],
    );
    let mut combat = CombatState::default();
    let mut queue = TriggerQueue::new();
    ironsmith::game_loop::apply_attacker_declarations(
        &mut game,
        &mut combat,
        &mut queue,
        &[
            AttackerDeclaration { creature: alice_attacker, target: AttackTarget::Player(p[2]) },
            AttackerDeclaration { creature: bob_attacker, target: AttackTarget::Player(p[3]) },
        ],
    )
    .expect("team attacks");
    game.turn.step = Some(ironsmith::Step::DeclareBlockers);
    ironsmith::game_loop::apply_blocker_declarations(
        &mut game,
        &mut combat,
        &mut queue,
        &[BlockerDeclaration { blocker, blocking: alice_attacker }],
        p[2],
    )
    .expect("block declared");
    game.combat = Some(combat.clone());
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm)
        .expect("block trigger stacked");
    while !game.stack.is_empty() {
        ironsmith::resolve_stack_entry(&mut game).expect("resolve");
    }
    assert_eq!(
        (game.player(p[0]).unwrap().life, game.player(p[1]).unwrap().life),
        (18, 20),
        "CR 805.10c: the blocker's 'attacking player' is the controller of the creature it blocks"
    );
}

// ---------------------------------------------------------------------------
// CR 807-811 variants
// ---------------------------------------------------------------------------

#[test]
fn cr_807_3_807_4a_4b_grand_melee_random_seats_and_one_marker_per_four_players() {
    for (count, markers) in [(10usize, 2usize), (15, 3), (16, 4)] {
        let (mut game, p) = p16_game(count);
        game.set_random_seed(807 + count as u64);
        game.enable_grand_melee().expect("Grand Melee");
        let state = game.grand_melee().unwrap();
        let mut seats = state.seats().to_vec();
        seats.sort_by_key(|player| player.0);
        assert_eq!(seats, p, "CR 807.3: seats are a permutation of the players");
        assert_eq!(
            state.marker_count(),
            markers,
            "CR 807.4a: one turn marker per full four players ({count} players)"
        );
        let views = game.grand_melee_marker_views();
        let seat_list = game.grand_melee().unwrap().seats().to_vec();
        for (index, view) in views.iter().enumerate() {
            assert_eq!(view.number as usize, index + 1);
            assert_eq!(
                view.holder,
                seat_list[index * 4],
                "CR 807.4b: marker n goes to the player 4(n-1) seats left of the starting player"
            );
        }
    }
}

#[test]
fn cr_807_4c_waiting_marker_moves_left_immediately_when_its_holder_leaves() {
    let (mut game, p) = p16_game(9);
    game.restore_grand_melee(p.clone()).expect("Grand Melee");
    game.next_turn();
    let marker_one = game
        .grand_melee_marker_views()
        .into_iter()
        .find(|view| view.number == 1)
        .unwrap();
    assert_eq!(marker_one.holder, p[1]);
    assert_eq!(marker_one.status, ironsmith::GrandMeleeMarkerStatus::Waiting);
    assert!(game.leave_game(p[1]).expect("departure fixture"));
    assert_eq!(game.grand_melee().unwrap().marker_count(), 2);
    let marker_one = game
        .grand_melee_marker_views()
        .into_iter()
        .find(|view| view.number == 1)
        .unwrap();
    assert_eq!(
        marker_one.holder, p[2],
        "CR 807.4c: a holder leaving before their turn passes the marker left immediately"
    );
}

#[test]
fn cr_808_5_809_7_810_5_team_resources_are_individual_but_hands_reviewable() {
    let (mut tvt, p) = p16_game(4);
    tvt.restore_team_vs_team(vec![vec![p[0], p[1]], vec![p[2], p[3]]], p.clone(), 0, p[0])
        .expect("Team vs. Team");
    let (mut emp, q) = p16_game(6);
    emp.restore_emperor(
        vec![q[0..3].to_vec(), q[3..6].to_vec()],
        q.clone(),
        0,
        q[1],
        vec![1, 2, 1, 1, 2, 1],
    )
    .expect("Emperor");
    let (mut thg, r) = p16_game(4);
    thg.set_random_seed(5);
    thg.enable_two_headed_giant(vec![vec![r[0], r[1]], vec![r[2], r[3]]])
        .expect("2HG");
    for (label, game, a, b, foe) in [
        ("808.5", &mut tvt, p[0], p[1], p[2]),
        ("809.7", &mut emp, q[0], q[1], q[3]),
        ("810.5", &mut thg, r[0], r[1], r[2]),
    ] {
        game.player_mut(a).unwrap().mana_pool.add(ironsmith::ManaSymbol::Colorless, 2);
        let hand_card = p16_library_card(game, a);
        let _ = game.move_object_by_effect(hand_card, Zone::Hand);
        assert_eq!(game.player(b).unwrap().mana_pool.total(), 0, "CR {label}: mana isn't shared");
        assert!(game.player(b).unwrap().hand.is_empty(), "CR {label}: hands aren't shared");
        assert!(
            game.can_review_teammate_hand(b, a),
            "CR {label}: teammates may review each other's hands"
        );
        assert!(!game.can_review_teammate_hand(foe, a), "CR {label}: opponents may not");
        assert!(game.are_teammates(a, b) && game.are_opponents(a, foe));
    }
}

#[test]
fn cr_809_6a_larger_emperor_teams_use_minimum_ranges() {
    let (mut game, p) = p16_game(8);
    game.set_random_seed(96);
    game.enable_emperor(vec![p[0..4].to_vec(), p[4..8].to_vec()])
        .expect("Emperor with four-player teams");
    let profile = game.emperor().unwrap();
    let ranges: Vec<_> = profile.seats().iter().map(|player| {
        game.limited_range_of_influence().unwrap().configured_range(*player).unwrap()
    }).collect();
    assert_eq!(
        ranges,
        vec![1, 3, 2, 1, 1, 3, 2, 1],
        "CR 809.6a: general 1/emperor/general 2/general 3 ranges are 1/3/2/1"
    );
}

fn p16_2hg() -> (GameState, Vec<PlayerId>) {
    let (mut game, p) = p16_game(4);
    game.set_random_seed(810);
    game.enable_two_headed_giant(vec![vec![p[0], p[1]], vec![p[2], p[3]]])
        .expect("2HG");
    (game, p)
}

#[test]
fn cr_810_8c_team_with_zero_life_loses_as_a_team() {
    let (mut game, p) = p16_2hg();
    game.lose_life(p[0], 30);
    assert_eq!(game.player(p[1]).unwrap().life, 0);
    ironsmith::apply_state_based_actions(&mut game).expect("SBA");
    assert!(
        game.player(p[0]).unwrap().has_lost && game.player(p[1]).unwrap().has_lost,
        "CR 810.8c: a team at 0 life loses the game"
    );
    assert!(game.player(p[2]).unwrap().is_in_game() && game.player(p[3]).unwrap().is_in_game());
}

#[test]
fn cr_810_9a_individual_life_values_use_the_team_life_total() {
    let (mut game, p) = p16_2hg();
    game.lose_life(p[1], 13);
    assert_eq!(game.player(p[0]).unwrap().life, 17);
    let source = p16_creature(&mut game, p[0], "Beacon Source");
    let mut dm = P16Dm;
    let mut ctx = EffectContext::new_default(source, p[0]).with_decision_maker(&mut dm);
    ironsmith::execute_effect(
        &mut game,
        &Effect::gain_life_player(
            Value::LifeTotal(PlayerFilter::Specific(p[0])),
            ChooseSpec::Player(PlayerFilter::Specific(p[0])),
        ),
        &mut ctx,
    )
    .expect("double life resolves");
    assert_eq!(
        (game.player(p[0]).unwrap().life, game.player(p[1]).unwrap().life),
        (34, 34),
        "CR 810.9a: doubling a head's life at team 17 gains 17, leaving the team at 34"
    );

    let (mut half, q) = p16_2hg();
    half.lose_life(q[0], 19);
    assert_eq!(half.player(q[1]).unwrap().life, 11);
    let source = p16_creature(&mut half, q[0], "Lurking Evil Source");
    let mut dm = P16Dm;
    let mut ctx = EffectContext::new_default(source, q[0]).with_decision_maker(&mut dm);
    ironsmith::execute_effect(
        &mut half,
        &Effect::lose_life_player(
            Value::HalfLifeTotalRoundedUp(PlayerFilter::Specific(q[0])),
            PlayerFilter::Specific(q[0]),
        ),
        &mut ctx,
    )
    .expect("half life resolves");
    assert_eq!(
        half.player(q[1]).unwrap().life,
        5,
        "CR 810.9a: half of team life 11 rounded up is 6, leaving the team at 5"
    );
}

#[test]
fn cr_810_9c_setting_one_heads_life_adjusts_the_team_by_the_difference() {
    use ironsmith::effects::SetLifeTotalEffect;
    let (mut game, p) = p16_2hg();
    game.lose_life(p[1], 5);
    let source = p16_creature(&mut game, p[2], "Life Setter");
    let mut dm = P16Dm;
    let mut ctx = EffectContext::new_default(source, p[2]).with_decision_maker(&mut dm);
    ironsmith::execute_effect(
        &mut game,
        &Effect::new(SetLifeTotalEffect::new(10, PlayerFilter::Specific(p[0]))),
        &mut ctx,
    )
    .expect("set life resolves");
    assert_eq!(
        (game.player(p[0]).unwrap().life, game.player(p[1]).unwrap().life),
        (10, 10),
        "CR 810.9c: a head at team 25 loses 15 and the team ends at 10"
    );
}

#[test]
fn cr_810_9g_9h_life_gain_and_loss_prohibitions_extend_to_the_team() {
    let cant_gain = p16_compile(
        "No Gain",
        "Mana cost: {1}\nType: Enchantment\nYou can't gain life.",
    );
    let (mut game, p) = p16_2hg();
    game.create_object_from_definition(&cant_gain, p[0], Zone::Battlefield);
    game.refresh_continuous_state().expect("refresh");
    game.gain_life(p[1], 5);
    assert_eq!(
        game.player(p[1]).unwrap().life,
        30,
        "CR 810.9g: if one head can't gain life, no player on that team can"
    );
    assert!(!game.can_gain_life(p[1]));
    assert!(game.can_gain_life(p[2]));

    let cant_lose = p16_compile(
        "No Loss",
        "Mana cost: {1}\nType: Enchantment\nYou can't lose life.",
    );
    let (mut game, p) = p16_2hg();
    game.create_object_from_definition(&cant_lose, p[0], Zone::Battlefield);
    game.refresh_continuous_state().expect("refresh");
    game.lose_life(p[1], 5);
    assert_eq!(
        game.player(p[1]).unwrap().life,
        30,
        "CR 810.9h: if one head can't lose life, no player on that team can"
    );
    assert!(
        !game.pay_life_simultaneously(&[(p[1], 3)]).unwrap_or(false),
        "CR 810.9h: the teammate can't pay a nonzero amount of life"
    );
}

#[test]
fn cr_810_10c_10d_poison_prohibition_and_poisoned_status_are_team_wide() {
    let melira = p16_compile(
        "Melira, Sylvok Outcast",
        "Mana cost: {1}{G}\nType: Legendary Creature — Human Scout\nPower/Toughness: 2/2\nYou can't get poison counters.\nCreatures you control can't have -1/-1 counters put on them.\nCreatures your opponents control lose infect.",
    );
    let (mut game, p) = p16_2hg();
    game.create_object_from_definition(&melira, p[0], Zone::Battlefield);
    game.refresh_continuous_state().expect("refresh");
    let _ = game.add_player_counters_with_source(p[1], ironsmith::CounterType::Poison, 3, None, None);
    assert_eq!(
        game.player(p[1]).unwrap().poison_counters,
        0,
        "CR 810.10c: if one head can't get poison counters, no teammate can"
    );

    let throatseeker = p16_compile(
        "Chained Throatseeker",
        "Mana cost: {5}{U}\nType: Creature — Phyrexian Horror\nPower/Toughness: 5/5\nInfect\nThis creature can't attack unless defending player is poisoned.",
    );
    let (mut game, p) = p16_2hg();
    let starting = game.two_headed_giant().unwrap().starting_team();
    let teams = game.two_headed_giant().unwrap().teams().to_vec();
    let attacker_player = teams[starting][0];
    let foes = teams[1 - starting].clone();
    let seeker = game.create_object_from_definition(&throatseeker, attacker_player, Zone::Battlefield);
    game.remove_summoning_sickness(seeker);
    game.refresh_continuous_state().expect("refresh");
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(ironsmith::Step::DeclareAttackers);
    assert!(
        p16_attack_options(&game, seeker).is_none_or(|t| !t.contains(&AttackTarget::Player(foes[1]))),
        "fixture: unpoisoned team can't be attacked"
    );
    game.add_player_counters_with_source(foes[0], ironsmith::CounterType::Poison, 1, None, None)
        .expect("poison");
    let targets = p16_attack_options(&game, seeker).unwrap_or_default();
    assert!(
        targets.contains(&AttackTarget::Player(foes[1])),
        "CR 810.10d: a head is poisoned when its team has a poison counter: {targets:?}"
    );
    let _ = p;
}
