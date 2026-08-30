use super::*;

#[test]
pub(super) fn parses_destroy_all_and_delayed_shapes() {
    let tokens = lex_line(
        "all creatures except for artifacts at the beginning of the next end step",
        0,
    )
    .unwrap();
    let shape = parse_destroy_clause_shape(&tokens);
    assert_eq!(shape.timing, Some(DelayedDestroyTimingShape::NextEndStep));
    let DestroyClauseKind::All(DestroyAllShape::ExceptFor {
        filter_tokens,
        exception_tokens,
    }) = shape.kind
    else {
        panic!("expected destroy-all exception");
    };
    assert_eq!(words(filter_tokens), vec!["creatures"]);
    assert_eq!(words(exception_tokens), vec!["artifacts"]);
}
