//! Source-authored and deliberately unrun (cf8 p04): "If target spell has
//! only one target and that target is <X>, change that spell's target to
//! another <Y>." (Meddle, Quicksilver Dragon). Any spell may be targeted; the
//! single-target test runs on resolution (CR 608.2b) and the new target must
//! be a different object matching <Y> (CR 115.7).
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

fn definitions(index: usize) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/single_target_retarget.json.fixture"
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

fn assert_conditional_retarget(debug: &str) {
    assert!(debug.contains("RetargetStackObjectEffect"), "{debug}");
    assert!(debug.contains("require_change: true"), "{debug}");
    assert!(debug.contains("new_target_restriction: Some(Object("), "{debug}");
    assert!(debug.contains("target_count"), "{debug}");
    assert!(debug.contains("Conditional"), "{debug}");
}

#[test]
fn meddle_moves_a_single_creature_target_to_another_creature() {
    for definition in definitions(0) {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let debug = format!("{definition:?}");
        assert_conditional_retarget(&debug);
    }
}

#[test]
fn quicksilver_dragon_redirects_spells_that_target_only_it() {
    for definition in definitions(1) {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let debug = format!("{definition:?}");
        assert_conditional_retarget(&debug);
        assert!(debug.contains("source: true"), "{debug}");
    }
}
