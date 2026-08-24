use super::*;
use crate::lexer::lex_line;

#[test]
fn classifies_exchange_control_sequence() {
    let lines = [
        lex_line("Choose any number of creatures target player controls.", 0).unwrap(),
        lex_line(
            "Choose the same number of creatures another target player controls.",
            1,
        )
        .unwrap(),
        lex_line("Those players exchange control of those creatures.", 2).unwrap(),
    ];
    let slices = lines.iter().map(Vec::as_slice).collect::<Vec<_>>();
    assert_eq!(
        parse_divvy_sequence_shape(&slices),
        Some(DivvySequenceShape::ExchangeCreatureControl)
    );
}

#[test]
fn classifies_multi_zone_search_exile_remainder_to_ordered_top() {
    let lines = [
        lex_line(
            "Search your library and graveyard for five cards and exile the rest.",
            0,
        )
        .unwrap(),
        lex_line(
            "Put the chosen cards on top of your library in any order.",
            1,
        )
        .unwrap(),
        lex_line("You lose half your life, rounded up.", 2).unwrap(),
    ];
    let slices = lines.iter().map(Vec::as_slice).collect::<Vec<_>>();
    assert_eq!(
        parse_divvy_sequence_shape(&slices),
        Some(DivvySequenceShape::SearchLibraryGraveyardExileRemainderToTop)
    );
}
