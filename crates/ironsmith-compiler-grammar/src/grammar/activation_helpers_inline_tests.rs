use super::super::super::lexer::lex_line;
use super::*;

#[test]
fn typed_add_mana_facts_preserve_choice_and_chosen_color_tails() {
    let choice = lex_line("any one type that a land you control could produce", 0).unwrap();
    let facts = parse_add_mana_clause_facts(&choice);
    let parsed_choice = facts.choice.unwrap();
    assert_eq!(parsed_choice.kind, AddManaChoiceKind::AnyOneType);
    assert_eq!(
        TokenWordView::new(parsed_choice.tail_tokens).word_refs(),
        ["that", "a", "land", "you", "control", "could", "produce"]
    );

    let chosen = lex_line("one mana of that color to your mana pool", 0).unwrap();
    let facts = parse_add_mana_clause_facts(&chosen);
    assert!(facts.one_that_color_tail.is_some());
    assert!(is_mana_pool_tail(facts.one_that_color_tail.unwrap()));
}

#[test]
fn typed_land_production_shape_preserves_filter_span_and_rejects_trailing_words() {
    let tokens = lex_line("that a land you control could produce", 0).unwrap();
    let LandCouldProduceShape::CouldProduceFilter(filter) =
        parse_land_could_produce_shape(&tokens).unwrap()
    else {
        panic!("expected a land-production filter");
    };
    assert_eq!(
        TokenWordView::new(filter).word_refs(),
        ["a", "land", "you", "control"]
    );

    let produced = lex_line("that land produced", 0).unwrap();
    let LandCouldProduceShape::TriggeringEventProducedFilter(filter) =
        parse_land_could_produce_shape(&produced).unwrap()
    else {
        panic!("expected a triggering-event production filter");
    };
    assert_eq!(TokenWordView::new(filter).word_refs(), ["land"]);

    let trailing = lex_line("that a land could produce this turn", 0).unwrap();
    assert!(matches!(
        parse_land_could_produce_shape(&trailing),
        Some(LandCouldProduceShape::UnsupportedTrailing)
    ));
}

#[test]
fn typed_mana_choice_parsers_return_colors_and_fixed_output_boundaries() {
    let options = lex_line("any combination of w u and b mana", 0).unwrap();
    assert_eq!(
        parse_any_combination_mana_colors(&options).unwrap(),
        Some(vec![Color::White, Color::Blue, Color::Black])
    );

    let fixed = lex_line("{W} {U} for each creature you control", 0).unwrap();
    let output = parse_fixed_mana_output(&fixed);
    assert_eq!(output.mana, vec![ManaSymbol::White, ManaSymbol::Blue]);
    assert_eq!(output.first_for_each_token, Some(2));
}

#[test]
fn any_color_among_parser_returns_the_dynamic_filter_span() {
    let tokens = lex_line(
        "one mana of any color among legendary permanents you control",
        0,
    )
    .unwrap();
    let span = parse_any_color_among_span(&tokens).expect("expected any-color-among span");
    assert_eq!(
        TokenWordView::new(span.filter_tokens).word_refs(),
        ["legendary", "permanents", "you", "control"]
    );

    let unrestricted = lex_line("one mana of any color", 0).unwrap();
    assert!(parse_any_color_among_span(&unrestricted).is_none());
}
