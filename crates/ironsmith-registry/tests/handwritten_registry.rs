use ironsmith_registry::RegistryCatalog;
use ironsmith_registry::cards::definitions::{basic_forest, lightning_bolt, llanowar_elves};

#[test]
fn registry_catalog_registers_representative_handwritten_cards() {
    let catalog = RegistryCatalog::with_builtin_cards();

    for name in ["Forest", "Llanowar Elves", "Lightning Bolt"] {
        assert!(
            catalog.inner().get(name).is_some(),
            "expected compiler-owning handwritten registry to include {name}"
        );
    }
}

#[test]
fn handwritten_registry_constructs_representative_typed_cards() {
    let forest = basic_forest();
    let elves = llanowar_elves();
    let bolt = lightning_bolt();

    assert_eq!(forest.name(), "Forest");
    assert!(forest.card.is_land());
    assert_eq!(elves.name(), "Llanowar Elves");
    assert!(elves.is_creature());
    assert_eq!(bolt.name(), "Lightning Bolt");
    assert!(bolt.is_spell());
}

#[test]
fn typed_handwritten_spell_retains_its_runtime_program() {
    let bolt = lightning_bolt();
    let program = bolt
        .spell_effect
        .expect("Lightning Bolt should have a typed spell program");
    assert_eq!(program.flattened_default_effects().len(), 1);
}
