//! Shared frozen-card loader for the cf8 p05 ("could not find verb") clusters.
//! Source-authored and deliberately unrun until the campaign build.
#![allow(dead_code)]
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;

pub fn rows(cluster: &str) -> Vec<serde_json::Value> {
    let all: Vec<serde_json::Value> =
        serde_json::from_str(include_str!("../../../../fixtures/cf8_p05_noverb_b.json.fixture"))
            .unwrap();
    all.into_iter().filter(|row| row["cluster"] == cluster).collect()
}

pub fn row(cluster: &str, name: &str) -> serde_json::Value {
    rows(cluster)
        .into_iter()
        .find(|row| row["name"] == name)
        .unwrap_or_else(|| panic!("{name} missing from {cluster}"))
}

pub fn source_text(row: &serde_json::Value) -> String {
    let mut text = String::new();
    let mana_cost = row["mana_cost"].as_str().unwrap();
    if !mana_cost.is_empty() {
        text.push_str(&format!("Mana cost: {mana_cost}\n"));
    }
    text.push_str(&format!("Type: {}\n", row["type_line"].as_str().unwrap()));
    if let (Some(power), Some(toughness)) = (row["power"].as_str(), row["toughness"].as_str())
        && power != "*"
        && toughness != "*"
    {
        text.push_str(&format!("Power/Toughness: {power}/{toughness}\n"));
    }
    if let Some(loyalty) = row["loyalty"].as_str() {
        text.push_str(&format!("Loyalty: {loyalty}\n"));
    }
    text.push_str(row["oracle_text"].as_str().unwrap());
    text
}

/// Assert each listed marker appears in the lowered definition's structure.
pub fn assert_markers(cluster: &str, name: &str, markers: &[&str]) {
    for definition in definitions(&row(cluster, name)) {
        let debug = debug(&definition);
        for marker in markers {
            assert!(debug.contains(marker), "{name}: missing {marker}");
        }
    }
}

/// The complete frozen card compiled strictly on the direct route and on the
/// artifact route (serialized, restored, validated and materialized).
pub fn definitions(row: &serde_json::Value) -> [CardDefinition; 2] {
    let name = row["name"].as_str().unwrap();
    let text = source_text(row);
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| {
        ironsmith_compiler_runtime::compile_to_runtime_definition(name, &text, false)
    });
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let direct = direct.unwrap_or_else(|error| panic!("{name}: {error}"));
    let (compiled, loss) = ironsmith_compiler::parse_loss::capture(|| {
        ironsmith_compiler_runtime::compile_to_artifact(name, &text, false)
    });
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (artifact, _) = compiled.unwrap_or_else(|error| panic!("{name}: {error}"));
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(artifact, restored);
    let decoded =
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
    for definition in [&direct, &decoded] {
        assert_eq!(definition.card.name, name);
        assert!(
            !ironsmith::cards::generated_definition_has_unimplemented_content(definition),
            "{name}: unimplemented content"
        );
    }
    [direct, decoded]
}

/// Structural view of a lowered definition for typed-name assertions.
pub fn debug(definition: &CardDefinition) -> String {
    format!("{definition:?}")
}
