use super::*;
use crate::runtime_backend::lexer::{TokenWordView, lex_line};

#[test]
fn leading_unless_split_handles_comma_and_payment_search_surfaces() {
    let comma = lex_line("Unless you pay {2}, counter that spell", 0).unwrap();
    let split = parse_leading_unless_clause_split_tokens(&comma).unwrap();
    assert_eq!(
        TokenWordView::new(&comma[split.effect]).word_refs(),
        ["counter", "that", "spell"]
    );

    let search = lex_line(
        "Unless that player pays {2} search your library for a basic land card",
        0,
    )
    .unwrap();
    let split = parse_leading_unless_clause_split_tokens(&search).unwrap();
    assert_eq!(search[split.effect.start].parser_text(), "search");
}

#[test]
fn parses_typed_unless_payment_subject_and_dynamic_life_kind() {
    let tokens = lex_line("unless its controller pays life equal to its toughness", 0).unwrap();
    let shape = parse_unless_pays_shape_tokens(&tokens).unwrap();
    assert_eq!(
        TokenWordView::new(shape.player_tokens).word_refs(),
        ["its", "controller"]
    );
    assert_eq!(shape.kind, UnlessPaymentKind::LifeEqualToItsToughness);
}
