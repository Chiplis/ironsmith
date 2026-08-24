use super::*;
use crate::lexer::{lex_line, render_token_slice};

#[test]
fn parses_each_player_and_controlled_creatures_damage() {
    let tokens = lex_line(
        "This creature deals X damage to each player and each creature they control.",
        0,
    )
    .unwrap();
    let parsed = parse_each_player_creatures_damage_tokens(&tokens).unwrap();
    assert_eq!(parsed.amount, Value::X);
}

#[test]
fn splits_compound_buff_and_unblockable_shape() {
    let tokens = lex_line("Target creature gets +2/+2 and can't be blocked.", 0).unwrap();
    let parsed = parse_compound_buff_unblockable_tokens(&tokens).unwrap();
    assert_eq!(
        render_token_slice(parsed.buff_tokens),
        "Target creature gets +2/+2"
    );
    assert_eq!(render_token_slice(parsed.subject_tokens), "Target creature");
    assert_eq!(
        render_token_slice(parsed.unblockable_tail_tokens),
        "can't be blocked"
    );
}

#[test]
fn parses_cant_be_blocked_then_base_power_toughness_shape() {
    let tokens = lex_line(
            "That creature can't be blocked this turn and has base power and toughness 1/1 until end of turn.",
            0,
        )
        .unwrap();
    let parsed = parse_cant_blocked_base_power_toughness_tokens(&tokens).unwrap();
    assert_eq!(render_token_slice(parsed.subject_tokens), "That creature");
    assert_eq!(parsed.power, Value::Fixed(1));
    assert_eq!(parsed.toughness, Value::Fixed(1));
}
