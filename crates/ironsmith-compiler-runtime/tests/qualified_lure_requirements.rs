//! Source-authored and deliberately unrun (cf8 p04): "All creatures your
//! opponents control able to block that creature this turn do so." — a
//! blocking requirement (CR 509.1c) over a qualified blocker set.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

fn definitions() -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/qualified_lure_requirements.json.fixture"
    ))
    .unwrap();
    let row = &rows[0];
    assert_eq!(row["name"], "You Look Upon the Tarrasque");
    assert_eq!(row["oracle_id"], "231a6f8a-624a-4bff-8648-0b4ef910a6aa");
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
fn second_mode_forces_only_opposing_creatures_to_block_the_target() {
    for definition in definitions() {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let debug = format!("{:?}", definition.spell_effect);
        assert!(debug.contains("ChooseModeEffect"), "{debug}");
        assert!(debug.contains("MustBlockSpecificAttacker"), "{debug}");
        // The blockers are the opponents' creatures, not every creature.
        let requirement = &debug[debug.find("MustBlockSpecificAttacker").unwrap()..];
        assert!(requirement.contains("controller: Some(Opponent)"), "{debug}");
        assert!(debug.contains("Indestructible"), "{debug}");
    }
}
