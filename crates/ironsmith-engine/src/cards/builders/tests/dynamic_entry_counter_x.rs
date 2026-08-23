#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn dynamic_entry_counter_grants_render_the_exact_typed_entry_clause() {
    for (name, expected) in [
        (
            "Communal Brewing",
            "Whenever you cast a creature spell, that creature enters with X additional +1/+1 counters on it, where X is the number of ingredient counters on this enchantment.",
        ),
        (
            "Runadi, Behemoth Caller",
            "Whenever you cast a creature spell with mana value 5 or greater, that creature enters with X additional +1/+1 counters on it, where X is its mana value minus 4.",
        ),
        (
            "Wildgrowth Archaic",
            "Whenever you cast a creature spell, that creature enters with X additional +1/+1 counters on it, where X is the number of colors of mana spent to cast it.",
        ),
    ] {
        let definition = parse_oracle_card_definition(name);
        let lines = unprocessed_compiled_lines(&definition);
        assert!(
            lines.iter().any(|line| line == expected),
            "{name} should preserve its exact dynamic entry-counter clause; got {lines:#?}"
        );
    }
}
#[test]
fn runadi_preserves_the_typed_counter_threshold_in_its_haste_filter() {
    let definition = parse_oracle_card_definition("Runadi, Behemoth Caller");
    let lines = unprocessed_compiled_lines(&definition);
    assert!(
        lines.iter().any(|line| {
            line == "Creatures you control with three or more +1/+1 counters on them have haste."
        }),
        "Runadi should retain its three-counter haste threshold; got {lines:#?}"
    );
}
