//! Source-authored and deliberately unrun (cf8 p04): "As an additional cost
//! to cast this spell, blight X. X can't be greater than the greatest
//! toughness among creatures you control." (Soul Immolation). The caster
//! announces X (CR 601.2b), bounded by the live aggregate, and pays it by
//! putting X -1/-1 counters on a creature they control (CR 601.2h).
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

fn definitions() -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> =
        serde_json::from_str(include_str!("../../../fixtures/blight_x_costs.json.fixture"))
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
fn soul_immolation_blights_an_announced_x_bounded_by_greatest_toughness() {
    for definition in definitions() {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let debug = format!("{definition:?}");
        assert!(debug.contains("PutCountersEffect"), "{debug}");
        assert!(debug.contains("MinusOneMinusOne"), "{debug}");
        assert!(debug.contains("Blight"), "{debug}");
        assert!(debug.contains("ThisSpellXMaximum"), "{debug}");
        assert!(debug.contains("GreatestToughness"), "{debug}");
        // The damage uses the same X.
        assert!(debug.contains("DealDamage") || debug.contains("Damage"), "{debug}");
    }
}
