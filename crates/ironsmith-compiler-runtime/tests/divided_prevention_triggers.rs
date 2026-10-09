//! Source-authored and deliberately unrun (cf8 p04): p03's divided prevention
//! reader ("prevent the next N damage that would be dealt this turn to any
//! number of targets, divided as you choose", CR 601.2d / 615.7) reached from
//! a triggered ability (Angel of Salvation; division announced as the trigger
//! is put on the stack, CR 603.3d) and from an activated ability with a
//! counted X (Serra's Hymn). No package code change: regression coverage only.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

fn definitions(index: usize) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/divided_prevention_triggers.json.fixture"
    ))
    .unwrap();
    let row = &rows[index];
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
fn angel_of_salvation_divides_five_prevention_among_any_number_of_targets() {
    for definition in definitions(0) {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let debug = format!("{definition:?}");
        assert!(debug.contains("divided: true"), "{debug}");
        assert!(debug.contains("Fixed(5)"), "{debug}");
    }
}

#[test]
fn serras_hymn_divides_x_equal_to_verse_counters() {
    for definition in definitions(1) {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let debug = format!("{definition:?}");
        assert!(debug.contains("divided: true"), "{debug}");
        assert!(debug.contains("Verse"), "{debug}");
    }
}
