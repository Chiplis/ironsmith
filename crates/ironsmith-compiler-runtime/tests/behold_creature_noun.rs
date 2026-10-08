//! Source-authored and deliberately unrun (cf8 p04): "you may behold a Gamma
//! creature" is the optional behold additional cost (CR 701.66 behold) whose
//! payment the later "if this spell's additional cost was paid" reads.
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

#[test]
fn hulks_thunderclap_offers_an_optional_gamma_behold() {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/behold_creature_noun.json.fixture"
    ))
    .unwrap();
    let row = &rows[0];
    assert_eq!(row["oracle_id"], "e377059d-a918-462f-920a-6b062bd5fcfd");
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
        assert_eq!(definition.optional_costs.len(), 1);
        let debug = format!("{:?}", definition.optional_costs[0]);
        assert!(debug.contains("Behold") && debug.contains("Gamma"), "{debug}");
        let spell = format!("{:?}", definition.spell_effect);
        assert!(spell.contains("ThisSpellPaidLabel") || spell.contains("Paid"), "{spell}");
    }
}
