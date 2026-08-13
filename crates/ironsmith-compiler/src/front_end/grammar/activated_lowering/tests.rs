use super::*;
use crate::runtime_backend::front_end::lexer::lex_line;

#[test]
fn classifies_activated_mana_surfaces() {
    let add = lex_line("You add {G}.", 0).unwrap();
    assert_eq!(
        parse_activated_mana_effect_kind(&add),
        Some(ActivatedManaEffectKind::AddMana)
    );
    let among = lex_line(
        "For each color among permanents you control, add one mana of that color.",
        0,
    )
    .unwrap();
    assert_eq!(
        parse_activated_mana_effect_kind(&among),
        Some(ActivatedManaEffectKind::ColorsAmong)
    );
}

#[test]
fn parses_x_definition_and_level_number() {
    let definition = lex_line("X is the mana value of that card.", 0).unwrap();
    let shape = parse_activated_x_definition_tokens(&definition).unwrap();
    assert_eq!(shape.intro, ActivatedXDefinitionIntro::XIs);
    assert!(shape.exiled_card_mana_value);

    let level = lex_line("Level 3.", 0).unwrap();
    assert_eq!(parse_level_number_tokens(&level), Some(3));
}

#[test]
fn infers_command_zone_with_optional_article_from_typed_effect_tokens() {
    let cost = lex_line("{1}{G}{W}{U}", 0).unwrap();
    for text in [
        "Put this card onto the battlefield from the command zone.",
        "Put this card onto the battlefield from command zone.",
    ] {
        let effect = lex_line(text, 0).unwrap();
        assert_eq!(
            parse_activated_functional_zones_tokens(&cost, &effect),
            vec![Zone::Command]
        );
    }
}
