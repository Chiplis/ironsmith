//! "creatures without flying or islandwalk" lack both keywords.
//! Source-authored, deliberately unrun.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/without_either_keyword.json.fixture")).unwrap()
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
fn stormtide_leviathan_restricts_creatures_lacking_both_keywords() {
    let rows = fixtures();
    let name = rows[0]["name"].as_str().unwrap();
    for definition in definitions(name, rows[0]["text"].as_str().unwrap()) {
        let debug = format!("{definition:?}");
        let attack = debug.find("Attack(").expect("attack restriction");
        let tail = &debug[attack..];
        let excluded = tail.find("excluded_static_abilities").expect("excluded keywords");
        let list = &tail[excluded..tail[excluded..].find(']').map_or(tail.len(), |end| excluded + end)];
        assert!(list.contains("Flying"), "{list}");
        // Islandwalk is a keyword marker; it must be excluded too.
        assert!(tail.contains("islandwalk"), "{tail}");
        assert!(!tail[..excluded].contains("any_of: [ObjectFilter"), "not a disjunction");
    }
}
