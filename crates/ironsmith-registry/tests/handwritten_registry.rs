use ironsmith_registry::CardRegistry;

#[test]
fn builtin_registry_constructs_representative_handwritten_cards() {
    let mut registry = CardRegistry::new();
    registry.ensure_cards_loaded(["Forest", "Llanowar Elves", "Lightning Bolt"]);

    for name in ["Forest", "Llanowar Elves", "Lightning Bolt"] {
        assert!(
            registry.get(name).is_some(),
            "expected handwritten registry to include {name}"
        );
    }
}

#[test]
fn targeted_compile_resolves_handwritten_cards() {
    let bolt =
        CardRegistry::try_compile_card("Lightning Bolt").expect("Lightning Bolt should compile");
    assert_eq!(bolt.name(), "Lightning Bolt");
    assert!(bolt.is_spell());

    let forest = CardRegistry::try_compile_card("Forest").expect("Forest should compile");
    assert_eq!(forest.name(), "Forest");
    assert!(forest.card.is_land());
}
