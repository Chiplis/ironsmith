use super::*;

#[test]
fn dependent_exile_keeps_the_revealing_player_as_actor() {
    let lexed = crate::lexer::lex_line(
            "Target opponent reveals X cards from their hand, where X is the number of Goblins you control. You choose one of those cards. That player exiles it.",
            0,
        )
        .expect("dependent hand reveal should lex");
    let parsed = parse_effect_sentences_lexed(&lexed).expect("dependent hand reveal should parse");
    let debug = format!("{parsed:#?}");
    assert!(debug.contains("player: That"), "{debug}");
}
