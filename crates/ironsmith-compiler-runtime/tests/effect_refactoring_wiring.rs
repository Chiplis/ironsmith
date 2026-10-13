//! Representative public payloads still compile and materialize after runtime
//! owner migration. Gameplay contracts are tested separately at their owners.
use ironsmith_compiled_artifact::CompiledCardArtifact;

#[test]
fn refactored_effect_families_compile_real_cards_without_loss_and_round_trip() {
    let cards: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/effect_refactoring_wiring.json.fixture"
    ))
    .unwrap();
    for card in cards {
        let name = card["name"].as_str().unwrap();
        let mut lines = vec![
            format!("Mana cost: {}", card["mana_cost"].as_str().unwrap()),
            format!("Type: {}", card["type_line"].as_str().unwrap()),
        ];
        if let (Some(power), Some(toughness)) = (card["power"].as_str(), card["toughness"].as_str())
        {
            lines.push(format!("Power/Toughness: {power}/{toughness}"));
        }
        if let Some(loyalty) = card["loyalty"].as_str() {
            lines.push(format!("Loyalty: {loyalty}"));
        }
        lines.push(card["oracle_text"].as_str().unwrap().to_string());
        let (compiled, loss) = ironsmith_compiler::parse_loss::capture(|| {
            ironsmith_compiler_runtime::compile_to_artifact(name, &lines.join("\n"), false)
        });
        let (artifact, direct) = compiled.unwrap_or_else(|error| panic!("{name}: {error}"));
        assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
        let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
        restored.validate().unwrap();
        let materialized =
            ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored)
                .unwrap();
        for definition in [direct, materialized] {
            assert!(
                !ironsmith::cards::generated_definition_has_unimplemented_content(&definition),
                "{name}"
            );
        }
    }
}
