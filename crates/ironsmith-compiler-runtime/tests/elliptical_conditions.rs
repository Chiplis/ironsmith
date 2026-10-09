//! cf8/p06 elliptical conditions: "... if it has X. If it doesn't, Y." The
//! second sentence elides the first one's predicate and is its false arm.
//! Full frozen bodies on the direct and artifact routes. Source-authored,
//! deliberately unrun.
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
fn if_it_doesnt_is_the_preceding_conditionals_false_arm() {
    for (name, true_arm, false_arm) in [
        ("Marcus, Mutant Mayor", "DrawCards", "PutCounters"),
        ("Weatherlight Compleated", "DrawCards", "Scry"),
    ] {
        for definition in definitions(name) {
            let debug = format!("{definition:?}");
            assert!(debug.contains("Conditional"), "{name}: {debug}");
            let draw = debug.find(true_arm).unwrap_or_else(|| panic!("{name}: {debug}"));
            let fallback = debug.rfind(false_arm).unwrap_or_else(|| panic!("{name}: {debug}"));
            assert!(draw < fallback, "{name}: the false arm follows the true arm");
            assert!(!debug.contains("IfResult"), "{name}: not a result of a prior action");
        }
    }
}
