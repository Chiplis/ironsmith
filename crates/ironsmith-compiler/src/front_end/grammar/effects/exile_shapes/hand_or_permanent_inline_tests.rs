use super::*;
use crate::lexer::lex_line;

fn lex(raw: &str) -> Vec<OwnedLexToken> {
    lex_line(raw, 0).unwrap()
}

#[test]
fn parses_each_opponent_hand_or_permanent_choice() {
    let parsed = parse_each_opponent_exile_choice_shape(&lex(
        "Each opponent exiles a card from their hand or a permanent they control",
    ))
    .unwrap();
    assert!(is_exile_hand_or_permanent_choice_shape(&parsed.choice));
    assert!(is_exile_hand_or_permanent_choice_shape(&lex(
        "card from that player's hand or permanent that player controls"
    )));
}

#[test]
fn parses_each_player_counted_permanents_and_or_hand_cards() {
    let parsed = parse_each_player_exile_counted_hand_permanent_shape(&lex(
        "Each player exiles X permanents they control and/or cards from their hand",
    ))
    .unwrap();
    assert_eq!(parsed.group, EachPlayerExileGroup::Player);

    let split_connector = parse_each_player_exile_counted_hand_permanent_shape(&lex(
        "Each opponent exiles X permanents they control and or cards from their hand",
    ))
    .unwrap();
    assert_eq!(split_connector.group, EachPlayerExileGroup::Opponent);
}
