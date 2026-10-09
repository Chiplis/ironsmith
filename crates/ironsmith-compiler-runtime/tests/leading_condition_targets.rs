//! cf8/p06 targets introduced inside a leading "if": the target is announced
//! with the spell (CR 601.2c) and the condition tests it on resolution
//! (CR 608.2b). Full frozen bodies on the direct and artifact routes.
//! Source-authored, deliberately unrun.
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
fn an_object_target_in_the_condition_is_declared_and_tested() {
    for definition in definitions("Blood Lust") {
        let debug = format!("{definition:?}");
        // One target slot, a resolution-time test of it, and the otherwise arm.
        assert!(debug.contains("TargetMatches") || debug.contains("TargetObjectMatches"), "{debug}");
        assert!(debug.contains("toughness"), "{debug}");
        assert_eq!(debug.matches("TargetOnlyEffect").count(), 1, "{debug}");
    }
}

#[test]
fn a_player_target_in_a_life_condition_is_declared_and_bound_to_that_player() {
    for definition in definitions("Hidetsugu's Second Rite") {
        let debug = format!("{definition:?}");
        assert!(debug.contains("LifeTotal(Target("), "{debug}");
        assert!(debug.contains("Fixed(10)"), "{debug}");
        assert_eq!(debug.matches("TargetOnlyEffect").count(), 1, "{debug}");
    }
}

#[test]
fn a_player_counter_condition_declares_its_target_and_gives_the_difference() {
    for definition in definitions("Vraska, Betrayal's Sting") {
        let debug = format!("{definition:?}");
        assert!(debug.contains("PlayerCounters(Target("), "{debug}");
        assert!(debug.contains("Fixed(9)"), "{debug}");
        assert!(debug.contains("Scaled("), "{debug}");
    }
}

#[test]
fn a_card_directly_above_the_source_in_the_graveyard() {
    for name in ["Death Spark", "Krovikan Horror"] {
        for definition in definitions(name) {
            let debug = format!("{definition:?}");
            assert!(debug.contains("SourceInGraveyardWithCardsAbove"), "{name}: {debug}");
            assert!(debug.contains("directly_above: true"), "{name}: {debug}");
        }
    }
}
