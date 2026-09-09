use super::*;
use crate::lexer::{lex_line, render_token_slice};

#[path = "tests/resource.rs"]
mod resource_programs;
use resource_programs::{
    quoted_emblem_payload_accepts_only_a_synthetic_outer_period,
    quoted_emblem_payload_does_not_consume_an_unquoted_followup,
};
#[path = "tests/ability.rs"]
mod ability_programs;
use ability_programs::captures_one_or_multiple_quoted_ability_groups;

#[test]
fn damaged_player_emblem_shape_keeps_only_the_quoted_payload() {
    let tokens = lex_line(r#"Each player dealt damage this way gets an emblem with "At the beginning of your upkeep, draw a card.""#, 0).unwrap();
    let shape = parse_damaged_player_emblem_payload_tokens(&tokens).unwrap();
    assert_eq!(shape.ability_groups.len(), 1);
    assert!(shape.ability_groups[0].first().unwrap().is_word("at"));
    assert!(!shape.ability_groups[0].iter().any(|token| token.kind == TokenKind::Quote));
    for text in [r#"Each player gets an emblem with "Draw a card.""#, r#"Each player dealt combat damage this turn gets an emblem with "Draw a card.""#] {
        assert!(parse_damaged_player_emblem_payload_tokens(&lex_line(text, 0).unwrap()).is_none());
    }
}
