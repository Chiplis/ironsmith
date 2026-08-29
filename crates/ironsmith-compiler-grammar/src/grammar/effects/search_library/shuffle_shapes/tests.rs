use crate::lexer::lex_line;

use super::*;

#[test]
fn parses_shuffle_shapes() {
    let tokens = lex_line("Each player shuffles their graveyard into their library", 0).unwrap();
    let shape = parse_shuffle_graveyard_shape_lexed(&tokens).unwrap();
    assert!(shape.each_player_subject);
    assert!(!shape.has_target_selector);

    let tokens = lex_line(
        "Shuffle all creature cards from target player's graveyard into that player's library",
        0,
    )
    .unwrap();
    let shape = parse_shuffle_graveyard_shape_lexed(&tokens).unwrap();
    assert!(shape.has_target_selector);
    assert!(shape.owner_library_destination);

    let tokens = lex_line("Its owner shuffles it into their library", 0).unwrap();
    let shape = parse_shuffle_object_shape_lexed(&tokens).unwrap();
    assert_eq!(shape.reference, SearchShuffleObjectReference::General);
    assert!(!shape.owner_library_destination);

    let tokens = lex_line("Shuffle it into its owner's library", 0).unwrap();
    let shape = parse_shuffle_object_shape_lexed(&tokens).unwrap();
    assert!(shape.owner_library_destination);

    let tokens = lex_line(
        "Shuffle target nontoken permanent you control into its owner's library",
        0,
    )
    .unwrap();
    let shape = parse_shuffle_object_shape_lexed(&tokens).unwrap();
    assert!(shape.owner_library_destination);

    let tokens = lex_line("Shuffle it into your library", 0).unwrap();
    let shape = parse_shuffle_object_shape_lexed(&tokens).unwrap();
    assert!(!shape.owner_library_destination);

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
    assert!(!shape.possessive_owner_subject);

    let tokens = lex_line("Target creature's owner shuffles it into their library", 0).unwrap();
    let shape = parse_shuffle_object_shape_lexed(&tokens).unwrap();
    assert_eq!(
        parser_token_word_refs(
            shape
                .owner_subject_target_tokens
                .as_deref()
                .expect("possessive owner target")
        ),
        ["target", "creature"]
    );
    assert_eq!(
        shape.reference,
        SearchShuffleObjectReference::SingularBackReference
    );
    assert!(shape.possessive_owner_subject);

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
