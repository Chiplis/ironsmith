use super::*;
use crate::lexer::lex_line;

#[test]
fn parses_tagged_and_from_among_put_shapes() {
    let tagged = lex_line(
        "put two of them into your hand and the rest on the bottom of your library",
        0,
    )
    .unwrap();
    let shape = parse_tagged_into_hand_shape(&tagged).unwrap();
    assert_eq!(
        shape.rest_destination,
        Some(RestDestinationShape::BottomOfLibrary)
    );
    assert!(shape.count.is_some());
    assert!(shape.plural_reference);

    let plural = lex_line("put them into your hand", 0).unwrap();
    let shape = parse_tagged_into_hand_shape(&plural).unwrap();
    assert!(shape.plural_reference);
    assert!(shape.count.is_none());

    let optional_single = lex_line("put up to one of them into your hand", 0).unwrap();
    let shape = parse_tagged_into_hand_shape(&optional_single).unwrap();
    assert!(!shape.plural_reference);
    assert_eq!(shape.count, Some(ChoiceCount::up_to(1)));

    let any_order = lex_line(
        "put two of them into your hand and the rest on the bottom of your library in any order",
        0,
    )
    .unwrap();
    assert_eq!(
        parse_tagged_into_hand_shape(&any_order)
            .unwrap()
            .bottom_order,
        Some(LibraryBottomOrderAst::ChooserChooses)
    );
    let random_order = lex_line(
            "put two of them into your hand and the rest on the bottom of your library in a random order",
            0,
        )
        .unwrap();
    assert_eq!(
        parse_tagged_into_hand_shape(&random_order)
            .unwrap()
            .bottom_order,
        Some(LibraryBottomOrderAst::Random)
    );

    let top_and_bottom = lex_line(
        "put up to one of them on top of your library and the rest on the bottom in a random order",
        0,
    )
    .unwrap();
    let shape = parse_tagged_on_top_library_shape(&top_and_bottom).unwrap();
    assert_eq!(shape.count, ChoiceCount::up_to(1));
    assert_eq!(shape.bottom_order, LibraryBottomOrderAst::Random);

    let among = lex_line(
        "up to one creature card from among them onto the battlefield and the rest into your hand",
        0,
    )
    .unwrap();
    let shape = parse_from_among_them_shape(&among).unwrap();
    assert_eq!(shape.destination, FromAmongDestinationShape::Battlefield);
    assert_eq!(shape.rest_destination, Some(RestDestinationShape::Hand));

    let revealed = lex_line(
        "put up to two creature cards from among the revealed cards into your hand instead of one",
        0,
    )
    .unwrap();
    let shape = parse_from_among_them_shape(&revealed).unwrap();
    assert_eq!(shape.count, ChoiceCount::up_to(2));
    assert_eq!(shape.destination, FromAmongDestinationShape::Hand);
    assert!(
        shape
            .filter_tokens
            .iter()
            .any(|token| token.is_word("creature"))
    );

    assert!(is_reorder_tagged_cards(
        &lex_line("put them back in any order", 0).unwrap()
    ));

    let battlefield_partition = lex_line(
            "put one of those cards onto the battlefield tapped under your control and the rest onto the battlefield tapped under their control",
            0,
        )
        .unwrap();
    let shape = parse_tagged_battlefield_partition_shape(&battlefield_partition)
        .expect("tagged battlefield partition");
    assert_eq!(shape.count, ChoiceCount::exactly(1));
    assert!(shape.chosen_tapped && shape.remainder_tapped);
    assert_eq!(
        shape.chosen_controller,
        PartitionBattlefieldControllerShape::You
    );
    assert_eq!(
        shape.remainder_controller,
        PartitionBattlefieldControllerShape::SubjectPlayer
    );
}

#[test]
fn parses_whole_revealed_collection_for_library_bottom_cleanup() {
    let tokens = lex_line(
        "the revealed cards on the bottom of your library in any order",
        0,
    )
    .unwrap();
    let shape = parse_revealed_remainder_shape(&tokens).expect("revealed collection");

    assert!(!shape.exclude_current_reference);
    assert!(!shape.random_order);
    assert_eq!(shape.surface, ironsmith_core::LibraryRemainderSurface::Rest);
}

#[test]
fn parses_authored_you_revealed_collection_for_library_bottom_cleanup() {
    let tokens = lex_line(
        "the cards you revealed this way on the bottom of your library in any order",
        0,
    )
    .unwrap();
    let shape = parse_revealed_remainder_shape(&tokens).expect("revealed collection");

    assert!(!shape.exclude_current_reference);
    assert!(!shape.random_order);
    assert_eq!(
        shape.surface,
        ironsmith_core::LibraryRemainderSurface::CardsYouRevealedThisWay
    );
}
