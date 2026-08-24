use super::*;

#[test]
pub(super) fn parses_each_of_any_number_as_an_optional_unbounded_subset() {
    let tokens = lex_line(
        "a loyalty counter from each of any number of permanents you control",
        0,
    )
    .unwrap();
    let RemoveClauseShape::Counters { destination, .. } =
        parse_remove_clause_shape(&tokens).unwrap()
    else {
        panic!("expected counter removal");
    };
    let RemoveCounterDestination::EachOfAnyNumber { filter_tokens } = destination else {
        panic!("expected an any-number subset, not an all-permanents destination");
    };
    assert_eq!(words(filter_tokens), vec!["permanents", "you", "control"]);
}
