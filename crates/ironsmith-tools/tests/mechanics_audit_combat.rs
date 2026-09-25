//! Regression tests for the combat & blocking mechanics audit (see the
//! combat ledger): combat damage division and trample (CR 510.1, 702.19),
//! lifelink/toxic/combat keywords, removal from combat (CR 506.4), and
//! trample over planeswalkers (CR 702.19c-g).
#![allow(clippy::too_many_arguments)]

use ironsmith::ability::Ability;
use ironsmith::card::PowerToughness;
use ironsmith::cards::CardDefinition;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::combat_state::{
    AttackTarget, CombatError, CombatState, declare_attackers, declare_blockers, get_attack_target,
    is_attacking,
};
use ironsmith::decision::{AttackerDeclaration, SelectFirstDecisionMaker};
use ironsmith::game_state::{Phase, Step, Target};
use ironsmith::ids::CardId;
use ironsmith::object::CounterType;
use ironsmith::static_abilities::{StaticAbility, StaticAbilityId};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardType, GameState, ObjectId, PlayerId, Zone};

// ============================================================================
// Shared fixtures
// ============================================================================

fn alice() -> PlayerId {
    PlayerId::from_index(0)
}
fn bob() -> PlayerId {
    PlayerId::from_index(1)
}
fn carol() -> PlayerId {
    PlayerId::from_index(2)
}

fn real(name: &str) -> CardDefinition {
    let payload = ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        name,
    )
    .unwrap()
    .remove(0);
    ironsmith_tools::compile_definition_from_payload(&payload)
        .unwrap_or_else(|error| panic!("{name} should compile: {error:?}"))
}

fn creature(name: &str, power: i32, toughness: i32) -> CardDefinitionBuilder {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(power, toughness))
}

fn game_with(players: usize) -> GameState {
    let names = ["Alice", "Bob", "Carol"][..players]
        .iter()
        .map(|name| name.to_string())
        .collect();
    let mut game = GameState::new(names, 20);
    game.turn.turn_number = 3;
    game.turn.active_player = alice();
    game.turn.priority_player = Some(alice());
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(Step::DeclareAttackers);
    game
}

/// Put `def` onto the battlefield under `controller`, ready to attack.
fn put(game: &mut GameState, def: &CardDefinition, controller: PlayerId) -> ObjectId {
    let id = game.create_object_from_definition(def, controller, Zone::Battlefield);
    game.remove_summoning_sickness(id);
    id
}

fn planeswalker(game: &mut GameState, controller: PlayerId, loyalty: u32) -> ObjectId {
    let def = CardDefinitionBuilder::new(CardId::new(), "Test Walker")
        .card_types(vec![CardType::Planeswalker])
        .loyalty(loyalty)
        .build();
    let id = put(game, &def, controller);
    game.object_mut(id)
        .unwrap()
        .counters
        .insert(CounterType::Loyalty, loyalty);
    id
}

/// Loyalty counters left on the planeswalker (0 once they're all removed).
fn loyalty(game: &GameState, walker: ObjectId) -> u32 {
    game.object(walker)
        .and_then(|object| object.counters.get(&CounterType::Loyalty).copied())
        .unwrap_or(0)
}

fn life(game: &GameState, player: PlayerId) -> i32 {
    game.player(player).unwrap().life
}

fn plus_counters(game: &GameState, id: ObjectId) -> u32 {
    game.object(id)
        .unwrap()
        .counters
        .get(&CounterType::PlusOnePlusOne)
        .copied()
        .unwrap_or(0)
}

/// Declare attacks through the turn-based action (queuing attack triggers),
/// then put and resolve every triggered ability.
fn attack_and_resolve(
    game: &mut GameState,
    attacks: &[(ObjectId, AttackTarget)],
) -> CombatState {
    let mut combat = CombatState::default();
    let mut queue = TriggerQueue::new();
    let declarations = attacks
        .iter()
        .map(|(creature, target)| AttackerDeclaration {
            creature: *creature,
            target: target.clone(),
        })
        .collect::<Vec<_>>();
    ironsmith::game_loop::apply_attacker_declarations(game, &mut combat, &mut queue, &declarations)
        .unwrap();
    resolve_triggers(game, &mut queue);
    combat
}

fn resolve_triggers(game: &mut GameState, queue: &mut TriggerQueue) {
    let mut dm = SelectFirstDecisionMaker;
    ironsmith::game_loop::put_triggers_on_stack_with_dm(game, queue, &mut dm).unwrap();
    while !game.stack.is_empty() {
        ironsmith::game_loop::resolve_stack_entry_with(game, &mut dm).unwrap();
        ironsmith::game_loop::put_triggers_on_stack_with_dm(game, queue, &mut dm).unwrap();
    }
    game.refresh_continuous_state();
}

/// Run a regular combat damage step against the game's current combat, then
/// resolve the combat damage triggers.
fn deal_combat_damage(game: &mut GameState) {
    let combat = game.combat.clone().expect("combat in progress");
    let events = ironsmith::game_loop::execute_combat_damage_step(game, &combat, false);
    let mut queue = TriggerQueue::new();
    ironsmith::game_loop::queue_combat_damage_triggers(game, &events, &mut queue);
    resolve_triggers(game, &mut queue);
}

fn next_prompt(game: &GameState) -> Option<ironsmith::game_loop::CombatDamageAssignmentPrompt> {
    let combat = game.combat.clone().expect("combat in progress");
    ironsmith::game_loop::next_combat_damage_assignment_prompt(game, &combat, false, None)
}

fn block(game: &mut GameState, blocks: Vec<(ObjectId, ObjectId)>) {
    let mut combat = game.combat.clone().unwrap_or_default();
    declare_blockers(game, &mut combat, blocks).unwrap();
    game.combat = Some(combat);
}

fn attack(game: &mut GameState, attacks: Vec<(ObjectId, AttackTarget)>) {
    let mut combat = CombatState::default();
    declare_attackers(game, &mut combat, attacks).unwrap();
    game.combat = Some(combat);
}

// ============================================================================
// A1: combat lifelink is a life-gain event (CR 120.3f, 702.15b)
// ============================================================================
mod lifelink {
    use super::*;

    fn board() -> (GameState, ObjectId, ObjectId) {
        let mut game = game_with(2);
        let pridemate = put(&mut game, &real("Ajani's Pridemate"), alice());
        let linker = put(&mut game, &creature("Lifelinker", 4, 4).lifelink().build(), alice());
        (game, pridemate, linker)
    }

    #[test]
    fn unblocked_lifelink_damage_gains_life_and_triggers_life_gain() {
        let (mut game, pridemate, linker) = board();
        attack(&mut game, vec![(linker, AttackTarget::Player(bob()))]);
        deal_combat_damage(&mut game);
        assert_eq!(life(&game, bob()), 16);
        assert_eq!(life(&game, alice()), 24, "lifelink gains 4");
        assert_eq!(plus_counters(&game, pridemate), 1, "whenever you gain life");
    }

    #[test]
    fn one_lifelink_source_dealing_damage_to_two_blockers_is_one_life_gain_event() {
        let (mut game, pridemate, linker) = board();
        let first = put(&mut game, &creature("Blocker A", 1, 1).build(), bob());
        let second = put(&mut game, &creature("Blocker B", 1, 1).build(), bob());
        attack(&mut game, vec![(linker, AttackTarget::Player(bob()))]);
        block(&mut game, vec![(first, linker), (second, linker)]);
        let prompt = next_prompt(&game).expect("two blockers: a division");
        prompt.record(
            &mut game,
            &[(Target::Object(first), 2), (Target::Object(second), 2)],
        );
        deal_combat_damage(&mut game);
        assert_eq!(life(&game, alice()), 24);
        assert_eq!(
            plus_counters(&game, pridemate),
            1,
            "a single source's lifelink gain is one event (CR 120.3f)"
        );
    }
}

// ============================================================================
// A2 / R2-1: combat damage division (CR 510.1c-e, 702.19b, 702.2c)
// ============================================================================
mod damage_division {
    use super::*;

    #[test]
    fn attacker_blocked_by_two_creatures_divides_damage_as_chosen() {
        let mut game = game_with(2);
        let attacker = put(&mut game, &creature("Attacker", 4, 4).build(), alice());
        let first = put(&mut game, &creature("Spawn", 2, 3).build(), bob());
        let second = put(&mut game, &creature("Hunter", 1, 1).build(), bob());
        attack(&mut game, vec![(attacker, AttackTarget::Player(bob()))]);
        block(&mut game, vec![(first, attacker), (second, attacker)]);
        let prompt = next_prompt(&game).expect("the attacking player divides the damage");
        assert_eq!(prompt.player, alice());
        assert_eq!(prompt.total, 4);
        assert!(
            prompt
                .validate(&game, &[(Target::Object(first), 1), (Target::Object(second), 1)])
                .is_err(),
            "must assign exactly its power"
        );
        assert!(
            prompt
                .validate(&game, &[(Target::Player(bob()), 4)])
                .is_err(),
            "a non-trampler can't assign damage to the player"
        );
        // CR 510.1c: any division among the blockers is legal (no ordering).
        prompt.record(
            &mut game,
            &[(Target::Object(first), 1), (Target::Object(second), 3)],
        );
        assert!(next_prompt(&game).is_none());
        deal_combat_damage(&mut game);
        assert_eq!(game.damage_on(first), 1);
        assert_eq!(game.damage_on(second), 3);
    }

    #[test]
    fn deathtouch_default_division_assigns_one_to_each_blocker() {
        let mut game = game_with(2);
        let attacker = put(&mut game, &creature("Deadly", 3, 3).deathtouch().build(), alice());
        let blockers = (0..3)
            .map(|i| put(&mut game, &creature(&format!("Wall {i}"), 0, 5).build(), bob()))
            .collect::<Vec<_>>();
        attack(&mut game, vec![(attacker, AttackTarget::Player(bob()))]);
        block(&mut game, blockers.iter().map(|b| (*b, attacker)).collect());
        let prompt = next_prompt(&game).unwrap();
        let default = prompt.default_assignment(&game);
        for blocker in &blockers {
            assert!(
                default.contains(&(Target::Object(*blocker), 1)),
                "1 deathtouch damage is lethal (CR 702.2c): {default:?}"
            );
        }
    }

    #[test]
    fn trample_lethal_check_counts_other_attackers_damage_to_the_same_blocker() {
        // CR 702.19b example: a 2/2 that blocks a 1/1 and a 3/3 trampler; the
        // trampler may assign 1 to the blocker and 2 to the player.
        let mut game = game_with(2);
        let small = put(&mut game, &creature("Small", 1, 1).build(), alice());
        let trampler = put(&mut game, &creature("Trampler", 3, 3).trample().build(), alice());
        let blocker = put(
            &mut game,
            &creature("Double Blocker", 2, 2)
                .with_ability(Ability::static_ability(
                    StaticAbility::can_block_additional_creature_each_combat(1),
                ))
                .build(),
            bob(),
        );
        attack(
            &mut game,
            vec![
                (small, AttackTarget::Player(bob())),
                (trampler, AttackTarget::Player(bob())),
            ],
        );
        block(&mut game, vec![(blocker, small), (blocker, trampler)]);
        let prompt = next_prompt(&game).expect("the trampler has a choice");
        assert_eq!(prompt.source, trampler);
        let split = [(Target::Object(blocker), 1), (Target::Player(bob()), 2)];
        assert!(prompt.validate(&game, &split).is_ok(), "the 1/1's damage counts toward lethal");
        assert!(
            prompt
                .validate(&game, &[(Target::Player(bob()), 3)])
                .is_err(),
            "still needs lethal in total"
        );
        prompt.record(&mut game, &split);
        deal_combat_damage(&mut game);
        assert_eq!(life(&game, bob()), 18);
        assert_eq!(game.damage_on(blocker), 2);
    }
}

// ============================================================================
// A3 / A4 / A10 (F_core C1, C2): trample and blocked attackers
// ============================================================================
mod trample_and_blocked {
    use super::*;

    #[test]
    fn blocked_trampler_with_no_blockers_left_assigns_all_damage_to_the_player() {
        let mut game = game_with(2);
        let trampler = put(&mut game, &creature("Trampler", 5, 5).trample().build(), alice());
        let blocker = put(&mut game, &creature("Chump", 1, 1).build(), bob());
        attack(&mut game, vec![(trampler, AttackTarget::Player(bob()))]);
        block(&mut game, vec![(blocker, trampler)]);
        game.move_object_by_effect(blocker, Zone::Graveyard);
        deal_combat_damage(&mut game);
        assert_eq!(life(&game, bob()), 15, "CR 702.19d");
    }

    #[test]
    fn trample_excess_goes_to_the_attacked_planeswalker() {
        let mut game = game_with(2);
        let walker = planeswalker(&mut game, bob(), 5);
        let trampler = put(&mut game, &creature("Trampler", 5, 5).trample().build(), alice());
        let blocker = put(&mut game, &creature("Bear", 2, 2).build(), bob());
        attack(&mut game, vec![(trampler, AttackTarget::Planeswalker(walker))]);
        block(&mut game, vec![(blocker, trampler)]);
        deal_combat_damage(&mut game);
        assert_eq!(game.damage_on(blocker), 2);
        assert_eq!(loyalty(&game, walker), 2, "3 excess to the planeswalker");
        assert_eq!(life(&game, bob()), 20);
    }

    #[test]
    fn blocked_non_trampler_whose_blocker_left_combat_deals_no_damage() {
        let mut game = game_with(2);
        let attacker = put(&mut game, &creature("Attacker", 3, 3).build(), alice());
        let blocker = put(&mut game, &creature("Blocker", 1, 1).build(), bob());
        attack(&mut game, vec![(attacker, AttackTarget::Player(bob()))]);
        block(&mut game, vec![(blocker, attacker)]);
        game.phase_out(blocker);
        assert!(
            game.combat.as_ref().unwrap().blockers[&attacker].is_empty(),
            "the phased-out blocker left combat"
        );
        deal_combat_damage(&mut game);
        assert_eq!(life(&game, bob()), 20, "CR 509.1h / 510.1c: still blocked");
    }
}

// ============================================================================
// A5: Toxic (CR 702.164)
// ============================================================================
mod toxic {
    use super::*;

    #[test]
    fn toxic_gives_poison_counters_with_combat_damage_to_a_player() {
        let mut game = game_with(2);
        let stalker = put(&mut game, &real("Branchblight Stalker"), alice());
        attack(&mut game, vec![(stalker, AttackTarget::Player(bob()))]);
        deal_combat_damage(&mut game);
        let power = game.calculated_power(stalker).unwrap();
        assert_eq!(life(&game, bob()), 20 - power);
        assert_eq!(game.player(bob()).unwrap().poison_counters, 2, "toxic 2");
    }

    #[test]
    fn toxic_gives_no_poison_when_its_damage_goes_to_a_creature() {
        let mut game = game_with(2);
        let stalker = put(&mut game, &real("Branchblight Stalker"), alice());
        let blocker = put(&mut game, &creature("Wall", 0, 8).build(), bob());
        attack(&mut game, vec![(stalker, AttackTarget::Player(bob()))]);
        block(&mut game, vec![(blocker, stalker)]);
        deal_combat_damage(&mut game);
        assert_eq!(game.player(bob()).unwrap().poison_counters, 0);
    }
}

// ============================================================================
// A6: Dethrone ignores players who left the game (CR 702.105a, 800.4a)
// ============================================================================
mod dethrone {
    use super::*;

    #[test]
    fn most_life_is_compared_among_players_still_in_the_game() {
        let mut game = game_with(3);
        game.player_mut(bob()).unwrap().life = 25;
        game.player_mut(carol()).unwrap().life = 40;
        game.concede_game(carol());
        assert!(!game.player(carol()).unwrap().is_in_game());
        let revolutionary = put(&mut game, &real("Enraged Revolutionary"), alice());
        attack_and_resolve(&mut game, &[(revolutionary, AttackTarget::Player(bob()))]);
        assert_eq!(plus_counters(&game, revolutionary), 1, "Bob has the most life now");
    }

    #[test]
    fn attacking_a_player_without_the_most_life_does_nothing() {
        let mut game = game_with(3);
        game.player_mut(carol()).unwrap().life = 40;
        let revolutionary = put(&mut game, &real("Enraged Revolutionary"), alice());
        attack_and_resolve(&mut game, &[(revolutionary, AttackTarget::Player(bob()))]);
        assert_eq!(plus_counters(&game, revolutionary), 0);
    }
}

// ============================================================================
// A7: Melee (CR 702.121)
// ============================================================================
mod melee {
    use super::*;

    #[test]
    fn melee_counts_each_opponent_attacked_this_combat() {
        let mut game = game_with(3);
        let liberator = put(&mut game, &real("Menagerie Liberator"), alice());
        let other = put(&mut game, &creature("Partner", 1, 1).build(), alice());
        let base = game.calculated_power(liberator).unwrap();
        attack_and_resolve(
            &mut game,
            &[
                (liberator, AttackTarget::Player(bob())),
                (other, AttackTarget::Player(carol())),
            ],
        );
        assert_eq!(game.calculated_power(liberator), Some(base + 2));
    }

    #[test]
    fn attacking_only_a_planeswalker_attacks_no_opponent() {
        let mut game = game_with(3);
        let walker = planeswalker(&mut game, bob(), 4);
        let liberator = put(&mut game, &real("Menagerie Liberator"), alice());
        let base = game.calculated_power(liberator).unwrap();
        attack_and_resolve(&mut game, &[(liberator, AttackTarget::Planeswalker(walker))]);
        assert_eq!(
            game.calculated_power(liberator),
            Some(base),
            "only opponents attacked count (CR 702.121a)"
        );
    }
}

// ============================================================================
// A8: Renown (CR 702.112)
// ============================================================================
mod renown {
    use super::*;

    #[test]
    fn renown_puts_counters_once_and_uses_counter_replacements() {
        let mut game = game_with(2);
        put(&mut game, &real("Hardened Scales"), alice());
        let knight = put(&mut game, &real("Knight of the Pilgrim's Road"), alice());
        attack(&mut game, vec![(knight, AttackTarget::Player(bob()))]);
        deal_combat_damage(&mut game);
        assert_eq!(
            plus_counters(&game, knight),
            2,
            "renown 1 plus Hardened Scales' extra counter"
        );
        game.untap(knight);
        attack(&mut game, vec![(knight, AttackTarget::Player(bob()))]);
        deal_combat_damage(&mut game);
        assert_eq!(plus_counters(&game, knight), 2, "a renowned creature gets no more");
    }
}

// ============================================================================
// A9: Provoke (CR 702.39)
// ============================================================================
mod provoke {
    use super::*;

    #[test]
    fn provoked_creature_untaps_and_must_block() {
        let mut game = game_with(2);
        let grappler = put(&mut game, &real("Goblin Grappler"), alice());
        let target = put(&mut game, &creature("Provoked", 2, 2).build(), bob());
        game.tap(target);
        let combat = attack_and_resolve(&mut game, &[(grappler, AttackTarget::Player(bob()))]);
        assert!(!game.is_tapped(target), "untaps it");
        let mut no_block = combat.clone();
        assert!(
            matches!(
                declare_blockers(&game, &mut no_block, vec![]),
                Err(CombatError::MustBlockRequirementNotMet { .. })
            ),
            "it must block the provoking creature if able"
        );
        let mut blocks = combat;
        assert!(declare_blockers(&game, &mut blocks, vec![(target, grappler)]).is_ok());
    }
}

// ============================================================================
// A11: "can block only creatures with flying" (CR 509.1b)
// ============================================================================
mod block_only_flyers {
    use super::*;

    #[test]
    fn it_grants_no_permission_to_block_a_flyer() {
        let mut game = game_with(2);
        let flyer = put(&mut game, &creature("Flyer", 2, 2).flying().build(), alice());
        let grounded = put(
            &mut game,
            &creature("Grounded Picker", 2, 2)
                .with_ability(Ability::static_ability(StaticAbility::can_block_only_flying()))
                .build(),
            bob(),
        );
        attack(&mut game, vec![(flyer, AttackTarget::Player(bob()))]);
        let mut combat = game.combat.clone().unwrap();
        assert!(
            declare_blockers(&game, &mut combat, vec![(grounded, flyer)]).is_err(),
            "without flying or reach it still can't block a flyer"
        );
    }

    #[test]
    fn a_flyer_that_blocks_only_flyers_cant_block_a_ground_creature() {
        let mut game = game_with(2);
        let ground = put(&mut game, &creature("Ground", 2, 2).build(), alice());
        let sprite = put(&mut game, &real("Cloud Sprite"), bob());
        attack(&mut game, vec![(ground, AttackTarget::Player(bob()))]);
        let mut combat = game.combat.clone().unwrap();
        assert!(declare_blockers(&game, &mut combat, vec![(sprite, ground)]).is_err());
    }
}

// ============================================================================
// A12: Rampage counts blockers on resolution (CR 702.23)
// ============================================================================
mod rampage {
    use super::*;

    #[test]
    fn rampage_counts_the_creatures_still_blocking_when_it_resolves() {
        let mut game = game_with(2);
        let pack = put(&mut game, &real("Wolverine Pack"), alice());
        let blockers = (0..3)
            .map(|i| put(&mut game, &creature(&format!("Blocker {i}"), 1, 1).build(), bob()))
            .collect::<Vec<_>>();
        let base = game.calculated_power(pack).unwrap();
        let combat = attack_and_resolve(&mut game, &[(pack, AttackTarget::Player(bob()))]);
        game.combat = Some(combat);
        let mut combat = game.combat.clone().unwrap();
        let mut queue = TriggerQueue::new();
        let declarations = blockers
            .iter()
            .map(|b| ironsmith::decision::BlockerDeclaration {
                blocker: *b,
                blocking: pack,
            })
            .collect::<Vec<_>>();
        ironsmith::game_loop::apply_blocker_declarations(
            &mut game,
            &mut combat,
            &mut queue,
            &declarations,
            bob(),
        )
        .unwrap();
        game.combat = Some(combat);
        // One blocker leaves combat before the trigger resolves.
        game.phase_out(blockers[0]);
        resolve_triggers(&mut game, &mut queue);
        assert_eq!(
            game.calculated_power(pack),
            Some(base + 2),
            "two blockers remain: +2/+2 for the one beyond the first"
        );
    }
}

// ============================================================================
// A13: Battle cry pumps each other attacking creature (CR 702.91a)
// ============================================================================
mod battle_cry {
    use super::*;

    #[test]
    fn battle_cry_pumps_attacking_creatures_other_players_control() {
        let mut game = game_with(3);
        let paladin = put(&mut game, &real("Accorder Paladin"), alice());
        let own = put(&mut game, &creature("Own", 1, 1).build(), alice());
        let teammate = put(&mut game, &creature("Teammate's", 1, 1).build(), carol());
        let mut combat = CombatState::default();
        let mut queue = TriggerQueue::new();
        ironsmith::game_loop::apply_attacker_declarations(
            &mut game,
            &mut combat,
            &mut queue,
            &[
                AttackerDeclaration {
                    creature: paladin,
                    target: AttackTarget::Player(bob()),
                },
                AttackerDeclaration {
                    creature: own,
                    target: AttackTarget::Player(bob()),
                },
            ],
        )
        .unwrap();
        // A teammate's creature attacking in the same combat (e.g. 2HG).
        combat.attackers.push(ironsmith::combat_state::AttackerInfo {
            creature: teammate,
            target: AttackTarget::Player(bob()),
        });
        game.combat = Some(combat);
        let paladin_power = game.calculated_power(paladin).unwrap();
        resolve_triggers(&mut game, &mut queue);
        assert_eq!(game.calculated_power(own), Some(2));
        assert_eq!(game.calculated_power(teammate), Some(2), "each other attacking creature");
        assert_eq!(game.calculated_power(paladin), Some(paladin_power), "not itself");
    }
}

// ============================================================================
// A14: a creature can't be declared blocking the same attacker twice
// ============================================================================
mod duplicate_blocker {
    use super::*;

    #[test]
    fn one_creature_listed_twice_does_not_satisfy_menace() {
        let mut game = game_with(2);
        let menace = put(&mut game, &creature("Menacer", 2, 2).menace().build(), alice());
        let blocker = put(&mut game, &creature("Blocker", 2, 2).build(), bob());
        attack(&mut game, vec![(menace, AttackTarget::Player(bob()))]);
        let mut combat = game.combat.clone().unwrap();
        assert!(matches!(
            declare_blockers(&game, &mut combat, vec![(blocker, menace), (blocker, menace)]),
            Err(CombatError::DuplicateBlocker(id)) if id == blocker
        ));
    }
}

// ============================================================================
// F_core C3 / R2-2 / R2-3: removal from combat (CR 506.4, 506.4c, 506.4e)
// ============================================================================
mod removal_from_combat {
    use super::*;
    use ironsmith::continuous::{ContinuousEffect, EffectTarget, Modification};

    #[test]
    fn an_attacker_whose_controller_changes_is_removed_from_combat() {
        let mut game = game_with(2);
        let attacker = put(&mut game, &creature("Turncoat", 3, 3).build(), alice());
        let source = put(&mut game, &creature("Thief", 1, 1).build(), bob());
        attack(&mut game, vec![(attacker, AttackTarget::Player(bob()))]);
        game.effect_store.continuous_effects.add_effect(
            ContinuousEffect::gain_control(source, bob(), attacker, bob())
                .until(ironsmith::Until::Forever),
        );
        game.refresh_continuous_state();
        assert!(!is_attacking(game.combat.as_ref().unwrap(), attacker));
        deal_combat_damage(&mut game);
        assert_eq!(life(&game, bob()), 20);
    }

    #[test]
    fn a_creature_attacking_a_planeswalker_that_left_attacks_nothing() {
        let mut game = game_with(2);
        let walker = planeswalker(&mut game, bob(), 3);
        let attacker = put(&mut game, &creature("Attacker", 3, 3).build(), alice());
        let blocker = put(&mut game, &creature("Blocker", 1, 1).build(), bob());
        attack(&mut game, vec![(attacker, AttackTarget::Planeswalker(walker))]);
        game.move_object_by_effect(walker, Zone::Graveyard);
        game.refresh_continuous_state();
        let combat = game.combat.clone().unwrap();
        assert!(is_attacking(&combat, attacker), "still an attacking creature");
        assert!(matches!(
            get_attack_target(&combat, attacker),
            Some(AttackTarget::Nothing { defending_player: Some(p), .. }) if *p == bob()
        ));
        assert!(
            declare_blockers(&game, &mut combat.clone(), vec![(blocker, attacker)]).is_ok(),
            "it can still be blocked (CR 506.4c)"
        );
        deal_combat_damage(&mut game);
        assert_eq!(life(&game, bob()), 20, "unblocked, it deals no combat damage");
    }

    #[test]
    fn a_planeswalker_attacked_as_only_a_planeswalker_that_becomes_a_battle_leaves_combat() {
        let mut game = game_with(2);
        let walker = planeswalker(&mut game, bob(), 3);
        let attacker = put(&mut game, &creature("Attacker", 3, 3).build(), alice());
        attack(&mut game, vec![(attacker, AttackTarget::Planeswalker(walker))]);
        game.refresh_continuous_state();
        game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(
            walker,
            bob(),
            EffectTarget::Specific(walker),
            Modification::SetCardTypes(vec![CardType::Battle]),
        ));
        game.refresh_continuous_state();
        assert!(
            matches!(
                get_attack_target(game.combat.as_ref().unwrap(), attacker),
                Some(AttackTarget::Nothing { .. })
            ),
            "CR 506.4: it stopped being a planeswalker; 506.4e needs it to have been both"
        );
    }
}

// ============================================================================
// B7: Aura-granted protection from the chosen color stops blocks (CR 702.16b)
// ============================================================================
mod chosen_color_protection {
    use super::*;

    #[test]
    fn enchanted_attacker_cant_be_blocked_by_creatures_of_the_chosen_color() {
        let mut game = game_with(2);
        let attacker = put(&mut game, &creature("Blessed", 2, 2).build(), alice());
        let aura = put(&mut game, &real("Cho-Manno's Blessing"), alice());
        game.set_chosen_color(aura, ironsmith::color::Color::Red);
        assert!(game.attach_object_to_target(
            aura,
            ironsmith::object::AttachmentTarget::Object(attacker)
        ));
        let red = put(
            &mut game,
            &creature("Red Blocker", 2, 2)
                .color_indicator(ironsmith::color::ColorSet::RED)
                .build(),
            bob(),
        );
        let green = put(
            &mut game,
            &creature("Green Blocker", 2, 2)
                .color_indicator(ironsmith::color::ColorSet::GREEN)
                .build(),
            bob(),
        );
        game.refresh_continuous_state();
        attack(&mut game, vec![(attacker, AttackTarget::Player(bob()))]);
        let combat = game.combat.clone().unwrap();
        assert!(declare_blockers(&game, &mut combat.clone(), vec![(red, attacker)]).is_err());
        assert!(declare_blockers(&game, &mut combat.clone(), vec![(green, attacker)]).is_ok());
    }
}

// ============================================================================
// E2-2: suspected creatures have menace and can't block (CR 701.60c)
// ============================================================================
mod suspected {
    use super::*;

    #[test]
    fn suspect_etb_grants_menace_and_cant_block() {
        let mut game = game_with(2);
        let hand = game.create_object_from_definition(&real("Person of Interest"), alice(), Zone::Hand);
        let goat = game
            .move_object_with_etb_processing(hand, Zone::Battlefield)
            .unwrap()
            .new_id;
        let mut queue = TriggerQueue::new();
        resolve_triggers(&mut game, &mut queue);
        game.remove_summoning_sickness(goat);
        assert!(game.object_has_static_ability_id(goat, StaticAbilityId::Menace));
        // It can't block.
        let bob_attacker = put(&mut game, &creature("Bob's Attacker", 1, 1).build(), bob());
        game.turn.active_player = bob();
        attack(&mut game, vec![(bob_attacker, AttackTarget::Player(alice()))]);
        let mut combat = game.combat.clone().unwrap();
        assert!(declare_blockers(&game, &mut combat, vec![(goat, bob_attacker)]).is_err());
        // Menace: one blocker isn't enough.
        game.turn.active_player = alice();
        let lone = put(&mut game, &creature("Lone Blocker", 2, 2).build(), bob());
        attack(&mut game, vec![(goat, AttackTarget::Player(bob()))]);
        let mut combat = game.combat.clone().unwrap();
        assert!(declare_blockers(&game, &mut combat, vec![(lone, goat)]).is_err());
    }
}

// ============================================================================
// D2-09 / D2-12: vehicles and animated equipment
// ============================================================================
mod artifacts {
    use super::*;
    use ironsmith::decision::{LegalAction, compute_legal_actions};

    fn main_phase(game: &mut GameState) {
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
    }

    #[test]
    fn a_vehicle_that_is_a_creature_cant_crew_itself() {
        let mut game = game_with(2);
        main_phase(&mut game);
        put(&mut game, &real("March of the Machines"), alice());
        let copter = put(&mut game, &real("Smuggler's Copter"), alice());
        game.refresh_continuous_state();
        assert!(game.current_is_creature(copter), "March animates the Vehicle");
        let crew = |game: &GameState| {
            compute_legal_actions(game, alice())
                .into_iter()
                .any(|action| matches!(action, LegalAction::ActivateAbility { source, .. } if source == copter))
        };
        assert!(!crew(&game), "crew taps other creatures (CR 702.122a)");
        put(&mut game, &creature("Pilot", 1, 1).build(), alice());
        game.refresh_continuous_state();
        assert!(crew(&game), "another creature can crew it");
    }

    #[test]
    fn an_equipment_that_is_a_creature_cant_equip() {
        let equip = |animated: bool| {
            let mut game = game_with(2);
            main_phase(&mut game);
            if animated {
                put(&mut game, &real("March of the Machines"), alice());
            }
            let bear = put(&mut game, &creature("Bear", 2, 2).build(), alice());
            let splitter = put(&mut game, &real("Bonesplitter"), alice());
            game.refresh_continuous_state();
            let mut dm = SelectFirstDecisionMaker;
            let mut ctx = ironsmith::effects::EffectContext::new(splitter, alice(), &mut dm);
            let _ = ironsmith::effects::execute_effect(
                &mut game,
                &ironsmith::Effect::attach_to(ironsmith::target::ChooseSpec::SpecificObject(bear)),
                &mut ctx,
            );
            game.object(bear).unwrap().attachments.contains(&splitter)
        };
        assert!(equip(false), "sanity: an ordinary Equipment attaches");
        assert!(!equip(true), "CR 301.5c: an Equipment that's a creature can't equip");
    }
}

// ============================================================================
// Round 3: trample over planeswalkers (CR 702.19c-g)
// ============================================================================
mod trample_over_planeswalkers {
    use super::*;

    fn thrasta(game: &mut GameState) -> ObjectId {
        let id = put(game, &real("Thrasta, Tempest's Roar"), alice());
        game.refresh_continuous_state();
        id
    }

    #[test]
    fn thrasta_has_the_distinct_keyword() {
        let mut game = game_with(2);
        let thrasta = thrasta(&mut game);
        assert!(game.object_has_static_ability_id(thrasta, StaticAbilityId::TrampleOverPlaneswalkers));
        let granted = put(
            &mut game,
            &creature("Only TOP", 3, 3)
                .with_ability(Ability::static_ability(
                    StaticAbility::trample_over_planeswalkers(),
                ))
                .build(),
            alice(),
        );
        game.refresh_continuous_state();
        assert!(
            !game.object_has_static_ability_id(granted, StaticAbilityId::Trample),
            "trample over planeswalkers isn't trample"
        );
    }

    #[test]
    fn excess_past_blockers_and_loyalty_can_go_to_the_planeswalkers_controller() {
        let mut game = game_with(2);
        let thrasta = thrasta(&mut game);
        let power = game.calculated_power(thrasta).unwrap() as u32;
        let walker = planeswalker(&mut game, bob(), 2);
        let blocker = put(&mut game, &creature("Chump", 1, 1).build(), bob());
        attack(&mut game, vec![(thrasta, AttackTarget::Planeswalker(walker))]);
        block(&mut game, vec![(blocker, thrasta)]);
        let prompt = next_prompt(&game).expect("a division with the planeswalker's controller");
        assert!(
            prompt
                .validate(
                    &game,
                    &[
                        (Target::Object(blocker), 1),
                        (Target::Object(walker), 1),
                        (Target::Player(bob()), power - 2),
                    ],
                )
                .is_err(),
            "the controller only after damage equal to loyalty (CR 702.19c)"
        );
        let default = prompt.default_assignment(&game);
        assert_eq!(
            default,
            vec![
                (Target::Object(blocker), 1),
                (Target::Object(walker), 2),
                (Target::Player(bob()), power - 3),
            ]
        );
        prompt.record(&mut game, &default);
        deal_combat_damage(&mut game);
        assert_eq!(loyalty(&game, walker), 0);
        assert_eq!(life(&game, bob()), 20 - (power as i32 - 3));
    }

    #[test]
    fn other_attackers_damage_counts_toward_the_planeswalkers_loyalty() {
        // CR 702.19c example: a 1/1 and a trampler over planeswalkers attack a
        // planeswalker with three loyalty.
        let mut game = game_with(2);
        let thrasta = thrasta(&mut game);
        let power = game.calculated_power(thrasta).unwrap() as u32;
        let walker = planeswalker(&mut game, bob(), 3);
        let small = put(&mut game, &creature("Small", 1, 1).build(), alice());
        attack(
            &mut game,
            vec![
                (small, AttackTarget::Planeswalker(walker)),
                (thrasta, AttackTarget::Planeswalker(walker)),
            ],
        );
        let prompt = next_prompt(&game).expect("unblocked: planeswalker or its controller");
        let split = [(Target::Object(walker), 2), (Target::Player(bob()), power - 2)];
        assert!(prompt.validate(&game, &split).is_ok());
        prompt.record(&mut game, &split);
        deal_combat_damage(&mut game);
        assert_eq!(loyalty(&game, walker), 0);
        assert_eq!(life(&game, bob()), 20 - (power as i32 - 2));
    }

    #[test]
    fn removed_planeswalker_lets_it_assign_damage_to_the_defending_player_only_with_top() {
        let mut game = game_with(2);
        let thrasta = thrasta(&mut game);
        let power = game.calculated_power(thrasta).unwrap();
        let trampler = put(&mut game, &creature("Plain Trampler", 3, 3).trample().build(), alice());
        let walker = planeswalker(&mut game, bob(), 3);
        attack(
            &mut game,
            vec![
                (thrasta, AttackTarget::Planeswalker(walker)),
                (trampler, AttackTarget::Planeswalker(walker)),
            ],
        );
        game.move_object_by_effect(walker, Zone::Exile);
        game.refresh_continuous_state();
        deal_combat_damage(&mut game);
        assert_eq!(
            life(&game, bob()),
            20 - power,
            "CR 702.19e for trample over planeswalkers; 702.19f: plain trample deals none"
        );
        assert!(matches!(
            get_attack_target(game.combat.as_ref().unwrap(), thrasta),
            Some(AttackTarget::Nothing { .. })
        ), "it isn't attacking that player");
    }
}
