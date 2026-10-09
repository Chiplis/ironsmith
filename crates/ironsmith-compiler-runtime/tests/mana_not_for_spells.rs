//! "This mana can't be spent to cast spells." (Thran Turbine): the added mana
//! carries a payment restriction excluding every spell (CR 106.6).
//! Source-authored, deliberately unrun.
use ironsmith_compiled_artifact::CompiledCardArtifact;

#[test]
fn thran_turbine_mana_cannot_pay_for_spells() {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/mana_not_for_spells.json.fixture"
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
        assert!(debug.contains("Not("), "{debug}");
        assert!(debug.contains("CastSpell"), "{debug}");
        assert!(debug.contains("MayEffect"), "{debug}");
    }
}
