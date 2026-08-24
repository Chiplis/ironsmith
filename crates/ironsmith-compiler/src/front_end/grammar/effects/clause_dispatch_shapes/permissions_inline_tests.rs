use super::*;
use crate::TokenWordView;
use crate::lexer::lex_line;

#[test]
fn parses_cast_and_counter_group_shapes() {
    let cast = lex_line(
            "You may cast any number of spells with mana value X or less from among those cards without paying their mana costs.",
            0,
        )
        .unwrap();
    assert!(matches!(
        parse_cast_any_tagged_shape(&cast).unwrap().mana_value,
        Some(Comparison::LessThanOrEqualExpr(_))
    ));
    let group = lex_line("For each two counters removed this way, draw a card.", 0).unwrap();
    assert_eq!(
        parse_counter_group_removed_shape(&group)
            .unwrap()
            .group_size,
        2
    );
}

#[test]
fn tagged_collection_cast_shape_preserves_cardinality_filter_and_global_cap() {
    let cases = [
        (
            "You may cast any number of spells with mana value X or less from among them without paying their mana costs.",
            ChoiceCount::any_number(),
            vec!["spells"],
        ),
        (
            "You may cast up to two sorcery spells with mana value 3 or less from among them without paying their mana costs.",
            ChoiceCount::up_to(2),
            vec!["sorcery", "spells"],
        ),
        (
            "You may cast an instant or sorcery spell with mana value X or less from among them without paying its mana cost.",
            ChoiceCount::up_to(1),
            vec!["instant", "or", "sorcery", "spell"],
        ),
        (
            "You may cast instant and sorcery spells with mana value X or less from among them without paying their mana costs.",
            ChoiceCount::any_number(),
            vec!["instant", "and", "sorcery", "spells"],
        ),
    ];
    for (text, count, subject) in cases {
        let tokens = lex_line(text, 0).unwrap();
        let parsed = parse_cast_tagged_collection_shape(&tokens)
            .unwrap_or_else(|| panic!("expected tagged collection cast shape for {text}"));
        assert_eq!(parsed.count, count, "{text}");
        assert_eq!(
            TokenWordView::new(parsed.subject_tokens).to_word_refs(),
            subject,
            "{text}"
        );
        assert!(parsed.mana_value.is_some(), "{text}");
    }
}

#[test]
fn parses_target_from_your_graveyard_this_turn_permission() {
    let tokens = lex_line(
        "You may cast target Zombie creature card from your graveyard this turn.",
        0,
    )
    .unwrap();
    let shape = parse_cast_target_from_your_graveyard_this_turn_shape(&tokens)
        .expect("targeted graveyard permission");

    assert_eq!(
        TokenWordView::new(shape.target_tokens).to_word_refs(),
        vec![
            "target",
            "zombie",
            "creature",
            "card",
            "from",
            "your",
            "graveyard"
        ]
    );
    let stripped = lex_line(
        "cast target Zombie creature card from your graveyard this turn.",
        0,
    )
    .unwrap();
    let stripped_shape = parse_cast_target_from_your_graveyard_this_turn_shape(&stripped)
        .expect("leading-may chain should route its stripped cast clause");
    assert_eq!(
        TokenWordView::new(stripped_shape.target_tokens).to_word_refs(),
        TokenWordView::new(shape.target_tokens).to_word_refs(),
    );
    let wrong_zone = lex_line(
        "You may cast target Zombie creature card from exile this turn.",
        0,
    )
    .unwrap();
    assert!(parse_cast_target_from_your_graveyard_this_turn_shape(&wrong_zone).is_none());
}
