//! Source-authored and deliberately unrun (cf8 p04): "Target opponent reveals
//! their hand. You may copy an instant or sorcery card in it. If you do, you
//! may cast the copy without paying its mana cost." (Reversal of Fortune).
//! Choosing the revealed card is the optional copy; casting the copy is a
//! second option (CR 707.12). The card stays in its owner's hand.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

fn definitions() -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/optional_copy_from_revealed_hand.json.fixture"
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
fn reversal_of_fortune_copies_a_revealed_spell_card_and_may_cast_it() {
    for definition in definitions() {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let debug = format!("{definition:?}");
        assert!(debug.contains("Hand"), "{debug}");
        assert!(debug.contains("as_copy: true"), "{debug}");
        assert!(debug.contains("without_paying_mana_cost: true"), "{debug}");
        // The copied card is never moved.
        assert!(!debug.contains("ExileEffect"), "{debug}");
    }
}
