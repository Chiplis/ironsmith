#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::GameState;
use crate::game_state::Target;
use crate::object::CounterType;

fn triggered_ability(definition: &CardDefinition) -> &TriggeredAbility {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("the card should have a triggered ability")
}

fn resolve_trigger(
    game: &mut GameState,
    source: ObjectId,
    controller: PlayerId,
    triggered: &TriggeredAbility,
    targets: Vec<Target>,
) {
    let trigger_identity = crate::triggers::compute_trigger_identity(triggered);
    let entry =
        crate::game_state::StackEntry::ability(source, controller, triggered.effects.clone())
            .with_targets(targets)
            .with_trigger_identity(trigger_identity);
    game.push_to_stack(entry);
    let mut decision_maker = crate::decision::SelectFirstDecisionMaker;
    crate::game_loop::resolve_stack_entry_with(game, &mut decision_maker)
        .expect("the triggered ability should resolve");
}

#[test]
fn rumor_gatherer_draws_only_on_the_second_resolution_and_resets_next_turn() {
    let definition = parse_oracle_card_definition("Rumor Gatherer");
    let triggered = triggered_ability(&definition);
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let library_card = CardDefinitionBuilder::new(CardId::new(), "Rumor Fodder")
        .card_types(vec![CardType::Sorcery])
        .build();
    for _ in 0..3 {
        game.create_object_from_definition(&library_card, alice, Zone::Library);
    }

    resolve_trigger(&mut game, source, alice, triggered, vec![]);
    assert_eq!(
        game.player(alice).expect("Alice should exist").hand.len(),
        0,
        "the first resolution should scry rather than draw"
    );

    resolve_trigger(&mut game, source, alice, triggered, vec![]);
    assert_eq!(
        game.player(alice).expect("Alice should exist").hand.len(),
        1,
        "the second resolution should replace scry with drawing a card"
    );

    game.next_turn();
    resolve_trigger(&mut game, source, alice, triggered, vec![]);
    assert_eq!(
        game.player(alice).expect("Alice should exist").hand.len(),
        1,
        "the resolution count should reset on the next turn"
    );
}

#[test]
fn scythecat_cub_doubles_only_that_target_on_the_second_resolution_and_resets_next_turn() {
    let definition = parse_oracle_card_definition("Scythecat Cub");
    let rendered = canonical_compiled_lines(&definition);
    assert!(
        rendered
            .iter()
            .all(|line| !line.contains("double the number of +1/+1 counters on each creature")),
        "Scythecat Cub's singular antecedent must not widen to every creature: {rendered:#?}"
    );
    assert!(
        rendered
            .iter()
            .any(|line| line
                .contains("double the number of +1/+1 counters on that creature instead")),
        "the replacement should render the shared target as an anaphor: {rendered:#?}"
    );
    let triggered = triggered_ability(&definition);
    let [segment] = triggered.effects.segments.as_slice() else {
        panic!(
            "Scythecat Cub should have one resolution segment: {:#?}",
            triggered.effects
        );
    };
    let [target_declaration, put_counters] = segment.default_effects.as_slice() else {
        panic!("expected a target prelude and default counter action: {segment:#?}");
    };
    let target_declaration = target_declaration
        .downcast_ref::<crate::effects::TaggedEffect>()
        .expect("the shared target declaration should be tagged");
    let target_tag = target_declaration.tag.clone();
    assert!(
        target_declaration
            .effect
            .downcast_ref::<crate::effects::TargetOnlyEffect>()
            .is_some(),
        "the default effects should expose one target declaration: {target_declaration:#?}"
    );
    let put_counters = put_counters
        .downcast_ref::<crate::effects::TaggedEffect>()
        .and_then(|tagged| {
            tagged
                .effect
                .downcast_ref::<crate::effects::PutCountersEffect>()
        })
        .expect("the default action should put a counter");
    assert!(
        matches!(&put_counters.target, ChooseSpec::Tagged(tag) if tag == &target_tag),
        "the default action should consume the declared target: {put_counters:#?}"
    );
    let [replacement] = segment.self_replacements[0].replacement_effects.as_slice() else {
        panic!("expected one counter-doubling replacement: {segment:#?}");
    };
    let replacement = replacement
        .downcast_ref::<crate::effects::DoubleCountersEffect>()
        .expect("the replacement should double counters");
    assert!(
        matches!(&replacement.target, ChooseSpec::Tagged(tag) if tag == &target_tag),
        "the replacement should consume the same declared target: {replacement:#?}"
    );
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let creature = CardDefinitionBuilder::new(CardId::new(), "Counter Recipient")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let target = game.create_object_from_definition(&creature, alice, Zone::Battlefield);
    let unrelated = game.create_object_from_definition(&creature, alice, Zone::Battlefield);
    game.add_counters(target, CounterType::PlusOnePlusOne, 2)
        .expect("the target should accept +1/+1 counters");
    game.add_counters(unrelated, CounterType::PlusOnePlusOne, 4)
        .expect("the unrelated creature should accept +1/+1 counters");

    resolve_trigger(
        &mut game,
        source,
        alice,
        triggered,
        vec![Target::Object(target)],
    );
    assert_eq!(game.counter_count(target, CounterType::PlusOnePlusOne), 3);
    assert_eq!(
        game.counter_count(unrelated, CounterType::PlusOnePlusOne),
        4
    );

    resolve_trigger(
        &mut game,
        source,
        alice,
        triggered,
        vec![Target::Object(target)],
    );
    assert_eq!(
        game.counter_count(target, CounterType::PlusOnePlusOne),
        6,
        "the second resolution should double the counters on the announced target"
    );
    assert_eq!(
        game.counter_count(unrelated, CounterType::PlusOnePlusOne),
        4,
        "\"that creature\" must not become an all-creatures filter"
    );

    game.next_turn();
    resolve_trigger(
        &mut game,
        source,
        alice,
        triggered,
        vec![Target::Object(target)],
    );
    assert_eq!(
        game.counter_count(target, CounterType::PlusOnePlusOne),
        7,
        "the next turn should begin with the ordinary put-one-counter result"
    );
    assert_eq!(
        game.counter_count(unrelated, CounterType::PlusOnePlusOne),
        4
    );
}
