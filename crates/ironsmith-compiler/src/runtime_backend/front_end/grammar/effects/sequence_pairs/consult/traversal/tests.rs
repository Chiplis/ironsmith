use super::*;
use crate::runtime_backend::front_end::lexer::lex_line;

fn lex(raw: &str) -> Vec<OwnedLexToken> {
    lex_line(raw, 0).unwrap()
}

#[test]
fn parses_active_and_passive_consult_traversal_surfaces() {
    let active = parse_consult_traversal_shape(&lex(
        "Exile cards from the top of your library until you exile a nonland card",
    ))
    .unwrap();
    assert_eq!(active.mode, LibraryConsultModeAst::Exile);
    assert_eq!(
        active.stop.stop_rule,
        LibraryConsultStopRuleAst::MatchCount(Value::Fixed(1))
    );
    assert!(permission_shapes::exact_tokens(
        &active.stop.filter,
        &["nonland", "card"]
    ));

    let passive = parse_consult_traversal_shape(&lex(
        "Reveal cards from the top of your library until two creature cards are revealed",
    ))
    .unwrap();
    assert_eq!(passive.mode, LibraryConsultModeAst::Reveal);
    assert_eq!(
        passive.stop.stop_rule,
        LibraryConsultStopRuleAst::MatchCount(Value::Fixed(2))
    );
    assert!(permission_shapes::exact_tokens(
        &passive.stop.filter,
        &["creature"]
    ));
}

#[test]
fn captures_a_preceding_effect_before_then() {
    let parsed = parse_consult_traversal_shape(&lex(
        "Target opponent mills a card, then they reveal cards from the top of their library until a land card is revealed",
    ))
    .unwrap();
    assert!(parsed.prefix.is_some());
    assert_eq!(parsed.player, ConsultTraversalPlayerShape::ThatPlayer);
}

#[test]
fn captures_where_x_value_and_inline_followup() {
    let parsed = parse_consult_traversal_shape(&lex(
        "Reveal cards from the top of your library until you reveal X permanent cards, where X is the number of colors among permanents you control, put any number of those permanent cards onto the battlefield",
    ))
    .unwrap();
    let Some(Value::ColorsAmong(filter)) = parsed.where_x else {
        panic!("expected colors-among binding, got {:?}", parsed.where_x);
    };
    assert_eq!(filter.controller, Some(crate::target::PlayerFilter::You));
    assert_eq!(
        TokenWordView::new(&parsed.trailing_effect).word_refs(),
        vec![
            "put",
            "any",
            "number",
            "of",
            "those",
            "permanent",
            "cards",
            "onto",
            "the",
            "battlefield"
        ]
    );
}

#[test]
fn captures_first_match_or_exposed_count_stop() {
    let parsed = parse_consult_traversal_shape(&lex(
        "Target opponent reveals cards from the top of their library until an artifact card or X cards are revealed, whichever comes first",
    ))
    .unwrap();
    assert_eq!(parsed.stop.stop_rule, LibraryConsultStopRuleAst::FirstMatch);
    assert_eq!(parsed.stop.max_exposed, Some(Value::X));
    assert!(permission_shapes::exact_tokens(
        &parsed.stop.filter,
        &["an", "artifact", "card"]
    ));
    assert!(parsed.trailing_effect.is_empty());
}

#[test]
fn does_not_absorb_an_outer_for_each_header_into_the_consult_subject() {
    let tokens = lex(
        "For each creature exiled this way, its controller reveals cards from the top of their library until they reveal a creature card, puts that card onto the battlefield, then puts the rest on the bottom of their library in a random order.",
    );
    assert!(parse_consult_traversal_shape(&tokens).is_none());

    let each_opponent = lex(
        "Each opponent reveals cards from the top of their library until they reveal X land cards, then puts all cards revealed this way into their graveyard.",
    );
    assert!(parse_consult_traversal_shape(&each_opponent).is_some());
}
