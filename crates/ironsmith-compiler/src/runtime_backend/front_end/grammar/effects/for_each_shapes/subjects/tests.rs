use super::*;
use crate::runtime_backend::front_end::lexer::{TokenWordView, lex_line};

#[test]
fn parses_object_and_target_player_subjects() {
    let object = lex_line("for each artifact attached to a creature", 0).unwrap();
    let shape = parse_for_each_object_subject_shape(&object).unwrap();
    assert_eq!(
        TokenWordView::new(shape.filter_tokens).to_word_refs(),
        vec!["artifact"]
    );

    let players = lex_line("two target players each draw a card", 0).unwrap();
    let shape = parse_for_each_target_players_shape(&players).unwrap();
    assert_eq!(shape.count, ChoiceCount::exactly(2));
}

#[test]
fn captures_object_filter_and_effect_around_comma() {
    let tokens = lex_line(
        "For each token you control that entered this turn, create a token that's a copy of it.",
        0,
    )
    .unwrap();
    let shape = parse_for_each_object_effect_shape(&tokens).unwrap();
    assert_eq!(
        TokenWordView::new(shape.filter_tokens).to_word_refs(),
        ["token", "you", "control", "that", "entered", "this", "turn"]
    );
    assert_eq!(
        TokenWordView::new(shape.effect_tokens).to_word_refs(),
        ["create", "a", "token", "thats", "a", "copy", "of", "it"]
    );

    let spent = lex_line(
        "For each mana from a Desert spent to cast this spell, create a tapped Treasure token.",
        0,
    )
    .unwrap();
    let spent = parse_for_each_spent_mana_effect_shape(&spent).unwrap();
    assert_eq!(
        TokenWordView::new(spent.source_tokens).to_word_refs(),
        ["a", "desert"]
    );

    let dynamic_targets = lex_line(
        "For each of X target permanents, create X tokens that are copies of that permanent.",
        0,
    )
    .unwrap();
    let dynamic_targets = parse_for_each_dynamic_target_effect_shape(&dynamic_targets).unwrap();
    assert_eq!(
        TokenWordView::new(dynamic_targets.filter_tokens).to_word_refs(),
        ["permanents"]
    );
}

#[test]
fn rejects_effect_clause_before_where_x_as_an_iterated_subject() {
    let tokens = lex_line(
        "Each non-Vampire creature gets -X/-X until end of turn, where X is the number of creatures you control.",
        0,
    )
    .unwrap();

    assert!(parse_for_each_object_effect_shape(&tokens).is_none());
}
