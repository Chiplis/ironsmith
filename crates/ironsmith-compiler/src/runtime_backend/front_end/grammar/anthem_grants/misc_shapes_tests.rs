use super::*;
use crate::runtime_backend::front_end::lexer::{lex_line, render_token_slice};

fn lex(text: &str) -> Vec<OwnedLexToken> {
    lex_line(text, 0).expect("lex fixture")
}

#[test]
fn splits_keyword_color_segment() {
    let tokens = lex("flying if it is white");
    let shape = parse_keyword_if_color_shape(&tokens).expect("color segment");
    assert!(!shape.keyword_tokens.is_empty());
    assert!(!shape.color_tail_tokens.is_empty());
}

#[test]
fn parses_metalcraft_equipment_equip_shape() {
    let tokens = lex(
        "Metalcraft — Equipment you control have equip {0} as long as you control three artifacts.",
    );
    assert!(parse_equipment_equip_shape(&tokens).is_some());
}

#[test]
fn typed_static_grant_migration_normalizes_trailing_segment() {
    let tokens = lex(", and flying.");
    let shape = parse_trailing_grant_segment(&tokens).expect("trailing grant segment");
    assert_eq!(
        shape
            .body_tokens
            .iter()
            .filter_map(OwnedLexToken::as_word)
            .collect::<Vec<_>>(),
        vec!["flying"]
    );
}

#[test]
fn splits_grants_on_conjunction_after_a_quoted_ability() {
    let tokens = lex("\"Equipped creature gets +2/+0\" and equip {2}.");
    let segments = split_trailing_grant_segments(&tokens);
    assert_eq!(segments.len(), 2);
    assert_eq!(
        render_token_slice(&segments[0]),
        "\"Equipped creature gets +2/+0\""
    );
    assert_eq!(render_token_slice(&segments[1]), "equip {2}.");
}

#[test]
fn splits_equip_grant_immediately_after_a_closed_quote() {
    let tokens = lex("\"Equipped creature gets +2/+0,\" equip Pirate {1}, and equip {3}.");
    let segments = split_trailing_grant_segments(&tokens);
    assert_eq!(segments.len(), 3);
    assert_eq!(
        render_token_slice(&segments[0]),
        "\"Equipped creature gets +2/+0,\""
    );
    assert_eq!(render_token_slice(&segments[1]), "equip Pirate {1}");
    assert_eq!(render_token_slice(&segments[2]), "and equip {3}.");
}
