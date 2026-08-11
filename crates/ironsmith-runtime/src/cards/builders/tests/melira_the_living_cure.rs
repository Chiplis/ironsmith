#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::object::CounterType;

#[test]
fn melira_limits_actual_poison_received_to_one_each_turn() {
    let definition = parse_oracle_card_definition("Melira, the Living Cure");
    let debug = format!("{definition:#?}");
    assert!(
        debug.contains("PlayerCounterPerTurnLimitReplacement")
            && debug.contains("counter_type: Poison")
            && debug.contains("maximum: 1"),
        "Melira must compile to the generic per-turn player-counter replacement: {debug}"
    );
    let rendered = compiled_text_lines(&definition).join("\n");
    assert!(
        rendered.contains(
            "If you would get one or more poison counters, instead you get one poison counter and you can't get additional poison counters this turn."
        ),
        "Melira's replacement surface should survive compilation: {rendered}"
    );

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    game.add_player_counters_with_source(alice, CounterType::Poison, 2, None, None);
    assert_eq!(game.player(alice).unwrap().poison_counters, 2);
    let melira = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.update_replacement_effects();

    let first = game
        .add_player_counters_with_source(alice, CounterType::Poison, 4, Some(melira), Some(alice))
        .expect("the first poison event should still add one counter");
    let first_markers = first
        .downcast::<crate::events::MarkersChangedEvent>()
        .expect("player counters should emit a markers-changed event");
    assert_eq!(first_markers.amount, 1);
    assert_eq!(
        game.player(alice).unwrap().poison_counters,
        3,
        "Melira's lock starts when her replacement applies; it is not a cap on counters received before that event"
    );

    assert!(
        game.add_player_counters_with_source(
            alice,
            CounterType::Poison,
            3,
            Some(melira),
            Some(alice),
        )
        .is_none(),
        "additional poison placements in the same turn should be replaced with zero"
    );
    assert_eq!(game.player(alice).unwrap().poison_counters, 3);

    game.turn_store.turn_history.clear_for_new_turn();
    let next_turn = game.add_player_counters_with_source(
        alice,
        CounterType::Poison,
        2,
        Some(melira),
        Some(alice),
    );
    assert!(next_turn.is_some(), "the allowance must reset next turn");
    assert_eq!(game.player(alice).unwrap().poison_counters, 4);

    game.move_object_by_effect(melira, Zone::Graveyard);
    game.update_replacement_effects();
    assert!(
        game.add_player_counters_with_source(alice, CounterType::Poison, 3, None, None)
            .is_none(),
        "the established turn lock must survive after Melira leaves the battlefield"
    );
    assert_eq!(game.player(alice).unwrap().poison_counters, 4);

    game.turn_store.turn_history.clear_for_new_turn();
    game.add_player_counters_with_source(alice, CounterType::Poison, 3, None, None);
    assert_eq!(
        game.player(alice).unwrap().poison_counters,
        7,
        "the cap must disappear when its source leaves the battlefield"
    );
}
