use crate::lexer::{lex_line, parser_token_word_refs};

use super::*;

#[test]
fn sacrifice_clause_classifies_unless_and_strips_verb() {
    let tokens = lex_line("Sacrifice that token, unless {R} was spent to cast it", 0).unwrap();
    let shape = parse_sacrifice_clause_shape(&tokens);
    assert_eq!(parser_token_word_refs(shape.body_tokens), ["that", "token"]);
    assert_eq!(
        shape.unless_kind,
        SacrificeUnlessKind::ManaSpent(ManaSymbol::Red)
    );
    assert!(shape.sacrifice_references_it);
    assert_eq!(
        shape
            .full_body_tokens
            .get(shape.unless_token_offset.unwrap())
            .and_then(OwnedLexToken::as_word),
        Some("unless")
    );
}

#[test]
fn sacrifice_object_shape_strips_choice_and_preserves_token_reference() {
    let tokens = lex_line("that token of their choice", 0).unwrap();
    let shape = parse_sacrifice_object_shape(&tokens);
    assert_eq!(
        shape.tagged_reference,
        Some(SacrificeTaggedReferenceKind::Token)
    );
    assert_eq!(
        parser_token_word_refs(shape.filter_tokens),
        ["that", "token"]
    );
}

#[test]
fn sacrifice_all_or_each_preserves_only_the_authored_each_surface() {
    for (text, expected_each) in [
        ("each other creature you control", true),
        ("all other creatures you control", false),
    ] {
        let tokens = lex_line(text, 0).unwrap();
        let Some(SacrificeQuantityShape::AllOrEach {
            each_surface,
            other,
            ..
        }) = parse_sacrifice_quantity_shape(&tokens)
        else {
            panic!("expected all/each sacrifice quantity: {text}");
        };
        assert_eq!(each_surface, expected_each, "{text}");
        assert!(other, "{text}");
    }
}

#[test]
fn sacrifice_object_shape_preserves_definite_object_references() {
    for text in [
        "that creature",
        "the creature",
        "that permanent",
        "the permanent",
        "that token",
        "the token",
    ] {
        let tokens = lex_line(text, 0).unwrap();
        let shape = parse_sacrifice_object_shape(&tokens);
        assert!(
            shape.tagged_reference.is_some(),
            "{text} should remain a reference to the established object"
        );
    }
}

#[test]
fn sacrifice_object_shape_distinguishes_one_member_of_tagged_set() {
    let tokens = lex_line("one of them", 0).unwrap();
    let shape = parse_sacrifice_object_shape(&tokens);

    assert_eq!(
        shape.tagged_reference,
        Some(SacrificeTaggedReferenceKind::OneOfTaggedSet)
    );
    assert_eq!(
        parser_token_word_refs(shape.filter_tokens),
        ["one", "of", "them"]
    );
}

#[test]
fn sacrifice_object_shape_preserves_all_of_plural_result_set() {
    for text in ["those permanents", "those creatures", "those tokens"] {
        let tokens = lex_line(text, 0).unwrap();
        let shape = parse_sacrifice_object_shape(&tokens);

        assert_eq!(
            shape.tagged_reference,
            Some(SacrificeTaggedReferenceKind::AllOfTaggedSet),
            "{text}"
        );
        assert_eq!(
            parser_token_word_refs(shape.filter_tokens),
            text.split_whitespace().collect::<Vec<_>>(),
            "{text}"
        );
    }
}

#[test]
fn aggregate_shape_returns_typed_axis_and_sides() {
    let tokens = lex_line(
        "a creature with the greatest power among creatures you control",
        0,
    )
    .unwrap();
    let shape = parse_sacrifice_aggregate_shape(&tokens).unwrap();
    assert_eq!(shape.kind, SacrificeAggregateKind::GreatestPower);
    assert_eq!(
        parser_token_word_refs(shape.object_tokens),
        ["a", "creature"]
    );
    assert_eq!(
        parser_token_word_refs(shape.among_tokens),
        ["creatures", "you", "control"]
    );
}

#[test]
fn sacrifice_count_shape_returns_count_other_and_filter() {
    let tokens = lex_line("two another creatures", 0).unwrap();
    let shape = parse_sacrifice_count_shape(&tokens);
    assert_eq!(shape.count, 2);
    assert!(shape.other);
    assert_eq!(parser_token_word_refs(shape.filter_tokens), ["creatures"]);
}

#[test]
fn sacrifice_fraction_rounded_shape_preserves_denominator_and_controlled_filter() {
    let tokens = lex_line(
        "half the creatures they control of their choice, rounded up",
        0,
    )
    .unwrap();
    let shape = parse_sacrifice_fraction_rounded_shape(&tokens).unwrap();
    assert_eq!(shape.denominator, 2);
    assert_eq!(
        parser_token_word_refs(shape.filter_tokens),
        ["creatures", "they", "control"]
    );

    let tokens = lex_line(
        "a tenth of the creatures they control of their choice, rounded up",
        0,
    )
    .unwrap();
    let shape = parse_sacrifice_fraction_rounded_shape(&tokens).unwrap();
    assert_eq!(shape.denominator, 10);
    assert!(shape.rounded_up);
    assert_eq!(
        parser_token_word_refs(shape.filter_tokens),
        ["creatures", "they", "control"]
    );
}

#[test]
fn sacrifice_fraction_shape_requires_a_rounding_surface_and_valid_unit_fraction() {
    for text in [
        "a tenth of the creatures they control of their choice",
        "a first of the creatures they control of their choice, rounded up",
        "a tenth creatures they control of their choice, rounded up",
    ] {
        let tokens = lex_line(text, 0).unwrap();
        assert!(
            parse_sacrifice_fraction_rounded_shape(&tokens).is_none(),
            "near miss must not claim {text:?}"
        );
    }
}

#[test]
fn sacrifice_all_except_shape_preserves_filter_and_keep_count() {
    let tokens = lex_line("all lands they control except for three", 0).unwrap();
    let Some(SacrificeQuantityShape::AllExcept {
        filter_tokens,
        keep_count,
        other,
    }) = parse_sacrifice_quantity_shape(&tokens)
    else {
        panic!("expected typed all-except quantity");
    };
    assert_eq!(keep_count, 3);
    assert!(!other);
    assert_eq!(
        parser_token_word_refs(filter_tokens),
        ["lands", "they", "control"]
    );

    for text in [
        "all lands they control except for zero",
        "all lands they control except for",
        "lands they control except for three",
    ] {
        let tokens = lex_line(text, 0).unwrap();
        assert!(
            !matches!(
                parse_sacrifice_quantity_shape(&tokens),
                Some(SacrificeQuantityShape::AllExcept { .. })
            ),
            "near miss must not claim {text:?}"
        );
    }
}
