//! Source-authored and deliberately unrun (cf8 p04): "return target creature
//! card ... to the battlefield tapped and attacking with a finality counter on
//! it" keeps `tapped and attacking` and the counter rider in one return
//! instruction (CR 508.4: put onto the battlefield attacking, never declared).
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

#[test]
fn grim_reaper_returns_one_tapped_attacking_creature_with_its_finality_counter() {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/tapped_attacking_entry_riders.json.fixture"
    ))
    .unwrap();
    let row = &rows[0];
    assert_eq!(row["oracle_id"], "edf429a8-6a4d-4d2a-b0ca-c0c72a0698a3");
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
        assert!(debug.contains("Finality"), "{debug}");
        assert!(debug.contains("attacking: true") || debug.contains("Attacking"), "{debug}");
        assert!(debug.contains("tapped: true") || debug.contains("Tapped"), "{debug}");
        assert!(!debug.contains("CounterSpellEffect"), "no second counter instruction: {debug}");
    }
}
