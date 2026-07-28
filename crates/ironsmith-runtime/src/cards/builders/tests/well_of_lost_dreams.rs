#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn well_of_lost_dreams_preserves_bounded_x_and_linked_draw_count() {
    let definition = parse_oracle_card_definition("Well of Lost Dreams");
    let lines = unprocessed_compiled_lines(&definition);

    assert_eq!(
        lines,
        [
            "Whenever you gain life, you may pay {X}, where X is less than or equal to the amount of life you gained. If you do, draw X cards."
        ]
    );

    let debug = format!("{:#?}", definition.abilities);
    for expected in [
        "x_value: None",
        "x_maximum: Some(EventValue(LifeAmount))",
        "count: EffectValue(",
    ] {
        assert!(
            debug.contains(expected),
            "Well of Lost Dreams should retain {expected:?}; got {debug}"
        );
    }
}
