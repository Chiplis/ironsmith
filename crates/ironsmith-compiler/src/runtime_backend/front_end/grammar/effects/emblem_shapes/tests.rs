use super::*;
use crate::runtime_backend::lexer::{lex_line, render_token_slice};

#[test]
fn captures_one_or_multiple_quoted_ability_groups() {
    let one = lex_line(
        r#"an emblem with "Whenever you cast a spell, draw a card.""#,
        0,
    )
    .unwrap();
    let parsed = parse_emblem_payload_tokens(&one).unwrap();
    assert_eq!(parsed.ability_groups.len(), 1);
    assert!(render_token_slice(parsed.ability_groups[0]).starts_with("Whenever"));

    let multiple = lex_line(
        r#"an emblem with "You have no maximum hand size." and "{T}: Draw a card.""#,
        0,
    )
    .unwrap();
    let parsed = parse_emblem_payload_tokens(&multiple).unwrap();
    assert_eq!(parsed.ability_groups.len(), 2);

    let get = lex_line(
        r#"get an emblem with "You have no maximum hand size." and "{T}: Draw a card.""#,
        0,
    )
    .unwrap();
    let parsed = parse_emblem_payload_tokens(&get).unwrap();
    assert_eq!(parsed.ability_groups.len(), 2);

    let explicit_you = lex_line(
        r#"You get an emblem with "You have no maximum hand size.""#,
        0,
    )
    .unwrap();
    assert!(
        parse_emblem_payload_tokens(&explicit_you)
            .unwrap()
            .explicit_you
    );
}
