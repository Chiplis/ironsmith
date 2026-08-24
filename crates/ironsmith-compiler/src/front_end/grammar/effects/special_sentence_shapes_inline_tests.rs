use super::*;
use crate::lexer::{TokenWordView, lex_line};

#[test]
fn parses_scaled_target_and_sweep_shapes() {
    let tokens = lex_line(
        "double the power and toughness of each creature you control until end of turn",
        0,
    )
    .unwrap();
    let ScaledPowerShape::ScaleAll {
        axes, multiplier, ..
    } = parse_scaled_power_shape(&tokens).unwrap()
    else {
        panic!("expected sweep");
    };
    assert_eq!(
        axes,
        ScaleAxes {
            power: true,
            toughness: true
        }
    );
    assert_eq!(multiplier, 1);

    let tokens = lex_line(
        "triple target creature's power and toughness until end of turn",
        0,
    )
    .unwrap();
    assert!(matches!(
        parse_scaled_power_shape(&tokens),
        Some(ScaledPowerShape::ScaleTarget { multiplier: 2, .. })
    ));

    let tokens = lex_line("until end of turn, double target creature's power", 0).unwrap();
    assert!(matches!(
        parse_scaled_power_shape(&tokens),
        Some(ScaledPowerShape::ScaleTarget {
            axes: ScaleAxes {
                power: true,
                toughness: false,
            },
            multiplier: 1,
            ..
        })
    ));
}

#[test]
fn parses_keyword_bundle_shape() {
    let tokens = lex_line(
            "until end of turn each other creature you control gets +1/+1 if it has flying +1/+1 if it has first strike and so on for double strike deathtouch and haste",
            0,
        )
        .unwrap();
    let shape = parse_keyword_bundle_pump_shape(&tokens).unwrap().unwrap();
    assert_eq!(shape.power, Value::Fixed(1));
    assert_eq!(shape.toughness, Value::Fixed(1));
    assert_eq!(shape.abilities.len(), 5);
}

#[test]
fn parses_punctuated_keyword_bundle_shape_without_truncating_the_trailing_list() {
    let tokens = lex_line(
            "until end of turn, each other creature you control gets +1/+1 if it has flying, +1/+1 if it has first strike, and so on for double strike, deathtouch, haste, hexproof, indestructible, lifelink, menace, protection, reach, trample, vigilance, and partner",
            0,
        )
        .unwrap();
    let shape = parse_keyword_bundle_pump_shape(&tokens).unwrap().unwrap();

    assert_eq!(shape.abilities.len(), 14);
    assert_eq!(shape.abilities.first(), Some(&StaticAbilityId::Flying));
    assert_eq!(shape.abilities.last(), Some(&StaticAbilityId::Partner));
}

#[test]
fn parses_sacrifice_then_draw_shape() {
    let tokens = lex_line(
        "sacrifice any number of artifacts enchantments and tokens then draw that many cards",
        0,
    )
    .unwrap();
    let shape = parse_sacrifice_then_draw_shape(&tokens).unwrap();
    assert!(shape.artifact_enchantment_or_token);
    assert_eq!(
        TokenWordView::new(shape.filter_tokens).to_word_refs(),
        vec!["artifacts", "enchantments", "and", "tokens"]
    );
}
