//! Source-authored and deliberately unrun (cf8 p04): "Target opponent mills
//! nine cards, then you may cast an instant or sorcery spell from that
//! player's graveyard without paying its mana cost. If that spell would be
//! put into a graveyard, exile it instead." (Sorcerous Squall). The leading
//! action is its own clause; the graveyard cast and its exile replacement
//! (CR 614.1a) are the existing graveyard-cast procedure.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

fn definitions() -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/leading_action_graveyard_cast.json.fixture"
    ))
    .unwrap();
    let row = &rows[0];
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
fn sorcerous_squall_mills_then_casts_with_an_exile_replacement() {
    for definition in definitions() {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let debug = format!("{definition:?}");
        assert!(debug.contains("Mill"), "{debug}");
        assert!(debug.contains("CastTagged"), "{debug}");
        assert!(debug.contains("RegisterFutureZoneReplacement"), "{debug}");
        assert!(debug.contains("Exile"), "{debug}");
    }
}
