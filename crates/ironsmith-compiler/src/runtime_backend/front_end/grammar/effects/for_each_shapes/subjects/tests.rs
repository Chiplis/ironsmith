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

    let opponents = lex_line("any number of target opponents each draw a card", 0).unwrap();
    let shape = parse_for_each_target_players_shape(&opponents).unwrap();
    assert_eq!(shape.count, ChoiceCount::any_number());
    assert_eq!(
        TokenWordView::new(shape.target_tokens).to_word_refs(),
        vec!["target", "opponents"]
    );
}

#[test]
fn target_player_each_shape_requires_a_targeted_participant_set_and_each_actor() {
    for text in [
        "target opponent draws a card for each creature you control",
        "any number of opponents each draw a card",
        "any number of target creatures each get +1/+1",
        "any number of target opponents draw a card",
    ] {
        let tokens = lex_line(text, 0).unwrap();
        assert!(
            parse_for_each_target_players_shape(&tokens).is_none(),
            "the target-participant fanout parser must not claim {text:?}"
        );
    }
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

    let symbol_spent = lex_line("For each {U}{U} spent to cast it, draw a card.", 0).unwrap();
    let symbol_spent = parse_for_each_mana_symbol_spent_effect_shape(&symbol_spent).unwrap();
    assert_eq!(symbol_spent.symbol, crate::mana::ManaSymbol::Blue);
    assert_eq!(symbol_spent.group_size, 2);
    assert_eq!(
        symbol_spent.reference,
        ironsmith_core::ManaSpentCastReferenceSurface::It
    );
    assert_eq!(
        TokenWordView::new(symbol_spent.effect_tokens).to_word_refs(),
        ["draw", "a", "card"]
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
fn in_exile_zone_noun_remains_inside_for_each_object_filter() {
    for leading in ["", "Then "] {
        let tokens = lex_line(
            &format!(
                "{leading}For each creature card you own in exile with a memory counter on it, create a tapped and attacking token that's a copy of it."
            ),
            0,
        )
        .unwrap();
        let shape = parse_for_each_object_effect_shape(&tokens).unwrap();
        assert_eq!(
            TokenWordView::new(shape.filter_tokens).to_word_refs(),
            [
                "creature", "card", "you", "own", "in", "exile", "with", "a", "memory", "counter",
                "on", "it"
            ]
        );
        assert_eq!(
            TokenWordView::new(shape.effect_tokens).to_word_refs(),
            [
                "create",
                "a",
                "tapped",
                "and",
                "attacking",
                "token",
                "thats",
                "a",
                "copy",
                "of",
                "it"
            ]
        );
    }
}

#[test]
fn genuine_exile_action_still_rejects_for_each_object_subject() {
    let tokens = lex_line("for each creature card you own exile it", 0).unwrap();
    assert!(parse_for_each_object_subject_shape(&tokens).is_none());

    let tokens = lex_line(
        "for each creature card you own in exile destroy a permanent",
        0,
    )
    .unwrap();
    assert!(parse_for_each_object_subject_shape(&tokens).is_none());
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
