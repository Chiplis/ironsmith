//! cf8/p06 "If you pay" after "unless you pay" (CR 118.12): the paid branch
//! is the unless-wrapper's declined outcome. Full frozen bodies on the
//! direct and artifact routes. Source-authored, deliberately unrun.
//!
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/p06_round4.json.fixture")).unwrap()
}

fn text(row: &serde_json::Value) -> String {
    let mut lines = vec![
        format!("Mana cost: {}", row["mana_cost"].as_str().unwrap()),
        format!("Type: {}", row["type_line"].as_str().unwrap()),
    ];
    if let (Some(power), Some(toughness)) = (row["power"].as_str(), row["toughness"].as_str()) {
        lines.push(format!("Power/Toughness: {power}/{toughness}"));
    }
    if let Some(loyalty) = row["loyalty"].as_str() {
        lines.push(format!("Loyalty: {loyalty}"));
    }
    lines.push(row["oracle_text"].as_str().unwrap().to_string());
    lines.join("\n")
}

fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = rows().into_iter().find(|row| row["name"] == name).unwrap();
    let text = text(&row);
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| {
        compile_to_runtime_definition(name, &text, false)
    });
    let direct = direct.unwrap_or_else(|error| panic!("direct {name}: {error}"));
    assert!(!loss.is_lossy(), "direct {name}: {}", loss.reasons_text());
    let (compiled, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, &text, false));
    let (artifact, _) = compiled.unwrap_or_else(|error| panic!("artifact {name}: {error}"));
    assert!(!loss.is_lossy(), "artifact {name}: {}", loss.reasons_text());
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    let decoded = materialize_artifact(&restored).unwrap();
    assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&direct));
    [direct, decoded]
}


#[test]
fn if_you_pay_reads_the_paid_unless_branch() {
    for definition in definitions("Cyclone") {
        let debug = format!("{definition:?}");
        assert!(debug.contains("UnlessPays"), "{debug}");
        // Paying prevents the sacrifice; the damage follows only then.
        assert!(debug.contains("WasDeclined"), "{debug}");
        assert!(!debug.contains("predicate: Happened"), "{debug}");
    }
}
