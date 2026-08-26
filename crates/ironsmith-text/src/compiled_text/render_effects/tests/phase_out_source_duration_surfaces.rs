use super::*;

#[test]
fn ordinary_out_word_is_not_a_short_source_alias() {
    let oracle = "When this enchantment enters, untap all creatures, then those creatures phase out until this enchantment leaves the battlefield. Put a time counter on this enchantment for each creature that phased out this way.\nVanishing";
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Out of Time")
        .card_types(vec![CardType::Enchantment])
        .parse_text(oracle)
        .expect("phase-out duration and its result count should compile");
    let debug = format!("{definition:#?}");

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        oracle,
        "{debug}"
    );
    assert!(debug.contains("PhaseOutEffect"), "{debug}");
    assert!(debug.contains("UntilSourceLeaves"), "{debug}");
    assert!(debug.contains("PutCountersEffect"), "{debug}");
    assert!(debug.contains("AffectedObjects"), "{debug}");
}
