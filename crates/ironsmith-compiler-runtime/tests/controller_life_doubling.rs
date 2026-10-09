//! Source-authored and deliberately unrun (cf8 p04): "double its
//! controller's life total" sets that player's life to twice its current
//! value (CR 119.5: the player gains or loses the needed amount).
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

#[test]
fn celestial_mantle_doubles_the_enchanted_creatures_controllers_life() {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/controller_life_doubling.json.fixture"
    ))
    .unwrap();
    let row = &rows[0];
    assert_eq!(row["oracle_id"], "7417bc73-a75a-4c3a-88d2-17b43e225c2c");
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
        assert!(debug.contains("SetLifeTotalEffect"), "{debug}");
        let set = &debug[debug.find("SetLifeTotalEffect").unwrap()..];
        assert!(set.contains("ControllerOf"), "its controller, not you: {debug}");
        assert!(set.contains("Scaled"), "{debug}");
    }
}
