//! "with deathtouch, hexproof, reach, or trample": any one of a serial
//! keyword list. Source-authored, deliberately unrun.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/keyword_list_filters.json.fixture")).unwrap()
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
fn mwonvuli_search_filter_is_a_four_way_keyword_disjunction() {
    let rows = fixtures();
    let name = rows[0]["name"].as_str().unwrap();
    for definition in definitions(name, rows[0]["text"].as_str().unwrap()) {
        let debug = format!("{definition:?}");
        for keyword in ["Deathtouch", "Hexproof", "Reach", "Trample"] {
            assert!(debug.contains(keyword), "{keyword}: {debug}");
        }
        assert!(debug.contains("any_of: [ObjectFilter"), "a disjunction, not a conjunction");
    }
}

#[test]
fn three_keyword_with_list_parses_as_any_of() {
    let text = "Mana cost: {G}\nType: Sorcery\nSearch your library for a creature card with flying, reach, or trample, reveal it, put it into your hand, then shuffle.";
    assert!(ironsmith_compiler_runtime::compile_to_runtime_definition("List probe", text, false).is_ok());
}
