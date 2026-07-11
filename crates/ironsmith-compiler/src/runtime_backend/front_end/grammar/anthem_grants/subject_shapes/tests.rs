use super::*;
use crate::Zone;
use crate::runtime_backend::lexer::lex_line;

#[test]
fn parses_commander_controller_subject_to_typed_filter() {
    let tokens = lex_line("Commanders you control", 0).unwrap();
    let Some(AnthemSubjectGrammarMatch::Filter(filter)) =
        parse_exact_anthem_subject_grammar(&tokens)
    else {
        panic!("expected typed commander subject");
    };

    assert_eq!(filter.zone, Some(Zone::Battlefield));
    assert_eq!(filter.controller, Some(PlayerFilter::You));
    assert!(filter.is_commander);
}

#[test]
fn parses_attacking_token_controller_subject_to_typed_filter() {
    let tokens = lex_line("Attacking tokens you control", 0).unwrap();
    let Some(AnthemSubjectGrammarMatch::Filter(filter)) =
        parse_exact_anthem_subject_grammar(&tokens)
    else {
        panic!("expected typed attacking-token subject");
    };

    assert_eq!(filter.zone, Some(Zone::Battlefield));
    assert_eq!(filter.controller, Some(PlayerFilter::You));
    assert!(filter.token);
    assert!(filter.attacking);
}

#[test]
fn classifies_speculative_fragments_without_suffix_recovery() {
    for fragment in [
        "all abilities and",
        "you draw two cards lose 2 life and",
        "as long as enchanted permanent is an equipment it",
    ] {
        let tokens = lex_line(fragment, 0).unwrap();
        assert_eq!(
            parse_exact_anthem_subject_grammar(&tokens),
            Some(AnthemSubjectGrammarMatch::RejectFragment),
            "{fragment}"
        );
    }
}

#[test]
fn leaves_unrelated_subjects_for_existing_typed_grammar() {
    let tokens = lex_line("Other creatures you control", 0).unwrap();
    assert_eq!(parse_exact_anthem_subject_grammar(&tokens), None);
}
