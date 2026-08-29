use super::*;

#[test]
fn source_exiled_move_and_decline_fallback_stay_one_conditional() {
    let lexed = crate::lexer::lex_line(
            "You may put the exiled card onto the battlefield if it's a creature card. If you don't put it onto the battlefield, put it into its owner's hand.",
            0,
        )
        .expect("source-exiled move should lex");
    let parsed = parse_effect_sentences_lexed(&lexed)
        .expect("source-exiled move and decline fallback should parse");

    assert_eq!(parsed.len(), 1, "{parsed:#?}");
}
