use super::super::super::super::lexer::{lex_line, render_token_slice};
use super::*;

#[test]
fn parses_equal_to_value_shapes() {
    let tokens = lex_line("equal to the amount of mana spent to cast this spell.", 0).unwrap();
    assert!(parse_equal_to_mana_spent_to_cast_tokens(&tokens));

    let tokens = lex_line(
        "equal to the greatest number of cards an opponent has drawn this turn.",
        0,
    )
    .unwrap();
    assert!(parse_equal_to_greatest_cards_drawn_this_turn_tokens(
        &tokens
    ));
}

#[test]
fn parses_where_x_metric_and_aggregate_shapes() {
    let tokens = lex_line("where X is the amount of life you've gained this turn.", 0).unwrap();
    assert_eq!(
        parse_where_x_player_metric_tokens(&tokens),
        Some(WhereXPlayerMetric::LifeGainedByYouThisTurn)
    );

    let tokens = lex_line("where X is the amount of life you lost this turn.", 0).unwrap();
    assert_eq!(
        parse_where_x_player_metric_tokens(&tokens),
        Some(WhereXPlayerMetric::LifeLostByYouThisTurn)
    );

    let tokens = lex_line(
        "where X is the greatest mana value among creatures you control.",
        0,
    )
    .unwrap();
    let parsed = parse_where_x_aggregate_filter_tokens(&tokens).unwrap();
    assert_eq!(parsed.aggregate, EtbAggregateKind::Greatest);
    assert_eq!(parsed.value_kind, EtbAggregateValueKind::ManaValue);
    assert_eq!(
        render_token_slice(parsed.filter_tokens),
        "creatures you control"
    );
}

#[test]
fn parses_where_x_count_variants() {
    let tokens = lex_line("where X is twice the number of creatures you control.", 0).unwrap();
    let parsed = parse_where_x_number_of_filter_tokens(&tokens).unwrap();
    assert_eq!(parsed.multiplier, 2);
    assert_eq!(
        render_token_slice(parsed.filter_tokens),
        "creatures you control"
    );

    let tokens = lex_line("where X is the number of cards in your hand minus two.", 0).unwrap();
    let parsed = parse_where_x_number_of_filter_offset_tokens(&tokens).unwrap();
    assert_eq!(parsed.operator, EtbNumberOffsetOperator::Minus);
    assert_eq!(
        render_token_slice(parsed.filter_tokens),
        "cards in your hand"
    );
    assert_eq!(render_token_slice(parsed.offset_tokens), "two");
}

#[test]
fn parses_where_x_reference_and_filter_shapes() {
    let tokens = lex_line(
        "where X is two plus the sacrificed creature's mana value.",
        0,
    )
    .unwrap();
    let parsed = parse_where_x_fixed_plus_reference_tokens(&tokens).unwrap();
    assert_eq!(
        parsed.reference_kind,
        EtbReferenceValueKind::TaggedCreatureManaValue
    );
    assert_eq!(render_token_slice(parsed.fixed_tokens), "two");

    let tokens = lex_line(
        "where X is the number of differently named creatures you control.",
        0,
    )
    .unwrap();
    let filter = parse_where_x_differently_named_filter_tokens(&tokens).unwrap();
    assert_eq!(render_token_slice(filter), "creatures you control");
}

#[test]
fn parses_where_x_source_stat_shapes() {
    let tokens = lex_line("where X is this creature's power.", 0).unwrap();
    assert_eq!(
        parse_where_x_source_stat_tokens(&tokens).map(|parsed| (parsed.kind, parsed.fallback)),
        Some((
            EtbSourceStatKind::Power,
            Some(EtbSourceStatFallback::Source),
        ))
    );

    let tokens = lex_line(
        "where X is this creature's power as this ability resolves.",
        0,
    )
    .unwrap();
    let parsed = parse_where_x_source_stat_tokens(&tokens).expect("resolution-time source stat");
    assert_eq!(parsed.kind, EtbSourceStatKind::Power);
    assert_eq!(parsed.fallback, Some(EtbSourceStatFallback::Source));
    assert!(parsed.as_this_ability_resolves);

    let tokens = lex_line("where X is that spell's mana value.", 0).unwrap();
    assert_eq!(
        parse_where_x_source_stat_tokens(&tokens).map(|parsed| (parsed.kind, parsed.fallback)),
        Some((
            EtbSourceStatKind::ManaValue,
            Some(EtbSourceStatFallback::TriggeringSpell),
        ))
    );

    let tokens = lex_line("where X is the sacrificed creature's toughness.", 0).unwrap();
    assert_eq!(
        parse_where_x_source_stat_tokens(&tokens).map(|parsed| (parsed.kind, parsed.fallback)),
        Some((
            EtbSourceStatKind::Toughness,
            Some(EtbSourceStatFallback::TaggedObject),
        ))
    );

    let tokens = lex_line("where X is that creature's mana value.", 0).unwrap();
    assert_eq!(
        parse_where_x_source_stat_tokens(&tokens).map(|parsed| (parsed.kind, parsed.fallback)),
        Some((
            EtbSourceStatKind::ManaValue,
            Some(EtbSourceStatFallback::TaggedObject),
        ))
    );
}
