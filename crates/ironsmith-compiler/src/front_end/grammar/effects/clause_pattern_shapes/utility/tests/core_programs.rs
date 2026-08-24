use super::*;

#[test]
pub(super) fn parses_utility_clause_shapes() {
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

    let sentence_separated = lex_line(
        "copy that spell X times. You may choose new targets for the copies.",
        0,
    )
    .unwrap();
    let sentence_separated_shape = parse_copy_clause_shape_tokens(&sentence_separated).unwrap();
    assert_eq!(sentence_separated_shape.tail.retarget_split, Some(4));
    assert!(sentence_separated_shape.tail.retarget_may);
}

#[test]
pub(super) fn parses_inflected_connive_clause_shapes() {
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
