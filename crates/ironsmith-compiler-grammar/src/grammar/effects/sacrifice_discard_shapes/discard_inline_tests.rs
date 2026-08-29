use crate::lexer::{lex_line, parser_token_word_refs, render_token_slice};

use super::*;

#[test]
fn discard_clause_returns_typed_count_and_sides() {
    let tokens = lex_line("two black cards at random", 0).unwrap();
    let shape = parse_discard_clause_shape(&tokens).unwrap();
    let DiscardClauseShape::Cards(cards) = shape else {
        panic!("expected counted cards shape");
    };
    assert_eq!(cards.count, Value::Fixed(2));
    assert_eq!(
        parse_discard_qualifier_shape(cards.qualifier_tokens),
        DiscardQualifierShape::Colors(ColorSet::BLACK)
    );
    assert_eq!(
        parse_discard_trailing_shape(cards.trailing_tokens),
        DiscardTrailingShape::Random
    );
}

#[test]
fn discard_clause_preserves_up_to_as_an_optional_bounded_choice() {
    let tokens = lex_line("up to two cards", 0).unwrap();
    let shape = parse_discard_clause_shape(&tokens).unwrap();
    let DiscardClauseShape::Cards(cards) = shape else {
        panic!("expected counted cards shape");
    };
    assert_eq!(cards.count, Value::Fixed(2));
    assert!(cards.any_number);
}

#[test]
fn discard_clause_preserves_one_or_more_as_nonzero_unbounded_choice() {
    let tokens = lex_line("one or more land cards", 0).unwrap();
    let shape = parse_discard_clause_shape(&tokens).unwrap();
    let DiscardClauseShape::Cards(cards) = shape else {
        panic!("expected counted cards shape");
    };
    assert!(cards.any_number);
    assert!(
        cards
            .count
            .has_surface_hint(ValueSurfaceHint::OneOrMoreChoice)
    );
    assert_eq!(parser_token_word_refs(cards.qualifier_tokens), vec!["land"]);
}

#[test]
fn discard_clause_recognizes_all_cards_in_players_hand_as_discard_hand() {
    for text in [
        "all the cards in their hand",
        "all cards in your hand",
        "all the cards from that player's hand",
    ] {
        let tokens = lex_line(text, 0).unwrap();
        assert!(matches!(
            parse_discard_clause_shape(&tokens),
            Ok(DiscardClauseShape::AllCardsInHand)
        ));
    }
    let direct = lex_line("your hand", 0).unwrap();
    assert!(matches!(
        parse_discard_clause_shape(&direct),
        Ok(DiscardClauseShape::Hand)
    ));
}

#[test]
fn discard_clause_preserves_equal_count_and_same_mana_reference() {
    let tokens = lex_line(
        "a number of cards equal to the damage dealt with the same mana value as that spell",
        0,
    )
    .unwrap();
    let shape = parse_discard_clause_shape(&tokens).unwrap();
    let DiscardClauseShape::EqualCount { count, .. } = shape else {
        panic!("expected equal-count discard shape");
    };
    assert!(count.has_surface_hint(ValueSurfaceHint::EqualTo));

    let trailing = lex_line("with the same mana value as that spell", 0).unwrap();
    assert_eq!(
        parse_discard_trailing_shape(&trailing),
        DiscardTrailingShape::SameManaValueAsTriggering
    );
}

#[test]
fn chosen_color_qualifier_accepts_article_normalization() {
    let tokens = lex_line("of the chosen color", 0).unwrap();
    assert_eq!(
        parse_discard_qualifier_shape(&tokens),
        DiscardQualifierShape::ChosenColor
    );
}

#[test]
fn alternative_shape_skips_color_or_and_finds_the_next_action() {
    let tokens = lex_line("two black or red cards or sacrifice a creature", 0).unwrap();
    let shape = parse_discard_alternative_shape(&tokens).unwrap();
    assert_eq!(
        parser_token_word_refs(shape.discard_tokens),
        ["two", "black", "or", "red", "cards"]
    );
}

#[test]
fn discard_unless_shape_returns_typed_predicate_tokens() {
    let tokens = lex_line("unless they pay {2}", 0).unwrap();
    let DiscardUnlessShape::Predicate(predicate_tokens) = parse_discard_unless_shape(&tokens)
    else {
        panic!("expected discard unless predicate");
    };
    assert_eq!(render_token_slice(predicate_tokens), "they pay {2}");
}

#[test]
fn additional_cost_color_tail_preserves_action_and_noun() {
    let tokens = lex_line("of each of the sacrificed creature's colors", 0).unwrap();
    assert_eq!(
        parse_additional_cost_object_colors_surface(&tokens),
        Some(ironsmith_core::AdditionalCostObjectSurface::new(
            ironsmith_core::AdditionalCostObjectAction::Sacrificed,
            ironsmith_core::SacrificedObjectKind::Creature,
        ))
    );
}
