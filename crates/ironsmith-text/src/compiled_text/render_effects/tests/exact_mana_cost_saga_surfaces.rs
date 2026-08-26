use super::*;

const URZAS_SAGA_TEXT: &str = "I — This Saga gains \"{T}: Add {C}.\"\n\
II — This Saga gains \"{2}, {T}: Create a 0/0 colorless Construct artifact creature token with 'This token gets +1/+1 for each artifact you control.'\"\n\
III — Search your library for an artifact card with mana cost {0} or {1}, put it onto the battlefield, then shuffle.";

fn compile_urzas_saga() -> crate::CardDefinition {
    crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Urza's Saga")
        .card_types(vec![CardType::Enchantment, CardType::Land])
        .subtypes(vec![Subtype::Saga])
        .parse_text(URZAS_SAGA_TEXT)
        .expect("typed Saga chapters should compile")
}

#[test]
fn public_saga_surface_preserves_nested_quote_and_exact_mana_cost_union() {
    let definition = compile_urzas_saga();
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        URZAS_SAGA_TEXT
    );

    let debug = format!("{definition:#?}");
    assert!(debug.contains("exact_mana_cost: Some"), "{debug}");
    assert!(debug.contains("Generic(\n"), "{debug}");
    assert!(debug.contains("0,"), "{debug}");
    assert!(debug.contains("1,"), "{debug}");
    assert!(
        !debug.contains("mana_value: Some"),
        "an exact printed cost must not degrade to a mana-value predicate: {debug}"
    );
}

#[test]
fn exact_mana_cost_union_keeps_colored_costs_distinct_and_requires_a_shared_base() {
    let generic = crate::mana::ManaCost::from_symbols(vec![crate::mana::ManaSymbol::Generic(1)]);
    let white = crate::mana::ManaCost::from_symbols(vec![crate::mana::ManaSymbol::White]);
    assert_ne!(generic, white);

    let generic_filter = ObjectFilter {
        exact_mana_cost: Some(generic),
        ..ObjectFilter::artifact()
    };
    let white_filter = ObjectFilter {
        exact_mana_cost: Some(white),
        ..ObjectFilter::artifact()
    };
    let union = ObjectFilter {
        any_of: vec![generic_filter, white_filter],
        ..ObjectFilter::default()
    };

    assert_eq!(
        union.description(),
        "artifact with mana cost {1} or {W}",
        "equal mana values must retain their distinct printed costs"
    );

    let mut changed_base = union;
    changed_base.any_of[1].card_types = vec![CardType::Creature];
    assert_eq!(
        changed_base.description(),
        "artifact with mana cost {1} or creature with mana cost {W}",
        "different branch bases must not be compacted into one exact-cost clause"
    );
}

#[test]
fn joint_create_saga_chapter_keeps_both_actors_and_one_target() {
    let text = "I — You and target opponent each create a Food token.\n\
II — Each opponent loses 3 life. Create a Treasure token.\n\
III — Create three tapped 1/1 white Spirit creature tokens with flying.";
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Oath of the Grey Host")
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![Subtype::Saga])
            .parse_text(text)
            .expect("joint-create Saga chapters should compile");

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        text
    );
    let debug = format!("{definition:#?}");
    assert!(debug.matches("CreateTokenEffect").count() >= 3, "{debug}");
    assert!(debug.contains("controller: Target(\n"), "{debug}");
    assert!(debug.contains("controller_target: Some(\n"), "{debug}");
}
