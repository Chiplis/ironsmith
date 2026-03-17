use ironsmith::{cards::CardDefinitionBuilder, ids::CardId, types::CardType};

fn parse_error_message(name: &str, text: &str, card_types: &[CardType]) -> String {
    let mut builder = CardDefinitionBuilder::new(CardId::new(), name);
    if !card_types.is_empty() {
        builder = builder.card_types(card_types.to_vec());
    }
    let err = builder
        .parse_text(text)
        .expect_err("test fixture should produce unsupported parse error");
    format!("{err:?}").to_ascii_lowercase()
}

#[test]
fn line_level_unsupported_includes_rule_id() {
    let rendered = parse_error_message(
        "Known Static Variant",
        "Play with the top card of your library revealed.",
        &[CardType::Enchantment],
    );
    assert!(
        rendered.contains("unsupported static clause")
            && rendered.contains("[rule=known-static-clause]"),
        "expected line-level unsupported diagnostic with rule id, got {rendered}"
    );
}

#[test]
fn clause_level_unsupported_includes_rule_id() {
    let rendered = parse_error_message(
        "Deep Water Variant",
        "{U}: Until end of turn, if you tap a land you control for mana, it produces {U} instead of any other type.",
        &[CardType::Instant],
    );
    assert!(
        rendered.contains("unsupported mana replacement clause")
            && rendered.contains("[rule=mana-replacement]"),
        "expected clause-level unsupported diagnostic with rule id, got {rendered}"
    );
}

#[test]
fn sentence_level_unsupported_includes_rule_id() {
    let rendered = parse_error_message(
        "Ring Variant",
        "The Ring tempts you.",
        &[CardType::Sorcery],
    );
    assert!(
        rendered.contains("unsupported ring tempts clause")
            && rendered.contains("[rule=ring-tempts]"),
        "expected sentence-level unsupported diagnostic with rule id, got {rendered}"
    );
}
