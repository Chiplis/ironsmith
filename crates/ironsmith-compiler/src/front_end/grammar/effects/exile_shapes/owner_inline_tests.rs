use super::*;
use crate::lexer::lex_line;

fn lex(raw: &str) -> Vec<OwnedLexToken> {
    lex_line(raw, 0).unwrap()
}

#[test]
fn parses_zone_owner_prefixes_with_the_historical_word_count() {
    let graveyard =
        parse_exile_graveyard_owner_shape(&lex("that player's graveyard cards")).unwrap();
    assert_eq!(graveyard.player, PlayerAst::That);
    assert_eq!(graveyard.consumed_words, 3);

    let library =
        parse_exile_library_owner_shape(&lex("their library"), PlayerAst::Implicit).unwrap();
    assert_eq!(library.player, PlayerAst::ItsController);
    assert_eq!(library.consumed_words, 2);
    assert!(is_each_opponent_library_shape(&lex(
        "each opponent's library"
    )));
    assert!(is_each_player_library_shape(&lex("each player's library")));

    let each_type = parse_exile_one_per_card_type_from_graveyard_shape(&lex(
        "up to one card of each card type from defending player's graveyard",
    ))
    .unwrap();
    assert_eq!(each_type.owner, PlayerAst::Defending);
}
