//! "Each opponent may discard a card or sacrifice a permanent of their choice.
//! Zoyowa deals 3 damage to each opponent who didn't." (Zoyowa Lava-Tongue):
//! the per-opponent optional choice and damage to the opponents who declined.
//! Source-authored, deliberately unrun.
use ironsmith_compiled_artifact::CompiledCardArtifact;

#[test]
fn zoyowa_damages_each_opponent_who_declined() {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/opponents_who_declined_damage.json.fixture"
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
        assert!(debug.contains("VillainousChoiceEffect"), "{debug}");
        assert!(debug.contains("MayEffect"), "{debug}");
        assert!(debug.contains("DidNotHappen"), "{debug}");
        assert!(debug.contains("DealDamageEffect"), "{debug}");
        assert!(debug.contains("Descend") || debug.contains("descend"), "{debug}");
    }
}
