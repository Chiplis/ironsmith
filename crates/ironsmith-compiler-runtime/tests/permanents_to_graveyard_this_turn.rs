//! "the number of artifacts that were put into graveyards from the
//! battlefield this turn" (Structural Assault): a turn-history count of every
//! player's matching permanents that moved battlefield -> graveyard, counted
//! after the same spell's destruction. Source-authored, deliberately unrun.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;

fn definitions(name: &str) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/permanents_to_graveyard_this_turn.json.fixture"
    ))
    .unwrap();
    let row = rows.into_iter().find(|row| row["name"] == name).unwrap();
    let text = row["text"].as_str().unwrap();
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
    assert_eq!(artifact, restored);
    let decoded =
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
    for definition in [&direct, &decoded] {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition));
    }
    [direct, decoded]
}

#[test]
fn structural_assault_counts_artifacts_moved_to_graveyards_this_turn() {
    for definition in definitions("Structural Assault") {
        let debug = format!("{:?}", definition.spell_effect);
        assert!(debug.contains("MovedZones"), "{debug}");
        assert!(debug.contains("from: Some(Battlefield)"), "{debug}");
        assert!(debug.contains("to: Some(Graveyard)"), "{debug}");
        assert!(debug.contains("Artifact"), "{debug}");
        assert!(debug.contains("Destroy"), "{debug}");
    }
}
