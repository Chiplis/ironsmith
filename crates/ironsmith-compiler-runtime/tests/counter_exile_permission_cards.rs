//! Source-authored and deliberately unrun (cf8 p04): the merged exact
//! counter/exile-permission owner (counter_exile_permission.rs) reads both
//! cards' complete three-sentence programs, including Thranduil's
//! permanent-spell gate and Kheru's face-up trigger body. Regression coverage
//! only; no package code change.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

fn definitions(index: usize) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/counter_exile_permission_cards.json.fixture"
    ))
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

#[test]
fn counter_exile_permission_programs_compile_whole() {
    for (index, gate) in [(0, "PermanentSpell"), (1, "AnySpell")] {
        for definition in definitions(index) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
            let debug = format!("{definition:?}");
            assert!(debug.contains(gate), "{debug}");
        }
    }
}
