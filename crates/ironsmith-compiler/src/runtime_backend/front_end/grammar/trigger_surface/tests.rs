use super::*;
use crate::runtime_backend::front_end::lexer::lex_line;

fn tokens(text: &str) -> Vec<OwnedLexToken> {
    lex_line(text, 0).unwrap()
}

fn assert_frequency_condition(text: &str, limit: u32, expected: crate::ConditionExpr) {
    assert_eq!(
        parse_trigger_frequency_condition_tokens(&tokens(text), Some(limit)),
        Some(expected)
    );
}

#[test]
fn typed_line_family_migration_distinguishes_trigger_prefix_from_labeled_surface() {
    let direct = tokens("Whenever you attack, draw a card.");
    assert_eq!(
        parse_trigger_intro_prefix_tokens(&direct),
        Some(TriggerIntroSurfaceAst::Whenever)
    );

    let labeled = tokens("Pack tactics — Whenever you attack, draw a card.");
    assert_eq!(parse_trigger_intro_prefix_tokens(&labeled), None);
    assert_eq!(
        parse_trigger_intro_surface_tokens(&labeled),
        Some(TriggerIntroSurfaceAst::Whenever)
    );
}

#[test]
fn recognizes_tapped_during_turn_fact() {
    let tokens = tokens("Whenever this creature becomes tapped during your turn, draw a card.");
    assert!(parse_becomes_tapped_during_your_turn_tokens(&tokens).is_some());
}

#[test]
fn recognizes_frequency_limit() {
    assert_eq!(
        parse_do_this_only_each_turn_limit_tokens(&tokens("Do this only twice each turn.")),
        Some(2)
    );
}

#[test]
fn parses_first_crewed_frequency_condition() {
    assert_frequency_condition(
        "Whenever this Vehicle becomes crewed for the first time each turn, draw a card.",
        1,
        crate::ConditionExpr::SourceFirstCrewedThisTurn,
    );
}

#[test]
fn parses_first_time_frequency_condition() {
    assert_frequency_condition(
        "Whenever one or more creatures attack you for the first time this turn, draw a card.",
        1,
        crate::ConditionExpr::FirstTimeThisTurn,
    );
}

#[test]
fn parses_do_this_frequency_condition() {
    assert_frequency_condition(
        "Do this only twice each turn.",
        2,
        crate::ConditionExpr::DoThisMaxTimesEachTurn(2),
    );
}

#[test]
fn parses_plain_max_frequency_condition() {
    assert_frequency_condition(
        "Whenever you cast a spell, draw a card.",
        3,
        crate::ConditionExpr::MaxTimesEachTurn(3),
    );
}
