use super::*;
use crate::lexer::lex_line;

fn lex(text: &str) -> Vec<OwnedLexToken> {
    lex_line(text, 0).expect("lex fixture")
}

#[test]
fn parses_leading_as_long_as_boundary() {
    let tokens = lex("As long as you control a Forest, creatures you control get +1/+1.");
    let get = primitives::find_prefix(&tokens, || primitives::kw("get").void())
        .expect("get")
        .0;
    let shape = parse_prefix_condition_shape(&tokens, get).expect("prefix");
    assert_eq!(shape.kind, AnthemPrefixConditionKind::AsLongAs);
    assert!(shape.comma_subject_start.is_some());
}

#[test]
fn splits_fixed_turn_prefix_only_at_an_authored_comma() {
    let tokens = lex("During turns other than yours, this Vehicle is an artifact creature.");
    let shape = parse_fixed_prefix_condition_shape(&tokens).expect("fixed prefix");
    assert_eq!(
        shape.kind,
        AnthemPrefixConditionKind::DuringTurnsOtherThanYours
    );
    assert_eq!(
        crate::lexer::render_token_slice(shape.subject_tokens),
        "this Vehicle is an artifact creature."
    );

    let no_comma = lex("During turns other than yours this Vehicle is an artifact creature.");
    assert!(parse_fixed_prefix_condition_shape(&no_comma).is_none());
    let as_long_as = lex("As long as it is your turn, this Vehicle is an artifact creature.");
    assert!(parse_fixed_prefix_condition_shape(&as_long_as).is_none());
}

#[test]
fn parses_typed_anthem_tail() {
    let tokens = lex("for each creature you control");
    assert!(matches!(
        parse_tail_shape(&tokens),
        Some(AnthemTailShape::ForEach(_))
    ));
}

#[test]
fn splits_authored_modifier_maximum_from_count_body() {
    let tokens = lex("for each of its creature types, to a maximum of 10.");
    let (body, maximum) = split_trailing_modifier_maximum(&tokens);

    assert_eq!(maximum, Some(10));
    assert_eq!(
        crate::lexer::render_token_slice(body),
        "for each of its creature types"
    );
}
