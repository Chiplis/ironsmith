//! Source-authored and deliberately unrun (cf8 p04): "target red or green
//! creature an opponent controls" is one color-disjunction qualifier, not two
//! coordinated effects.
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

#[test]
fn tidebinder_mage_taps_one_red_or_green_target_and_locks_its_untap() {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/color_disjunction_targets.json.fixture"
    ))
    .unwrap();
    let row = &rows[0];
    assert_eq!(row["oracle_id"], "f881378b-b539-4ea8-981d-e01ee82af105");
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
    for definition in [direct.unwrap(), materialize_artifact(&restored).unwrap()] {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let debug = format!("{definition:?}");
        assert_eq!(debug.matches("TapEffect").count(), 1, "{debug}");
        assert!(debug.contains("Red") && debug.contains("Green"), "{debug}");
        assert!(debug.contains("Opponent"), "{debug}");
        assert!(debug.contains("Untap") || debug.contains("DoesntUntap"), "{debug}");
    }
}
