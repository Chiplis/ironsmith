#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn assert_card_has_exact_compiled_line(name: &str, expected: &str) {
    let definition = parse_oracle_card_definition(name);
    let lines = unprocessed_compiled_lines(&definition);
    assert!(
        lines.iter().any(|line| line == expected),
        "{name} should retain the exact token-copy follow-up surfaces; got {lines:#?}"
    );
}

#[test]
fn kindred_charge_keeps_plural_haste_and_exile_followups() {
    assert_card_has_exact_compiled_line(
        "Kindred Charge",
        "Choose a creature type. For each creature you control of the chosen type, create a token that's a copy of that creature. Those tokens gain haste. Exile them at the beginning of the next end step.",
    );
}

#[test]
fn cadric_keeps_singular_haste_and_sacrifice_followups() {
    assert_card_has_exact_compiled_line(
        "Cadric, Soul Kindler",
        "Whenever another nontoken legendary permanent you control enters, you may pay {1}. If you do, create a token that's a copy of it. That token gains haste. Sacrifice it at the beginning of the next end step.",
    );
}

#[test]
fn inline_copy_exception_family_stays_inline() {
    assert_card_has_exact_compiled_line(
        "Kiki-Jiki, Mirror Breaker",
        "{T}: Create a token that's a copy of target nonlegendary creature you control, except it has haste. Sacrifice it at the beginning of the next end step.",
    );
}
