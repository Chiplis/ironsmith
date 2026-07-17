use super::super::super::super::lexer::lex_line;
use super::*;

#[test]
fn splits_multi_subjects_and_trims_distributive_each() {
    let tokens = lex_line("White creatures each and blue creatures each", 0).unwrap();
    let segments = parse_multi_subject_segments(&tokens).unwrap();
    assert_eq!(segments.len(), 2);
    assert_eq!(segments[0].len(), 2);
    assert_eq!(segments[1].len(), 2);
}

#[test]
fn parses_base_power_toughness_shapes() {
    let tokens = lex_line("This creature has base power and toughness 3/4.", 0).unwrap();
    let shape = parse_base_power_toughness_shape(&tokens).unwrap();
    assert_eq!((shape.power, shape.toughness), (3, 4));

    let tokens = lex_line(
        "As long as enchanted permanent is a creature, it has base power and toughness 1/1.",
        0,
    )
    .unwrap();
    let shape = parse_base_power_toughness_shape(&tokens).unwrap();
    assert!(matches!(
        shape.condition,
        BasePowerToughnessConditionShape::Tokens(_)
    ));
    assert_eq!((shape.power, shape.toughness), (1, 1));

    let tokens = lex_line(
        "During your turn, this creature has base power and toughness 5/2.",
        0,
    )
    .unwrap();
    let shape = parse_base_power_toughness_shape(&tokens).unwrap();
    assert_eq!(shape.condition, BasePowerToughnessConditionShape::YourTurn);
    assert_eq!((shape.power, shape.toughness), (5, 2));

    let tokens = lex_line(
        "As long as you control an artifact, this creature has base power and toughness 4/4 and has flying.",
        0,
    )
    .unwrap();
    let shape = parse_base_power_toughness_grant_shape(&tokens).unwrap();
    assert_eq!((shape.power, shape.toughness), (4, 4));
    assert!(!shape.ability_tokens.is_empty());

    let tokens = lex_line(
        "Enchanted creature has base power 0 and has \"At the beginning of your upkeep, you lose 1 life.\"",
        0,
    )
    .unwrap();
    let shape = parse_base_power_grant_shape(&tokens).unwrap();
    assert_eq!(shape.power, 0);
    assert!(!shape.ability_tokens.is_empty());
}

#[test]
fn parses_negated_creature_with_conditions() {
    let tokens = lex_line(
        "As long as you control an artifact, this permanent isn't a creature unless you control a creature.",
        0,
    )
    .unwrap();
    let shape = parse_isnt_creature_shape(&tokens).unwrap().unwrap();
    assert!(shape.leading_condition_tokens.is_some());
    assert!(shape.unless_condition_tokens.is_some());
}
