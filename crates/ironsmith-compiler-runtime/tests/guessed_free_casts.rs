//! Source-authored and deliberately unrun (cf8 p04): The Seventh Doctor's
//! guess. You choose a card in your hand; the defending player guesses
//! whether its mana value is greater than the number of artifacts you
//! control; on a wrong guess you may cast it free; if no spell was cast this
//! way, you investigate. The guess is the defending player's choice between
//! two answers, checked against the card on resolution (CR 608.2c).
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

fn definitions() -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> =
        serde_json::from_str(include_str!("../../../fixtures/guessed_free_casts.json.fixture"))
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
fn seventh_doctor_casts_on_a_wrong_guess_and_investigates_otherwise() {
    for definition in definitions() {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let debug = format!("{definition:?}");
        // The defending player picks between the two answers.
        assert!(debug.contains("Defending"), "{debug}");
        assert!(debug.contains("Greater") && debug.contains("Not greater"), "{debug}");
        // The answer is checked against the chosen card's mana value.
        assert!(debug.contains("ManaValueOf"), "{debug}");
        assert!(debug.contains("GreaterThan"), "{debug}");
        // A free cast of the chosen card, else Investigate.
        assert!(debug.contains("CastTagged"), "{debug}");
        assert!(debug.contains("without_paying_mana_cost: true"), "{debug}");
        assert!(debug.contains("Investigate"), "{debug}");
    }
}
