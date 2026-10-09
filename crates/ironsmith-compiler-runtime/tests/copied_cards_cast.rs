//! Source-authored and deliberately unrun (cf8 p04): "Copy <cards> [N times].
//! You may cast [any number of] the copies [without paying their mana
//! costs]." Each copy is cast through the tagged-cast copy mode, once per
//! copy made; every cast is optional (CR 707.12).
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

fn definitions(index: usize) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> =
        serde_json::from_str(include_str!("../../../fixtures/copied_cards_cast.json.fixture"))
            .unwrap();
    let row = &rows[index];
    let name = row["name"].as_str().unwrap();
    let text = row["text"].as_str().unwrap();
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| {
        compile_to_runtime_definition(name, text, false)
    });
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (compiled, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, _) = compiled.unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [direct.unwrap(), materialize_artifact(&restored).unwrap()]
}

fn copy_casts(debug: &str) -> usize {
    debug.matches("as_copy: true").count()
}

#[test]
fn mnemonic_deluge_casts_three_free_copies_of_the_exiled_card() {
    for definition in definitions(0) {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let debug = format!("{definition:?}");
        assert!(copy_casts(&debug) >= 3, "{debug}");
        assert!(debug.contains("without_paying_mana_cost: true"), "{debug}");
    }
}

#[test]
fn arcane_bombardment_casts_a_copy_of_each_card_exiled_with_it() {
    for definition in definitions(1) {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let debug = format!("{definition:?}");
        assert!(copy_casts(&debug) >= 1, "{debug}");
        assert!(debug.contains("__source_exiled__") || debug.contains("SourceExiled"), "{debug}");
    }
}

#[test]
fn chandra_ultimate_chooses_an_exiled_spell_card_and_casts_three_copies() {
    for definition in definitions(2) {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let debug = format!("{definition:?}");
        assert!(copy_casts(&debug) >= 3, "{debug}");
        assert!(debug.contains("ChooseObjects"), "{debug}");
    }
}

#[test]
fn bloodthirsty_adversary_casts_copies_inside_its_reflexive_payment_result() {
    for definition in definitions(3) {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let debug = format!("{definition:?}");
        assert!(copy_casts(&debug) >= 1, "{debug}");
        assert!(debug.contains("without_paying_mana_cost: true"), "{debug}");
        assert!(debug.contains("PlusOnePlusOne"), "{debug}");
    }
}

#[test]
fn zethi_casts_copies_of_exiled_cards_with_kick_counters() {
    for definition in definitions(4) {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let debug = format!("{definition:?}");
        assert!(copy_casts(&debug) >= 1, "{debug}");
        assert!(debug.contains("kick"), "{debug}");
        assert!(debug.contains("Exile"), "{debug}");
    }
}
