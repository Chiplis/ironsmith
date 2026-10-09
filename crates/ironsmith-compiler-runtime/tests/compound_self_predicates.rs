//! "Alexios attacks each combat if able, can't be sacrificed, and can't attack
//! its owner.": one source subject over a serial list of complete static
//! predicates becomes one static ability per member. Source-authored,
//! deliberately unrun.
use ironsmith::ability::AbilityKind;
use ironsmith_compiled_artifact::CompiledCardArtifact;

#[test]
fn alexios_keeps_all_three_self_predicates() {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/compound_self_predicates.json.fixture"
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
        let statics = definition
            .abilities
            .iter()
            .filter(|ability| matches!(ability.kind, AbilityKind::Static(_)))
            .count();
        // Trample plus the three members of the compound line.
        assert!(statics >= 4, "{:?}", definition.abilities);
        let debug = format!("{:?}", definition.abilities);
        assert!(debug.contains("BeSacrificed"), "{debug}");
    }
}
