use super::*;
use crate::runtime_backend::front_end::lexer::{TokenWordView, lex_line};

fn words(tokens: &[OwnedLexToken]) -> Vec<&str> {
    TokenWordView::new(tokens).word_refs()
}

#[test]
fn captures_gain_then_get_subject_ability_and_pump_tail() {
    let tokens = lex_line(
        "Target creature gains trample and gets +1/+0 until end of turn.",
        0,
    )
    .unwrap();
    let shape = parse_gain_then_get_shape(&tokens).unwrap();
    assert_eq!(words(shape.subject_tokens), ["target", "creature"]);
    assert_eq!(words(shape.ability_tokens), ["trample"]);
    assert_eq!(
        words(shape.pump_tokens),
        ["+1/+0", "until", "end", "of", "turn"]
    );
}

#[test]
fn captures_get_then_lose_signed_pump_and_ability_tail() {
    let tokens = lex_line(
        "Target creature gets -2/-0 and loses flying until your next turn.",
        0,
    )
    .unwrap();
    let shape = parse_get_then_ability_shape(&tokens).unwrap();
    assert_eq!(shape.ability_verb, SharedAbilityVerb::Lose);
    assert_eq!(words(shape.subject_tokens), ["target", "creature"]);
    assert_eq!(words(shape.pump_tokens), ["-2/-0"]);
    assert_eq!(
        words(shape.ability_tokens),
        ["flying", "until", "your", "next", "turn"]
    );
}

#[test]
fn get_then_ability_subject_excludes_leading_duration() {
    let tokens = lex_line(
        "Until end of turn, this creature gets -1/-1 and gains your choice of double strike, protection from red, vigilance, or shadow.",
        0,
    )
    .unwrap();
    let shape = parse_get_then_ability_shape(&tokens).unwrap();
    assert_eq!(shape.ability_verb, SharedAbilityVerb::Gain);
    assert_eq!(words(shape.subject_tokens), ["this", "creature"]);
    assert_eq!(words(shape.pump_tokens), ["-1/-1"]);
    assert_eq!(
        words(shape.ability_tokens),
        [
            "your",
            "choice",
            "of",
            "double",
            "strike",
            "protection",
            "from",
            "red",
            "vigilance",
            "or",
            "shadow"
        ]
    );
}

#[test]
fn rejects_completed_player_action_before_shared_pump_subject() {
    let tokens = lex_line(
        "You draw X cards and the chosen creatures get +X/+X and gain trample until end of turn, where X is the difference between the chosen creatures' powers.",
        0,
    )
    .unwrap();
    assert!(parse_get_then_ability_shape(&tokens).is_none());
}

#[test]
fn keeps_leading_duration_and_where_x_inside_typed_captures() {
    let tokens = lex_line(
        "Until end of turn, target creature gains trample and gets +X/+0, where X is the number of creatures you control.",
        0,
    )
    .unwrap();
    let shape = parse_gain_then_get_shape(&tokens).unwrap();
    assert_eq!(
        words(shape.subject_tokens),
        ["until", "end", "of", "turn", "target", "creature"]
    );
    assert!(words(shape.pump_tokens).starts_with(&["+x/+0", "where", "x", "is"]));
}

#[test]
fn captures_attached_object_and_related_creature_type_subject() {
    let tokens = lex_line(
        "Enchanted creature and other creatures that share a creature type with it get +1/+0 and gain first strike until end of turn.",
        0,
    )
    .unwrap();
    let shape = parse_attached_and_related_get_ability_shape(&tokens).unwrap();
    assert_eq!(shape.subject, AttachedReferenceSubject::EnchantedCreature);
    assert_eq!(words(shape.pump_tokens), ["+1/+0"]);
    assert_eq!(words(shape.ability_tokens), ["first", "strike"]);
    assert_eq!(shape.duration, Until::EndOfTurn);
}

#[test]
fn captures_attached_object_and_related_creature_type_pump_without_keyword() {
    let tokens = lex_line(
        "Enchanted creature and other creatures that share a creature type with it get +1/+1 until end of turn.",
        0,
    )
    .unwrap();
    let shape = parse_attached_and_related_get_shape(&tokens).unwrap();
    assert_eq!(shape.subject, AttachedReferenceSubject::EnchantedCreature);
    assert_eq!(words(shape.pump_tokens), ["+1/+1"]);
    assert_eq!(shape.duration, Until::EndOfTurn);
}
