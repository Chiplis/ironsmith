use super::*;
use crate::runtime_backend::front_end::lexer::lex_line;

#[test]
fn parses_sacrifice_choice_shapes() {
    let tokens = lex_line(
        "sacrifice any number of artifacts then draw that many cards",
        0,
    )
    .unwrap();
    let shape = parse_sacrifice_choice_shape(&tokens).expect("sacrifice shape");
    assert_eq!(shape.count, ChoiceCount::any_number());
    assert!(shape.tail_tokens.is_some());

    let tokens = lex_line("sacrifice one or more creatures", 0).unwrap();
    let shape = parse_sacrifice_choice_shape(&tokens).expect("minimum shape");
    assert_eq!(shape.count, ChoiceCount::at_least(1));
}

#[test]
fn parses_exile_counter_and_destroy_attached_shapes() {
    let tokens = lex_line("exile Arc Blade with three time counters on it", 0).unwrap();
    assert!(parse_exile_source_counter_shape(&tokens).is_some());

    let tokens = lex_line("destroy all Auras that were attached to target creature", 0).unwrap();
    let shape = parse_destroy_attached_shape(&tokens).expect("attached shape");
    assert!(primitives::parse_prefix(shape.target_tokens, primitives::kw("target")).is_some());
}

#[test]
fn finds_color_choice_phrase_span() {
    let tokens = lex_line("creatures of the color of your choice", 0).unwrap();
    assert_eq!(
        parse_color_choice_phrase_span(&tokens),
        Some(ChoicePhraseSpan { start: 1, len: 6 })
    );
}
