//! "it deals 2 damage to you unless it came under your control this turn"
//! (Erg Raiders): a resolution-time state exception gating the damage.
//! Source-authored, deliberately unrun.
use ironsmith_compiled_artifact::CompiledCardArtifact;

#[test]
fn erg_raiders_damage_is_skipped_when_it_came_under_your_control_this_turn() {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/unless_came_under_control.json.fixture"
    ))
    .unwrap();
    let name = rows[0]["name"].as_str().unwrap();
    let text = rows[0]["text"].as_str().unwrap();
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| {
        ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false)
    });
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let direct = direct.unwrap_or_else(|error| panic!("{name}: {error}"));
    let (compiled, loss) = ironsmith_compiler::parse_loss::capture(|| {
        ironsmith_compiler_runtime::compile_to_artifact(name, text, false)
    });
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (artifact, _) = compiled.unwrap_or_else(|error| panic!("{name}: {error}"));
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    let decoded =
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
    for definition in [direct, decoded] {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let debug = format!("{:?}", definition.abilities);
        assert!(debug.contains("SourceCameUnderYourControlThisTurn"), "{debug}");
        assert!(debug.contains("ConditionalEffect"), "{debug}");
        assert!(debug.contains("DealDamageEffect"), "{debug}");
        assert!(debug.contains("SourceAttackedThisTurn"), "{debug}");
    }
}
