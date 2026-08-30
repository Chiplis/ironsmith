use super::*;

#[test]
pub(super) fn parses_number_of_counters_equal_to_referenced_card_mana_value() {
    let tokens = lex_line(
        "a number of loyalty counters equal to that card's mana value from Jace",
        0,
    )
    .unwrap();
    let RemoveClauseShape::Counters {
        amount,
        counter_descriptor,
        destination,
        ..
    } = parse_remove_clause_shape(&tokens).unwrap()
    else {
        panic!("expected dynamic counter removal");
    };
    assert!(
        matches!(amount.unhinted(), Value::ManaValueOf(_)),
        "{amount:?}"
    );
    assert!(amount.has_surface_hint(ironsmith_core::ValueSurfaceHint::EqualTo));
    assert_eq!(words(counter_descriptor), vec!["loyalty"]);
    let RemoveCounterDestination::Single { target_tokens } = destination else {
        panic!("expected source-named single destination");
    };
    assert_eq!(words(target_tokens), vec!["jace"]);
}
