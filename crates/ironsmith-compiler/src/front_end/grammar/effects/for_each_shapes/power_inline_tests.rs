use super::*;
use crate::lexer::lex_line;

#[test]
fn parses_typed_base_characteristics() {
    let power = lex_line("until end of turn, target creature has base power X", 0).unwrap();
    assert_eq!(
        parse_base_power_clause_shape(&power)
            .unwrap()
            .unwrap()
            .power,
        Value::X
    );

    let pt = lex_line("target creature has base power and toughness 3/4", 0).unwrap();
    let shape = parse_base_power_toughness_clause_shape(&pt)
        .unwrap()
        .unwrap();
    assert_eq!(shape.power, Value::Fixed(3));
    assert_eq!(shape.toughness, Value::Fixed(4));
    assert_eq!(shape.duration, Until::Forever);
    assert!(shape.where_x_tokens.is_none());

    let permanent_multi_target = lex_line(
        "any number of target Shapeshifter creatures you control have base power and toughness 4/4",
        0,
    )
    .unwrap();
    let shape = parse_base_power_toughness_clause_shape(&permanent_multi_target)
        .unwrap()
        .unwrap();
    assert_eq!(shape.duration, Until::Forever);

    let pt = lex_line(
        "until your next turn, creatures target player controls have base power and toughness 1/1",
        0,
    )
    .unwrap();
    let shape = parse_base_power_toughness_clause_shape(&pt)
        .unwrap()
        .unwrap();
    assert_eq!(shape.duration, Until::YourNextTurn);
    assert!(shape.where_x_tokens.is_none());
    assert_eq!(
        render_token_slice(shape.target_tokens),
        "creatures target player controls"
    );

    let dynamic = lex_line(
            "until end of turn, creatures you control have base power and toughness X/X, where X is the number of cards in your graveyard",
            0,
        )
        .unwrap();
    let shape = parse_base_power_toughness_clause_shape(&dynamic)
        .unwrap()
        .unwrap();
    assert_eq!(shape.power, Value::X);
    assert_eq!(shape.toughness, Value::X);
    assert_eq!(shape.duration, Until::EndOfTurn);
    assert_eq!(
        render_token_slice(shape.where_x_tokens.unwrap()),
        "where X is the number of cards in your graveyard"
    );
}
