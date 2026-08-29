use crate::lexer::lex_line;

use super::*;

#[test]
fn parses_zone_pair_and_for_each_shapes() {
    let tokens = lex_line("Exile all cards from target player's hand and graveyard", 0).unwrap();
    let pair = parse_search_exile_zone_pair_shape_lexed(&tokens).unwrap();
    assert_eq!(pair.first_zone, Zone::Hand);
    assert_eq!(pair.second_zone, Zone::Graveyard);

    let opponents = lex_line(
        "Exile all cards from all opponents' hands and graveyards",
        0,
    )
    .unwrap();
    let pair = parse_search_exile_zone_pair_shape_lexed(&opponents).unwrap();
    assert_eq!(pair.owner, PlayerFilter::Opponent);
    assert_eq!(pair.first_zone, Zone::Hand);
    assert_eq!(pair.second_zone, Zone::Graveyard);

    let changed = lex_line(
        "Exile all cards from all creatures' hands and graveyards",
        0,
    )
    .unwrap();
    assert!(
        parse_search_exile_zone_pair_shape_lexed(&changed).is_none(),
        "a non-player possessor must not acquire the all-opponents route"
    );

    let tokens = lex_line(
        "For each permanent destroyed this way, its controller draws a card",
        0,
    )
    .unwrap();
    let shape = parse_search_for_each_way_shape_lexed(&tokens).unwrap();
    assert_eq!(shape.kind, SearchForEachWayKind::DestroyedOrDied);
    assert_eq!(
        parser_token_word_refs(shape.iterated_filter_tokens.unwrap()),
        vec!["permanent"]
    );
    assert!(!shape.effect_tokens.unwrap().is_empty());

    let tokens = lex_line(
        "For each nontoken creature destroyed this way, you create a Treasure token",
        0,
    )
    .unwrap();
    let shape = parse_search_for_each_way_shape_lexed(&tokens).unwrap();
    assert_eq!(shape.kind, SearchForEachWayKind::DestroyedOrDied);
    assert_eq!(
        parser_token_word_refs(shape.iterated_filter_tokens.unwrap()),
        vec!["nontoken", "creature"]
    );

    let tokens = lex_line("For each creature card exiled this way, you gain 1 life", 0).unwrap();
    let shape = parse_search_for_each_way_shape_lexed(&tokens).unwrap();
    assert_eq!(shape.kind, SearchForEachWayKind::Exiled);
    assert_eq!(
        parser_token_word_refs(shape.iterated_filter_tokens.unwrap()),
        vec!["creature", "card"]
    );

    let tokens = lex_line(
        "For each land sacrificed this way, its controller may search their library",
        0,
    )
    .unwrap();
    let shape = parse_search_for_each_way_shape_lexed(&tokens).unwrap();
    assert_eq!(shape.kind, SearchForEachWayKind::Sacrificed);
    assert_eq!(
        parser_token_word_refs(shape.iterated_filter_tokens.unwrap()),
        vec!["land"]
    );

    let tokens = lex_line(
        "For each creature card put into a graveyard this way, you create a Zombie token",
        0,
    )
    .unwrap();
    let shape = parse_search_for_each_way_shape_lexed(&tokens).unwrap();
    assert_eq!(shape.kind, SearchForEachWayKind::PutIntoGraveyard);
    assert_eq!(
        parser_token_word_refs(shape.iterated_filter_tokens.unwrap()),
        vec!["creature", "card"]
    );
}
