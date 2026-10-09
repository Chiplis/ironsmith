//! Blocker-count evasion (CR 509.1b/c) on a labeled self line and granted to
//! an authored set of creatures. Source-authored, deliberately unrun.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/blocker_count_restrictions.json.fixture"))
        .unwrap()
}

fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
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
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition));
    }
    [direct, decoded]
}

#[test]
fn blocker_count_restrictions_keep_their_counts_and_recipients_on_both_routes() {
    let rows = fixtures();
    assert_eq!(rows.len(), 3);
    for row in &rows {
        let name = row["name"].as_str().unwrap();
        for definition in definitions(name, row["text"].as_str().unwrap()) {
            let debug = format!("{definition:?}");
            match name {
                "Hexmark Destroyer" => {
                    assert!(debug.contains("CantBeBlockedExceptByNOrMore(6)"), "{name}");
                    assert!(!debug.contains("GrantStaticAbility"), "{name}: the source itself");
                }
                "Sonorous Howlbonder" => {
                    assert!(debug.contains("CantBeBlockedExceptByNOrMore(3)"), "{name}");
                    assert!(debug.contains("Menace"), "{name}: granted only to menace creatures");
                }
                "Rocksteady, Crash Courser" => {
                    assert!(debug.matches("CantBeBlockedByMoreThan(1)").count() >= 2, "{name}");
                    assert!(debug.contains("Boar"), "{name}: Boars you control");
                }
                other => panic!("unexpected {other}"),
            }
        }
    }
}
