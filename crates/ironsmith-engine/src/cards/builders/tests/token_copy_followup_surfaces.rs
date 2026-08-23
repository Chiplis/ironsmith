#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::{oracle_text_by_name, parse_oracle_card_definition};
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

#[test]
fn rionya_keeps_dynamic_token_copies_and_plural_followups() {
    let name = "Rionya, Fire Dancer";
    let definition = parse_oracle_card_definition(name);
    let debug = format!("{definition:#?}");
    assert!(
        debug.contains("CreateTokenCopyEffect")
            && debug.contains("TurnHistoryCount")
            && debug.contains("has_haste: true")
            && debug.contains("exile_at_next_end_step: true"),
        "Rionya must create a history-counted set of hasty temporary token copies: {debug}"
    );
    assert!(
        !debug.contains("CopySpellEffect"),
        "Rionya targets a battlefield creature to copy into tokens, not a spell: {debug}"
    );

    let compiled = unprocessed_compiled_lines(&definition);
    assert!(
        compiled.iter().any(|line| line
            == "At the beginning of combat on your turn, create X tokens that are copies of another target creature you control, where X is 1 plus the number of instant and sorcery spells you've cast this turn. They gain haste. Exile them at the beginning of the next end step."),
        "Rionya's typed token-copy program should retain its count and plural followups: {compiled:?}"
    );

    let oracle = oracle_text_by_name()
        .get(name)
        .unwrap_or_else(|| panic!("missing oracle text for {name}"));
    let (_, _, similarity, _, mismatch) = crate::semantic_compare::compare_card_semantics_scored(
        name,
        oracle,
        &compiled,
        crate::semantic_compare::report_embedding_config(),
    );
    assert!(
        similarity >= 0.99 && !mismatch,
        "Rionya must clear the strict semantic floor, score={similarity}, mismatch={mismatch}, compiled={compiled:?}"
    );
}
