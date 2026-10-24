//! CR audit (pending-entry pass, package p06): assertions express the rules, so confirmed defects fail.
//! Do not use `should_panic`, `ignore`, or assertions of current broken behavior.

#![allow(unused_imports, dead_code)]

use ironsmith::{EffectContext, GameState, PlayerId};

use ironsmith::{DecisionMaker, EffectExecutor, PlayerFilter, SelectOptionsContext};

fn p06_compile(name: &str, text: &str) -> ironsmith::cards::CardDefinition {
    ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false)
        .unwrap_or_else(|err| panic!("fixture compile failed for {name}: {err:?}"))
}

fn p06_two_player() -> (GameState, PlayerId, PlayerId) {
    let game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    (game, PlayerId::from_index(0), PlayerId::from_index(1))
}

fn p06_bear(game: &mut GameState, controller: PlayerId, name: &str) -> ironsmith::ObjectId {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::{Ability, CardId, CardType, PowerToughness, StaticAbility, Zone};
    let def = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .with_ability(Ability::static_ability(StaticAbility::haste()))
        .build();
    game.create_object_from_definition(&def, controller, Zone::Battlefield)
}

/// Place the game at a turn position. `combat_index` is the number of combat
/// phases started this turn (1 = first combat).
fn p06_set_position(
    game: &mut GameState,
    active: PlayerId,
    phase: ironsmith::Phase,
    step: Option<ironsmith::Step>,
    combat_index: u32,
) {
    game.turn.active_player = active;
    game.turn.priority_player = Some(active);
    game.turn.phase = phase;
    game.turn.step = step;
    game.turn_store.combat_phases_started_this_turn = combat_index;
}

fn p06_can_cast(game: &GameState, player: PlayerId, id: ironsmith::ObjectId) -> bool {
    use ironsmith::LegalAction;
    ironsmith::compute_legal_actions(game, player)
        .unwrap()
        .iter()
        .any(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == id))
}

fn p06_can_activate(game: &GameState, player: PlayerId, id: ironsmith::ObjectId) -> bool {
    use ironsmith::LegalAction;
    ironsmith::compute_legal_actions(game, player)
        .unwrap()
        .iter()
        .any(|a| matches!(a, LegalAction::ActivateAbility { source, .. } if *source == id))
}

#[derive(Clone, Copy, Debug)]
enum P06Pos {
    Upkeep,
    FirstMain,
    BeginCombat(u32),
    DeclareAttackers(u32),
    DeclareBlockers(u32),
    CombatDamage(u32),
    EndCombat(u32),
    NextMain,
}

fn p06_apply_pos(game: &mut GameState, active: PlayerId, pos: P06Pos, blocks_done: bool) {
    use ironsmith::{Phase, Step};
    match pos {
        P06Pos::Upkeep => p06_set_position(game, active, Phase::Beginning, Some(Step::Upkeep), 0),
        P06Pos::FirstMain => p06_set_position(game, active, Phase::FirstMain, None, 0),
        P06Pos::BeginCombat(n) => {
            p06_set_position(game, active, Phase::Combat, Some(Step::BeginCombat), n)
        }
        P06Pos::DeclareAttackers(n) => {
            p06_set_position(game, active, Phase::Combat, Some(Step::DeclareAttackers), n)
        }
        P06Pos::DeclareBlockers(n) => {
            p06_set_position(game, active, Phase::Combat, Some(Step::DeclareBlockers), n)
        }
        P06Pos::CombatDamage(n) => {
            p06_set_position(game, active, Phase::Combat, Some(Step::CombatDamage), n)
        }
        P06Pos::EndCombat(n) => {
            p06_set_position(game, active, Phase::Combat, Some(Step::EndCombat), n)
        }
        P06Pos::NextMain => p06_set_position(game, active, Phase::NextMain, None, 1),
    }
    if matches!(pos, P06Pos::FirstMain | P06Pos::Upkeep) {
        game.combat = None;
    } else {
        let mut combat = ironsmith::combat_state::new_combat();
        combat.block_declaration_complete = blocks_done;
        game.combat = Some(combat);
    }
}

/// Returns (position, castable) mismatches against `expected`.
fn p06_cast_window_mismatches(
    name: &str,
    text: &str,
    cases: &[(P06Pos, bool, bool)],
) -> Vec<String> {
    use ironsmith::Zone;
    let def = p06_compile(name, text);
    assert!(
        !ironsmith::cards::generated_definition_has_unimplemented_content(&def),
        "fixture {name} must compile fully"
    );
    let mut mismatches = Vec::new();
    for &(pos, blocks_done, expected) in cases {
        let (mut game, alice, bob) = p06_two_player();
        p06_bear(&mut game, alice, "Friendly target");
        p06_bear(&mut game, bob, "Opposing target");
        let spell = game.create_object_from_definition(&def, alice, Zone::Hand);
        p06_apply_pos(&mut game, alice, pos, blocks_done);
        let got = p06_can_cast(&game, alice, spell);
        if got != expected {
            mismatches.push(format!(
                "{name} at {pos:?} (blocks_done={blocks_done}): castable={got}, expected={expected}"
            ));
        }
    }
    mismatches
}

const P06_MASTER_WARCRAFT: &str = "Mana cost: {0}\nType: Instant\nCast this spell only before attackers are declared.\nYou choose which creatures attack this turn.\nYou choose which creatures block this turn and how those creatures block.";
const P06_RAPID_FIRE: &str = "Mana cost: {0}\nType: Instant\nCast this spell only before blockers are declared.\nTarget creature gains first strike until end of turn. If it doesn't have rampage, that creature gains rampage 2 until end of turn.";
const P06_ALEATORY: &str = "Mana cost: {0}\nType: Instant\nCast this spell only during combat after blockers are declared.\nFlip a coin. If you win the flip, target creature gets +1/+1 until end of turn.\nDraw a card at the beginning of the next turn's upkeep.";
const P06_DURING_COMBAT_BEFORE_BLOCKERS: &str = "Mana cost: {0}\nType: Instant\nCast this spell only during combat before blockers are declared.\nTarget creature can't block this turn.\nDraw a card at the beginning of the next turn's upkeep.";

#[test]
fn cr_506_8a_before_attackers_spell_is_castable_until_declare_attackers_step_begins() {
    use P06Pos::*;
    let mismatches = p06_cast_window_mismatches(
        "Master Warcraft",
        P06_MASTER_WARCRAFT,
        &[
            (Upkeep, false, true),
            (FirstMain, false, true),
            (BeginCombat(1), false, true),
            (DeclareAttackers(1), false, false),
            (DeclareBlockers(1), true, false),
            (NextMain, false, false),
        ],
    );
    assert_eq!(
        mismatches,
        Vec::<String>::new(),
        "CR 506.8a: \"only before attackers are declared\" means before the declare attackers step begins (beginning phase and precombat main included)"
    );
}

#[test]
fn cr_506_8b_before_blockers_spell_windows_in_the_first_combat() {
    use P06Pos::*;
    let mismatches = p06_cast_window_mismatches(
        "Rapid Fire",
        P06_RAPID_FIRE,
        &[
            (Upkeep, false, true),
            (FirstMain, false, true),
            (BeginCombat(1), false, true),
            (DeclareAttackers(1), false, true),
            (DeclareBlockers(1), true, false),
            (CombatDamage(1), true, false),
            (EndCombat(1), true, false),
            (NextMain, false, false),
        ],
    );
    assert_eq!(
        mismatches,
        Vec::<String>::new(),
        "CR 506.8b: \"only before blockers are declared\" ends when the declare blockers step begins"
    );
}

#[test]
fn cr_506_8d_before_blockers_spell_uses_only_the_first_combat_phase() {
    use P06Pos::*;
    let mismatches = p06_cast_window_mismatches(
        "Rapid Fire",
        P06_RAPID_FIRE,
        &[
            (BeginCombat(2), false, false),
            (DeclareAttackers(2), false, false),
        ],
    );
    assert_eq!(
        mismatches,
        Vec::<String>::new(),
        "CR 506.8d: a spell castable only before blockers are declared (no \"during combat\") may be cast only before that point of the turn's FIRST combat phase"
    );
}

#[test]
fn cr_506_8c_during_combat_timing_applies_in_every_combat_phase() {
    use P06Pos::*;
    let mismatches = p06_cast_window_mismatches(
        "Panic",
        P06_DURING_COMBAT_BEFORE_BLOCKERS,
        &[
            (FirstMain, false, false),
            (BeginCombat(1), false, true),
            (DeclareAttackers(1), false, true),
            (DeclareBlockers(1), true, false),
            (BeginCombat(2), false, true),
            (DeclareAttackers(2), false, true),
            (DeclareBlockers(2), true, false),
            (NextMain, false, false),
        ],
    );
    assert_eq!(
        mismatches,
        Vec::<String>::new(),
        "CR 506.8c: \"during combat before blockers are declared\" may be cast during any of the turn's combat phases"
    );
}

#[test]
fn cr_506_8e_before_blockers_spell_with_skipped_blockers_step_ends_at_end_of_combat() {
    use P06Pos::*;
    // No attackers were declared (block declaration never completes): the
    // declare blockers and combat damage steps are skipped (CR 508.8).
    let mismatches = p06_cast_window_mismatches(
        "Rapid Fire",
        P06_RAPID_FIRE,
        &[
            (DeclareAttackers(1), false, true),
            (EndCombat(1), false, false),
        ],
    );
    assert_eq!(
        mismatches,
        Vec::<String>::new(),
        "CR 506.8e: with the declare blockers step skipped the spell may be cast only before the end of combat step"
    );
}

#[test]
fn cr_506_8f_after_blockers_spell_is_uncastable_when_blockers_step_is_skipped() {
    use P06Pos::*;
    let mismatches = p06_cast_window_mismatches(
        "Aleatory",
        P06_ALEATORY,
        &[
            (BeginCombat(1), false, false),
            (DeclareAttackers(1), false, false),
            // Blockers step skipped this combat: never castable.
            (EndCombat(1), false, false),
            // Blockers were declared this combat: castable afterwards.
            (DeclareBlockers(1), true, true),
            (CombatDamage(1), true, true),
            (EndCombat(1), true, true),
            (NextMain, false, false),
        ],
    );
    assert_eq!(
        mismatches,
        Vec::<String>::new(),
        "CR 506.8f: \"only during combat after blockers are declared\" cannot be cast in a combat whose declare blockers step was skipped"
    );
}

#[test]
fn cr_506_8g_activation_restrictions_follow_combat_timing_rules() {
    use P06Pos::*;
    use ironsmith::{ManaSymbol, Zone};
    let fixtures: [(&str, &str, Vec<(P06Pos, bool, bool)>); 3] = [
        (
            "Acidic Dagger",
            "Mana cost: {4}\nType: Artifact\n{0}, {T}: Whenever target creature deals combat damage to a non-Wall creature this turn, destroy that non-Wall creature. When the targeted creature leaves the battlefield this turn, sacrifice this artifact. Activate only before blockers are declared.",
            vec![
                (FirstMain, false, true),
                (BeginCombat(1), false, true),
                (DeclareAttackers(1), false, true),
                (DeclareBlockers(1), true, false),
                (BeginCombat(2), false, false),
                (NextMain, false, false),
            ],
        ),
        (
            "Arcum's Whistle",
            "Mana cost: {3}\nType: Artifact\n{0}, {T}: Choose target non-Wall creature the active player has controlled continuously since the beginning of the turn. That player may pay {X}, where X is that creature's mana value. If they don't pay, the creature attacks this turn if able, and at the beginning of the next end step, destroy it if it didn't attack this turn. Activate only before attackers are declared.",
            vec![
                (Upkeep, false, true),
                (FirstMain, false, true),
                (BeginCombat(1), false, true),
                (DeclareAttackers(1), false, false),
                (BeginCombat(2), false, false),
            ],
        ),
        (
            "Trap Runner",
            "Mana cost: {2}{W}{W}\nType: Creature — Human Soldier\nPower/Toughness: 2/3\n{T}: Target unblocked attacking creature becomes blocked. Activate only during combat after blockers are declared.",
            vec![
                (DeclareAttackers(1), false, false),
                (EndCombat(1), false, false),
                (DeclareBlockers(1), true, true),
                (NextMain, false, false),
            ],
        ),
    ];
    let mut mismatches = Vec::new();
    for (name, text, cases) in fixtures {
        let def = p06_compile(name, text);
        assert!(
            !ironsmith::cards::generated_definition_has_unimplemented_content(&def),
            "fixture {name} must compile fully"
        );
        for (pos, blocks_done, expected) in cases {
            let (mut game, alice, bob) = p06_two_player();
            let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
            // Targets: an unblocked attacker controlled by the active player.
            let target = p06_bear(&mut game, alice, "Target creature");
            p06_apply_pos(&mut game, alice, pos, blocks_done);
            if let Some(combat) = game.combat.as_mut() {
                if matches!(pos, DeclareAttackers(_) | DeclareBlockers(_) | CombatDamage(_) | EndCombat(_)) {
                    combat.attackers.push(ironsmith::combat_state::AttackerInfo {
                        creature: target,
                        target: ironsmith::combat_state::AttackTarget::Player(bob),
                    });
                }
            }
            let _ = ManaSymbol::Colorless;
            let got = p06_can_activate(&game, alice, source);
            if got != expected {
                mismatches.push(format!(
                    "{name} at {pos:?} (blocks_done={blocks_done}): activatable={got}, expected={expected}"
                ));
            }
        }
    }
    assert_eq!(
        mismatches,
        Vec::<String>::new(),
        "CR 506.8g: activation timing relative to combat follows rules 506.8a-f"
    );
}

// ---------------------------------------------------------------------------
// Combat-phase driver (CR 506-511)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct P06PriorityObs {
    state: String,
    step: Option<ironsmith::Step>,
    phase: ironsmith::Phase,
    priority: Option<PlayerId>,
    queued: usize,
}

#[derive(Debug, Default)]
struct P06Run {
    obs: Vec<P06PriorityObs>,
    attackers_prompted_for: Vec<PlayerId>,
    blockers_prompted_for: Vec<PlayerId>,
    stack_at_blocker_prompt: Vec<usize>,
}

/// Drive the turn runner until `stop` holds. At each priority window the
/// observation is recorded, `on_priority` runs, then queued triggers are put
/// on the stack and resolved (mimicking all players passing).
fn p06_drive(
    game: &mut GameState,
    runner: &mut ironsmith::TurnRunner,
    queue: &mut ironsmith::triggers::TriggerQueue,
    attackers: Vec<ironsmith::AttackerDeclaration>,
    blockers: Vec<ironsmith::BlockerDeclaration>,
    stop: impl Fn(&ironsmith::TurnRunnerState) -> bool,
    mut on_priority: impl FnMut(&mut GameState, &ironsmith::TurnRunnerState, usize),
) -> P06Run {
    use ironsmith::TurnAction;
    use ironsmith::decisions::context::DecisionContext;
    let mut run = P06Run::default();
    let mut attackers = Some(attackers);
    let mut blockers = Some(blockers);
    for _ in 0..120 {
        if stop(runner.state()) {
            return run;
        }
        match runner.advance(game, queue).unwrap() {
            TurnAction::Continue => {}
            TurnAction::Decision(DecisionContext::Attackers(ctx)) => {
                run.attackers_prompted_for.push(ctx.player);
                runner.respond_attackers(attackers.take().unwrap_or_default());
            }
            TurnAction::Decision(DecisionContext::Blockers(ctx)) => {
                run.blockers_prompted_for.push(ctx.player);
                run.stack_at_blocker_prompt.push(game.stack.len());
                runner.respond_blockers(blockers.take().unwrap_or_default(), ctx.player);
            }
            TurnAction::RunPriority => {
                let queued = queue.entries.len();
                run.obs.push(P06PriorityObs {
                    state: format!("{:?}", runner.state()),
                    step: game.turn.step,
                    phase: game.turn.phase,
                    priority: game.turn.priority_player,
                    queued,
                });
                let state = runner.state().clone();
                on_priority(game, &state, queued);
                ironsmith::game_loop::put_triggers_on_stack(game, queue).unwrap();
                while !game.stack.is_empty() {
                    ironsmith::game_loop::resolve_stack_entry(game).unwrap();
                }
                runner.priority_done();
            }
            other => panic!("unexpected runner action: {other:?}"),
        }
    }
    panic!("runner did not reach the stop state; at {:?}", runner.state());
}

fn p06_combat_game(players: usize) -> GameState {
    let names = ["Alice", "Bob", "Carol", "Dan"];
    let mut game = GameState::new(
        names[..players].iter().map(|n| n.to_string()).collect(),
        20,
    );
    game.turn.active_player = PlayerId::from_index(0);
    game.turn.priority_player = Some(PlayerId::from_index(0));
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    game
}

fn p06_creature_with(
    game: &mut GameState,
    controller: PlayerId,
    name: &str,
    power: i32,
    toughness: i32,
    abilities: Vec<ironsmith::Ability>,
) -> ironsmith::ObjectId {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::{Ability, CardId, CardType, PowerToughness, StaticAbility, Zone};
    let mut builder = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(power, toughness))
        .with_ability(Ability::static_ability(StaticAbility::haste()));
    for ability in abilities {
        builder = builder.with_ability(ability);
    }
    let def = builder.build();
    game.create_object_from_definition(&def, controller, Zone::Battlefield)
}

fn p06_enchantment_with(
    game: &mut GameState,
    controller: PlayerId,
    name: &str,
    abilities: Vec<ironsmith::Ability>,
) -> ironsmith::ObjectId {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::{CardId, CardType, Zone};
    let mut builder =
        CardDefinitionBuilder::new(CardId::new(), name).card_types(vec![CardType::Enchantment]);
    for ability in abilities {
        builder = builder.with_ability(ability);
    }
    let def = builder.build();
    game.create_object_from_definition(&def, controller, Zone::Battlefield)
}

fn p06_attack(
    creature: ironsmith::ObjectId,
    target: ironsmith::combat_state::AttackTarget,
) -> ironsmith::AttackerDeclaration {
    ironsmith::AttackerDeclaration { creature, target }
}

fn p06_is_attacking(game: &GameState, id: ironsmith::ObjectId) -> bool {
    game.combat
        .as_ref()
        .is_some_and(|c| ironsmith::combat_state::is_attacking(c, id))
}

#[test]
fn cr_506_2_two_player_combat_roles_attacker_targets_and_blocker_declaration() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::combat_state::AttackTarget;
    use ironsmith::{CardId, CardType, Subtype, TurnRunner, TurnRunnerState, Zone};
    let mut game = p06_combat_game(2);
    let (alice, bob) = (PlayerId::from_index(0), PlayerId::from_index(1));
    let attacker = p06_creature_with(&mut game, alice, "Alice attacker", 2, 2, vec![]);
    let bob_creature = p06_creature_with(&mut game, bob, "Bob creature", 2, 2, vec![]);
    let pw = CardDefinitionBuilder::new(CardId::new(), "Bob walker")
        .card_types(vec![CardType::Planeswalker])
        .loyalty(5)
        .build();
    let pw = game.create_object_from_definition(&pw, bob, Zone::Battlefield);
    let alice_pw = CardDefinitionBuilder::new(CardId::new(), "Alice walker")
        .card_types(vec![CardType::Planeswalker])
        .loyalty(5)
        .build();
    let alice_pw = game.create_object_from_definition(&alice_pw, alice, Zone::Battlefield);
    let battle = CardDefinitionBuilder::new(CardId::new(), "Siege Bob protects")
        .card_types(vec![CardType::Battle])
        .subtypes(vec![Subtype::Siege])
        .defense(5)
        .build();
    let battle = game.create_object_from_definition(&battle, alice, Zone::Battlefield);
    assert!(game.set_battle_protector(battle, bob));
    game.turn.phase = ironsmith::Phase::Combat;
    game.turn.step = Some(ironsmith::Step::DeclareAttackers);
    let options =
        ironsmith::compute_legal_attackers(&game, &ironsmith::combat_state::new_combat());
    let creatures: Vec<_> = options.iter().map(|o| o.creature).collect();
    assert_eq!(
        creatures,
        vec![attacker],
        "CR 506.2: only creatures the active player controls may attack"
    );
    let mut targets = options[0].valid_targets.clone();
    targets.sort_by_key(|t| format!("{t:?}"));
    let mut expected = vec![
        AttackTarget::Player(bob),
        AttackTarget::Planeswalker(pw),
        AttackTarget::Battle(battle),
    ];
    expected.sort_by_key(|t| format!("{t:?}"));
    assert_eq!(
        targets, expected,
        "CR 506.2: the defending player, their planeswalkers and battles they protect may be attacked (not Alice or her walker {alice_pw:?})"
    );

    // Full step: Bob (the nonactive player) declares blockers; no stack use.
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::BeginCombat);
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let run = p06_drive(
        &mut game,
        &mut runner,
        &mut queue,
        vec![p06_attack(attacker, AttackTarget::Player(bob))],
        vec![ironsmith::BlockerDeclaration {
            blocker: bob_creature,
            blocking: attacker,
        }],
        |s| matches!(s, TurnRunnerState::DeclareBlockersPriority),
        |_, _, _| {},
    );
    assert_eq!(run.attackers_prompted_for, vec![alice], "CR 508.1: the active player declares attackers");
    assert_eq!(run.blockers_prompted_for, vec![bob], "CR 506.2/509.1: the nonactive player is the defending player and declares blockers");
    assert_eq!(run.stack_at_blocker_prompt, vec![0], "CR 508.1/509.1: declarations do not use the stack");
    assert!(game.combat.as_ref().unwrap().blockers.get(&attacker).is_some_and(|b| b.contains(&bob_creature)));
}

#[test]
fn cr_506_2a_multiplayer_opponents_automatically_become_defending_players() {
    use ironsmith::combat_state::AttackTarget;
    let mut game = p06_combat_game(3);
    let (alice, bob, carol) = (
        PlayerId::from_index(0),
        PlayerId::from_index(1),
        PlayerId::from_index(2),
    );
    let attacker = p06_creature_with(&mut game, alice, "Alice attacker", 2, 2, vec![]);
    p06_creature_with(&mut game, bob, "Bob creature", 2, 2, vec![]);
    game.turn.phase = ironsmith::Phase::Combat;
    game.turn.step = Some(ironsmith::Step::DeclareAttackers);
    let options =
        ironsmith::compute_legal_attackers(&game, &ironsmith::combat_state::new_combat());
    assert_eq!(options.len(), 1, "CR 506.2: only the active player's creatures may attack");
    let targets = &options[0].valid_targets;
    assert!(
        targets.contains(&AttackTarget::Player(bob))
            && targets.contains(&AttackTarget::Player(carol))
            && !targets.contains(&AttackTarget::Player(alice)),
        "CR 506.2a/802.2: under the attack-multiple-players option every opponent is a defending player; got {targets:?}"
    );
    assert_eq!(attacker, options[0].creature);
}

#[test]
fn cr_507_2_508_2_510_3_511_1_active_player_gets_priority_in_every_combat_step() {
    use ironsmith::combat_state::AttackTarget;
    use ironsmith::{Step, TurnRunner, TurnRunnerState};
    let mut game = p06_combat_game(2);
    let (alice, bob) = (PlayerId::from_index(0), PlayerId::from_index(1));
    let attacker = p06_creature_with(&mut game, alice, "Attacker", 2, 2, vec![]);
    let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::BeginCombat);
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let run = p06_drive(
        &mut game,
        &mut runner,
        &mut queue,
        vec![p06_attack(attacker, AttackTarget::Player(bob))],
        vec![],
        |s| matches!(s, TurnRunnerState::NextMain | TurnRunnerState::NextMainPriority),
        |_, _, _| {},
    );
    let steps: Vec<_> = run.obs.iter().map(|o| (o.step, o.priority)).collect();
    assert_eq!(
        steps,
        vec![
            (Some(Step::BeginCombat), Some(alice)),
            (Some(Step::DeclareAttackers), Some(alice)),
            (Some(Step::DeclareBlockers), Some(alice)),
            (Some(Step::CombatDamage), Some(alice)),
            (Some(Step::EndCombat), Some(alice)),
        ],
        "CR 507.2/508.2/509.2/510.3/511.1: the active player receives priority in each combat step"
    );
    assert_eq!(game.player(bob).unwrap().life, 18);
}

#[test]
fn cr_508_2b_510_3a_triggered_abilities_are_waiting_before_priority() {
    use ironsmith::combat_state::AttackTarget;
    use ironsmith::triggers::Trigger;
    use ironsmith::{Ability, Effect, Step, TurnRunner, TurnRunnerState};
    let mut game = p06_combat_game(2);
    let (alice, bob) = (PlayerId::from_index(0), PlayerId::from_index(1));
    let attacker = p06_creature_with(
        &mut game,
        alice,
        "Triggering attacker",
        3,
        3,
        vec![
            Ability::triggered(Trigger::this_attacks(), vec![Effect::gain_life(1)]),
            Ability::triggered(Trigger::this_deals_combat_damage(), vec![Effect::gain_life(1)]),
        ],
    );
    let blocker = p06_creature_with(
        &mut game,
        bob,
        "Dying blocker",
        1,
        1,
        vec![Ability::triggered(Trigger::this_dies(), vec![Effect::gain_life(1)])],
    );
    let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::BeginCombat);
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let run = p06_drive(
        &mut game,
        &mut runner,
        &mut queue,
        vec![p06_attack(attacker, AttackTarget::Player(bob))],
        vec![ironsmith::BlockerDeclaration { blocker, blocking: attacker }],
        |s| matches!(s, TurnRunnerState::EndCombatPriority),
        |_, _, _| {},
    );
    let queued: Vec<_> = run.obs.iter().map(|o| (o.step, o.queued)).collect();
    assert_eq!(
        queued,
        vec![
            (Some(Step::BeginCombat), 0),
            (Some(Step::DeclareAttackers), 1),
            (Some(Step::DeclareBlockers), 0),
            (Some(Step::CombatDamage), 2),
            (Some(Step::EndCombat), 0),
        ],
        "CR 508.2b/510.3a: attack triggers, and damage plus SBA-death triggers, are waiting when the active player would receive priority"
    );
    assert_eq!(game.player(alice).unwrap().life, 22);
    assert_eq!(game.player(bob).unwrap().life, 21);
}


fn p06_queue_after_declaration(
    game: &mut GameState,
    declarations: &[ironsmith::AttackerDeclaration],
) -> (Result<(), String>, usize, ironsmith::combat_state::CombatState) {
    let mut combat = ironsmith::combat_state::new_combat();
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    game.refresh_continuous_state().unwrap();
    let result = ironsmith::game_loop::apply_attacker_declarations(
        game,
        &mut combat,
        &mut queue,
        declarations,
    )
    .map(|_| ())
    .map_err(|e| format!("{e:?}"));
    (result, queue.entries.len(), combat)
}

fn p06_set_declare_attackers(game: &mut GameState) {
    game.turn.phase = ironsmith::Phase::Combat;
    game.turn.step = Some(ironsmith::Step::DeclareAttackers);
}

fn p06_token_def(name: &str) -> ironsmith::cards::CardDefinition {
    ironsmith::cards::builders::CardDefinitionBuilder::new(ironsmith::CardId::new(), name)
        .card_types(vec![ironsmith::CardType::Creature])
        .power_toughness(ironsmith::PowerToughness::fixed(1, 1))
        .build()
}

fn p06_enter_attacking(
    game: &mut GameState,
    source: ironsmith::ObjectId,
    controller: PlayerId,
    defender: PlayerId,
) -> usize {
    use ironsmith::effects::CreateTokenEffect;
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    CreateTokenEffect::one(p06_token_def("Entering attacker"))
        .attacking_player(PlayerFilter::Specific(defender))
        .execute(game, &mut EffectContext::new_default(source, controller))
        .unwrap();
    ironsmith::game_loop::try_drain_pending_trigger_events(game, &mut queue).unwrap();
    queue.entries.len()
}

#[test]
fn cr_508_3b_is_attacked_triggers_once_per_attacked_player_and_not_for_entering_attackers() {
    use ironsmith::combat_state::AttackTarget;
    use ironsmith::triggers::Trigger;
    use ironsmith::{Ability, Effect};
    let mut game = p06_combat_game(2);
    let (alice, bob) = (PlayerId::from_index(0), PlayerId::from_index(1));
    p06_enchantment_with(
        &mut game,
        alice,
        "Opponent-attacked observer",
        vec![Ability::triggered(
            Trigger::players_attacked_one_or_more(PlayerFilter::Opponent),
            vec![Effect::gain_life(1)],
        )],
    );
    let a = p06_creature_with(&mut game, alice, "A", 2, 2, vec![]);
    let b = p06_creature_with(&mut game, alice, "B", 2, 2, vec![]);
    p06_set_declare_attackers(&mut game);
    let (result, queued, combat) = p06_queue_after_declaration(
        &mut game,
        &[
            p06_attack(a, AttackTarget::Player(bob)),
            p06_attack(b, AttackTarget::Player(bob)),
        ],
    );
    result.unwrap();
    game.combat = Some(combat);
    let entering = p06_enter_attacking(&mut game, a, alice, bob);
    assert_eq!(
        (queued, entering),
        (1, 0),
        "CR 508.3b: \"whenever a player is attacked\" triggers once when creatures are declared attacking that player, and not for a creature put onto the battlefield attacking"
    );
}

#[test]
fn cr_508_3c_d_attacks_with_triggers_per_creature_and_you_attack_triggers_once() {
    use ironsmith::combat_state::AttackTarget;
    use ironsmith::triggers::Trigger;
    use ironsmith::{Ability, Effect, ObjectFilter};
    let (alice, bob) = (PlayerId::from_index(0), PlayerId::from_index(1));
    let mut results = Vec::new();
    for per_creature in [true, false] {
        for attackers in [0usize, 1, 2] {
            let mut game = p06_combat_game(2);
            let trigger = if per_creature {
                Trigger::attacks(ObjectFilter::creature().you_control())
            } else {
                Trigger::attacks_one_or_more(ObjectFilter::creature().you_control())
            };
            p06_enchantment_with(
                &mut game,
                alice,
                "Attack observer",
                vec![Ability::triggered(trigger, vec![Effect::gain_life(1)])],
            );
            let ids: Vec<_> = (0..2)
                .map(|i| p06_creature_with(&mut game, alice, &format!("C{i}"), 2, 2, vec![]))
                .collect();
            p06_set_declare_attackers(&mut game);
            let decl: Vec<_> = ids[..attackers]
                .iter()
                .map(|id| p06_attack(*id, AttackTarget::Player(bob)))
                .collect();
            let (result, queued, combat) = p06_queue_after_declaration(&mut game, &decl);
            result.unwrap();
            game.combat = Some(combat);
            let entering = p06_enter_attacking(&mut game, ids[0], alice, bob);
            results.push((per_creature, attackers, queued + entering));
        }
    }
    assert_eq!(
        results,
        vec![
            (true, 0, 0),
            (true, 1, 1),
            (true, 2, 2),
            (false, 0, 0),
            (false, 1, 1),
            (false, 2, 1),
        ],
        "CR 508.3c: \"attacks with a creature\" triggers per declared creature; CR 508.3d: \"you attack\" triggers once if one or more are declared; neither for creatures put onto the battlefield attacking"
    );
}

#[test]
fn cr_508_3e_attacks_a_player_ignores_planeswalker_attacks_and_entering_attackers() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::combat_state::AttackTarget;
    use ironsmith::{CardId, CardType, Zone};
    let sensor = p06_compile(
        "Long-Range Sensor",
        "Mana cost: {2}\nType: Artifact\nWhenever you attack a player, put a charge counter on this artifact.",
    );
    assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&sensor));
    let (alice, bob) = (PlayerId::from_index(0), PlayerId::from_index(1));
    let mut results = Vec::new();
    for case in 0..3 {
        let mut game = p06_combat_game(2);
        let source = game.create_object_from_definition(&sensor, alice, Zone::Battlefield);
        let a = p06_creature_with(&mut game, alice, "A", 2, 2, vec![]);
        let b = p06_creature_with(&mut game, alice, "B", 2, 2, vec![]);
        let pw = CardDefinitionBuilder::new(CardId::new(), "Bob walker")
            .card_types(vec![CardType::Planeswalker])
            .loyalty(5)
            .build();
        let pw = game.create_object_from_definition(&pw, bob, Zone::Battlefield);
        p06_set_declare_attackers(&mut game);
        let decl = match case {
            0 => vec![
                p06_attack(a, AttackTarget::Player(bob)),
                p06_attack(b, AttackTarget::Player(bob)),
            ],
            _ => vec![p06_attack(a, AttackTarget::Planeswalker(pw))],
        };
        let (result, mut queued, combat) = p06_queue_after_declaration(&mut game, &decl);
        result.unwrap();
        if case == 2 {
            game.combat = Some(combat);
            queued += p06_enter_attacking(&mut game, source, alice, bob);
        }
        results.push(queued);
    }
    assert_eq!(
        results,
        vec![1, 0, 0],
        "CR 508.3e: \"whenever you attack a player\" triggers once for creatures declared attacking that player, not for attacking only a planeswalker or for a creature put onto the battlefield attacking"
    );
}

#[test]
fn cr_506_6_attacks_a_player_alone_differs_from_attacks_alone() {
    use ironsmith::combat_state::AttackTarget;
    use ironsmith::triggers::Trigger;
    use ironsmith::{Ability, Effect, ObjectFilter};
    let (alice, bob, carol) = (
        PlayerId::from_index(0),
        PlayerId::from_index(1),
        PlayerId::from_index(2),
    );
    let mut results = Vec::new();
    for split in [true, false] {
        let mut game = p06_combat_game(3);
        let a = p06_creature_with(
            &mut game,
            alice,
            "Lone player attacker",
            2,
            2,
            vec![Ability::triggered(
                Trigger::attacks_player_alone(ObjectFilter::source()),
                vec![Effect::gain_life(1)],
            )],
        );
        let b = p06_creature_with(
            &mut game,
            alice,
            "Other attacker",
            2,
            2,
            vec![Ability::triggered(
                Trigger::attacks_alone(ObjectFilter::source()),
                vec![Effect::gain_life(10)],
            )],
        );
        p06_set_declare_attackers(&mut game);
        let second = if split { carol } else { bob };
        let (result, queued, _) = p06_queue_after_declaration(
            &mut game,
            &[
                p06_attack(a, AttackTarget::Player(bob)),
                p06_attack(b, AttackTarget::Player(second)),
            ],
        );
        result.unwrap();
        results.push((split, queued));
    }
    assert_eq!(
        results,
        vec![(true, 1), (false, 0)],
        "CR 506.6: attacking different players, each creature attacks a player alone (but neither attacks alone, CR 506.5); attacking the same player, neither attacks that player alone"
    );
}

#[test]
fn cr_506_6_attacking_alone_status_ends_when_another_creature_enters_attacking() {
    use ironsmith::combat_state::AttackTarget;
    use ironsmith::Zone;
    let prowler = p06_compile(
        "Dream Prowler",
        "Mana cost: {2}{U}{U}\nType: Creature — Illusion\nPower/Toughness: 1/5\nThis creature can't be blocked as long as it's attacking alone.",
    );
    assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&prowler));
    let (alice, bob) = (PlayerId::from_index(0), PlayerId::from_index(1));
    let mut game = p06_combat_game(2);
    let attacker = game.create_object_from_definition(&prowler, alice, Zone::Battlefield);
    let blocker = p06_creature_with(&mut game, bob, "Blocker", 2, 2, vec![]);
    p06_set_declare_attackers(&mut game);
    let (result, _, combat) =
        p06_queue_after_declaration(&mut game, &[p06_attack(attacker, AttackTarget::Player(bob))]);
    result.unwrap();
    game.combat = Some(combat);
    game.refresh_continuous_state().unwrap();
    let blockable = |game: &GameState| {
        ironsmith::compute_legal_blockers(game, game.combat.as_ref().unwrap(), bob)
            .iter()
            .any(|o| o.attacker == attacker && o.valid_blockers.contains(&blocker))
    };
    let alone_blockable = blockable(&game);
    p06_enter_attacking(&mut game, attacker, alice, bob);
    game.refresh_continuous_state().unwrap();
    let accompanied_blockable = blockable(&game);
    assert_eq!(
        (alone_blockable, accompanied_blockable),
        (false, true),
        "CR 506.6: a creature is attacking alone only while no other creature is attacking (an entering attacker ends it)"
    );
}

#[test]
fn cr_508_1c_restrictions_make_attack_declarations_illegal() {
    use ironsmith::combat_state::AttackTarget;
    use ironsmith::Zone;
    let flunkies = p06_compile(
        "Mogg Flunkies",
        "Mana cost: {2}{R}\nType: Creature — Goblin\nPower/Toughness: 3/3\nThis creature can't attack or block alone.",
    );
    assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&flunkies));
    let (alice, bob) = (PlayerId::from_index(0), PlayerId::from_index(1));
    let mut results = Vec::new();
    for with_partner in [false, true] {
        let mut game = p06_combat_game(2);
        let f = game.create_object_from_definition(&flunkies, alice, Zone::Battlefield);
        let partner = p06_creature_with(&mut game, alice, "Partner", 2, 2, vec![]);
        p06_set_declare_attackers(&mut game);
        let mut decl = vec![p06_attack(f, AttackTarget::Player(bob))];
        if with_partner {
            decl.push(p06_attack(partner, AttackTarget::Player(bob)));
        }
        let (result, _, combat) = p06_queue_after_declaration(&mut game, &decl);
        results.push((
            with_partner,
            result.is_ok(),
            ironsmith::combat_state::is_attacking(&combat, f),
            game.is_tapped(f),
        ));
    }
    let mut game = p06_combat_game(2);
    let barred = p06_creature_with(
        &mut game,
        alice,
        "Barred",
        2,
        2,
        vec![ironsmith::Ability::static_ability(ironsmith::StaticAbility::cant_attack())],
    );
    p06_set_declare_attackers(&mut game);
    let (barred_result, _, _) =
        p06_queue_after_declaration(&mut game, &[p06_attack(barred, AttackTarget::Player(bob))]);
    assert_eq!(
        (results, barred_result.is_ok()),
        (vec![(false, false, false, false), (true, true, true, true)], false),
        "CR 508.1c: a declaration disobeying a restriction (can't attack alone / can't attack) is illegal"
    );
}

#[test]
fn cr_508_1d_requirements_must_be_obeyed_to_the_maximum_possible() {
    use ironsmith::combat_state::AttackTarget;
    use ironsmith::{Ability, StaticAbility};
    let (alice, bob) = (PlayerId::from_index(0), PlayerId::from_index(1));
    let mut results = Vec::new();
    for (tapped, attack) in [(false, false), (false, true), (true, false)] {
        let mut game = p06_combat_game(2);
        let brute = p06_creature_with(
            &mut game,
            alice,
            "Must attack",
            2,
            2,
            vec![Ability::static_ability(StaticAbility::must_attack())],
        );
        if tapped {
            game.tap(brute);
        }
        p06_set_declare_attackers(&mut game);
        let decl = if attack {
            vec![p06_attack(brute, AttackTarget::Player(bob))]
        } else {
            vec![]
        };
        let (result, _, combat) = p06_queue_after_declaration(&mut game, &decl);
        results.push((
            tapped,
            attack,
            result.is_ok(),
            combat.creature_had_to_attack_this_combat(brute),
        ));
    }
    assert_eq!(
        results,
        vec![
            (false, false, false, false),
            (false, true, true, true),
            (true, false, true, false),
        ],
        "CR 508.1d: an untapped must-attack creature must be declared; when it can't attack (tapped) the empty declaration obeys the maximum possible"
    );
}

#[test]
fn cr_506_7_had_to_attack_requires_a_requirement_not_lack_of_alternatives() {
    use ironsmith::combat_state::AttackTarget;
    let (alice, bob) = (PlayerId::from_index(0), PlayerId::from_index(1));
    let mut game = p06_combat_game(2);
    let free = p06_creature_with(&mut game, alice, "Free attacker", 2, 2, vec![]);
    let forced = p06_creature_with(
        &mut game,
        alice,
        "Forced attacker",
        2,
        2,
        vec![ironsmith::Ability::static_ability(ironsmith::StaticAbility::must_attack())],
    );
    p06_set_declare_attackers(&mut game);
    let (result, _, combat) = p06_queue_after_declaration(
        &mut game,
        &[
            p06_attack(free, AttackTarget::Player(bob)),
            p06_attack(forced, AttackTarget::Player(bob)),
        ],
    );
    result.unwrap();
    assert_eq!(
        (
            combat.creature_had_to_attack_this_combat(free),
            combat.creature_had_to_attack_this_combat(forced)
        ),
        (false, true),
        "CR 506.7: only a creature that an effect required to attack \"had to attack\""
    );
}

#[test]
fn cr_508_1e_bands_require_banding_and_are_recorded() {
    use ironsmith::combat_state::AttackTarget;
    use ironsmith::{Ability, StaticAbility, TurnRunner, TurnRunnerState};
    let (alice, bob) = (PlayerId::from_index(0), PlayerId::from_index(1));
    let mut results = Vec::new();
    for with_banding in [true, false] {
        let mut game = p06_combat_game(2);
        let abilities = if with_banding {
            vec![Ability::static_ability(StaticAbility::banding())]
        } else {
            vec![]
        };
        let bander = p06_creature_with(&mut game, alice, "Bander", 2, 2, abilities);
        let partner = p06_creature_with(&mut game, alice, "Partner", 2, 2, vec![]);
        p06_set_declare_attackers(&mut game);
        let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::DeclareAttackersApply);
        runner.respond_attackers_with_bands(
            vec![
                p06_attack(bander, AttackTarget::Player(bob)),
                p06_attack(partner, AttackTarget::Player(bob)),
            ],
            vec![vec![bander, partner]],
        );
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let outcome = runner.advance(&mut game, &mut queue);
        let bands = game
            .combat
            .as_ref()
            .map(|c| c.attacking_bands.clone())
            .unwrap_or_default();
        results.push((with_banding, outcome.is_ok() && !bands.is_empty()));
    }
    assert_eq!(
        results,
        vec![(true, true), (false, false)],
        "CR 508.1e/702.22c: an announced band is recorded only when a member has banding"
    );
}

#[test]
fn cr_508_1k_511_2_511_3_attackers_persist_until_combat_ends_and_end_of_combat_timing() {
    use ironsmith::combat_state::AttackTarget;
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::triggers::Trigger;
    use ironsmith::{Ability, Effect, Step, TurnRunner, TurnRunnerState, Until};
    let mut game = p06_combat_game(2);
    let (alice, bob) = (PlayerId::from_index(0), PlayerId::from_index(1));
    p06_enchantment_with(
        &mut game,
        alice,
        "End of combat observer",
        vec![Ability::triggered(Trigger::end_of_combat(), vec![Effect::gain_life(1)])],
    );
    let attacker = p06_creature_with(&mut game, alice, "Attacker", 2, 2, vec![]);
    let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::BeginCombat);
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut seen = Vec::new();
    let run = p06_drive(
        &mut game,
        &mut runner,
        &mut queue,
        vec![p06_attack(attacker, AttackTarget::Player(bob))],
        vec![],
        |s| matches!(s, TurnRunnerState::NextMain | TurnRunnerState::NextMainPriority),
        |game, _state, queued| {
            let step = game.turn.step;
            if step == Some(Step::DeclareAttackers) {
                ApplyContinuousEffect::new(
                    EffectTarget::Specific(attacker),
                    Modification::ModifyPowerToughness { power: 3, toughness: 0 },
                    Until::EndOfCombat,
                )
                .execute(game, &mut EffectContext::new_default(attacker, alice))
                .unwrap();
                game.refresh_continuous_state().unwrap();
            }
            seen.push((
                step,
                p06_is_attacking(game, attacker),
                game.is_tapped(attacker),
                game.current_power(attacker),
                queued,
            ));
        },
    );
    let _ = run;
    game.refresh_continuous_state().unwrap();
    assert_eq!(
        seen,
        vec![
            (Some(Step::BeginCombat), false, false, Some(2), 0),
            (Some(Step::DeclareAttackers), true, true, Some(5), 0),
            (Some(Step::DeclareBlockers), true, true, Some(5), 0),
            (Some(Step::CombatDamage), true, true, Some(5), 0),
            (Some(Step::EndCombat), true, true, Some(5), 1),
        ],
        "CR 508.1k: declared creatures become (tapped) attacking creatures until combat ends; CR 511.2: \"at end of combat\" triggers as the end of combat step begins and \"until end of combat\" lasts through it"
    );
    assert!(
        !p06_is_attacking(&game, attacker),
        "CR 511.3: after the end of combat step all creatures are removed from combat"
    );
    assert_eq!(
        game.current_power(attacker),
        Some(2),
        "CR 511.2: \"until end of combat\" effects expire at the end of the combat phase"
    );
    assert_eq!(game.player(bob).unwrap().life, 15);
}

#[test]
fn cr_508_1m_508_2a_attack_triggers_check_only_at_declaration() {
    use ironsmith::combat_state::AttackTarget;
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::{Until, Zone};
    let observer = p06_compile(
        "Power attack observer",
        "Mana cost: {2}\nType: Enchantment\nWhenever a creature with power 4 or greater attacks, you gain 1 life.",
    );
    assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&observer));
    let (alice, bob) = (PlayerId::from_index(0), PlayerId::from_index(1));
    let mut results = Vec::new();
    for big in [true, false] {
        let mut game = p06_combat_game(2);
        game.create_object_from_definition(&observer, alice, Zone::Battlefield);
        let attacker = p06_creature_with(&mut game, alice, "Attacker", if big { 4 } else { 1 }, 4, vec![]);
        p06_set_declare_attackers(&mut game);
        let (result, queued, combat) =
            p06_queue_after_declaration(&mut game, &[p06_attack(attacker, AttackTarget::Player(bob))]);
        result.unwrap();
        game.combat = Some(combat);
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        ApplyContinuousEffect::new(
            EffectTarget::Specific(attacker),
            Modification::ModifyPowerToughness { power: 5, toughness: 0 },
            Until::EndOfTurn,
        )
        .execute(&mut game, &mut EffectContext::new_default(attacker, alice))
        .unwrap();
        game.refresh_continuous_state().unwrap();
        ironsmith::game_loop::try_drain_pending_trigger_events(&mut game, &mut queue).unwrap();
        results.push((big, queued, queue.entries.len()));
    }
    assert_eq!(
        results,
        vec![(true, 1, 0), (false, 0, 0)],
        "CR 508.1m/508.2a: attack triggers fire when attackers are declared, not when an attacker later comes to match the condition"
    );
}

#[test]
fn cr_508_6_raid_counts_declared_attacks_not_creatures_put_onto_battlefield_attacking() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::combat_state::AttackTarget;
    use ironsmith::effects::CreateTokenEffect;
    use ironsmith::{CardId, CardType, ManaSymbol, PowerToughness, Zone};
    let champion = p06_compile(
        "Bloodsoaked Champion",
        "Mana cost: {B}\nType: Creature — Human Warrior\nPower/Toughness: 2/1\nThis creature can't block.\nRaid — {1}{B}: Return this card from your graveyard to the battlefield. Activate only if you attacked this turn.",
    );
    assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&champion));
    let (alice, bob) = (PlayerId::from_index(0), PlayerId::from_index(1));
    let mut results = Vec::new();
    for declared in [false, true] {
        let mut game = p06_combat_game(2);
        let card = game.create_object_from_definition(&champion, alice, Zone::Graveyard);
        let attacker = p06_creature_with(&mut game, alice, "Attacker", 2, 2, vec![]);
        p06_set_declare_attackers(&mut game);
        let decl = if declared {
            vec![p06_attack(attacker, AttackTarget::Player(bob))]
        } else {
            vec![]
        };
        let (result, _, combat) = p06_queue_after_declaration(&mut game, &decl);
        result.unwrap();
        game.combat = Some(combat);
        if !declared {
            let token = CardDefinitionBuilder::new(CardId::new(), "Entering attacker")
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(1, 1))
                .build();
            CreateTokenEffect::one(token)
                .attacking_player(PlayerFilter::Specific(bob))
                .execute(&mut game, &mut EffectContext::new_default(attacker, alice))
                .unwrap();
        }
        game.turn.priority_player = Some(alice);
        game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Black, 2);
        results.push((declared, p06_can_activate(&game, alice, card)));
    }
    assert_eq!(
        results,
        vec![(false, false), (true, true)],
        "CR 508.6: a player has attacked only if they declared creatures as attackers; a creature put onto the battlefield attacking doesn't count"
    );
}

#[test]
fn cr_508_4b_entering_attacker_naming_a_departed_player_does_not_attack() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effects::CreateTokenEffect;
    use ironsmith::{CardId, CardType, PowerToughness};
    let mut game = p06_combat_game(3);
    let (alice, carol) = (PlayerId::from_index(0), PlayerId::from_index(2));
    let source = p06_creature_with(&mut game, alice, "Source", 2, 2, vec![]);
    p06_set_declare_attackers(&mut game);
    game.combat = Some(ironsmith::combat_state::new_combat());
    game.concede_game(carol).unwrap();
    assert!(!game.player(carol).unwrap().is_in_game());
    let before = game.battlefield.clone();
    let token = CardDefinitionBuilder::new(CardId::new(), "Entering attacker")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    CreateTokenEffect::one(token)
        .attacking_player(PlayerFilter::Specific(carol))
        .execute(&mut game, &mut EffectContext::new_default(source, alice))
        .unwrap();
    let new: Vec<_> = game
        .battlefield
        .iter()
        .copied()
        .filter(|id| !before.contains(id))
        .collect();
    assert_eq!(new.len(), 1, "fixture: the token is still created");
    assert!(
        !p06_is_attacking(&game, new[0]),
        "CR 508.4b: an effect naming a player no longer in the game doesn't make the creature attacking"
    );
}

#[test]
fn cr_510_1e_illegal_total_damage_assignment_is_rejected() {
    use ironsmith::combat_state::AttackTarget;
    use ironsmith::decisions::context::DecisionContext;
    use ironsmith::{Ability, StaticAbility, Target, TurnAction, TurnRunner, TurnRunnerState};
    let mut game = p06_combat_game(2);
    let (alice, bob) = (PlayerId::from_index(0), PlayerId::from_index(1));
    let attacker = p06_creature_with(
        &mut game,
        alice,
        "Trampler",
        4,
        4,
        vec![Ability::static_ability(StaticAbility::trample())],
    );
    let b1 = p06_creature_with(&mut game, bob, "B1", 2, 2, vec![]);
    let b2 = p06_creature_with(&mut game, bob, "B2", 2, 2, vec![]);
    let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::BeginCombat);
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut checked = None;
    for _ in 0..80 {
        if matches!(runner.state(), TurnRunnerState::EndCombatPriority) {
            break;
        }
        match runner.advance(&mut game, &mut queue).unwrap() {
            TurnAction::Continue => {}
            TurnAction::RunPriority => runner.priority_done(),
            TurnAction::Decision(DecisionContext::Attackers(_)) => {
                runner.respond_attackers(vec![p06_attack(attacker, AttackTarget::Player(bob))])
            }
            TurnAction::Decision(DecisionContext::Blockers(ctx)) => runner.respond_blockers(
                vec![
                    ironsmith::BlockerDeclaration { blocker: b1, blocking: attacker },
                    ironsmith::BlockerDeclaration { blocker: b2, blocking: attacker },
                ],
                ctx.player,
            ),
            TurnAction::Decision(DecisionContext::Distribute(_)) => {
                let check = |d: &[(Target, u32)]| runner.validate_combat_damage_distribution(&game, d).is_ok();
                checked = Some((
                    check(&[(Target::Object(b1), 2), (Target::Object(b2), 1), (Target::Player(bob), 1)]),
                    check(&[(Target::Object(b1), 1), (Target::Object(b2), 1)]),
                    check(&[(Target::Object(b1), 2), (Target::Object(b2), 2)]),
                    check(&[(Target::Object(b1), 2), (Target::Object(b2), 2), (Target::Player(bob), 0)]),
                ));
                runner.respond_distribute(vec![(Target::Object(b1), 2), (Target::Object(b2), 2)]);
            }
            other => panic!("unexpected {other:?}"),
        }
    }
    let checked = checked.expect("fixture: a damage division prompt was reached");
    assert_eq!(
        (checked.0, checked.1, checked.2),
        (false, false, true),
        "CR 510.1e: the whole assignment is checked; trample to the player before lethal to every blocker, or assigning less than all damage, is illegal"
    );
}

#[test]
fn cr_512_1_ending_phase_has_end_and_cleanup_steps() {
    use ironsmith::{Phase, Step, TurnAction, TurnRunner, TurnRunnerState};
    let mut game = p06_combat_game(2);
    game.turn.phase = Phase::NextMain;
    let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::EndStep);
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut steps = Vec::new();
    for _ in 0..40 {
        let action = runner.advance(&mut game, &mut queue).unwrap();
        if game.turn.phase == Phase::Ending && steps.last() != Some(&game.turn.step) {
            steps.push(game.turn.step);
        }
        match action {
            TurnAction::Continue => {}
            TurnAction::RunPriority => runner.priority_done(),
            TurnAction::TurnComplete => break,
            other => panic!("unexpected {other:?}"),
        }
        if matches!(runner.state(), TurnRunnerState::Complete) {
            break;
        }
    }
    assert_eq!(
        steps,
        vec![Some(Step::End), Some(Step::Cleanup)],
        "CR 512.1: the ending phase consists of the end step then the cleanup step"
    );
}

// ---------------------------------------------------------------------------
// Attack costs (CR 508.1g-j)
// ---------------------------------------------------------------------------

struct P06SelectObjects(Vec<Vec<ironsmith::ObjectId>>);
impl DecisionMaker for P06SelectObjects {
    fn decide_objects(
        &mut self,
        _game: &GameState,
        _ctx: &ironsmith::decisions::context::SelectObjectsContext,
    ) -> Vec<ironsmith::ObjectId> {
        if self.0.is_empty() { Vec::new() } else { self.0.remove(0) }
    }
}

#[test]
fn cr_508_1g_optional_enlist_cost_is_chosen_and_paid_only_when_selected() {
    use ironsmith::combat_state::AttackTarget;
    use ironsmith::Zone;
    let crusher = p06_compile(
        "Barkweave Crusher",
        "Mana cost: {3}{G}\nType: Creature — Elemental Warrior\nPower/Toughness: 3/3\nEnlist (As this creature attacks, you may tap a nonattacking creature you control without summoning sickness. When you do, add its power to this creature's until end of turn.)",
    );
    assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&crusher));
    let (alice, bob) = (PlayerId::from_index(0), PlayerId::from_index(1));
    let mut results = Vec::new();
    for pay in [false, true] {
        let mut game = p06_combat_game(2);
        let attacker = game.create_object_from_definition(&crusher, alice, Zone::Battlefield);
        let support = p06_creature_with(&mut game, alice, "Support", 2, 2, vec![]);
        game.refresh_continuous_state().unwrap();
        p06_set_declare_attackers(&mut game);
        let mut combat = ironsmith::combat_state::new_combat();
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let mut dm = P06SelectObjects(if pay { vec![vec![support]] } else { vec![vec![]] });
        ironsmith::game_loop::apply_attacker_declarations_with_dm(
            &mut game,
            &mut combat,
            &mut queue,
            &[p06_attack(attacker, AttackTarget::Player(bob))],
            &mut dm,
        )
        .unwrap();
        results.push((pay, game.is_tapped(support), ironsmith::combat_state::is_attacking(&combat, attacker), queue.entries.len()));
    }
    assert_eq!(
        results,
        vec![(false, false, true, 0), (true, true, true, 1)],
        "CR 508.1g/702.154a: the optional enlist cost is paid (tapping the support) only if chosen; the attack is legal either way"
    );
}

#[test]
fn cr_508_1h_508_1j_total_attack_cost_must_be_paid_in_full() {
    use ironsmith::combat_state::AttackTarget;
    use ironsmith::{ManaSymbol, Zone};
    let propaganda = p06_compile(
        "Propaganda",
        "Mana cost: {2}{U}\nType: Enchantment\nCreatures can't attack you unless their controller pays {2} for each creature they control that's attacking you.",
    );
    assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&propaganda));
    let (alice, bob) = (PlayerId::from_index(0), PlayerId::from_index(1));
    let mut results = Vec::new();
    for mana in [3u8, 4] {
        let mut game = p06_combat_game(2);
        game.create_object_from_definition(&propaganda, bob, Zone::Battlefield);
        let a = p06_creature_with(&mut game, alice, "A", 2, 2, vec![]);
        let b = p06_creature_with(&mut game, alice, "B", 2, 2, vec![]);
        game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Colorless, mana as u32);
        p06_set_declare_attackers(&mut game);
        let (result, _, combat) = p06_queue_after_declaration(
            &mut game,
            &[p06_attack(a, AttackTarget::Player(bob)), p06_attack(b, AttackTarget::Player(bob))],
        );
        results.push((
            mana,
            result.is_ok(),
            ironsmith::combat_state::is_attacking(&combat, a),
            game.is_tapped(a),
            game.player(alice).unwrap().mana_pool.total(),
        ));
    }
    assert_eq!(
        results,
        vec![(3, false, false, false, 3), (4, true, true, true, 0)],
        "CR 508.1h/508.1j: the total cost to attack ({{2}} per attacker = {{4}}) is locked in and paid in full; partial payment is not allowed and an unpayable declaration is undone"
    );
}

#[test]
fn cr_508_1i_attack_cost_offers_a_mana_ability_window() {
    use ironsmith::combat_state::AttackTarget;
    use ironsmith::decisions::context::DecisionContext;
    use ironsmith::{TurnAction, TurnRunner, TurnRunnerState, Zone};
    let propaganda = p06_compile(
        "Propaganda",
        "Mana cost: {2}{U}\nType: Enchantment\nCreatures can't attack you unless their controller pays {2} for each creature they control that's attacking you.",
    );
    let rock = p06_compile("Two-mana rock", "Mana cost: {0}\nType: Artifact\n{T}: Add {C}{C}.");
    let (alice, bob) = (PlayerId::from_index(0), PlayerId::from_index(1));
    let mut game = p06_combat_game(2);
    game.create_object_from_definition(&propaganda, bob, Zone::Battlefield);
    let rock = game.create_object_from_definition(&rock, alice, Zone::Battlefield);
    let attacker = p06_creature_with(&mut game, alice, "Attacker", 2, 2, vec![]);
    game.turn.phase = ironsmith::Phase::Combat;
    let mut runner = TurnRunner::from_state_for_sync(TurnRunnerState::DeclareAttackersDecision);
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut offered_rock = false;
    let mut reached_priority = false;
    for _ in 0..20 {
        match runner.advance(&mut game, &mut queue).unwrap() {
            TurnAction::Decision(DecisionContext::Attackers(_)) => {
                runner.respond_attackers(vec![p06_attack(attacker, AttackTarget::Player(bob))])
            }
            TurnAction::Decision(DecisionContext::SelectOptions(ctx)) => {
                if let Some(option) = ctx.options.iter().find(|o| o.object_id == Some(rock)) {
                    offered_rock = true;
                    runner.respond_options(vec![option.index]);
                } else if let Some(option) = ctx.options.iter().find(|o| {
                    o.description == "Finish activating mana abilities" || o.description == "Confirm payment"
                }) {
                    runner.respond_options(vec![option.index]);
                } else {
                    panic!("unexpected options {ctx:?}");
                }
            }
            TurnAction::Continue => {}
            TurnAction::RunPriority => {
                reached_priority = true;
                break;
            }
            other => panic!("unexpected {other:?}"),
        }
    }
    assert!(
        offered_rock && reached_priority && game.is_tapped(rock) && p06_is_attacking(&game, attacker),
        "CR 508.1i: with a mana cost to attack the active player gets to activate mana abilities, then pays (offered={offered_rock}, priority={reached_priority}, rock tapped={})",
        game.is_tapped(rock)
    );
}

// ---------------------------------------------------------------------------
// Casting spells / activating abilities (CR 601-602)
// ---------------------------------------------------------------------------

#[derive(Default)]
struct P06Dm {
    x: u32,
    prefer_target: Option<ironsmith::Target>,
    option_players: Vec<(PlayerId, String)>,
    target_players: Vec<PlayerId>,
    sacrifice: Option<ironsmith::ObjectId>,
}

impl DecisionMaker for P06Dm {
    fn decide_number(
        &mut self,
        _game: &GameState,
        ctx: &ironsmith::decisions::context::NumberContext,
    ) -> u32 {
        self.x.max(ctx.min)
    }
    fn decide_targets(
        &mut self,
        _game: &GameState,
        ctx: &ironsmith::decisions::context::TargetsContext,
    ) -> Vec<ironsmith::Target> {
        self.target_players.push(ctx.player);
        ctx.requirements
            .iter()
            .filter_map(|r| {
                if let Some(t) = self.prefer_target.filter(|t| r.legal_targets.contains(t)) {
                    Some(t)
                } else {
                    r.legal_targets.first().copied()
                }
            })
            .collect()
    }
    fn decide_options(
        &mut self,
        _game: &GameState,
        ctx: &SelectOptionsContext,
    ) -> Vec<usize> {
        self.option_players.push((ctx.player, ctx.description.clone()));
        if let Some(o) = ctx.options.iter().find(|o| o.legal && (o.description == "Confirm payment")) {
            return vec![o.index];
        }
        let n = ctx.min.max(1).min(ctx.max.max(1));
        ctx.options.iter().filter(|o| o.legal).take(n).map(|o| o.index).collect()
    }
    fn decide_objects(
        &mut self,
        _game: &GameState,
        ctx: &ironsmith::decisions::context::SelectObjectsContext,
    ) -> Vec<ironsmith::ObjectId> {
        if let Some(id) = self.sacrifice.filter(|id| ctx.candidates.iter().any(|c| c.id == *id && c.legal)) {
            return vec![id];
        }
        ctx.candidates.iter().filter(|c| c.legal).map(|c| c.id).take(ctx.min).collect()
    }
}

fn p06_complete_action<D: DecisionMaker>(
    game: &mut GameState,
    player: PlayerId,
    action: ironsmith::LegalAction,
    dm: &mut D,
    queue: &mut ironsmith::triggers::TriggerQueue,
) -> Result<(), String> {
    use ironsmith::{PriorityLoopState, PriorityResponse};
    let requested_method = match &action {
        ironsmith::LegalAction::CastSpell { casting_method, .. } => Some(casting_method.clone()),
        _ => None,
    };
    let mut state = PriorityLoopState::new(game.players_in_game());
    assert_eq!(game.turn.priority_player, Some(player));
    let mut result = ironsmith::game_loop::apply_priority_response_with_dm(
        game,
        queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    );
    for _ in 0..24 {
        if result.is_err()
            || (state.pending_cast.is_none()
                && state.pending_method_selection.is_none()
                && state.pending_activation.is_none())
        {
            break;
        }
        if let Some(pending) = &state.pending_method_selection {
            let index = pending
                .available_methods
                .iter()
                .position(|choice| Some(&choice.method) == requested_method.as_ref())
                .ok_or_else(|| "requested casting method lost".to_string())?;
            result = ironsmith::game_loop::apply_priority_response_with_dm(
                game,
                queue,
                &mut state,
                &PriorityResponse::CastingMethodChoice(index),
                dm,
            );
        } else {
            let context = match &result {
                Ok(ironsmith::decision::GameProgress::NeedsDecisionCtx(context)) => context.clone(),
                other => return Err(format!("pending action has no decision: {other:?}")),
            };
            result = ironsmith::game_loop::apply_decision_context_with_dm(
                game, queue, &mut state, &context, dm,
            );
        }
    }
    result.map_err(|error| format!("{error:?}"))?;
    if state.pending_cast.is_some()
        || state.pending_method_selection.is_some()
        || state.pending_activation.is_some()
    {
        return Err("action exceeded decision bound".into());
    }
    Ok(())
}

fn p06_cast_actions(game: &GameState, player: PlayerId, id: ironsmith::ObjectId) -> Vec<ironsmith::LegalAction> {
    ironsmith::compute_legal_actions(game, player)
        .unwrap()
        .into_iter()
        .filter(|a| matches!(a, ironsmith::LegalAction::CastSpell { spell_id, .. } if *spell_id == id))
        .collect()
}

fn p06_activate_actions(game: &GameState, player: PlayerId, id: ironsmith::ObjectId) -> Vec<ironsmith::LegalAction> {
    ironsmith::compute_legal_actions(game, player)
        .unwrap()
        .into_iter()
        .filter(|a| matches!(a, ironsmith::LegalAction::ActivateAbility { source, .. } if *source == id))
        .collect()
}

fn p06_main_phase(game: &mut GameState, active: PlayerId, priority: PlayerId) {
    game.turn.active_player = active;
    game.turn.priority_player = Some(priority);
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
}

#[test]
fn cr_601_2_601_2g_601_2i_casting_moves_pays_with_mana_abilities_triggers_and_returns_priority() {
    use ironsmith::triggers::Trigger;
    use ironsmith::{Ability, Effect, Zone};
    let bolt = p06_compile("Lightning Bolt", "Mana cost: {R}\nType: Instant\nLightning Bolt deals 3 damage to any target.");
    let rock = p06_compile("Red rock", "Mana cost: {0}\nType: Artifact\n{T}: Add {R}.");
    let (mut game, alice, bob) = p06_two_player();
    p06_main_phase(&mut game, alice, alice);
    let rock = game.create_object_from_definition(&rock, alice, Zone::Battlefield);
    p06_enchantment_with(
        &mut game,
        alice,
        "Cast observer",
        vec![Ability::triggered(Trigger::spell_cast(None, PlayerFilter::You), vec![Effect::gain_life(1)])],
    );
    let card = game.create_object_from_definition(&bolt, alice, Zone::Hand);
    let stable = game.object(card).unwrap().stable_id;
    let action = p06_cast_actions(&game, alice, card).into_iter().next().expect("fixture: Bolt castable");
    let mut dm = P06Dm { prefer_target: Some(ironsmith::Target::Player(bob)), ..Default::default() };
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    p06_complete_action(&mut game, alice, action, &mut dm, &mut queue).unwrap();
    let spell = game.find_object_by_stable_id(stable).unwrap();
    let on_stack = game.object(spell).unwrap().zone == Zone::Stack;
    let cast_trigger_waiting = queue.entries.len() + game.stack.len().saturating_sub(1);
    assert_eq!(
        (on_stack, game.is_tapped(rock), game.player(alice).unwrap().mana_pool.total(), cast_trigger_waiting >= 1, game.turn.priority_player, game.player(bob).unwrap().life),
        (true, true, 0, true, Some(alice), 20),
        "CR 601.2/601.2g/601.2i: the spell moves to the stack, a mana ability is activated to pay, cast triggers trigger, and the caster receives priority before it resolves"
    );
}

#[test]
fn cr_601_3a_601_2e_601_6_prohibited_quality_depends_on_x_choice() {
    use ironsmith::{ManaSymbol, Zone};
    let winnower = p06_compile(
        "Void Winnower",
        "Mana cost: {9}\nType: Creature — Eldrazi\nPower/Toughness: 11/9\nYour opponents can't cast spells with even mana values.",
    );
    let x_spell = p06_compile("X life", "Mana cost: {X}{R}{R}\nType: Sorcery\nThis spell deals X damage to any target.");
    assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&winnower));
    let mut results = Vec::new();
    for x in [0u32, 1] {
        let (mut game, alice, bob) = p06_two_player();
        p06_main_phase(&mut game, alice, alice);
        game.create_object_from_definition(&winnower, bob, Zone::Battlefield);
        let card = game.create_object_from_definition(&x_spell, alice, Zone::Hand);
        game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Red, 5);
        game.refresh_continuous_state().unwrap();
        let actions = p06_cast_actions(&game, alice, card);
        let offered = !actions.is_empty();
        let mut cast_ok = false;
        if let Some(action) = actions.into_iter().next() {
            let mut dm = P06Dm { x, ..Default::default() };
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            let _ = p06_complete_action(&mut game, alice, action, &mut dm, &mut queue);
            cast_ok = game.stack.len() == 1;
        }
        let in_hand = game.player(alice).unwrap().hand.contains(&card);
        results.push((x, offered, cast_ok, in_hand, game.player(alice).unwrap().mana_pool.total()));
    }
    assert_eq!(
        results,
        vec![(0, true, false, true, 5), (1, true, true, false, 2)],
        "CR 601.3a: the player may begin casting because X could make the mana value odd; CR 601.2e/601.6: with X=0 (mana value 2) the cast is illegal and fully reversed"
    );
}

#[test]
fn cr_601_3b_flash_permission_for_noncreature_spells_applies_to_bestow_cast() {
    use ironsmith::{CastingMethod, ManaSymbol, Zone};
    let floodcaller = p06_compile(
        "Valley Floodcaller",
        "Mana cost: {2}{U}\nType: Creature — Otter Wizard\nPower/Toughness: 2/2\nFlash\nYou may cast noncreature spells as though they had flash.",
    );
    let siren = p06_compile(
        "Hypnotic Siren",
        "Mana cost: {U}\nType: Enchantment Creature — Siren\nPower/Toughness: 1/1\nBestow {5}{U}{U} (If you cast this card for its bestow cost, it's an Aura spell with enchant creature. It becomes a creature again if it's not attached.)\nFlying\nYou control enchanted creature.\nEnchanted creature gets +1/+1 and has flying.",
    );
    assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&siren));
    let (mut game, alice, bob) = p06_two_player();
    p06_main_phase(&mut game, bob, alice);
    game.create_object_from_definition(&floodcaller, alice, Zone::Battlefield);
    p06_bear(&mut game, bob, "Bob creature");
    let card = game.create_object_from_definition(&siren, alice, Zone::Hand);
    game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Blue, 7);
    game.refresh_continuous_state().unwrap();
    let methods: Vec<_> = p06_cast_actions(&game, alice, card)
        .into_iter()
        .filter_map(|a| match a {
            ironsmith::LegalAction::CastSpell { casting_method, .. } => Some(casting_method),
            _ => None,
        })
        .collect();
    let normal = methods.iter().any(|m| *m == CastingMethod::Normal);
    let bestow = methods.iter().any(|m| *m != CastingMethod::Normal);
    assert_eq!(
        (normal, bestow),
        (false, true),
        "CR 601.3b: on an opponent's turn the bestowed (Aura, noncreature) cast may begin as though it had flash, but the normal creature cast may not; got {methods:?}"
    );
}

#[test]
fn cr_601_3c_flash_only_with_additional_cost_may_begin_and_pays_the_extra() {
    use ironsmith::{ManaSymbol, Zone};
    let rout = p06_compile(
        "Rout",
        "Mana cost: {3}{W}{W}\nType: Sorcery\nYou may cast this spell as though it had flash if you pay {2} more to cast it. (You may cast it any time you could cast an instant.)\nDestroy all creatures. They can't be regenerated.",
    );
    assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&rout));
    let (mut game, alice, bob) = p06_two_player();
    p06_main_phase(&mut game, bob, alice);
    let card = game.create_object_from_definition(&rout, alice, Zone::Hand);
    game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::White, 8);
    let action = p06_cast_actions(&game, alice, card).into_iter().next();
    let offered = action.is_some();
    let mut spent = 0;
    if let Some(action) = action {
        let mut dm = P06Dm::default();
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        p06_complete_action(&mut game, alice, action, &mut dm, &mut queue).unwrap();
        spent = 8 - game.player(alice).unwrap().mana_pool.total();
    }
    assert_eq!(
        (offered, game.stack.len(), spent),
        (true, 1, 7),
        "CR 601.3c: a spell castable as though it had flash only with an additional cost may begin casting at instant speed, paying {{2}} more"
    );
}

#[test]
fn cr_601_6a_conditional_flash_survives_losing_the_condition_during_casting() {
    use ironsmith::{Ability, ManaSymbol, StaticAbility, Zone};
    let mutual = p06_compile(
        "Mutual Destruction",
        "Mana cost: {B}\nType: Sorcery\nThis spell has flash as long as you control a permanent with flash.\nAs an additional cost to cast this spell, sacrifice a creature.\nDestroy target creature.",
    );
    assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&mutual));
    let mut results = Vec::new();
    for has_flash in [false, true] {
        let (mut game, alice, bob) = p06_two_player();
        p06_main_phase(&mut game, bob, alice);
        let abilities = if has_flash { vec![Ability::static_ability(StaticAbility::flash())] } else { vec![] };
        let fodder = p06_creature_with(&mut game, alice, "Only creature", 1, 1, abilities);
        let victim = p06_bear(&mut game, bob, "Bob creature");
        let card = game.create_object_from_definition(&mutual, alice, Zone::Hand);
        game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Black, 1);
        game.refresh_continuous_state().unwrap();
        let action = p06_cast_actions(&game, alice, card).into_iter().next();
        let offered = action.is_some();
        let mut cast = false;
        if let Some(action) = action {
            let mut dm = P06Dm {
                prefer_target: Some(ironsmith::Target::Object(victim)),
                sacrifice: Some(fodder),
                ..Default::default()
            };
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            let r = p06_complete_action(&mut game, alice, action, &mut dm, &mut queue);
            cast = r.is_ok() && game.stack.len() == 1 && game.object(fodder).is_none_or(|o| o.zone != Zone::Battlefield);
        }
        results.push((has_flash, offered, cast));
    }
    assert_eq!(
        results,
        vec![(false, false, false), (true, true, true)],
        "CR 601.3d: the spell may begin casting at instant speed only while the flash condition holds; CR 601.6a: sacrificing the flash permanent as a cost doesn't make the cast illegal"
    );
}

#[test]
fn cr_601_7_601_7a_opponent_chooses_mode_while_casting() {
    use ironsmith::{ManaSymbol, Zone};
    let library = p06_compile(
        "Library of Lat-Nam",
        "Mana cost: {4}{U}\nType: Sorcery\nAn opponent chooses one —\n• You draw three cards at the beginning of the next turn's upkeep.\n• You search your library for a card, put that card into your hand, then shuffle.",
    );
    assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&library));
    let mut results = Vec::new();
    for players in [2usize, 3] {
        let mut game = p06_combat_game(players);
        let alice = PlayerId::from_index(0);
        p06_main_phase(&mut game, alice, alice);
        let card = game.create_object_from_definition(&library, alice, Zone::Hand);
        game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Blue, 5);
        let action = p06_cast_actions(&game, alice, card).into_iter().next().expect("fixture: castable");
        let mut dm = P06Dm::default();
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        p06_complete_action(&mut game, alice, action, &mut dm, &mut queue).unwrap();
        let mode_choosers: Vec<_> = dm
            .option_players
            .iter()
            .filter(|(_, d)| !d.contains("payment"))
            .map(|(p, _)| *p)
            .collect();
        results.push((players, mode_choosers.iter().any(|p| *p != alice), mode_choosers));
    }
    assert!(
        results.iter().all(|(_, opponent_chose, _)| *opponent_chose),
        "CR 601.7/601.7a: an opponent (chosen by the caster if several) chooses the mode when the controller normally would; observed choosers {results:?}"
    );
}

#[test]
fn cr_602_3b_controller_chooses_before_the_opponent_during_activation() {
    use ironsmith::{ManaSymbol, Zone};
    let echo = p06_compile(
        "Echo Chamber",
        "Mana cost: {4}\nType: Artifact\n{0}, {T}: An opponent chooses target creature they control. Create a token that's a copy of that creature. That token gains haste until end of turn. Exile the token at the beginning of the next end step. Activate only as a sorcery.",
    );
    let assassin = p06_compile(
        "Mogg Assassin",
        "Mana cost: {2}{R}\nType: Creature — Goblin Assassin\nPower/Toughness: 2/1\n{T}: You choose target creature an opponent controls, and that opponent chooses target creature. Flip a coin. If you win the flip, destroy the creature you chose. If you lose the flip, destroy the creature your opponent chose.",
    );
    let mut out = Vec::new();
    let _ = &echo;
    for (name, def) in [("Mogg Assassin", &assassin)] {
        let supported = !ironsmith::cards::generated_definition_has_unimplemented_content(def);
        let (mut game, alice, bob) = p06_two_player();
        p06_main_phase(&mut game, alice, alice);
        let source = game.create_object_from_definition(def, alice, Zone::Battlefield);
        p06_bear(&mut game, alice, "Alice creature");
        p06_bear(&mut game, bob, "Bob creature");
        let _ = ManaSymbol::Red;
        let action = p06_activate_actions(&game, alice, source).into_iter().next();
        let mut dm = P06Dm::default();
        if let Some(action) = action {
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            let _ = p06_complete_action(&mut game, alice, action, &mut dm, &mut queue);
        }
        out.push((name, supported, dm.target_players.clone()));
    }
    let (alice, bob) = (PlayerId::from_index(0), PlayerId::from_index(1));
    assert_eq!(
        out,
        vec![("Mogg Assassin", true, vec![alice, bob])],
        "CR 602.3b: the controller chooses first, then the other player"
    );
}

#[test]
fn cr_601_1a_playing_an_exiled_card_means_land_play_or_cast() {
    use ironsmith::{LegalAction, ManaSymbol, Zone};
    let impulse = p06_compile(
        "Act on Impulse",
        "Mana cost: {0}\nType: Sorcery\nExile the top three cards of your library. Until end of turn, you may play those cards.",
    );
    assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&impulse));
    let land = p06_compile("Test Mountain", "Type: Basic Land — Mountain\n({T}: Add {R}.)");
    let shock = p06_compile("Shock bolt", "Mana cost: {R}\nType: Instant\nThis spell deals 2 damage to any target.");
    let (mut game, alice, _bob) = p06_two_player();
    p06_main_phase(&mut game, alice, alice);
    let l = game.create_object_from_definition(&land, alice, Zone::Library);
    let s = game.create_object_from_definition(&shock, alice, Zone::Library);
    let card = game.create_object_from_definition(&impulse, alice, Zone::Hand);
    let land_stable = game.object(l).unwrap().stable_id;
    let shock_stable = game.object(s).unwrap().stable_id;
    let action = p06_cast_actions(&game, alice, card).into_iter().next().expect("fixture castable");
    let mut dm = P06Dm::default();
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    p06_complete_action(&mut game, alice, action, &mut dm, &mut queue).unwrap();
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    game.turn.priority_player = Some(alice);
    game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Red, 1);
    let l = game.find_object_by_stable_id(land_stable).unwrap();
    let s = game.find_object_by_stable_id(shock_stable).unwrap();
    assert_eq!(game.object(l).unwrap().zone, Zone::Exile);
    let actions = ironsmith::compute_legal_actions(&game, alice).unwrap();
    let land_play = actions.iter().any(|a| matches!(a, LegalAction::PlayLand { land_id } if *land_id == l));
    let land_cast = actions.iter().any(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == l));
    let spell_cast = actions.iter().any(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == s));
    assert_eq!(
        (land_play, land_cast, spell_cast),
        (true, false, true),
        "CR 601.1a: \"play\" a card means playing a land card as a land and casting a spell card"
    );
}

#[test]
fn cr_601_8_602_4_cost_changes_do_not_affect_objects_already_on_the_stack() {
    use ironsmith::{ManaSymbol, Zone};
    let bolt = p06_compile("Lightning Bolt", "Mana cost: {R}\nType: Instant\nLightning Bolt deals 3 damage to any target.");
    let pinger = p06_compile("Pinger", "Mana cost: {0}\nType: Artifact\n{1}: This artifact deals 1 damage to any target.");
    let thalia_like = p06_compile(
        "Tax enchantment",
        "Mana cost: {0}\nType: Enchantment\nNoncreature spells cost {1} more to cast.\nActivated abilities cost {2} more to activate unless they're mana abilities.",
    );
    let (mut game, alice, bob) = p06_two_player();
    p06_main_phase(&mut game, alice, alice);
    let card = game.create_object_from_definition(&bolt, alice, Zone::Hand);
    let pinger = game.create_object_from_definition(&pinger, alice, Zone::Battlefield);
    game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Red, 2);
    let mut dm = P06Dm { prefer_target: Some(ironsmith::Target::Player(bob)), ..Default::default() };
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let a = p06_cast_actions(&game, alice, card).into_iter().next().unwrap();
    p06_complete_action(&mut game, alice, a, &mut dm, &mut queue).unwrap();
    game.turn.priority_player = Some(alice);
    let a = p06_activate_actions(&game, alice, pinger).into_iter().next().unwrap();
    p06_complete_action(&mut game, alice, a, &mut dm, &mut queue).unwrap();
    assert_eq!(game.stack.len(), 2);
    assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
    // The cost increase arrives while both are on the stack.
    game.create_object_from_definition(&thalia_like, bob, Zone::Battlefield);
    game.refresh_continuous_state().unwrap();
    while !game.stack.is_empty() {
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    }
    assert_eq!(
        game.player(bob).unwrap().life,
        16,
        "CR 601.8/602.4: a cost-altering effect doesn't affect a spell or ability already on the stack; both resolve"
    );
}

#[test]
fn cr_602_1e_activation_cost_payment_modification_applies_to_the_total_cost() {
    use ironsmith::Zone;
    let epic = p06_compile(
        "Heirloom Epic",
        "Mana cost: {1}\nType: Artifact — Book\n{4}, {T}: Draw a card. For each mana in this ability's activation cost, you may tap an untapped creature you control rather than pay that mana. Activate only as a sorcery.",
    );
    let field = p06_compile(
        "Suppression Field",
        "Mana cost: {1}{W}\nType: Enchantment\nActivated abilities cost {2} more to activate unless they're mana abilities.",
    );
    let supported = !ironsmith::cards::generated_definition_has_unimplemented_content(&epic);
    let mut results = Vec::new();
    for creatures in [5usize, 6] {
        let (mut game, alice, bob) = p06_two_player();
        p06_main_phase(&mut game, alice, alice);
        let source = game.create_object_from_definition(&epic, alice, Zone::Battlefield);
        game.create_object_from_definition(&field, bob, Zone::Battlefield);
        for i in 0..creatures {
            p06_bear(&mut game, alice, &format!("Helper {i}"));
        }
        let mut library_card = p06_compile("Filler", "Type: Basic Land — Plains");
        let _ = &mut library_card;
        game.create_object_from_definition(&library_card, alice, Zone::Library);
        game.refresh_continuous_state().unwrap();
        results.push((creatures, p06_can_activate(&game, alice, source)));
    }
    assert_eq!(
        (supported, results),
        (true, vec![(5, false), (6, true)]),
        "CR 602.1e: \"tap a creature rather than pay mana\" for this ability's activation cost applies to the total cost including the {{2}} increase (6 creatures pay {{6}})"
    );
}

#[test]
fn cr_602_5b_once_each_turn_restriction_survives_a_control_change() {
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::{Until, Zone};
    let relic = p06_compile(
        "Tally relic",
        "Mana cost: {0}\nType: Artifact\n{0}: You gain 1 life. Activate only once each turn.",
    );
    let (mut game, alice, bob) = p06_two_player();
    p06_main_phase(&mut game, alice, alice);
    let source = game.create_object_from_definition(&relic, alice, Zone::Battlefield);
    let a = p06_activate_actions(&game, alice, source).into_iter().next().expect("fixture");
    let mut dm = P06Dm::default();
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    p06_complete_action(&mut game, alice, a, &mut dm, &mut queue).unwrap();
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    let again_alice = p06_can_activate(&{ let mut g = game.clone(); g.turn.priority_player = Some(alice); g }, alice, source);
    ApplyContinuousEffect::new(EffectTarget::Specific(source), Modification::ChangeController(bob), Until::EndOfTurn)
        .execute(&mut game, &mut EffectContext::new_default(source, bob))
        .unwrap();
    game.refresh_continuous_state().unwrap();
    game.turn.priority_player = Some(bob);
    let bob_can = p06_can_activate(&game, bob, source);
    assert_eq!(
        (again_alice, bob_can),
        (false, false),
        "CR 602.5b: an \"activate only once each turn\" restriction continues to apply to the object after its controller changes"
    );
}

#[test]
fn cr_602_5c_restriction_on_acquired_ability_is_separate_from_identical_printed_ability() {
    use ironsmith::continuous::{EffectTarget, Modification};
    use ironsmith::effects::ApplyContinuousEffect;
    use ironsmith::{LegalAction, Until, Zone};
    let relic = p06_compile(
        "Tally relic",
        "Mana cost: {0}\nType: Artifact\n{0}: You gain 1 life. Activate only once each turn.",
    );
    let (mut game, alice, _bob) = p06_two_player();
    p06_main_phase(&mut game, alice, alice);
    let printed = game.create_object_from_definition(&relic, alice, Zone::Battlefield);
    let granter = game.create_object_from_definition(&relic, alice, Zone::Battlefield);
    let ability = game
        .object(granter)
        .unwrap()
        .abilities_vec()
        .into_iter()
        .find(|a| matches!(a.kind, ironsmith::ability::AbilityKind::Activated(_)))
        .expect("fixture activated ability");
    ApplyContinuousEffect::new(EffectTarget::Specific(printed), Modification::AddAbilityGeneric(ability), Until::EndOfTurn)
        .execute(&mut game, &mut EffectContext::new_default(granter, alice))
        .unwrap();
    game.refresh_continuous_state().unwrap();
    let indices = |game: &GameState| -> Vec<usize> {
        ironsmith::compute_legal_actions(game, alice)
            .unwrap()
            .into_iter()
            .filter_map(|a| match a {
                LegalAction::ActivateAbility { source, ability_index } if source == printed => Some(ability_index),
                _ => None,
            })
            .collect()
    };
    let before = indices(&game);
    assert_eq!(before.len(), 2, "fixture: printed and acquired abilities are both available");
    let a = LegalAction::ActivateAbility { source: printed, ability_index: before[0] };
    let mut dm = P06Dm::default();
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    p06_complete_action(&mut game, alice, a, &mut dm, &mut queue).unwrap();
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    game.turn.priority_player = Some(alice);
    let after = indices(&game);
    assert_eq!(
        after,
        vec![before[1]],
        "CR 602.5c: using the printed once-per-turn ability doesn't restrict the identically worded acquired ability"
    );
}

#[test]
fn cr_602_5e_activate_only_as_an_instant_uses_instant_timing() {
    use ironsmith::{Phase, Step, Zone};
    let relic = p06_compile(
        "Brass relic",
        "Mana cost: {0}\nType: Artifact\n{0}: You gain 1 life. Activate only as an instant.",
    );
    let filler = p06_compile("Filler instant", "Mana cost: {0}\nType: Instant\nYou gain 1 life.");
    let (mut game, alice, bob) = p06_two_player();
    let source = game.create_object_from_definition(&relic, alice, Zone::Battlefield);
    let mut results = Vec::new();
    // Opponent's upkeep with an object on the stack: instant timing allows it.
    game.turn.active_player = bob;
    game.turn.phase = Phase::Beginning;
    game.turn.step = Some(Step::Upkeep);
    game.turn.priority_player = Some(bob);
    let f = game.create_object_from_definition(&filler, bob, Zone::Hand);
    let a = p06_cast_actions(&game, bob, f).into_iter().next().unwrap();
    let mut dm = P06Dm::default();
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    p06_complete_action(&mut game, bob, a, &mut dm, &mut queue).unwrap();
    game.turn.priority_player = Some(alice);
    results.push(p06_can_activate(&game, alice, source));
    assert_eq!(
        results,
        vec![true],
        "CR 602.5e: \"activate only as an instant\" follows instant timing: any time the player has priority, even on an opponent's turn with a nonempty stack"
    );
}

#[test]
fn cr_601_4_kicker_choice_allows_choosing_any_number_of_modes() {
    use ironsmith::{ManaSymbol, Zone};
    let inscription = p06_compile(
        "Inscription of Abundance",
        "Mana cost: {1}{G}\nType: Instant\nKicker {2}{G}\nChoose one. If this spell was kicked, choose any number instead.\n• Put two +1/+1 counters on target creature.\n• Target player gains X life, where X is the greatest power among creatures they control.\n• Target creature you control fights target creature you don't control.",
    );
    assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&inscription));
    struct Kick(P06Dm, Vec<usize>);
    impl DecisionMaker for Kick {
        fn decide_boolean(&mut self, _g: &GameState, _c: &ironsmith::decisions::context::BooleanContext) -> bool { true }
        fn decide_options(&mut self, g: &GameState, c: &SelectOptionsContext) -> Vec<usize> {
            if c.options.len() == 3 && c.options.iter().any(|o| o.description.contains("+1/+1")) {
                self.1.push(c.max);
                return c.options.iter().filter(|o| o.legal).map(|o| o.index).take(c.max).collect();
            }
            if let Some(o) = c.options.iter().find(|o| o.legal && o.description.to_lowercase().contains("kick")) {
                return vec![o.index];
            }
            self.0.decide_options(g, c)
        }
        fn decide_targets(&mut self, g: &GameState, c: &ironsmith::decisions::context::TargetsContext) -> Vec<ironsmith::Target> { self.0.decide_targets(g, c) }
        fn decide_number(&mut self, g: &GameState, c: &ironsmith::decisions::context::NumberContext) -> u32 { self.0.decide_number(g, c) }
    }
    let (mut game, alice, bob) = p06_two_player();
    p06_main_phase(&mut game, alice, alice);
    p06_bear(&mut game, alice, "Alice creature");
    p06_bear(&mut game, bob, "Bob creature");
    let card = game.create_object_from_definition(&inscription, alice, Zone::Hand);
    game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Green, 5);
    let actions = p06_cast_actions(&game, alice, card);
    let mut max_seen = Vec::new();
    for action in actions {
        let mut g = game.clone();
        let mut dm = Kick(P06Dm::default(), Vec::new());
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let _ = p06_complete_action(&mut g, alice, action, &mut dm, &mut queue);
        max_seen.extend(dm.1);
    }
    assert!(
        max_seen.iter().any(|m| *m >= 2),
        "CR 601.4: when the kicker will be paid the caster may choose any number of modes; observed mode-choice maxima {max_seen:?}"
    );
}


#[test]
fn cr_601_3d_spell_with_conditional_flash_may_be_cast_while_condition_is_met() {
    use ironsmith::{ManaSymbol, Zone};
    let tide = p06_compile(
        "Crashing Tide",
        "Mana cost: {2}{U}\nType: Sorcery\nThis spell has flash as long as you control a Merfolk.\nReturn target creature to its owner's hand.\nDraw a card.",
    );
    let merfolk = p06_compile("Merfolk scout", "Mana cost: {U}\nType: Creature — Merfolk\nPower/Toughness: 1/1");
    assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&tide));
    let mut results = Vec::new();
    for with_merfolk in [false, true] {
        let (mut game, alice, bob) = p06_two_player();
        p06_main_phase(&mut game, bob, alice);
        if with_merfolk {
            game.create_object_from_definition(&merfolk, alice, Zone::Battlefield);
        }
        p06_bear(&mut game, bob, "Bob creature");
        let card = game.create_object_from_definition(&tide, alice, Zone::Hand);
        game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Blue, 3);
        game.refresh_continuous_state().unwrap();
        results.push((with_merfolk, !p06_cast_actions(&game, alice, card).is_empty()));
    }
    assert_eq!(
        results,
        vec![(false, false), (true, true)],
        "CR 601.3d: a spell that has flash while a condition is met may be cast at instant speed while that condition is met"
    );
}

#[test]
fn cr_602_3a_caster_picks_which_opponent_makes_the_activation_choice() {
    use ironsmith::Zone;
    let echo = p06_compile(
        "Echo Chamber",
        "Mana cost: {4}\nType: Artifact\n{0}, {T}: An opponent chooses target creature they control. Create a token that's a copy of that creature. That token gains haste until end of turn. Exile the token at the beginning of the next end step. Activate only as a sorcery.",
    );
    let mut game = p06_combat_game(3);
    let (alice, bob, carol) = (PlayerId::from_index(0), PlayerId::from_index(1), PlayerId::from_index(2));
    p06_main_phase(&mut game, alice, alice);
    let source = game.create_object_from_definition(&echo, alice, Zone::Battlefield);
    p06_bear(&mut game, bob, "Bob creature");
    p06_bear(&mut game, carol, "Carol creature");
    let action = p06_activate_actions(&game, alice, source).into_iter().next().expect("fixture activatable");
    let mut dm = P06Dm::default();
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let result = p06_complete_action(&mut game, alice, action, &mut dm, &mut queue);
    let alice_chose_opponent = dm.option_players.iter().any(|(p, _)| *p == alice);
    let opponent_targeted = dm.target_players.iter().all(|p| *p == bob || *p == carol) && !dm.target_players.is_empty();
    assert!(
        result.is_ok() && alice_chose_opponent && opponent_targeted,
        "CR 602.3a: with several opponents the controller decides which opponent chooses the target; result={result:?}, option choosers={:?}, target choosers={:?}",
        dm.option_players,
        dm.target_players
    );
}

#[test]
fn cr_601_7b_controller_chooses_before_the_other_player_while_casting() {
    use ironsmith::{ManaSymbol, Zone};
    let def = p06_compile(
        "Wager of blades",
        "Mana cost: {R}\nType: Instant\nYou choose target creature an opponent controls, and that opponent chooses target creature. Flip a coin. If you win the flip, destroy the creature you chose. If you lose the flip, destroy the creature your opponent chose.",
    );
    let (mut game, alice, bob) = p06_two_player();
    p06_main_phase(&mut game, alice, alice);
    p06_bear(&mut game, alice, "Alice creature");
    p06_bear(&mut game, bob, "Bob creature");
    let card = game.create_object_from_definition(&def, alice, Zone::Hand);
    game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Red, 1);
    let mut dm = P06Dm::default();
    if let Some(action) = p06_cast_actions(&game, alice, card).into_iter().next() {
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let _ = p06_complete_action(&mut game, alice, action, &mut dm, &mut queue);
    }
    assert_eq!(
        dm.target_players,
        vec![alice, bob],
        "CR 601.7b: the spell's controller chooses its target first, then the opponent chooses theirs"
    );
}
