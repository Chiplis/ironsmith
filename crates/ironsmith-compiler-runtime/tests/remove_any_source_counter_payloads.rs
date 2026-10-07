//! UNRUN: full-card fixtures from the October 7 current-main measurement.
//! Compilation of an early-error cohort is not a claim of complete card rules
//! support. The five original failures remain prior-source coverage holds.

use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::{
    encode_runtime_definition, materialize_artifact, materialize_definition,
};

fn counter_payloads(value: &serde_json::Value) -> Vec<String> {
    fn visit(value: &serde_json::Value, found: &mut Vec<String>) {
        match value {
            serde_json::Value::Object(fields) => {
                if fields.get("kind").and_then(serde_json::Value::as_str) == Some("RemoveAnyCountersFromSourceEffect") {
                    let payload: ironsmith_core::RemoveAnyCountersFromSourceEffect =
                        serde_json::from_value(fields["payload"].clone()).unwrap();
                    found.push(serde_json::to_string(&payload).unwrap());
                }
                for child in fields.values() { visit(child, found); }
            }
            serde_json::Value::Array(values) => {
                for child in values { visit(child, found); }
            }
            _ => {}
        }
    }
    let mut found = Vec::new();
    visit(value, &mut found);
    found.sort();
    found
}

fn assert_runtime_route(name: &str, route: &str, definition: CardDefinition, expected: &[String]) {
    assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition), "{route}: {name}");
    let encoded = encode_runtime_definition(definition).unwrap_or_else(|error| panic!("{route} native encoding: {name}: {error}"));
    assert_eq!(counter_payloads(&serde_json::to_value(&encoded).unwrap()), expected, "{route}: {name}");
    let restored = materialize_definition(encoded.clone()).unwrap_or_else(|error| panic!("{route} native materialization: {name}: {error}"));
    let encoded_again = encode_runtime_definition(restored).unwrap();
    assert_eq!(serde_json::to_value(encoded_again).unwrap(), serde_json::to_value(encoded).unwrap(), "{route} native roundtrip: {name}");
}

fn check_cohort(status: &str, expected_count: usize) {
    let fixtures: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/remove_any_source_counter_payloads.json.fixture"
    )).unwrap();
    let selected: Vec<_> = fixtures.iter().filter(|row| row["prior_status"] == status).collect();
    assert_eq!(selected.len(), expected_count);
    let identities: std::collections::HashSet<_> = selected.iter().map(|row| row["oracle_id"].as_str().unwrap()).collect();
    assert_eq!(identities.len(), expected_count);
    for row in selected {
        let text = row["text"].as_str().unwrap();
        for name in row["measured_entry_names"].as_array().unwrap() {
            let name = name.as_str().unwrap();
            let (direct, direct_loss) = ironsmith_compiler::parse_loss::capture(||
                compile_to_runtime_definition(name, text, false));
            let direct = direct.unwrap_or_else(|error| panic!("direct compiler: {name}: {error}"));
            assert!(!direct_loss.is_lossy(), "direct {name}: {}", direct_loss.reasons_text());

            let (compiled, artifact_loss) = ironsmith_compiler::parse_loss::capture(||
                compile_to_artifact(name, text, false));
            let (artifact, artifact_materialized) = compiled.unwrap_or_else(|error| panic!("artifact compiler: {name}: {error}"));
            assert!(!artifact_loss.is_lossy(), "artifact {name}: {}", artifact_loss.reasons_text());
            let expected = counter_payloads(&serde_json::to_value(&artifact.payload.definition).unwrap());
            assert!(!expected.is_empty(), "{name}: measured source-counter payload must remain in the complete graph");
            let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
            assert_eq!(artifact, decoded, "artifact envelope roundtrip: {name}");
            let json_materialized = materialize_artifact(&decoded).unwrap();

            assert_runtime_route(name, "direct compiler", direct, &expected);
            assert_runtime_route(name, "artifact compiler", artifact_materialized, &expected);
            assert_runtime_route(name, "artifact JSON materialization", json_materialized, &expected);
        }
    }
}

#[test]
fn formerly_supported_56_source_counter_regressions_compile_on_each_route() {
    check_cohort("formerly_supported_regression", 56);
}

#[test]
fn original_five_source_counter_holds_compile_on_each_route_without_coverage_admission() {
    check_cohort("original_failure_prior_source_hold", 5);
}
