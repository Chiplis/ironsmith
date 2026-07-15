use super::*;
use crate::runtime_backend::front_end::lexer::lex_line;

#[test]
fn parses_passive_and_copular_shapes() {
    let sacrifice = lex_line("Each creature is sacrificed by its controller.", 0).unwrap();
    assert!(parse_passive_sacrifice_shape(&sacrifice).is_some());
    let goad = lex_line("The token is goaded for the rest of the game.", 0).unwrap();
    let goad = parse_passive_goad_shape(&goad).unwrap();
    assert!(matches!(goad.target, GoadTargetShape::TaggedToken));
    assert!(goad.for_rest_of_game);
    let ordinary = lex_line("The token is goaded.", 0).unwrap();
    assert!(
        !parse_passive_goad_shape(&ordinary)
            .unwrap()
            .for_rest_of_game
    );
    let animation = lex_line(
        "Target land is a 3/3 creature in addition to its other types.",
        0,
    )
    .unwrap();
    assert!(parse_copular_animation_shape(&animation).is_some());
    let contracted = lex_line("It's an enchantment.", 0).unwrap();
    let contracted = parse_copular_animation_shape(&contracted).unwrap();
    assert_eq!(parser_token_word_refs(contracted.subject_tokens), ["its"]);
    assert_eq!(
        parser_token_word_refs(contracted.animation_tokens),
        ["an", "enchantment"]
    );
    assert!(parse_copular_animation_shape(&lex_line("Its an enchantment.", 0).unwrap()).is_some());
}

#[test]
fn parses_discarded_this_way_modifier_as_event_scaled_pump() {
    let modifier = lex_line(
        "+2/+0 until end of turn for each card discarded this way",
        0,
    )
    .unwrap();
    assert_eq!(
        parse_discarded_this_way_modifier_shape(&modifier),
        Some(DiscardedThisWayModifierShape {
            power: 2,
            toughness: 0,
        })
    );
}

#[test]
fn captures_tagged_card_type_condition_effect() {
    let tokens = lex_line(
        "If any of those cards share a card type with that spell, draw a card.",
        0,
    )
    .unwrap();
    let shape = parse_tagged_shares_card_type_condition_tokens(&tokens).unwrap();
    assert!(!shape.effect_tokens.is_empty());
}
