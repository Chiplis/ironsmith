use crate::runtime_backend::lexer::lex_line;

use super::*;

#[test]
fn parses_shuffle_shapes() {
    let tokens = lex_line("Each player shuffles their graveyard into their library", 0).unwrap();
    let shape = parse_shuffle_graveyard_shape_lexed(&tokens).unwrap();
    assert!(shape.each_player_subject);
    assert!(!shape.has_target_selector);

    let tokens = lex_line("Its owner shuffles it into their library", 0).unwrap();
    let shape = parse_shuffle_object_shape_lexed(&tokens).unwrap();
    assert_eq!(shape.reference, SearchShuffleObjectReference::General);

    let tokens = lex_line(
        "The owner of target creature shuffles it into their library",
        0,
    )
    .unwrap();
    let shape = parse_shuffle_object_shape_lexed(&tokens).unwrap();
    assert_eq!(
        shape.reference,
        SearchShuffleObjectReference::SingularBackReference
    );

    let tokens = lex_line("Its owner shuffles them into their library", 0).unwrap();
    let shape = parse_shuffle_object_shape_lexed(&tokens).unwrap();
    assert_eq!(
        shape.reference,
        SearchShuffleObjectReference::PluralTaggedReference
    );

    let tokens = lex_line(
        "Each of them searches their library for a card, then shuffles and puts that card on top.",
        0,
    )
    .unwrap();
    assert!(parse_each_chosen_player_search_put_top_shape(&tokens).is_some());
}
