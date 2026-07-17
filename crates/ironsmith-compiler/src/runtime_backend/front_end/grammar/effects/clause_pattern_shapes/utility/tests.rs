use super::*;
use crate::runtime_backend::front_end::lexer::{TokenWordView, lex_line};

#[test]
fn parses_utility_clause_shapes() {
    let double = lex_line("Double the number of +1/+1 counters on target creature", 0).unwrap();
    assert!(matches!(
        parse_double_counters_tokens(&double).unwrap().holder,
        DoubleCounterHolderShape::Target(_)
    ));

    let block = lex_line(
        "Target creature can block two additional creatures this turn",
        0,
    )
    .unwrap();
    assert_eq!(
        parse_can_block_additional_tokens(&block)
            .unwrap()
            .additional,
        2
    );

    let kicked = lex_line(
        "target creature, then choose another target creature for each time this spell was kicked",
        0,
    )
    .unwrap();
    assert!(parse_kicked_additional_targets_tokens(&kicked).is_some());

    let copy = lex_line(
        "copy that spell, you may choose new targets for the copy",
        0,
    )
    .unwrap();
    let copy_shape = parse_copy_clause_shape_tokens(&copy).unwrap();
    assert_eq!(copy_shape.tail.retarget_split, Some(2));
    assert!(copy_shape.tail.retarget_may);
    assert!(!copy_shape.tail.retarget_single_target);

    let singular_copy = lex_line(
        "copy this spell and may choose a new target for that copy",
        0,
    )
    .unwrap();
    let singular_shape = parse_copy_clause_shape_tokens(&singular_copy).unwrap();
    assert!(singular_shape.tail.retarget_single_target);
}

#[test]
fn double_counter_holder_distinguishes_singular_source_from_filter_wide_sets() {
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

#[test]
fn parses_inflected_connive_clause_shapes() {
    let source = lex_line("it connives.", 0).unwrap();
    let shape = parse_connive_clause_shape_tokens(&source).unwrap();
    assert!(matches!(shape.subject, ConniveSubjectShape::Target(_)));
    assert!(shape.count_tokens.is_empty());

    let dynamic = lex_line(
        "target attacking creature connives X, where X is the number of attacking creatures.",
        0,
    )
    .unwrap();
    let shape = parse_connive_clause_shape_tokens(&dynamic).unwrap();
    assert_eq!(
        TokenWordView::new(shape.count_tokens).to_word_refs(),
        [
            "x",
            "where",
            "x",
            "is",
            "the",
            "number",
            "of",
            "attacking",
            "creatures"
        ]
    );

    let convoked = lex_line("Each creature that convoked this spell connives.", 0).unwrap();
    assert!(matches!(
        parse_connive_clause_shape_tokens(&convoked)
            .unwrap()
            .subject,
        ConniveSubjectShape::ConvokedThisSpell
    ));
}
