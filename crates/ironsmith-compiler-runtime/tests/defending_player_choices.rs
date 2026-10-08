//! Source-authored and deliberately unrun (cf8 p04): "defending player
//! chooses ..." makes the defending player (CR 506.2) the chooser of an
//! object the rest of the ability then acts on.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

fn definitions(name: &str, oracle_id: &str) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/defending_player_choices.json.fixture"
    ))
    .unwrap();
    let row = rows.iter().find(|row| row["name"] == name).unwrap();
    assert_eq!(row["oracle_id"], oracle_id);
    let text = row["text"].as_str().unwrap();
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| {
        compile_to_runtime_definition(name, text, false)
    });
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (compiled, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (artifact, _) = compiled.unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    let decoded = materialize_artifact(&restored).unwrap();
    assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&decoded));
    [direct.unwrap(), decoded]
}

#[test]
fn crashing_boars_lets_the_defender_pick_its_forced_blocker() {
    for definition in definitions("Crashing Boars", "a4c59feb-e76a-4e08-b296-21c3e61cfc64") {
        let debug = format!("{definition:?}");
        assert!(debug.contains("ChooseObjectsEffect"), "{debug}");
        assert!(debug.contains("chooser: Defending"), "{debug}");
        assert!(debug.contains("MustBlockSpecificAttacker") || debug.contains("MustBlock"), "{debug}");
    }
}

#[test]
fn drana_lets_the_defender_pick_the_returned_creature_card() {
    for definition in definitions("Drana, the Last Bloodchief", "3ae3b901-4d36-40e2-b861-26209fa1e823") {
        let debug = format!("{definition:?}");
        assert!(debug.contains("chooser: Defending"), "{debug}");
        assert!(debug.contains("Graveyard"), "{debug}");
        assert!(debug.contains("Vampire"), "{debug}");
    }
}
