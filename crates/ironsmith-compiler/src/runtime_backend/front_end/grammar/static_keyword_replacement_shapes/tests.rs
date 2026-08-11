use super::*;
use crate::cards::builders::TextSpan;
use crate::runtime_backend::front_end::lexer::lex_line;

fn tokens(words: &[&str]) -> Vec<OwnedLexToken> {
    words
        .iter()
        .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
        .collect()
}

#[test]
fn parses_prevention_prefixes() {
    let line = tokens(&[
        "prevent",
        "all",
        "combat",
        "damage",
        "that",
        "would",
        "be",
        "dealt",
        "to",
        "creatures",
    ]);
    assert_eq!(parse_combat_prevention_prefix(&line).unwrap().end, 9);
}

#[test]
fn parses_draw_reveal_matching_rest_bottom_shape() {
    let line = tokens(&[
        "if", "you", "would", "draw", "a", "card", "instead", "reveal", "the", "top", "three",
        "cards", "of", "your", "library", "put", "all", "creature", "cards", "revealed", "this",
        "way", "into", "your", "hand", "and", "the", "rest", "on", "the", "bottom", "of", "your",
        "library", "in", "a", "random", "order",
    ]);
    let parsed = parse_draw_reveal_matching_rest_bottom(&line).unwrap();
    assert_eq!(parsed.count, 3);
    assert_eq!(parsed.card_type_word, "creature");
    assert_eq!(parsed.order, LibraryBottomOrderShape::Random);
}

#[test]
fn parses_discard_or_redirect_replacement_as_one_typed_shape() {
    for text in [
        "If this artifact would enter, you may discard a land card instead. If you do, put this artifact onto the battlefield. If you don't, put it into its owner's graveyard.",
        "If Mox Diamond would enter the battlefield, you may discard a land card instead. If you don't, put it into its owner's graveyard.",
        "If this artifact would enter the battlefield, you may discard a land card instead. If you do, put this artifact onto the battlefield. If you don't, put it into its owner's graveyard.",
    ] {
        let line = lex_line(text, 0).expect("replacement text should lex");
        let shape = parse_discard_or_redirect_replacement(&line)
            .expect("replacement sentences should form one typed shape");
        assert_eq!(shape.discard_type, CardType::Land, "{text}");
        assert_eq!(shape.redirect_zone, Zone::Graveyard, "{text}");
    }
}

#[test]
fn parses_sacrifice_or_redirect_replacement_as_one_typed_shape() {
    let text = "If this land would enter, sacrifice two untapped lands instead. If you do, put this land onto the battlefield. If you don't, put it into its owner's graveyard.";
    let line = lex_line(text, 0).expect("replacement text should lex");
    let shape = parse_sacrifice_or_redirect_replacement(&line)
        .expect("replacement sentences should form one typed shape");
    assert_eq!(shape.count, 2);
    assert_eq!(
        TokenWordView::new(shape.filter_tokens).word_refs(),
        ["untapped", "lands"]
    );
    assert_eq!(shape.redirect_zone, Zone::Graveyard);
}
