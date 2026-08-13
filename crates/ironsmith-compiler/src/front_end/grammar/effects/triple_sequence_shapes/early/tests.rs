use super::*;
use crate::runtime_backend::front_end::lexer::lex_line;

#[test]
fn parses_filtered_search_with_chosen_name_condition() {
    let first = lex_line(
        "search that player's library for a card, then that player chooses a card name.",
        0,
    )
    .expect("lex search/name sentence");
    let conditional = lex_line(
        "If you searched for a creature card that doesn't have that name, you may put it onto the battlefield under your control.",
        0,
    )
    .expect("lex search/name condition");
    let shuffle = lex_line("Then that player shuffles.", 0).expect("lex shuffle sentence");

    assert!(
        parse_search_then_name_shape(&first, &conditional, &shuffle).is_some(),
        "expected typed search/name sequence shape"
    );
}
