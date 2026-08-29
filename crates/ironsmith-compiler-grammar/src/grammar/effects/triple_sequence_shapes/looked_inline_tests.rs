use super::*;
use crate::lexer::{lex_line, split_lexed_sentences};

#[test]
fn parses_reveal_one_gain_mana_value_shape() {
    let tokens = lex_line(
            "Reveal the top three cards of your library and put one of them into your hand. You gain life equal to that card's mana value. Put all other cards revealed this way into your graveyard.",
            0,
        )
        .unwrap();
    let sentences = split_lexed_sentences(&tokens);
    assert!(
        parse_reveal_one_gain_mana_value_shape(sentences[0], sentences[1], sentences[2]).is_some()
    );
}

#[test]
fn parses_all_matching_looked_cards_as_mandatory_full_set() {
    let tokens = lex_line(
            "all land cards from among them onto the battlefield tapped and the rest on the bottom of your library in a random order",
            0,
        )
        .unwrap();
    let shape = parse_looked_move_action_shape(&tokens).expect("all-matching move shape");

    assert!(shape.all_matching);
    assert_eq!(shape.count, ChoiceCount::any_number());
    assert_eq!(tokens[shape.filter.start].as_word(), Some("land"));
    assert!(matches!(
        shape.destination,
        LookedMoveDestinationShape::Battlefield { tapped: true, .. }
    ));
}

#[test]
fn parses_explicit_looked_card_battlefield_controller() {
    let tokens = lex_line(
            "a nonland permanent card with mana value X or less from among them onto the battlefield under your control",
            0,
        )
        .unwrap();
    let shape = parse_looked_move_action_shape(&tokens).expect("looked-card move shape");

    assert!(matches!(
        shape.destination,
        LookedMoveDestinationShape::Battlefield {
            controller: Some(BattlefieldControllerShape::You),
            ..
        }
    ));
}

#[test]
fn parses_return_from_among_them_to_hand_surface() {
    let tokens = lex_line("a permanent card from among them to your hand", 0).unwrap();
    let shape = parse_looked_move_action_shape(&tokens).expect("return-to-hand move shape");

    assert!(matches!(
        shape.destination,
        LookedMoveDestinationShape::Hand
    ));
    assert_eq!(tokens[shape.filter.start].as_word(), Some("permanent"));
}

#[test]
fn parses_revealed_cards_not_deployed_as_exact_remainder() {
    let tokens = lex_line(
            "Then put all cards revealed this way that weren't put onto the battlefield on the bottom of your library in a random order.",
            0,
        )
        .unwrap();
    assert_eq!(
        parse_looked_remainder_shape(&tokens),
        Some(LookedRemainderShape::LibraryBottom(
            LibraryBottomOrderAst::Random
        ))
    );
}

#[test]
fn recognizes_same_name_permanent_candidate_restriction() {
    let tokens = lex_line(
            "You may put one of those cards onto the battlefield if it has the same name as a permanent.",
            0,
        )
        .unwrap();
    assert!(is_looked_same_name_permanent_battlefield_action(&tokens));
}
