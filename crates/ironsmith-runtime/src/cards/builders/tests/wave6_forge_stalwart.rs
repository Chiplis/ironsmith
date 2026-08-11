#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const FORGE_ORACLE: &str = "{T}: Add {C}.\n{T}: Choose target commander that entered this turn. Put a +1/+1 counter on it if it's a creature and a loyalty counter on it if it's a planeswalker.";
const STALWART_ORACLE: &str = "Menace\nWhenever one or more counters are put on a creature you control, if it's the first time counters have been put on that creature this turn, put a +1/+1 counter on that creature.";

fn creature(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build()
}

fn planeswalker(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Planeswalker])
        .loyalty(3)
        .build()
}

fn forge_counter_program(
    definition: &CardDefinition,
) -> (&crate::resolution::ResolutionSegment, crate::tag::TagKey) {
    let activated = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .nth(1)
        .expect("Forge should retain its second activated ability");
    let [target_segment, counter_segment] = activated.effects.segments.as_slice() else {
        panic!("expected target and counter segments: {activated:#?}");
    };
    let [target_root] = target_segment.default_effects.as_slice() else {
        panic!("expected one target declaration: {activated:#?}");
    };
    let target_tag = target_root
        .downcast_ref::<crate::effects::TaggedEffect>()
        .expect("target declaration should retain identity")
        .tag
        .clone();
    (counter_segment, target_tag)
}

fn execute_forge_counter_arms(
    definition: &CardDefinition,
    target_definition: &CardDefinition,
) -> (u32, u32) {
    let (counter_segment, target_tag) = forge_counter_program(definition);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(definition, alice, Zone::Battlefield);
    let target = game.create_object_from_definition(target_definition, alice, Zone::Battlefield);
    let snapshot = crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
        game.object(target).expect("target exists"),
        &game,
    );
    let mut context =
        crate::effects::ExecutionContext::new_default(source, alice).with_tagged_objects(
            std::collections::HashMap::from([(target_tag, vec![snapshot])]),
        );
    let program = crate::resolution::ResolutionProgram::new(vec![counter_segment.clone()]);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut context,
        alice,
        source,
        &program,
        None,
        &[],
    )
    .expect("Forge conditional counter arms should resolve");
    (
        game.counter_count(target, crate::CounterType::PlusOnePlusOne),
        game.counter_count(target, crate::CounterType::Loyalty),
    )
}

#[test]
fn forge_counter_arms_share_the_declared_commander_target() {
    let definition = parse_oracle_card_definition("Forge of Heroes");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        FORGE_ORACLE
    );

    let (segment, target_tag) = forge_counter_program(&definition);
    let [sequence_root] = segment.default_effects.as_slice() else {
        panic!("expected one coordinated counter sequence: {segment:#?}");
    };
    let sequence = sequence_root
        .downcast_ref::<crate::effects::SequenceEffect>()
        .expect("counter arms should remain coordinated");
    let [creature_root, planeswalker_root] = sequence.effects.as_slice() else {
        panic!("expected creature and planeswalker arms: {sequence:#?}");
    };
    for root in [creature_root, planeswalker_root] {
        let put = root
            .downcast_ref::<crate::effects::TaggedEffect>()
            .and_then(|tagged| {
                tagged
                    .effect
                    .downcast_ref::<crate::effects::PutCountersEffect>()
            })
            .expect("each arm should remain a typed counter placement");
        let ChooseSpec::Object(filter) = put.target.unhinted() else {
            panic!("counter arm should use a tagged object filter: {put:#?}");
        };
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag == target_tag
                && constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
        }));
    }

    assert_eq!(
        execute_forge_counter_arms(&definition, &creature("Commander Creature")),
        (1, 0)
    );
    assert_eq!(
        execute_forge_counter_arms(&definition, &planeswalker("Commander Planeswalker")),
        (0, 4),
        "a planeswalker starts with three loyalty and receives only one loyalty counter"
    );
}

fn put_counter_and_queue(
    game: &mut crate::GameState,
    queue: &mut crate::triggers::TriggerQueue,
    permanent: ObjectId,
) -> usize {
    let event = game
        .add_counters(permanent, crate::CounterType::Charge, 1)
        .expect("counter recipient should exist");
    game.record_turn_history_event(&event);
    let entries = crate::triggers::check_triggers(game, &event);
    let count = entries.len();
    for entry in entries {
        queue.add(entry);
    }
    if count > 0 {
        crate::game_loop::put_triggers_on_stack(game, queue)
            .expect("matching Stalwart trigger should go on the stack");
    }
    count
}

fn resolve_only_trigger(game: &mut crate::GameState) {
    assert_eq!(game.stack.len(), 1, "expected exactly one Stalwart trigger");
    crate::game_loop::resolve_stack_entry(game).expect("Stalwart trigger should resolve");
}

#[test]
fn stalwart_first_counter_event_is_scoped_to_each_creature_and_turn() {
    let definition = parse_oracle_card_definition("Stalwart Successor");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        STALWART_ORACLE
    );

    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Stalwart should retain its counter trigger");
    assert_eq!(
        triggered.intervening_if,
        Some(crate::ConditionExpr::TriggeringObjectHadCountersPutFirstTimeThisTurn)
    );
    assert!(
        triggered
            .trigger
            .downcast_ref::<crate::triggers::CounterPutOnTrigger>()
            .is_some()
    );

    let alice = PlayerId::from_index(0);
    let mut game = crate::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let _stalwart = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let alpha = game.create_object_from_definition(&creature("Alpha"), alice, Zone::Battlefield);
    let beta = game.create_object_from_definition(&creature("Beta"), alice, Zone::Battlefield);
    let mut queue = crate::triggers::TriggerQueue::new();

    assert_eq!(put_counter_and_queue(&mut game, &mut queue, alpha), 1);
    resolve_only_trigger(&mut game);
    assert_eq!(
        game.counter_count(alpha, crate::CounterType::PlusOnePlusOne),
        1
    );

    assert_eq!(
        put_counter_and_queue(&mut game, &mut queue, alpha),
        0,
        "a second counter event on the same creature this turn must not trigger"
    );
    assert_eq!(
        put_counter_and_queue(&mut game, &mut queue, beta),
        1,
        "a different creature keeps its own first counter event"
    );
    resolve_only_trigger(&mut game);
    assert_eq!(
        game.counter_count(beta, crate::CounterType::PlusOnePlusOne),
        1
    );

    game.turn_store.turn_history.clear_for_new_turn();
    assert_eq!(
        put_counter_and_queue(&mut game, &mut queue, alpha),
        1,
        "counter history resets for the next turn"
    );
    resolve_only_trigger(&mut game);
    assert_eq!(
        game.counter_count(alpha, crate::CounterType::PlusOnePlusOne),
        2
    );
}
