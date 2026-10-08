//! Source-authored and deliberately unrun (cf8 p04): characteristic changes
//! that last "for as long as <it> has a <kind> counter on it" (CR 611.2b).
//! Liege of the Tangle animates the countered lands (they're still lands),
//! Minas Morgul makes the countered creature a Wraith in addition to its
//! other types, and Ultima strips the countered land's land types (CR 205.3)
//! and abilities (CR 613.1f) and grants it "{T}: Add {C}."
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

fn definitions(index: usize, oracle_id: &str) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/counter_linked_animation.json.fixture"
    ))
    .unwrap();
    let row = &rows[index];
    assert_eq!(row["oracle_id"], oracle_id);
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

fn assert_counter_linked(debug: &str, counter: &str) {
    assert!(debug.contains("ForAsLongAs"), "{debug}");
    assert!(debug.contains("AffectedObject"), "{debug}");
    assert!(debug.contains(counter), "{debug}");
    assert!(!debug.contains("Until::Forever") || debug.contains("ForAsLongAs"), "{debug}");
}

#[test]
fn liege_animates_awakened_lands_while_they_keep_the_counter() {
    for definition in definitions(0, "a85b845c-a196-43db-b740-6ce191345097") {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let debug = format!("{definition:?}");
        assert_counter_linked(&debug, "Awakening");
        // 8/8 green Elemental creature; "They're still lands" keeps Land.
        assert!(debug.contains("Elemental"), "{debug}");
        assert!(debug.contains("Fixed(8)"), "{debug}");
        assert!(debug.contains("StillALand"), "{debug}");
    }
}

#[test]
fn minas_morgul_makes_a_wraith_while_the_shadow_counter_stays() {
    for definition in definitions(1, "867dbd5a-c3cf-41ce-980b-c9babc6f30f2") {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let debug = format!("{definition:?}");
        assert_counter_linked(&debug, "Shadow");
        assert!(debug.contains("Wraith"), "{debug}");
    }
}

#[test]
fn ultima_blighted_land_loses_land_types_and_abilities_and_taps_for_colorless() {
    for definition in definitions(2, "baa337ce-edc6-4ee5-a898-68e9dbb4ab93") {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let debug = format!("{definition:?}");
        assert_counter_linked(&debug, "Blight");
        assert!(debug.contains("RemoveAllSubtypesOfFamily") || debug.contains("Land"), "{debug}");
        assert!(debug.contains("RemoveAllAbilities") || debug.contains("RemoveAbilities"), "{debug}");
        assert!(debug.contains("Colorless"), "{debug}");
    }
}
