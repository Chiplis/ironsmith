use super::*;
use crate::lexer::lex_line;

fn lex(raw: &str) -> Vec<OwnedLexToken> {
    lex_line(raw, 0).unwrap()
}

#[test]
fn parses_looked_card_dispositions_and_from_among_filter() {
    assert_eq!(
        parse_looked_card_disposition(&lex(
            "Put one of them into your hand and the rest on the bottom of your library in any order"
        )),
        Some(LookedCardDisposition::HandAndLibraryBottom(
            LibraryBottomOrderAst::ChooserChooses
        ))
    );
    assert_eq!(
        parse_looked_card_disposition(&lex(
            "Put one of them into your hand and the rest on the bottom of your library in a random order"
        )),
        Some(LookedCardDisposition::HandAndLibraryBottom(
            LibraryBottomOrderAst::Random
        ))
    );
    let shape = parse_looked_card_into_hand_shape(&lex(
        "a creature card from among those cards into your hand",
    ))
    .unwrap();
    assert!(!shape.filter.is_empty());

    assert!(matches!(
        parse_reveal_top_matching_followup_shape(&lex(
            "Put all land cards revealed this way into your hand and the rest into your graveyard"
        )),
        Some(RevealTopMatchingFollowupShape {
            remainder: RevealTopRemainder::Graveyard,
            ..
        })
    ));
}

#[test]
fn parses_face_down_selection_with_exact_graveyard_remainder() {
    assert!(matches!(
        parse_look_exile_face_down_shape(&lex(
            "Look at the top three cards of that player's library, exile one of them face down, then put the rest into their graveyard"
        )),
        Some(LookExileFaceDownShape::CountedGraveyardRemainder {
            count,
            ..
        }) if count == ChoiceCount::exactly(1)
    ));
}

#[test]
fn parses_complete_looked_card_partitions_with_independent_orders() {
    assert_eq!(
        parse_looked_card_partition_shape(&lex(
            "Put one of them into your hand and the rest on top of your library in any order"
        )),
        Some(LookedCardPartitionShape {
            selected_count: ChoiceCount::exactly(1),
            selected_destination: LookedPartitionDestination::Hand,
            remainder_destination: LookedPartitionDestination::LibraryTop(
                LibraryBottomOrderAst::ChooserChooses
            ),
        })
    );
    assert_eq!(
        parse_looked_card_partition_shape(&lex(
            "Put one of those cards into that player's graveyard and the rest on top of their library in any order"
        )),
        Some(LookedCardPartitionShape {
            selected_count: ChoiceCount::exactly(1),
            selected_destination: LookedPartitionDestination::Graveyard,
            remainder_destination: LookedPartitionDestination::LibraryTop(
                LibraryBottomOrderAst::ChooserChooses
            ),
        })
    );
    assert_eq!(
        parse_looked_card_partition_shape(&lex(
            "Put any number of them on the bottom of that library in a random order and the rest on top of the library in any order"
        )),
        Some(LookedCardPartitionShape {
            selected_count: ChoiceCount::any_number(),
            selected_destination: LookedPartitionDestination::LibraryBottom(
                LibraryBottomOrderAst::Random
            ),
            remainder_destination: LookedPartitionDestination::LibraryTop(
                LibraryBottomOrderAst::ChooserChooses
            ),
        })
    );
    assert_eq!(
        parse_looked_card_partition_shape(&lex(
            "Put two of them into your hand and the other into your graveyard"
        )),
        Some(LookedCardPartitionShape {
            selected_count: ChoiceCount::exactly(2),
            selected_destination: LookedPartitionDestination::Hand,
            remainder_destination: LookedPartitionDestination::Graveyard,
        })
    );
    for text in [
        "Put up to one of them on top of your library and the rest on the bottom in a random order",
        "Put up to one of them on top of your library and the rest on the bottom of your library in a random order",
    ] {
        assert_eq!(
            parse_looked_card_partition_shape(&lex(text)),
            Some(LookedCardPartitionShape {
                selected_count: ChoiceCount::up_to(1),
                selected_destination: LookedPartitionDestination::LibraryTop(
                    LibraryBottomOrderAst::ChooserChooses
                ),
                remainder_destination: LookedPartitionDestination::LibraryBottom(
                    LibraryBottomOrderAst::Random
                ),
            })
        );
    }
}

#[test]
fn looked_card_partition_requires_full_consumption_and_top_remainder() {
    assert!(
            parse_looked_card_partition_shape(&lex(
                "Put one of them into your hand and the rest on top of your library in any order, then draw a card"
            ))
            .is_none()
        );
    for control in [
        "Put one of them into your hand and the rest on the bottom of your library in any order",
        "Put them back in any order",
    ] {
        assert!(parse_looked_card_partition_shape(&lex(control)).is_none());
    }
}
