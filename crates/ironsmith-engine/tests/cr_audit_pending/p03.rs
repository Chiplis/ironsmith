//! CR audit (pending-entry pass, package p03): assertions express the rules, so confirmed defects fail.
//! Do not use `should_panic`, `ignore`, or assertions of current broken behavior.

#![allow(unused_imports, dead_code)]

use ironsmith::{EffectContext, EffectExecutor, GameState, PlayerFilter, PlayerId};

use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction};
use ironsmith::{ObjectId, Phase, Step, Zone};

fn p03_compile(name: &str, text: &str) -> CardDefinition {
    ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false)
        .unwrap_or_else(|error| panic!("fixture card {name} must compile: {error:?}"))
}

fn p03_game() -> (GameState, PlayerId, PlayerId) {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    (game, alice, bob)
}

fn p03_creature(game: &mut GameState, owner: PlayerId, name: &str, p: i32, t: i32) -> ObjectId {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::{CardId, CardType, PowerToughness};
    let definition = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(p, t))
        .build();
    game.create_object_from_definition(&definition, owner, Zone::Battlefield)
}

fn p03_priority_player(progress: &GameProgress) -> Option<PlayerId> {
    match progress {
        GameProgress::NeedsDecisionCtx(ironsmith::decisions::context::DecisionContext::Priority(
            ctx,
        )) => Some(ctx.player),
        _ => None,
    }
}

/// Run combat damage for a fixed set of attack and block declarations.
fn p03_combat_damage(
    game: &mut GameState,
    attackers: &[(ObjectId, PlayerId)],
    blocks: &[(ObjectId, ObjectId)],
    defending: PlayerId,
) -> usize {
    use ironsmith::combat_state::{self, AttackTarget};
    use ironsmith::{AttackerDeclaration, BlockerDeclaration, TriggerQueue};
    for (attacker, _) in attackers {
        game.remove_summoning_sickness(*attacker);
    }
    for (blocker, _) in blocks {
        game.remove_summoning_sickness(*blocker);
    }
    let mut queue = TriggerQueue::new();
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(Step::DeclareAttackers);
    let mut combat = combat_state::new_combat();
    let declarations: Vec<_> = attackers
        .iter()
        .map(|(creature, player)| AttackerDeclaration {
            creature: *creature,
            target: AttackTarget::Player(*player),
        })
        .collect();
    ironsmith::game_loop::apply_attacker_declarations(game, &mut combat, &mut queue, &declarations)
        .expect("legal attack declaration");
    game.turn.step = Some(Step::DeclareBlockers);
    let blockers: Vec<_> = blocks
        .iter()
        .map(|(blocker, attacker)| BlockerDeclaration {
            blocker: *blocker,
            blocking: *attacker,
        })
        .collect();
    ironsmith::game_loop::apply_blocker_declarations(
        game,
        &mut combat,
        &mut queue,
        &blockers,
        defending,
    )
    .expect("legal block declaration");
    game.turn.step = Some(Step::CombatDamage);
    ironsmith::game_loop::execute_combat_damage_step(game, &combat, false).len()
}

// ---------------------------------------------------------------------------
// CR 117: timing and priority
// ---------------------------------------------------------------------------

#[test]
fn cr_117_1_117_3d_priority_passes_in_turn_order_and_two_passes_end_the_step() {
    use ironsmith::{PriorityLoopState, PriorityResponse, TriggerQueue};
    for players in 2..=4usize {
        let mut game = GameState::new((0..players).map(|i| format!("P{i}")).collect(), 20);
        let active = PlayerId::from_index(1 % players as u8);
        game.turn.active_player = active;
        game.turn.priority_player = Some(active);
        game.turn.phase = Phase::FirstMain;
        let mut queue = TriggerQueue::new();
        let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
        let first = ironsmith::game_loop::advance_priority_with_dm(&mut game, &mut queue, &mut dm)
            .unwrap();
        assert_eq!(
            p03_priority_player(&first),
            Some(active),
            "CR 117.1/117.3a: the active player is the one given priority"
        );
        let mut state = PriorityLoopState::new(players);
        let mut seen = vec![active];
        let mut ended = false;
        for pass in 1..=players {
            let progress = ironsmith::game_loop::apply_priority_response_with_dm(
                &mut game,
                &mut queue,
                &mut state,
                &PriorityResponse::PriorityAction(LegalAction::PassPriority),
                &mut dm,
            )
            .unwrap();
            if pass < players {
                let next = PlayerId::from_index(((active.index() + pass) % players) as u8);
                assert_eq!(
                    game.turn.priority_player,
                    Some(next),
                    "CR 117.3d: after a pass the next player in turn order receives priority ({players} players, pass {pass})"
                );
                seen.push(next);
            } else {
                ended = matches!(progress, GameProgress::Continue);
            }
        }
        assert!(
            ended,
            "CR 117.4/117.3d: all players passing in succession with an empty stack ends the step"
        );
        assert_eq!(seen.len(), players);
    }
}

#[test]
fn cr_117_3b_117_7_newest_object_resolves_first_and_active_player_then_gets_priority() {
    use ironsmith::game_state::StackEntry;
    use ironsmith::{Effect, PriorityLoopState, PriorityResponse, TriggerQueue};
    for responder_first in [false, true] {
        let (mut game, alice, bob) = p03_game();
        let source_a = p03_creature(&mut game, alice, "Alice source", 1, 1);
        let source_b = p03_creature(&mut game, bob, "Bob source", 1, 1);
        // Bob's ability was put on the stack first, Alice responded with hers.
        game.push_to_stack(StackEntry::ability(source_b, bob, vec![Effect::gain_life(1)]));
        game.push_to_stack(StackEntry::ability(source_a, alice, vec![Effect::gain_life(10)]));
        let start = if responder_first { bob } else { alice };
        game.turn.priority_player = Some(start);
        let mut queue = TriggerQueue::new();
        let mut state = PriorityLoopState::new(2);
        let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
        for _ in 0..2 {
            ironsmith::game_loop::apply_priority_response_with_dm(
                &mut game,
                &mut queue,
                &mut state,
                &PriorityResponse::PriorityAction(LegalAction::PassPriority),
                &mut dm,
            )
            .unwrap();
        }
        assert_eq!(
            (game.player(alice).unwrap().life, game.player(bob).unwrap().life, game.stack.len()),
            (30, 20, 1),
            "CR 117.7: the object put on the stack in response resolves first"
        );
        let progress =
            ironsmith::game_loop::advance_priority_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        assert_eq!(
            p03_priority_player(&progress),
            Some(alice),
            "CR 117.3b: the active player receives priority after resolution (start={start:?})"
        );
    }
}

#[test]
fn cr_117_2a_117_2d_117_5_state_based_actions_then_triggers_before_priority() {
    use ironsmith::effects::DealDamageEffect;
    use ironsmith::{ChooseSpec, TriggerQueue};
    let (mut game, alice, _bob) = p03_game();
    let definition = p03_compile(
        "Dying Observer",
        "Mana cost: {1}{W}\nType: Creature — Spirit\nPower/Toughness: 1/1\nWhen this creature dies, you gain 3 life.",
    );
    let creature = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let source = p03_creature(&mut game, alice, "Pinger", 1, 1);
    let mut ctx = EffectContext::new_default(source, alice);
    DealDamageEffect::new(1, ChooseSpec::SpecificObject(creature))
        .execute(&mut game, &mut ctx)
        .unwrap();
    assert!(
        game.object(creature).is_some_and(|o| o.zone == Zone::Battlefield) && game.stack.is_empty(),
        "CR 117.2d: lethal damage is not checked until a player would receive priority"
    );
    let mut queue = TriggerQueue::new();
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    let progress =
        ironsmith::game_loop::advance_priority_with_dm(&mut game, &mut queue, &mut dm).unwrap();
    assert!(
        game.object(creature).is_none_or(|o| o.zone != Zone::Battlefield),
        "CR 117.5: the state-based action is performed before priority"
    );
    assert_eq!(
        game.stack.len(),
        1,
        "CR 117.2a/117.5: the dies trigger is put on the stack before the player receives priority"
    );
    assert_eq!(p03_priority_player(&progress), Some(alice));
    assert_eq!(game.player(alice).unwrap().life, 20, "nothing happens when the ability triggers");
}

#[test]
fn cr_117_2b_static_ability_applies_without_using_the_stack() {
    let (mut game, alice, _bob) = p03_game();
    let creature = p03_creature(&mut game, alice, "Bear", 2, 2);
    let anthem = p03_compile(
        "Glorious Anthem",
        "Mana cost: {1}{W}{W}\nType: Enchantment\nCreatures you control get +1/+1.",
    );
    game.create_object_from_definition(&anthem, alice, Zone::Battlefield);
    assert!(game.stack.is_empty());
    assert_eq!(
        (game.calculated_power(creature), game.calculated_toughness(creature)),
        (Some(3), Some(3)),
        "CR 117.2b: a static ability affects the game continuously, without priority or the stack"
    );
}

#[test]
fn cr_117_2c_117_3a_turn_based_actions_and_beginning_triggers_precede_priority() {
    use ironsmith::{TriggerQueue, TurnAction, TurnRunner};
    let (mut game, alice, _bob) = p03_game();
    game.turn.turn_number = 3;
    let filler = p03_compile("Library Filler", "Mana cost: {G}\nType: Creature — Bear\nPower/Toughness: 2/2");
    for _ in 0..5 {
        game.create_object_from_definition(&filler, alice, Zone::Library);
    }
    let upkeep = p03_compile(
        "Upkeep Watcher",
        "Mana cost: {W}\nType: Enchantment\nAt the beginning of your upkeep, you gain 1 life.",
    );
    game.create_object_from_definition(&upkeep, alice, Zone::Battlefield);
    let hand_before = game.player(alice).unwrap().hand.len();
    let mut runner = TurnRunner::new();
    let mut queue = TriggerQueue::new();
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    let mut windows: Vec<(Option<Step>, usize, usize, Option<PlayerId>)> = Vec::new();
    for _ in 0..200 {
        match runner.advance(&mut game, &mut queue).unwrap() {
            TurnAction::Continue => {}
            TurnAction::RunPriority => {
                let step = game.turn.step;
                let progress =
                    ironsmith::game_loop::advance_priority_with_dm(&mut game, &mut queue, &mut dm)
                        .unwrap();
                windows.push((
                    step,
                    game.stack.len(),
                    game.player(alice).unwrap().hand.len(),
                    p03_priority_player(&progress),
                ));
                ironsmith::game_loop::run_priority_loop_with(&mut game, &mut queue, &mut dm)
                    .unwrap();
                runner.priority_done();
                if windows.len() >= 2 {
                    break;
                }
            }
            other => panic!("unexpected turn action in fixture: {other:?}"),
        }
    }
    assert_eq!(windows.len(), 2, "upkeep and draw priority windows must be reached");
    assert_eq!(
        windows[0],
        (Some(Step::Upkeep), 1, hand_before, Some(alice)),
        "CR 117.3a: no priority in untap; in upkeep the beginning trigger is on the stack before the active player gets priority"
    );
    assert_eq!(
        windows[1],
        (Some(Step::Draw), 0, hand_before + 1, Some(alice)),
        "CR 117.2c/117.3a: the draw turn-based action happens before the active player receives priority"
    );
}

#[test]
fn cr_117_2e_choices_during_resolution_are_not_priority_decisions() {
    use ironsmith::game_state::StackEntry;
    struct Recorder {
        booleans: usize,
        priorities: usize,
    }
    impl DecisionMaker for Recorder {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            _ctx: &ironsmith::decisions::context::BooleanContext,
        ) -> bool {
            self.booleans += 1;
            true
        }
        fn decide_priority(
            &mut self,
            _game: &GameState,
            _ctx: &ironsmith::decisions::context::PriorityContext,
        ) -> LegalAction {
            self.priorities += 1;
            LegalAction::PassPriority
        }
    }
    let (mut game, alice, _bob) = p03_game();
    let filler = p03_compile("Library Filler", "Mana cost: {G}\nType: Creature — Bear\nPower/Toughness: 2/2");
    game.create_object_from_definition(&filler, alice, Zone::Library);
    let spell = p03_compile(
        "Optional Draw",
        "Mana cost: {U}\nType: Sorcery\nYou may draw a card. If you do, you gain 2 life.",
    );
    let spell = game.create_object_from_definition(&spell, alice, Zone::Stack);
    game.push_to_stack(StackEntry::new(spell, alice));
    let mut dm = Recorder { booleans: 0, priorities: 0 };
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    assert_eq!(game.player(alice).unwrap().life, 22);
    assert_eq!(
        (dm.booleans, dm.priorities),
        (1, 0),
        "CR 117.2e: a resolving spell asks for a choice, but no player has priority during resolution"
    );
}

// ---------------------------------------------------------------------------
// CR 120: damage
// ---------------------------------------------------------------------------

#[test]
fn cr_120_1_120_1a_120_2_any_object_deals_damage_but_only_to_damageable_recipients() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::effects::DealDamageEffect;
    use ironsmith::{CardId, CardType, ChooseSpec};
    let (mut game, alice, bob) = p03_game();
    // A land is a legal damage source (CR 120.2).
    let land = CardDefinitionBuilder::new(CardId::new(), "Damage-dealing land")
        .card_types(vec![CardType::Land])
        .build();
    let source = game.create_object_from_definition(&land, alice, Zone::Battlefield);
    let victim = p03_creature(&mut game, bob, "Victim", 2, 5);
    let artifact = CardDefinitionBuilder::new(CardId::new(), "Plain artifact")
        .card_types(vec![CardType::Artifact])
        .build();
    let artifact = game.create_object_from_definition(&artifact, bob, Zone::Battlefield);
    let mut ctx = EffectContext::new_default(source, alice);
    DealDamageEffect::new(2, ChooseSpec::SpecificObject(victim))
        .execute(&mut game, &mut ctx)
        .unwrap();
    DealDamageEffect::new(3, ChooseSpec::Player(PlayerFilter::Specific(bob)))
        .execute(&mut game, &mut ctx)
        .unwrap();
    let _ = DealDamageEffect::new(4, ChooseSpec::SpecificObject(artifact)).execute(&mut game, &mut ctx);
    assert_eq!(
        (game.damage_on(victim), game.player(bob).unwrap().life),
        (2, 17),
        "CR 120.1/120.2: a land can deal damage to creatures and players"
    );
    assert_eq!(
        game.damage_on(artifact),
        0,
        "CR 120.1a: damage can't be dealt to a noncreature, nonplaneswalker, nonbattle permanent"
    );
}

#[test]
fn cr_120_4b_120_4c_120_4d_wither_lifelink_prevention_and_boon_reflection_example() {
    use ironsmith::effects::DealDamageEffect;
    use ironsmith::game_state::{StackEntry, Target};
    use ironsmith::{Ability, ChooseSpec, CounterType, StaticAbility};
    let (mut game, alice, bob) = p03_game();
    let boon = p03_compile(
        "Boon Reflection",
        "Mana cost: {4}{W}\nType: Enchantment\nIf you would gain life, you gain twice that much life instead.",
    );
    game.create_object_from_definition(&boon, alice, Zone::Battlefield);
    let attacker = {
        use ironsmith::cards::builders::CardDefinitionBuilder;
        use ironsmith::{CardId, CardType, PowerToughness};
        let definition = CardDefinitionBuilder::new(CardId::new(), "Wither lifelink 3/3")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(3, 3))
            .with_ability(Ability::static_ability(StaticAbility::wither()))
            .with_ability(Ability::static_ability(StaticAbility::lifelink()))
            .build();
        game.create_object_from_definition(&definition, alice, Zone::Battlefield)
    };
    let blocker = p03_creature(&mut game, bob, "Blocking 2/2", 2, 2);
    let shield = p03_compile(
        "Prevention spell",
        "Mana cost: {W}\nType: Instant\nPrevent the next 2 damage that would be dealt to target creature this turn.",
    );
    let shield = game.create_object_from_definition(&shield, bob, Zone::Stack);
    game.push_to_stack(StackEntry::new(shield, bob).with_targets(vec![Target::Object(blocker)]));
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    let mut ctx = EffectContext::new_default(attacker, alice);
    DealDamageEffect::new(3, ChooseSpec::SpecificObject(blocker))
        .execute(&mut game, &mut ctx)
        .unwrap();
    assert_eq!(
        (
            game.counter_count(blocker, CounterType::MinusOneMinusOne),
            game.damage_on(blocker),
            game.player(alice).unwrap().life,
        ),
        (1, 0, 22),
        "CR 120.4 example: 3 damage minus 2 prevented is 1 damage -> one -1/-1 counter and 1 life gained, doubled to 2"
    );
}

#[test]
fn cr_120_2a_302_4a_302_4b_combat_damage_equals_power_and_toughness_is_lethal_threshold() {
    use ironsmith::TriggerQueue;
    let (mut game, alice, bob) = p03_game();
    let attacker = p03_creature(&mut game, alice, "Attacker 3/3", 3, 3);
    let unblocked = p03_creature(&mut game, alice, "Unblocked 4/1", 4, 1);
    let blocker = p03_creature(&mut game, bob, "Blocker 2/3", 2, 3);
    p03_combat_damage(
        &mut game,
        &[(attacker, bob), (unblocked, bob)],
        &[(blocker, attacker)],
        bob,
    );
    assert_eq!(
        (game.damage_on(blocker), game.damage_on(attacker), game.player(bob).unwrap().life),
        (3, 2, 16),
        "CR 120.2a/302.4a: each attacking and blocking creature deals combat damage equal to its power"
    );
    let mut queue = TriggerQueue::new();
    ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
    assert!(
        game.object(blocker).is_none_or(|o| o.zone != Zone::Battlefield),
        "CR 302.4b: damage equal to toughness destroys the creature"
    );
    assert!(
        game.object(attacker).is_some_and(|o| o.zone == Zone::Battlefield),
        "CR 302.4b: damage below toughness does not"
    );
}

#[test]
fn cr_120_3g_toxic_combat_damage_gives_poison_counters_in_addition_to_damage() {
    let (mut game, alice, bob) = p03_game();
    let toxic = p03_compile(
        "Toxic Attacker",
        "Mana cost: {2}{G}\nType: Creature — Phyrexian Rat\nPower/Toughness: 2/2\nToxic 2",
    );
    let attacker = game.create_object_from_definition(&toxic, alice, Zone::Battlefield);
    p03_combat_damage(&mut game, &[(attacker, bob)], &[], bob);
    assert_eq!(
        (game.player(bob).unwrap().life, game.player(bob).unwrap().poison_counters),
        (18, 2),
        "CR 120.3g: toxic combat damage to a player causes life loss and poison counters equal to the toxic value"
    );
}

// ---------------------------------------------------------------------------
// CR 2xx / 3xx: card parts, types and subtypes
// ---------------------------------------------------------------------------

#[test]
fn cr_205_3b_302_3_one_and_two_word_creature_subtypes_are_separate_subtypes() {
    use ironsmith::types::Subtype;
    let goblin = p03_compile(
        "Goblin Wizard Test",
        "Mana cost: {1}{R}\nType: Creature — Goblin Wizard\nPower/Toughness: 1/1",
    );
    assert_eq!(
        goblin.card.subtypes,
        vec![Subtype::Goblin, Subtype::Wizard],
        "CR 302.3: 'Creature — Goblin Wizard' has the subtypes Goblin and Wizard"
    );
    let doctor = p03_compile(
        "Time Lord Doctor Test",
        "Mana cost: {2}{W}\nType: Legendary Creature — Time Lord Doctor\nPower/Toughness: 2/3",
    );
    assert_eq!(
        doctor.card.subtypes,
        vec![Subtype::TimeLord, Subtype::Doctor],
        "CR 205.3b: the two-word creature type Time Lord is one subtype"
    );
    let equipment = p03_compile(
        "Equipment Vehicle Test",
        "Mana cost: {2}\nType: Artifact — Equipment Vehicle\nCrew 1\nEquip {1}",
    );
    assert_eq!(
        equipment.card.subtypes,
        vec![Subtype::Equipment, Subtype::Vehicle],
        "CR 205.3b/301.3: each word after the dash is a separate artifact subtype"
    );
}

#[test]
fn cr_205_1b_in_addition_to_its_other_types_retains_prior_types_and_subtypes() {
    use ironsmith::game_state::{StackEntry, Target};
    use ironsmith::types::Subtype;
    use ironsmith::CardType;
    let (mut game, alice, _bob) = p03_game();
    let bear = p03_compile("Grizzly Bears", "Mana cost: {1}{G}\nType: Creature — Bear\nPower/Toughness: 2/2");
    let bear = game.create_object_from_definition(&bear, alice, Zone::Battlefield);
    let spell = p03_compile(
        "Type adder",
        "Mana cost: {U}\nType: Instant\nTarget creature becomes an artifact in addition to its other types until end of turn.",
    );
    let spell = game.create_object_from_definition(&spell, alice, Zone::Stack);
    game.push_to_stack(StackEntry::new(spell, alice).with_targets(vec![Target::Object(bear)]));
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    let chars = game.calculated_characteristics(bear).unwrap();
    assert!(
        chars.card_types.contains(&CardType::Artifact)
            && chars.card_types.contains(&CardType::Creature)
            && chars.subtypes.contains(&Subtype::Bear),
        "CR 205.1b: the creature keeps its prior types and subtypes, got types {:?} subtypes {:?}",
        chars.card_types,
        chars.subtypes
    );
}

#[test]
fn cr_300_2_artifact_creature_is_subject_to_artifact_and_creature_effects() {
    use ironsmith::game_state::{StackEntry, Target};
    for (index, text) in [
        "Mana cost: {1}{R}\nType: Instant\nDestroy target artifact.",
        "Mana cost: {1}{B}\nType: Instant\nDestroy target creature.",
    ]
    .into_iter()
    .enumerate()
    {
        let (mut game, alice, bob) = p03_game();
        let golem = p03_compile(
            "Artifact Golem",
            "Mana cost: {3}\nType: Artifact Creature — Golem\nPower/Toughness: 3/3",
        );
        let golem = game.create_object_from_definition(&golem, bob, Zone::Battlefield);
        let spell = p03_compile(&format!("Removal {index}"), text);
        let spell = game.create_object_from_definition(&spell, alice, Zone::Stack);
        game.push_to_stack(StackEntry::new(spell, alice).with_targets(vec![Target::Object(golem)]));
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert!(
            game.object(golem).is_none_or(|o| o.zone != Zone::Battlefield),
            "CR 300.2: an artifact creature is affected by effects that affect either type (case {index})"
        );
    }
}

#[test]
fn cr_301_4_artifacts_may_be_colored_and_colorless_objects_need_not_be_artifacts() {
    use ironsmith::{Color, ColorSet};
    let (mut game, alice, _bob) = p03_game();
    let white_artifact = p03_compile(
        "White Artifact",
        "Mana cost: {1}{W}\nType: Artifact\n{T}: You gain 1 life.",
    );
    let white_artifact = game.create_object_from_definition(&white_artifact, alice, Zone::Battlefield);
    let eldrazi = p03_compile(
        "Colorless Eldrazi",
        "Mana cost: {4}\nType: Creature — Eldrazi\nPower/Toughness: 4/4",
    );
    let eldrazi = game.create_object_from_definition(&eldrazi, alice, Zone::Battlefield);
    let white = game.calculated_characteristics(white_artifact).unwrap().colors;
    let colorless = game.calculated_characteristics(eldrazi).unwrap().colors;
    assert!(white.contains(Color::White), "CR 301.4: an artifact with {{W}} in its cost is white");
    assert_eq!(colorless, ColorSet::default(), "CR 301.4: a creature with only generic mana is colorless");
}

#[test]
fn cr_304_2_instant_follows_its_text_then_goes_to_owners_graveyard() {
    use ironsmith::game_state::StackEntry;
    let (mut game, alice, bob) = p03_game();
    let spell = p03_compile("Healing Salve Test", "Mana cost: {W}\nType: Instant\nYou gain 3 life.");
    // Bob controls a spell Alice owns: it still goes to Alice's graveyard.
    let spell = game.create_object_from_definition(&spell, alice, Zone::Stack);
    game.push_to_stack(StackEntry::new(spell, bob));
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    assert_eq!(game.player(bob).unwrap().life, 23, "CR 304.2: the instant's actions are followed");
    let graveyard = &game.player(alice).unwrap().graveyard;
    assert!(
        graveyard.len() == 1 && game.stack.is_empty(),
        "CR 304.2: the resolved instant is put into its owner's graveyard"
    );
}

#[test]
fn cr_303_7_303_7a_only_the_newest_role_from_one_controller_remains() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::object::AttachmentTarget;
    use ironsmith::types::Subtype;
    use ironsmith::{CardId, CardType, TriggerQueue};
    let (mut game, alice, bob) = p03_game();
    let host = p03_creature(&mut game, alice, "Role host", 2, 2);
    let role = |name: &str| {
        p03_compile(
            name,
            "Type: Enchantment — Aura Role\nEnchant creature\nEnchanted creature gets +1/+1.",
        )
    };
    let old = game.create_object_from_definition(&role("Old Role"), alice, Zone::Battlefield);
    assert!(game.attach_object_to_target(old, AttachmentTarget::Object(host)));
    let other = game.create_object_from_definition(&role("Bob Role"), bob, Zone::Battlefield);
    assert!(game.attach_object_to_target(other, AttachmentTarget::Object(host)));
    let new = game.create_object_from_definition(&role("New Role"), alice, Zone::Battlefield);
    assert!(game.attach_object_to_target(new, AttachmentTarget::Object(host)));
    assert!(
        game.object(new).unwrap().subtypes.contains(&Subtype::Role),
        "fixture: the compiled Aura has the Role subtype"
    );
    let mut queue = TriggerQueue::new();
    ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut queue).unwrap();
    let on_battlefield = |game: &GameState, id| game.object(id).is_some_and(|o| o.zone == Zone::Battlefield);
    let state = (on_battlefield(&game, old), on_battlefield(&game, other), on_battlefield(&game, new));
    assert!(
        state == (false, true, true),
        "CR 303.7a: the older Role controlled by the same player goes to the graveyard; another player's Role stays; (old, other, new) on battlefield = {state:?}"
    );
    assert_eq!(game.player(alice).unwrap().graveyard.len(), 1, "CR 303.7a: into its owner's graveyard");
}

// ---------------------------------------------------------------------------
// Batch 2: text-box parts, subtypes of planes/dungeons, costs, timing permissions
// ---------------------------------------------------------------------------

/// Take a priority action and answer every follow-up choice with the first
/// legal option until the action is complete.
fn p03_take_action(game: &mut GameState, action: LegalAction) {
    use ironsmith::{PriorityLoopState, PriorityResponse, TriggerQueue};
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut queue = TriggerQueue::new();
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    let mut progress = ironsmith::game_loop::apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    )
    .expect("fixture action must start");
    for _ in 0..32 {
        match &progress {
            GameProgress::NeedsDecisionCtx(ironsmith::decisions::context::DecisionContext::Priority(_)) => return,
            GameProgress::NeedsDecisionCtx(context) => {
                let context = context.clone();
                progress = ironsmith::game_loop::apply_decision_context_with_dm(
                    game, &mut queue, &mut state, &context, &mut dm,
                )
                .expect("fixture action choice must apply");
            }
            _ => return,
        }
    }
    panic!("fixture action did not finish");
}


/// Debug rendering of compiled abilities with presentation-only metadata and
/// per-compile identities (instance ids, linked-exile hashes, display labels)
/// normalized away, so two texts can be compared for rules content.
fn p03_rules_content(abilities: &[ironsmith::ability::Ability], labels: &[&str]) -> String {
    fn strip_between(s: &str, start: &str, end: &str) -> String {
        let mut out = String::new();
        let mut rest = s;
        while let Some(i) = rest.find(start) {
            out.push_str(&rest[..i + start.len()]);
            rest = &rest[i + start.len()..];
            match rest.find(end) {
                Some(j) => {
                    out.push('_');
                    rest = &rest[j..];
                }
                None => break,
            }
        }
        out.push_str(rest);
        out
    }
    let mut text = format!("{abilities:?}");
    for label in labels {
        text = text.replace(&format!("presentation_label: Some(AbilityWord(\"{label}\"))"), "presentation_label: None");
        text = text.replace(&format!("\"__ironsmith_activation_label:{label}\""), "");
    }
    text = strip_between(&text, "LinkedExileDefinition([", "])");
    strip_between(&text, "InstanceId(", ")")
}

#[test]
fn cr_207_2_207_2a_207_2b_reminder_text_has_no_game_function() {
    let plain = p03_compile(
        "Reminder Plain",
        "Mana cost: {1}{W}\nType: Creature — Bird\nPower/Toughness: 1/1\nFlying\nWhen this creature enters, scry 1.",
    );
    let reminded = p03_compile(
        "Reminder Text",
        "Mana cost: {1}{W}\nType: Creature — Bird\nPower/Toughness: 1/1\nFlying (This creature can't be blocked except by creatures with flying or reach.)\nWhen this creature enters, scry 1. (Look at the top card of your library. You may put that card on the bottom.)",
    );
    assert_eq!(
        p03_rules_content(&plain.abilities, &[]),
        p03_rules_content(&reminded.abilities, &[]),
        "CR 207.2a: reminder text summarizes a rule and adds no abilities"
    );
}

#[test]
fn cr_207_2c_ability_word_has_no_rules_meaning() {
    let plain = p03_compile(
        "Ability Word Plain",
        "Mana cost: {1}{G}\nType: Creature — Elf\nPower/Toughness: 1/1\nWhenever a land you control enters, you gain 1 life.",
    );
    let worded = p03_compile(
        "Ability Word Landfall",
        "Mana cost: {1}{G}\nType: Creature — Elf\nPower/Toughness: 1/1\nLandfall — Whenever a land you control enters, you gain 1 life.",
    );
    assert_eq!(
        p03_rules_content(&plain.abilities, &["Landfall"]),
        p03_rules_content(&worded.abilities, &["Landfall"]),
        "CR 207.2c: an ability word does not change the ability"
    );
}

#[test]
fn cr_207_2d_flavor_word_has_no_rules_meaning() {
    let plain = p03_compile(
        "Flavor Word Plain",
        "Mana cost: {2}{U}\nType: Creature — Dog\nPower/Toughness: 2/2\n{3}{U}: Return this creature to its owner's hand.",
    );
    let worded = p03_compile(
        "Flavor Word Displacement",
        "Mana cost: {2}{U}\nType: Creature — Dog\nPower/Toughness: 2/2\nDisplacement — {3}{U}: Return this creature to its owner's hand.",
    );
    assert_eq!(
        p03_rules_content(&plain.abilities, &["Displacement"]),
        p03_rules_content(&worded.abilities, &["Displacement"]),
        "CR 207.2d: a flavor word (Displacer Beast's 'Displacement') does not change the ability"
    );
}

#[test]
fn cr_205_3n_plane_has_its_planar_type_subtype() {
    let compiled = ironsmith_compiler_runtime::compile_to_runtime_definition(
        "Tazeem",
        "Type: Plane — Zendikar\nCreatures can't block.",
        false,
    );
    let subtypes = compiled
        .as_ref()
        .map(|definition| definition.card.subtypes.len())
        .map_err(|error| format!("{error:?}"));
    assert_eq!(
        subtypes,
        Ok(1),
        "CR 205.3n: 'Plane — Zendikar' has the planar type Zendikar as a subtype"
    );
}

#[test]
fn cr_205_3p_undercity_has_its_dungeon_type_subtype() {
    let payload: serde_json::Value = serde_json::from_str(include_str!(
        "../../../ironsmith-card-source/data/dungeons.json"
    ))
    .expect("dungeon data");
    let undercity = payload["dungeons"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["name"] == "Undercity")
        .expect("Undercity source");
    let block = format!(
        "Type: {}\n{}",
        undercity["type_line"].as_str().unwrap(),
        undercity["oracle_text"].as_str().unwrap()
    );
    assert_eq!(undercity["type_line"], "Dungeon — Undercity");
    let definition = p03_compile("Undercity", &block);
    assert!(
        definition.card.card_types.contains(&ironsmith::CardType::Dungeon),
        "fixture: Undercity is a dungeon card"
    );
    assert_eq!(
        definition.card.subtypes.len(),
        1,
        "CR 205.3p: Undercity has the dungeon type Undercity as a subtype, got {:?}",
        definition.card.subtypes
    );
}

#[test]
fn cr_118_9a_118_9b_alternative_cost_is_optional_and_exclusive() {
    use ironsmith::alternative_cast::CastingMethod;
    use ironsmith::ManaSymbol;
    let (mut game, alice, _bob) = p03_game();
    let spell = p03_compile(
        "Optional Alternative",
        "Mana cost: {2}{U}\nType: Instant\nYou may pay 3 life rather than pay this spell's mana cost.\nYou may exile a card from your hand rather than pay this spell's mana cost.\nDraw a card.",
    );
    let spell = game.create_object_from_definition(&spell, alice, Zone::Hand);
    let filler = p03_compile("Hand Filler", "Mana cost: {G}\nType: Creature — Bear\nPower/Toughness: 2/2");
    game.create_object_from_definition(&filler, alice, Zone::Hand);
    let pool = &mut game.player_mut(alice).unwrap().mana_pool;
    pool.add(ManaSymbol::Blue, 1);
    pool.add(ManaSymbol::Colorless, 2);
    let methods: Vec<CastingMethod> = ironsmith::compute_legal_actions(&game, alice)
        .unwrap()
        .into_iter()
        .filter_map(|action| match action {
            LegalAction::CastSpell { spell_id, casting_method, .. } if spell_id == spell => {
                Some(casting_method)
            }
            _ => None,
        })
        .collect();
    assert!(
        methods.contains(&CastingMethod::Normal),
        "CR 118.9b: alternative costs are optional, so the mana cost can still be paid; got {methods:?}"
    );
    let alternatives = methods
        .iter()
        .filter(|m| matches!(m, CastingMethod::Alternative(_)))
        .count();
    assert_eq!(
        alternatives, 2,
        "CR 118.9a: each alternative cost is offered as its own single choice; got {methods:?}"
    );
}

#[test]
fn cr_304_5_instant_timing_permission_is_not_precluded_by_spell_casting_restrictions() {
    use ironsmith::ManaSymbol;
    let setup = |with_shikari: bool| {
        let (mut game, alice, bob) = p03_game();
        game.turn.active_player = bob;
        game.turn.priority_player = Some(alice);
        let teferi = p03_compile(
            "Sorcery Speed Lock",
            "Mana cost: {1}{W}{U}\nType: Enchantment\nEach opponent can cast spells only any time they could cast a sorcery.",
        );
        game.create_object_from_definition(&teferi, bob, Zone::Battlefield);
        let instant = p03_compile("Free Instant", "Mana cost: {0}\nType: Instant\nYou gain 1 life.");
        let instant = game.create_object_from_definition(&instant, alice, Zone::Hand);
        if with_shikari {
            let shikari = p03_compile(
                "Leonin Shikari",
                "Mana cost: {1}{W}\nType: Creature — Cat Soldier\nPower/Toughness: 2/2\nYou may activate equip abilities any time you could cast an instant.",
            );
            game.create_object_from_definition(&shikari, alice, Zone::Battlefield);
        } else {
            p03_creature(&mut game, alice, "Equip target", 2, 2);
        }
        let equipment = p03_compile(
            "Bonesplitter",
            "Mana cost: {1}\nType: Artifact — Equipment\nEquipped creature gets +2/+0.\nEquip {1}",
        );
        let equipment = game.create_object_from_definition(&equipment, alice, Zone::Battlefield);
        game.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Colorless, 1);
        let legal = ironsmith::compute_legal_actions(&game, alice).unwrap();
        let can_cast_instant = legal
            .iter()
            .any(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == instant));
        let can_equip = legal
            .iter()
            .any(|a| matches!(a, LegalAction::ActivateAbility { source, .. } if *source == equipment));
        (can_cast_instant, can_equip)
    };
    let (instant_without, equip_without) = setup(false);
    assert!(
        !instant_without && !equip_without,
        "fixture: the lock stops instants and equip is sorcery-speed on the opponent's turn"
    );
    let (instant_with, equip_with) = setup(true);
    assert!(!instant_with, "fixture: the lock still stops the instant spell");
    assert!(
        equip_with,
        "CR 304.5: 'any time you could cast an instant' only requires priority; an effect precluding instant spells doesn't stop it"
    );
}

#[test]
fn cr_118_8c_cast_if_able_does_not_force_a_hidden_zone_additional_cost() {
    use ironsmith::game_state::StackEntry;
    struct Decline(usize);
    impl DecisionMaker for Decline {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            _ctx: &ironsmith::decisions::context::BooleanContext,
        ) -> bool {
            self.0 += 1;
            false
        }
    }
    let (mut game, alice, _bob) = p03_game();
    let filler = p03_compile("Library Filler", "Mana cost: {G}\nType: Creature — Bear\nPower/Toughness: 2/2");
    for _ in 0..4 {
        game.create_object_from_definition(&filler, alice, Zone::Library);
    }
    let costly = p03_compile(
        "Hidden Cost Sorcery",
        "Mana cost: {1}{U}\nType: Sorcery\nAs an additional cost to cast this spell, discard a creature card.\nDraw two cards.",
    );
    game.create_object_from_definition(&costly, alice, Zone::Library);
    let creature_in_hand = game.create_object_from_definition(&filler, alice, Zone::Hand);
    let omen = p03_compile(
        "Forced Cast",
        "Mana cost: {2}\nType: Sorcery\nExile the top card of your library. If it's a land card, put it onto the battlefield. Otherwise, cast it without paying its mana cost if able.",
    );
    assert!(
        !ironsmith::cards::generated_definition_has_unimplemented_content(&omen),
        "fixture: the 'cast it ... if able' instruction must compile"
    );
    let omen = game.create_object_from_definition(&omen, alice, Zone::Stack);
    game.push_to_stack(StackEntry::new(omen, alice));
    let mut dm = Decline(0);
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
    let asked = dm.0;
    let player = game.player(alice).unwrap();
    let graveyard: Vec<String> = player
        .graveyard
        .iter()
        .filter_map(|id| game.object(*id).map(|o| o.name.to_string()))
        .collect();
    let stack: Vec<String> = game
        .stack
        .iter()
        .filter_map(|e| game.object(e.object_id).map(|o| o.name.to_string()))
        .collect();
    assert!(
        player.hand.contains(&creature_in_hand),
        "CR 118.8c: graveyard={graveyard:?} stack={stack:?} booleans_asked={}: the player isn't required to cast a spell whose mandatory additional cost involves a creature card in a hidden zone",
        asked
    );
}

#[test]
fn cr_303_4_aura_spell_enters_attached_to_its_target() {
    use ironsmith::game_state::{StackEntry, Target};
    use ironsmith::object::AttachmentTarget;
    let (mut game, alice, _bob) = p03_game();
    let creature = p03_creature(&mut game, alice, "Aura host", 2, 2);
    let aura = p03_compile(
        "Holy Strength Test",
        "Mana cost: {W}\nType: Enchantment — Aura\nEnchant creature\nEnchanted creature gets +1/+2.",
    );
    let aura = game.create_object_from_definition(&aura, alice, Zone::Stack);
    game.push_to_stack(StackEntry::new(aura, alice).with_targets(vec![Target::Object(creature)]));
    ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    let aura_now = game
        .battlefield
        .iter()
        .copied()
        .find(|id| game.object(*id).is_some_and(|o| &*o.name == "Holy Strength Test"))
        .expect("CR 303.4: the resolved Aura is on the battlefield");
    assert_eq!(
        game.object(aura_now).unwrap().attached_to,
        Some(AttachmentTarget::Object(creature)),
        "CR 303.4: an Aura enters the battlefield attached to the object its enchant ability allows"
    );
    assert_eq!(game.calculated_toughness(creature), Some(4));
}

#[test]
fn cr_120_2b_spell_damage_source_is_the_spell_itself() {
    use ironsmith::game_state::{StackEntry, Target};
    let mut results = Vec::new();
    for (name, cost) in [("Red Bolt", "{R}"), ("Colorless Bolt", "{2}")] {
        let (mut game, alice, bob) = p03_game();
        let knight = p03_compile(
            "Pro-Red Knight",
            "Mana cost: {W}{W}\nType: Creature — Human Knight\nPower/Toughness: 2/3\nProtection from red",
        );
        let knight = game.create_object_from_definition(&knight, bob, Zone::Battlefield);
        let text = format!("Mana cost: {cost}\nType: Instant\nThis spell deals 2 damage to target creature.");
        let spell = p03_compile(name, &text);
        let spell = game.create_object_from_definition(&spell, alice, Zone::Stack);
        game.push_to_stack(StackEntry::new(spell, alice).with_targets(vec![Target::Object(knight)]));
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        results.push(game.damage_on(knight));
    }
    assert_eq!(
        results,
        vec![0, 2],
        "CR 120.2b: the spell is the damage source, so its own color decides protection"
    );
}

#[test]
fn cr_120_9_that_much_damage_refers_only_to_the_specified_source() {
    use ironsmith::TriggerQueue;
    let (mut game, alice, bob) = p03_game();
    let cleric = p03_compile(
        "Damage Linked Cleric",
        "Mana cost: {1}{W}\nType: Creature — Human Cleric\nPower/Toughness: 2/2\nWhenever this creature deals combat damage to a player, you gain that much life.",
    );
    let cleric = game.create_object_from_definition(&cleric, alice, Zone::Battlefield);
    let brute = p03_creature(&mut game, alice, "Simultaneous Brute", 5, 5);
    p03_combat_damage(&mut game, &[(cleric, bob), (brute, bob)], &[], bob);
    assert_eq!(game.player(bob).unwrap().life, 13);
    game.turn.priority_player = Some(alice);
    let mut queue = TriggerQueue::new();
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    ironsmith::game_loop::run_priority_loop_with(&mut game, &mut queue, &mut dm).unwrap();
    assert_eq!(
        game.player(alice).unwrap().life,
        22,
        "CR 120.9: 'that much' counts only the damage dealt by the trigger's source, not the simultaneous 5"
    );
}

#[test]
fn cr_301_7_vehicle_crew_makes_it_an_artifact_creature() {
    use ironsmith::CardType;
    let (mut game, alice, _bob) = p03_game();
    let vehicle = p03_compile(
        "Test Vehicle",
        "Mana cost: {2}\nType: Artifact — Vehicle\nPower/Toughness: 3/3\nCrew 1",
    );
    let vehicle = game.create_object_from_definition(&vehicle, alice, Zone::Battlefield);
    let crew = p03_creature(&mut game, alice, "Crew member", 2, 2);
    let before = game.calculated_characteristics(vehicle).unwrap();
    assert!(!before.card_types.contains(&CardType::Creature));
    let action = ironsmith::compute_legal_actions(&game, alice)
        .unwrap()
        .into_iter()
        .find(|a| matches!(a, LegalAction::ActivateAbility { source, .. } if *source == vehicle))
        .expect("CR 301.7/702.122: the crew ability can be activated");
    p03_take_action(&mut game, action);
    while !game.stack.is_empty() {
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
    }
    let after = game.calculated_characteristics(vehicle).unwrap();
    assert!(game.is_tapped(crew), "crewing taps the crew");
    assert!(
        after.card_types.contains(&CardType::Artifact) && after.card_types.contains(&CardType::Creature),
        "CR 301.7: a crewed Vehicle becomes an artifact creature, got {:?}",
        after.card_types
    );
}

#[test]
fn cr_303_3_304_3_enchantment_and_spell_subtypes_are_single_words() {
    use ironsmith::types::Subtype;
    let shrine = p03_compile(
        "Shrine Test",
        "Mana cost: {1}{W}\nType: Enchantment — Shrine\nAt the beginning of your upkeep, you gain 1 life.",
    );
    assert_eq!(shrine.card.subtypes, vec![Subtype::Shrine], "CR 303.3");
    let arcane = p03_compile("Arcane Test", "Mana cost: {U}\nType: Instant — Arcane\nDraw a card.");
    assert_eq!(arcane.card.subtypes, vec![Subtype::Arcane], "CR 304.3: Arcane is a spell type");
    assert!(Subtype::Arcane.belongs_to_family(ironsmith::types::SubtypeFamily::Spell));
}

#[test]
fn cr_207_5_cryptic_spires_circled_colors_are_its_mana_and_color_identity() {
    use ironsmith::color::{Color, ColorSet};
    let spires = p03_compile(
        "Cryptic Spires",
        "Type: Land\nAs you create your deck, circle two of the colors below.\nThis land enters tapped.\n{T}: Add one mana of either of the circled colors.",
    );
    let (mut game, alice, _bob) = p03_game();
    let in_library = game.create_object_from_definition(&spires, alice, Zone::Library);
    game.record_deck_construction_note(in_library, "Green and black").unwrap();
    let source = game
        .move_object(in_library, Zone::Battlefield, ironsmith::events::EventCause::effect())
        .expect("land moves to the battlefield");
    let circled = ColorSet::from_color(Color::Black).union(ColorSet::from_color(Color::Green));
    assert_eq!(game.deck_construction_chosen_colors(source), circled);
    let identity = game.object(source).unwrap().color_identity();
    assert!(
        identity.contains(Color::Black) && identity.contains(Color::Green),
        "CR 207.5: the circled colors' mana symbols are part of the printed rules text and affect color identity, got {identity:?}"
    );
}

#[test]
fn cr_118_5a_zero_cost_spell_and_ability_are_not_cast_or_activated_automatically() {
    use ironsmith::TriggerQueue;
    let (mut game, alice, _bob) = p03_game();
    let thopter = p03_compile(
        "Zero Cost Thopter",
        "Mana cost: {0}\nType: Artifact Creature — Thopter\nPower/Toughness: 0/2\nFlying",
    );
    let thopter = game.create_object_from_definition(&thopter, alice, Zone::Hand);
    let engine = p03_compile(
        "Zero Cost Engine",
        "Mana cost: {3}\nType: Artifact\n{0}: You gain 1 life.",
    );
    let engine = game.create_object_from_definition(&engine, alice, Zone::Battlefield);
    let legal = ironsmith::compute_legal_actions(&game, alice).unwrap();
    let cast = legal
        .iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == thopter))
        .cloned();
    assert!(cast.is_some(), "CR 118.5a: a {{0}} spell is cast the same way as any other spell");
    assert!(
        legal
            .iter()
            .any(|a| matches!(a, LegalAction::ActivateAbility { source, .. } if *source == engine)),
        "CR 118.5a: a {{0}} ability is activated the normal way"
    );
    let mut queue = TriggerQueue::new();
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    ironsmith::game_loop::run_priority_loop_with(&mut game, &mut queue, &mut dm).unwrap();
    assert!(
        game.player(alice).unwrap().hand.contains(&thopter) && game.player(alice).unwrap().life == 20,
        "CR 118.5a: when every player passes, the {{0}} spell isn't cast and the {{0}} ability isn't activated automatically"
    );
    game.turn.priority_player = Some(alice);
    game.turn.phase = Phase::FirstMain;
    p03_take_action(&mut game, cast.unwrap());
    assert_eq!(game.stack.len(), 1, "CR 118.5a: choosing to cast it puts it on the stack");
}
