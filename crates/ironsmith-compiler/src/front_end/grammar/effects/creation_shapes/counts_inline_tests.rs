use super::super::super::super::super::lexer::lex_line;
use super::*;

#[test]
fn parses_dynamic_creation_counts() {
    let tokens = lex_line("creatures that died this turn", 0).unwrap();
    assert!(matches!(
        parse_creation_for_each_dynamic_count_tokens(&tokens).map(Value::into_unhinted),
        Some(Value::CreaturesDiedThisTurn)
    ));
    let tokens = lex_line("colors of mana spent to cast this spell", 0).unwrap();
    assert!(matches!(
        parse_creation_for_each_dynamic_count_tokens(&tokens).map(Value::into_unhinted),
        Some(Value::ColorsOfManaSpentToCastThisSpell)
    ));

    let tokens = lex_line("creature in your party", 0).unwrap();
    let party = parse_creation_for_each_dynamic_count_tokens(&tokens)
        .expect("party creation count should parse");
    assert!(party.has_surface_hint(ValueSurfaceHint::ForEach));
    assert_eq!(party.into_unhinted(), Value::PartySize(PlayerFilter::You));

    let tokens = lex_line("permanent exiled this way", 0).unwrap();
    let prior_exile = parse_creation_for_each_dynamic_count_tokens(&tokens)
        .expect("typed prior-exile creation count should parse");
    assert!(prior_exile.has_surface_hint(ValueSurfaceHint::ForEach));
    let Value::PendingPriorEffectMetric(query) = prior_exile.into_unhinted() else {
        panic!("expected typed prior-effect metric");
    };
    assert_eq!(
        query.action,
        Some(ironsmith_core::PriorEffectAction::Exiled)
    );
    assert_eq!(
        query.filter.expect("permanent filter").card_types,
        ObjectFilter::permanent_card().card_types
    );

    let tokens = lex_line("mana from a Cave spent to cast it", 0).unwrap();
    let spent_mana = parse_creation_for_each_dynamic_count_tokens(&tokens)
        .expect("creation count should retain mana-payment provenance");
    let Value::ManaFromSourceSpentToCastThisSpell {
        source_filter,
        include_source_noun,
        reference,
    } = spent_mana.unhinted()
    else {
        panic!("expected typed mana-source count, got {spent_mana:#?}");
    };
    assert!(!include_source_noun);
    assert_eq!(
        *reference,
        ironsmith_core::ManaSpentCastReferenceSurface::It
    );
    assert_eq!(source_filter.subtypes, [crate::Subtype::Cave]);
}

#[test]
fn parses_colored_mana_symbols_as_dynamic_creation_count() {
    let tokens = lex_line(
        "white mana symbol in the mana costs of permanents you control",
        0,
    )
    .unwrap();
    let value = parse_creation_for_each_dynamic_count_tokens(&tokens)
        .expect("colored mana-symbol creation count should parse");
    assert!(value.has_surface_hint(ValueSurfaceHint::ForEach));
    let Value::ManaSymbolsInManaCostOf { spec, color } = value.into_unhinted() else {
        panic!("expected structured mana-symbol creation count");
    };
    assert_eq!(color, crate::color::Color::White);
    let ChooseSpec::All(filter) = spec.unhinted() else {
        panic!("expected aggregate permanent scope");
    };
    assert_eq!(filter.zone, Some(Zone::Battlefield));
    assert_eq!(filter.controller, Some(PlayerFilter::You));
}
