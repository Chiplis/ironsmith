use super::*;

#[test]
pub(super) fn double_counter_holder_distinguishes_singular_source_from_filter_wide_sets() {
    for (text, expected_surface) in [
        (
            "Double the number of +1/+1 counters on this creature",
            "this creature",
        ),
        (
            "Double the number of growth counters on this enchantment",
            "this enchantment",
        ),
        ("Double the number of +1/+1 counters on it", "it"),
    ] {
        let tokens = lex_line(text, 0).unwrap();
        let holder = parse_double_counters_tokens(&tokens).unwrap().holder;
        let DoubleCounterHolderShape::Source { surface, .. } = &holder else {
            panic!("expected singular source holder for {text}, got {holder:?}");
        };
        assert_eq!(surface.display_text(), expected_surface);
    }

    let tokens = lex_line("Double the number of +1/+1 counters on that creature", 0).unwrap();
    assert!(matches!(
        parse_double_counters_tokens(&tokens).unwrap().holder,
        DoubleCounterHolderShape::Target(_)
    ));

    for text in [
        "Double the number of +1/+1 counters on each creature you control",
        "Double the number of charge counters on all artifacts you control",
        "Double the number of +1/+1 counters on those creatures",
        "Double the number of +1/+1 counters on each of those creatures",
        "Double the number of +1/+1 counters on that player's creatures",
    ] {
        let tokens = lex_line(text, 0).unwrap();
        assert!(matches!(
            parse_double_counters_tokens(&tokens).unwrap().holder,
            DoubleCounterHolderShape::Filter(_)
        ));
    }
}
