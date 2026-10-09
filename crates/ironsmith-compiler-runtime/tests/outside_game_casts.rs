//! Source-authored and deliberately unrun (cf8 p04): "Cast any number of
//! Eldrazi spells from among cards you own outside the game without paying
//! their mana costs." chooses owned cards from outside the game (CR 400.11)
//! and casts them for free.
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

#[test]
fn spawnsire_casts_owned_eldrazi_from_outside_the_game_for_free() {
    let rows: Vec<serde_json::Value> =
        serde_json::from_str(include_str!("../../../fixtures/outside_game_casts.json.fixture")).unwrap();
    let row = &rows[0];
    assert_eq!(row["oracle_id"], "b90d2f4d-b4ea-40af-aea0-1ab4234ab80f");
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
        assert!(debug.contains("OutsideGame"), "{debug}");
        assert!(debug.contains("Eldrazi"), "{debug}");
        assert!(debug.contains("owner: Some(You)"), "{debug}");
        assert!(debug.contains("without_paying_mana_cost: true"), "{debug}");
    }
}
