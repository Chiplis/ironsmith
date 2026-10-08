//! Source-authored and deliberately unrun (cf8 p04): "exile <this source>
//! and each <filter>" names two recipients of one exile instruction.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

fn definitions(name: &str, oracle_id: &str) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/source_and_each_exile.json.fixture"
    ))
    .unwrap();
    let row = rows.iter().find(|row| row["name"] == name).unwrap();
    assert_eq!(row["oracle_id"], oracle_id);
    let text = row["text"].as_str().unwrap();
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| {
        compile_to_runtime_definition(name, text, false)
    });
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (compiled, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (artifact, _) = compiled.unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    let decoded = materialize_artifact(&restored).unwrap();
    for definition in [&decoded] {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition));
    }
    [direct.unwrap(), decoded]
}

#[test]
fn ajani_and_fraying_line_exile_themselves_and_the_named_set() {
    for (name, oracle_id) in [
        ("Ajani, Strength of the Pride", "c3974ec7-7af0-4b65-baf6-4c1c4fe8a5c4"),
        ("Fraying Line", "b69d2bd6-3e99-4c51-89af-0acedd48739e"),
    ] {
        for definition in definitions(name, oracle_id) {
            let debug = format!("{definition:?}");
            assert!(debug.matches("Exile").count() >= 2, "{name}: {debug}");
            assert!(debug.contains("source: true") || debug.contains("Source"), "{name}: {debug}");
        }
    }
    for definition in definitions("Ajani, Strength of the Pride", "c3974ec7-7af0-4b65-baf6-4c1c4fe8a5c4") {
        let debug = format!("{definition:?}");
        assert!(debug.contains("Opponent"), "each artifact and creature your opponents control: {debug}");
    }
}
