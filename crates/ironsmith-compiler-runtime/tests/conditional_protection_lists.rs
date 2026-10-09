//! Source-authored and deliberately unrun (cf8 p04): "Until end of turn,
//! creatures you control gain protection from white if you control a Plains,
//! from blue if you control an Island, ..." (Dominaria's Judgment): each
//! protection quality is its own conditional grant, checked on resolution
//! (CR 608.2c).
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

fn definitions() -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/conditional_protection_lists.json.fixture"
    ))
    .unwrap();
    let row = &rows[0];
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
fn dominarias_judgment_grants_each_color_only_with_its_basic_land_type() {
    for definition in definitions() {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let debug = format!("{definition:?}");
        assert_eq!(debug.matches("ProtectionFrom").count() >= 5, true, "{debug}");
        for land in ["Plains", "Island", "Swamp", "Mountain", "Forest"] {
            assert!(debug.contains(land), "{land}: {debug}");
        }
        assert!(debug.contains("Conditional"), "{debug}");
    }
}
