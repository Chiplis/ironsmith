#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn permanent_with_counters(
    game: &mut crate::GameState,
    name: &str,
    controller: PlayerId,
    card_types: Vec<CardType>,
    plus_one_plus_one: u32,
    charge: u32,
) -> ObjectId {
    let definition = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(card_types)
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let id = game.create_object_from_definition(&definition, controller, Zone::Battlefield);
    let object = game.object_mut(id).expect("test permanent should exist");
    if plus_one_plus_one > 0 {
        object
            .counters
            .insert(CounterType::PlusOnePlusOne, plus_one_plus_one);
    }
    if charge > 0 {
        object.counters.insert(CounterType::Charge, charge);
    }
    id
}

#[test]
fn ascendant_acolyte_retains_the_typed_counter_aggregate() {
    let definition = parse_oracle_card_definition("Ascendant Acolyte");
    let debug = format!("{definition:#?}");
    let rendered = unprocessed_compiled_lines(&definition);

    assert!(
        debug.contains("CountersOn")
            && debug.contains("PlusOnePlusOne")
            && debug.contains("CountersAmong")
            && debug.contains("other: true"),
        "entry count must total +1/+1 counters among other controlled creatures: {debug}"
    );
    assert!(
        rendered[0].contains("for each +1/+1 counter on other creatures you control")
            || rendered[0].contains("for each +1/+1 counter among other creatures you control"),
        "the rendered count must retain the counter metric: {rendered:#?}"
    );
}

#[test]
fn ascendant_acolyte_counts_counters_not_creatures_when_it_enters() {
    let definition = parse_oracle_card_definition("Ascendant Acolyte");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    permanent_with_counters(
        &mut game,
        "Alice Creature A",
        alice,
        vec![CardType::Creature],
        3,
        4,
    );
    permanent_with_counters(
        &mut game,
        "Alice Creature B",
        alice,
        vec![CardType::Creature],
        1,
        0,
    );
    permanent_with_counters(
        &mut game,
        "Alice Artifact",
        alice,
        vec![CardType::Artifact],
        6,
        0,
    );
    permanent_with_counters(
        &mut game,
        "Bob Creature",
        bob,
        vec![CardType::Creature],
        8,
        0,
    );

    let hand_id = game.create_object_from_definition(&definition, alice, Zone::Hand);
    let acolyte_id = game
        .move_object_with_etb_processing(hand_id, Zone::Battlefield)
        .expect("Ascendant Acolyte should enter")
        .new_id;
    let counters = game
        .object(acolyte_id)
        .expect("Ascendant Acolyte should exist")
        .counters
        .get(&CounterType::PlusOnePlusOne)
        .copied()
        .unwrap_or_default();

    assert_eq!(
        counters, 4,
        "only the four +1/+1 counters on Alice's two other creatures count"
    );
}
