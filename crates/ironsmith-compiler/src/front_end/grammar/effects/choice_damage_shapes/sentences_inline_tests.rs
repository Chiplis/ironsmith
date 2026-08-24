use super::*;
use crate::lexer::lex_line;

fn lex(text: &str) -> Vec<OwnedLexToken> {
    lex_line(text, 0).unwrap()
}

#[test]
fn captures_drain_reveal_and_unless_sentences() {
    let drain = lex("Each opponent loses X life and you gain X life, where X is 3.");
    assert!(parse_opponent_drain_sentence_shape(&drain).is_some());

    let reveal = lex("Reveal two creature cards from your hand.");
    assert_eq!(
        TokenWordView::new(
            parse_reveal_selected_hand_shape(&reveal)
                .unwrap()
                .descriptor_tokens
        )
        .to_word_refs(),
        ["two", "creature", "cards"]
    );

    let unless = lex("Destroy target creature unless its controller pays {2}.");
    assert!(parse_unless_sentence_shape(&unless).is_some());

    let relative = lex(
        "This spell deals damage to each opponent who controls more lands than you equal to the difference.",
    );
    let shape = parse_relative_opponent_damage_difference_shape(&relative).unwrap();
    assert_eq!(
        TokenWordView::new(shape.source_tokens).to_word_refs(),
        ["this", "spell"]
    );
    assert_eq!(
        TokenWordView::new(shape.filter_tokens).to_word_refs(),
        ["lands"]
    );
}
