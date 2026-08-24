use super::*;

#[test]
fn ordinary_control_condition_is_not_a_prior_result_prefix() {
    let ordinary = crate::lexer::lex_line(
        "If you don't control a Human, you lose life equal to that creature's toughness.",
        0,
    )
    .expect("lex ordinary state condition");
    assert!(split_leading_result_prefix_lexed(&ordinary).is_none());

    let prior_result = crate::lexer::lex_line("If you don't, you lose 3 life.", 0)
        .expect("lex prior-result condition");
    assert!(matches!(
        split_leading_result_prefix_lexed(&prior_result),
        Some(LeadingResultPrefixSpec {
            kind: LeadingResultPrefixKind::If,
            predicate: IfResultPredicate::DidNot,
            ..
        })
    ));
}
