use super::*;

#[cfg(test)]
#[test]
pub(super) fn selected_hand_reveal_token_creation_uses_the_unabridged_source_program() {
    let text = "Each player may reveal any number of creature cards from their hand. Then each player creates a 2/2 green Bear creature token for each card they revealed this way.";
    let tokens =
        crate::lexer::lex_line(text, 0).expect("selected hand reveal statement should lex");
    let effects = typed_selected_hand_reveal_token_creation_statement(&tokens)
        .expect("typed source sequence should be recognized");
    let debug = format!("{effects:#?}");
    assert!(debug.contains("ChooseObjects"), "{debug}");
    assert!(debug.contains("RevealTagged"), "{debug}");
    assert!(debug.contains("CardsRevealedThisWay"), "{debug}");
}

#[cfg(test)]
#[test]
pub(super) fn whole_hand_reveal_does_not_match_selected_hand_sequence() {
    let text = "Each player may reveal their hand. Then each player creates a 1/1 green Saproling creature token.";
    let tokens = crate::lexer::lex_line(text, 0).expect("whole hand reveal near miss should lex");
    assert!(typed_selected_hand_reveal_token_creation_statement(&tokens).is_none());
}
