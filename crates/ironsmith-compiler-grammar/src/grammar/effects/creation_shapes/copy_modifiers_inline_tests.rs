use super::*;

#[test]
fn parses_copy_modifiers_into_typed_spec() {
    let spec = parse_copy_modifier_words(&[
        "except",
        "it",
        "is",
        "a",
        "red",
        "dragon",
        "with",
        "flying",
        "and",
        "isnt",
        "legendary",
    ])
    .unwrap();
    assert!(spec.set_colors.is_some());
    assert_eq!(spec.set_subtypes, Some(vec![Subtype::Dragon]));
    assert_eq!(spec.removed_supertypes, vec![Supertype::Legendary]);
    assert_eq!(spec.granted_abilities, vec![StaticAbility::flying()]);
}

#[test]
fn parses_counted_keyword_copy_modifier() {
    let spec =
        parse_copy_modifier_words(&["except", "its", "1/1", "and", "has", "toxic", "1"]).unwrap();
    assert_eq!(
        spec.granted_abilities,
        vec![StaticAbility::keyword_marker("toxic 1")]
    );
}

#[test]
fn creature_type_addition_keeps_existing_subtypes() {
    let spec = parse_copy_modifier_words(&[
        "except",
        "it",
        "is",
        "a",
        "reflection",
        "in",
        "addition",
        "to",
        "its",
        "other",
        "creature",
        "types",
    ])
    .expect("typed creature-subtype addition should parse");

    assert_eq!(spec.added_subtypes, [Subtype::Reflection]);
    assert!(spec.set_subtypes.is_none());
}

#[test]
fn parses_explicit_spell_copy_color_exception() {
    let spec = parse_copy_modifier_words(&["except", "that", "the", "copy", "is", "red"]).unwrap();

    assert_eq!(spec.set_colors, Some(ColorSet::RED));
}

#[test]
fn parses_starting_loyalty_copy_exception() {
    let spec = parse_copy_modifier_words(&[
        "except",
        "it",
        "isnt",
        "legendary",
        "and",
        "its",
        "starting",
        "loyalty",
        "is",
        "1",
    ])
    .unwrap();

    assert_eq!(spec.removed_supertypes, vec![Supertype::Legendary]);
    assert_eq!(spec.starting_loyalty, Some(1));
}

#[test]
fn parses_dynamic_totals_and_colorless_copy_exception() {
    let spec = parse_copy_modifier_words(&[
        "except",
        "its",
        "power",
        "is",
        "equal",
        "to",
        "the",
        "total",
        "power",
        "of",
        "those",
        "creatures",
        "its",
        "toughness",
        "is",
        "equal",
        "to",
        "the",
        "total",
        "toughness",
        "of",
        "those",
        "creatures",
        "and",
        "it",
        "s",
        "a",
        "colorless",
        "eldrazi",
        "creature",
    ])
    .expect("typed aggregate copy exception should parse");

    assert!(spec.set_base_power_toughness_to_source_totals);
    assert_eq!(spec.set_colors, Some(ColorSet::new()));
    assert_eq!(spec.set_card_types, Some(vec![CardType::Creature]));
    assert_eq!(spec.set_subtypes, Some(vec![Subtype::Eldrazi]));
}

#[test]
fn rejects_half_of_an_aggregate_copy_pt_exception() {
    let error = parse_copy_modifier_words(&[
        "except",
        "its",
        "power",
        "is",
        "equal",
        "to",
        "the",
        "total",
        "power",
        "of",
        "those",
        "creatures",
    ])
    .expect_err("one aggregate characteristic must not silently imply the other");

    assert!(error.to_string().contains("both values"));
}
