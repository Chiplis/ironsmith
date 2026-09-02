use super::frequency::parse_do_this_only_each_turn_limit_tokens;
use super::*;
use crate::cards::builders::{PredicateAst, TriggerFrequencyPredicateAst};
use crate::lexer::lex_line;

fn tokens(text: &str) -> Vec<OwnedLexToken> {
    lex_line(text, 0).unwrap()
}

fn assert_frequency_condition(text: &str, limit: u32, expected: TriggerFrequencyPredicateAst) {
    assert_eq!(
        parse_trigger_frequency_condition_tokens(&tokens(text), Some(limit)),
        Some(PredicateAst::TriggerFrequency(expected))
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
        TriggerFrequencyPredicateAst::SourceFirstCrewedThisTurn,
    );
}

#[test]
fn parses_first_time_frequency_condition() {
    assert_frequency_condition(
        "Whenever one or more creatures attack you for the first time this turn, draw a card.",
        1,
        TriggerFrequencyPredicateAst::FirstTimeThisTurn,
    );
}

#[test]
fn parses_first_time_during_each_of_your_turns_frequency_condition() {
    let tokens = tokens(
        "Whenever you gain life for the first time during each of your turns, create a token.",
    );
    let frequency = parse_trigger_frequency_tokens(&tokens);
    assert!(frequency.first_time_during_each_of_your_turns);
    assert_eq!(
        parse_trigger_frequency_condition_tokens(&tokens, Some(1)),
        Some(PredicateAst::TriggerFrequency(
            TriggerFrequencyPredicateAst::FirstTimeThisTurn
        ))
    );
}

#[test]
fn parses_do_this_frequency_condition() {
    assert_frequency_condition(
        "Do this only twice each turn.",
        2,
        TriggerFrequencyPredicateAst::DoThisMaxTimesEachTurn(2),
    );
}

#[test]
fn parses_plain_max_frequency_condition() {
    assert_frequency_condition(
        "Whenever you cast a spell, draw a card.",
        3,
        TriggerFrequencyPredicateAst::MaxTimesEachTurn(3),
    );
}
