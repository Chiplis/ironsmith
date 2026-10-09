//! Gameplay scenarios for the appended delayed-trigger kinds (deals damage,
//! attacks alone). Source-authored, deliberately UNRUN.
use ironsmith::combat_state::{AttackTarget, CombatState};
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::effects::{EffectContext, ScheduleDelayedTriggerEffect, execute_effect};
use ironsmith::events::cause::EventCause;
use ironsmith::events::{DamageEvent, DamageTarget};
use ironsmith::game_loop::{apply_attacker_declarations, put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::target::{ObjectFilter, PlayerFilter};
use ironsmith::triggers::{Trigger, TriggerEvent, TriggerQueue, check_delayed_triggers};
use ironsmith::{Effect, GameState, ObjectId, Phase, PlayerId, Zone};
use ironsmith_compiler_runtime::compile_to_runtime_definition;
use ironsmith_core::DelayedTriggerSpec;

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);

fn game() -> GameState {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game
}

fn creature(game: &mut GameState, owner: PlayerId) -> ObjectId {
    let def = compile_to_runtime_definition("Probe creature", "Type: Creature\nPower/Toughness: 2/2", false).unwrap();
    let id = game.create_object_from_definition(&def, owner, Zone::Battlefield);
    game.remove_summoning_sickness(id);
    id
}

fn register(game: &mut GameState, source: ObjectId, spec: DelayedTriggerSpec, watched: Vec<ObjectId>) {
    let schedule = ScheduleDelayedTriggerEffect::new(
        Trigger::from_delayed_trigger_spec(spec),
        vec![Effect::gain_life(1)],
        false,
        watched,
        PlayerFilter::You,
    )
    .until_end_of_turn();
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = EffectContext::new(source, A, &mut dm);
    execute_effect(game, &Effect::new(schedule), &mut ctx).unwrap();
}

fn settle(game: &mut GameState, mut queue: TriggerQueue) {
    for _ in 0..10 {
        put_triggers_on_stack_with_dm(game, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
        if game.stack.is_empty() {
            return;
        }
        resolve_stack_entry_with(game, &mut SelectFirstDecisionMaker).unwrap();
    }
    panic!("stack did not settle");
}

fn damage(source: ObjectId, combat: bool) -> TriggerEvent {
    let cause = if combat { EventCause::combat_damage(source) } else { EventCause::from_effect(source, A) };
    TriggerEvent::new_with_provenance(
        DamageEvent::with_cause(source, DamageTarget::Player(B), 2, combat, cause),
        Default::default(),
    )
}

#[test]
fn watched_object_deals_any_damage_and_only_it_triggers() {
    let mut game = game();
    let spell_source = creature(&mut game, A);
    let watched = creature(&mut game, A);
    let other = creature(&mut game, A);
    register(&mut game, spell_source, DelayedTriggerSpec::DealsDamage { source: ObjectFilter::source() }, vec![watched]);

    assert!(check_delayed_triggers(&mut game, &damage(other, true)).is_empty(), "an unwatched creature");
    for combat in [true, false] {
        let entries = check_delayed_triggers(&mut game, &damage(watched, combat));
        assert_eq!(entries.len(), 1, "combat={combat}: combat and noncombat damage both count (CR 120.1)");
        let mut queue = TriggerQueue::new();
        for entry in entries {
            queue.add(entry);
        }
        let life = game.player(A).unwrap().life;
        settle(&mut game, queue);
        assert_eq!(game.player(A).unwrap().life, life + 1);
    }

    game.turn.turn_number += 1;
    assert!(check_delayed_triggers(&mut game, &damage(watched, true)).is_empty(), "this-turn registration expired");
}

fn declare(game: &mut GameState, attackers: &[ObjectId]) -> TriggerQueue {
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
    game.turn_store.combat_phases_started_this_turn += 1;
    game.combat = None;
    let declarations = attackers
        .iter()
        .map(|creature| {
            game.untap(*creature);
            ironsmith::decision::AttackerDeclaration { creature: *creature, target: AttackTarget::Player(B) }
        })
        .collect::<Vec<_>>();
    let mut queue = TriggerQueue::new();
    apply_attacker_declarations(game, &mut CombatState::default(), &mut queue, &declarations).unwrap();
    queue
}

#[test]
fn attacks_alone_fires_only_for_a_lone_attacker() {
    let mut controlled = ObjectFilter::creature();
    controlled.controller = Some(PlayerFilter::You);
    for (attackers, expected) in [(1usize, 1usize), (2, 0)] {
        let mut game = game();
        let saga = creature(&mut game, A);
        let first = creature(&mut game, A);
        let second = creature(&mut game, A);
        register(&mut game, saga, DelayedTriggerSpec::AttacksAlone(controlled.clone()), Vec::new());
        let ids = [first, second];
        let queue = declare(&mut game, &ids[..attackers]);
        assert_eq!(queue.entries.len(), expected, "{attackers} attacker(s) (CR 506.5)");
        let life = game.player(A).unwrap().life;
        settle(&mut game, queue);
        assert_eq!(game.player(A).unwrap().life, life + expected as i32);
    }
}
