//! "put a soul counter on this Equipment for each player who lost life this
//! turn" (Reaper's Scythe): the number of distinct players with life loss in
//! this turn's history. Source-authored, deliberately unrun.
use ironsmith_compiled_artifact::CompiledCardArtifact;

#[test]
fn reapers_scythe_counts_every_player_who_lost_life() {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/players_lost_life_counts.json.fixture"
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
        assert!(debug.contains("PlayersLostLife(Any)"), "{debug}");
        assert!(debug.contains("PutCounters"), "{debug}");
    }
}
