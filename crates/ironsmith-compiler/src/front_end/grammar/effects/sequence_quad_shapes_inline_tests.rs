use super::*;
use crate::lexer::lex_line;

fn lex(raw: &str) -> Vec<OwnedLexToken> {
    lex_line(raw, 0).unwrap()
}

#[test]
fn parses_counted_exile_and_filter_shapes() {
    let counted = parse_counted_looked_card_exile_shape(&lex(
        "Exile up to two of them face down, then put the rest on the bottom",
    ))
    .unwrap();
    assert_eq!(counted.count, ChoiceCount::up_to(2));
    assert!(counted.includes_remainder);

    let reveal_tokens = lex("You may reveal up to one creature card from among them");
    let reveal = parse_may_reveal_looked_card_shape(&reveal_tokens).unwrap();
    assert_eq!(reveal.count, ChoiceCount::up_to(1));
    assert_eq!(
        TokenWordView::new(reveal.filter_tokens).word_refs(),
        vec!["creature", "card"]
    );
    assert!(parse_otherwise_revealed_hand_shape(&lex(
        "Otherwise, put the revealed cards into your hand"
    )));

    let compound_tokens = lex(
        "Exile up to one nonland card from among them and put the rest on the bottom of your library in a random order",
    );
    let compound = parse_exile_looked_card_and_remainder_shape(&compound_tokens).unwrap();
    assert_eq!(compound.count, ChoiceCount::up_to(1));
    assert_eq!(compound.order, LibraryBottomOrderAst::Random);
    assert_eq!(
        TokenWordView::new(compound.filter_tokens).word_refs(),
        vec!["nonland", "card"]
    );
}

#[test]
fn parses_named_and_sequence_markers() {
    let named_tokens =
        lex("If you reveal a card named black lotus this way, put it onto the battlefield");
    let named = parse_named_revealed_card_shape(&named_tokens).unwrap();
    assert_eq!(
        TokenWordView::new(named.name_tokens).word_refs(),
        vec!["black", "lotus"]
    );
    assert!(parse_then_shuffle_shape(&lex("then shuffle")));
}

#[test]
fn parses_discover_the_impossible_sequence_shapes() {
    assert!(parse_exile_one_and_bottom_remainder_shape(&lex(
        "Exile one of them face down and put the rest on the bottom of your library in a random order"
    )));

    let cast_tokens = lex(
        "You may cast the exiled card without paying its mana cost if it's an instant spell with mana value 2 or less",
    );
    let cast = parse_exiled_card_cast_filter_shape(&cast_tokens)
        .expect("free-cast condition should expose its typed filter surface");
    assert_eq!(
        TokenWordView::new(cast.filter_tokens).word_refs(),
        vec![
            "an", "instant", "spell", "with", "mana", "value", "2", "or", "less"
        ]
    );

    assert!(parse_exiled_card_hand_followup_shape(&lex(
        "If you don't, put that card into your hand"
    )));
}

#[test]
fn preserves_authored_and_or_in_looked_card_choice_shape() {
    let and_or_tokens = lex("Choose a creature card and/or a land card from among them");
    let and_or = parse_choose_looked_card_and_or_shape(&and_or_tokens)
        .expect("typed looked-card choice should parse");
    assert!(and_or.uses_and_or);

    let or_tokens = lex("Choose a creature card or a land card from among them");
    let or = parse_choose_looked_card_and_or_shape(&or_tokens)
        .expect("ordinary-or looked-card choice should still parse its surface");
    assert!(!or.uses_and_or);
}
