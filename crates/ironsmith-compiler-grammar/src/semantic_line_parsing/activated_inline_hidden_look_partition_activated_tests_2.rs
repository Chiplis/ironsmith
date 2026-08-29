use super::*;

fn parse(text: &str) -> Option<Vec<EffectAst>> {
    let tokens = crate::lexer::lex_line(text, 0).expect("activated body should lex");
    parse_hidden_look_partition_activated(&tokens).expect("typed activated partition parser")
}

#[test]
fn activated_body_keeps_one_hidden_exiled_card_and_its_permission_linked() {
    let tokens = crate::lexer::lex_line(
            "Look at the top three cards of your library. Exile one face down and put the rest on the bottom of your library in any order. For as long as it remains exiled, you may cast it if it's a creature spell.",
            0,
        )
        .expect("activated body should lex");
    let effects = parse_activated_effects_lexed("", &tokens, 0)
        .expect("activated route should keep the exact hidden looked-card partition");
    let debug = format!("{effects:#?}");
    assert!(debug.contains("ChooseObjects"), "{debug}");
    assert!(
        debug.contains("GrantPlayTaggedForAsLongAsExiled"),
        "{debug}"
    );
    assert!(debug.contains("Creature"), "{debug}");
}

#[test]
fn unrelated_exile_one_sentence_is_not_claimed() {
    assert!(
            parse(
                "Look at the top three cards of your library. Exile one face up and put the rest on the bottom of your library in any order. Draw a card."
            )
            .is_none()
        );
}
