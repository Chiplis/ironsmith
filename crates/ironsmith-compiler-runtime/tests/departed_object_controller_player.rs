//! "Whenever an artifact an opponent controls is put into a graveyard from the
//! battlefield / a creature an opponent controls dies, ... that player": the
//! only player the event names is the departed object's last-known controller
//! (CR 603.10a). Frozen complete bodies; source-authored and UNRUN.
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::effect::Value;
use ironsmith::effects::DealDamageEffect;
use ironsmith::game_loop::{drain_pending_trigger_events_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::game_state::Phase;
use ironsmith::target::{ChooseSpec, PlayerFilter};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiler_runtime::compile_to_runtime_definition;

#[path = "cf8_p08/support.rs"]
mod support;

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
const C: PlayerId = PlayerId::from_index(2);

const BODIES: &[(&str, &str)] = &[
    ("Pain Distributor", "Mana cost: {2}{R}\nType: Creature — Devil Citizen\nPower/Toughness: 2/3\nMenace\nWhenever a player casts their first spell each turn, they create a Treasure token.\nWhenever an artifact an opponent controls is put into a graveyard from the battlefield, this creature deals 1 damage to that player."),
    ("Sardian Avenger", "Mana cost: {1}{R}\nType: Creature — Goblin Warrior\nPower/Toughness: 1/1\nFirst strike, trample\nWhenever this creature attacks, it gets +X/+0 until end of turn, where X is the number of artifacts your opponents control.\nWhenever an artifact an opponent controls is put into a graveyard from the battlefield, this creature deals 1 damage to that player."),
    ("Shriek, Treblemaker", "Mana cost: {2}{B/R}\nType: Legendary Creature — Mutant Villain\nPower/Toughness: 2/3\nAt the beginning of your first main phase, you may discard a card. When you do, target creature can't block this turn.\nSonic Blast — Whenever a creature an opponent controls dies, Shriek deals 1 damage to that player."),
];

#[test]
fn that_player_is_the_departed_objects_aliased_controller() {
    for (name, body) in BODIES {
        for definition in support::definitions(name, body) {
            let damage: Vec<DealDamageEffect> = support::find_all::<DealDamageEffect>(&definition)
                .into_iter()
                .filter(|damage| damage.amount == Value::Fixed(1))
                .collect();
            assert_eq!(damage.len(), 1, "{name}: {damage:?}");
            assert!(
                matches!(damage[0].target.base(), ChooseSpec::Player(PlayerFilter::AliasedControllerOf(_))),
                "{name}: {:?}",
                damage[0].target
            );
            assert!(!damage[0].target.is_target(), "{name}: no new target is declared");
        }
    }
}

fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game
}

fn permanent(game: &mut GameState, owner: PlayerId, body: &str) -> ObjectId {
    let definition = compile_to_runtime_definition("Scenario permanent", body, false).unwrap();
    game.create_object_from_definition(&definition, owner, Zone::Battlefield)
}

fn settle(game: &mut GameState) {
    let mut queue = TriggerQueue::new();
    drain_pending_trigger_events_with_dm(game, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
    put_triggers_on_stack_with_dm(game, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
    for _ in 0..8 {
        if game.stack.is_empty() {
            return;
        }
        resolve_stack_entry_with(game, &mut SelectFirstDecisionMaker).unwrap();
    }
    panic!("stack did not settle");
}

#[test]
fn only_the_player_who_controlled_the_departed_artifact_is_dealt_damage() {
    for definition in support::definitions("Sardian Avenger", BODIES[1].1) {
        let mut game = game();
        game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let bob_artifact = permanent(&mut game, B, "Type: Artifact");
        permanent(&mut game, C, "Type: Artifact");
        game.move_object_by_game_rule(bob_artifact, Zone::Graveyard).unwrap();
        settle(&mut game);
        assert_eq!(game.player(B).unwrap().life, 19);
        assert_eq!(game.player(C).unwrap().life, 20);
        assert_eq!(game.player(A).unwrap().life, 20);
    }
}

#[test]
fn shriek_damages_the_controller_of_the_creature_that_died() {
    for definition in support::definitions("Shriek, Treblemaker", BODIES[2].1) {
        let mut game = game();
        game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let cara_creature = permanent(&mut game, C, "Type: Creature — Bear\nPower/Toughness: 2/2");
        game.move_object_by_game_rule(cara_creature, Zone::Graveyard).unwrap();
        settle(&mut game);
        assert_eq!(game.player(C).unwrap().life, 19);
        assert_eq!(game.player(B).unwrap().life, 20);
    }
}
