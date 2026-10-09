//! Shared frozen-body helpers for the cf8 p01 (lossy / semantic-marker)
//! regression files. Source-authored, deliberately unrun.
#![allow(dead_code)]
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;

pub fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../../fixtures/p01_lossy_semantic_markers.json.fixture"
    ))
    .unwrap()
}

pub fn row(name: &str) -> serde_json::Value {
    rows()
        .into_iter()
        .find(|row| row["card_name"] == name)
        .unwrap_or_else(|| panic!("{name} is not in the p01 fixture"))
}

/// The complete frozen body compiled strictly and without parse loss on the
/// direct route and through the serialized artifact.
pub fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = row(name);
    let text = row["text"].as_str().unwrap();
    definitions_for_text(name, text)
}

/// Same as [`definitions`] for a collateral card outside the package fixture,
/// given its complete Oracle text (type line and body as the fixture stores it).
pub fn definitions_for_text(name: &str, text: &str) -> [CardDefinition; 2] {
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| {
        ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false)
    });
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let direct = direct.unwrap_or_else(|error| panic!("{name}: {error}"));
    let (compiled, loss) = ironsmith_compiler::parse_loss::capture(|| {
        ironsmith_compiler_runtime::compile_to_artifact(name, text, false)
    });
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (artifact, _) = compiled.unwrap_or_else(|error| panic!("{name}: {error}"));
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(artifact, restored);
    let decoded =
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
    for definition in [&direct, &decoded] {
        assert!(
            !ironsmith::cards::generated_definition_has_unimplemented_content(definition),
            "{name}: unimplemented content"
        );
    }
    [direct, decoded]
}

pub fn rendered(definition: &CardDefinition) -> String {
    ironsmith_text::canonical_compiled_lines(definition)
        .join("\n")
        .to_ascii_lowercase()
}

/// The internal-marker and dropped-marker phrases the authoritative audit
/// rejects must not appear in (or be missing from) the rendered body.
pub fn assert_no_internal_markers(name: &str, text: &str) {
    for marker in ["tagged '", "tagged object '", "valuecomparison {", "custom condition"] {
        assert!(!text.contains(marker), "{name}: leaked {marker:?} in {text}");
    }
}
