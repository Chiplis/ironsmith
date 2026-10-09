//! "This creature can't block alone." (CR 509.1b block requirement).
//! Source-authored, deliberately unrun.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/source_cant_block_alone.json.fixture")).unwrap()
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
fn craven_hulk_cant_block_alone_on_both_routes() {
    let rows = fixtures();
    let name = rows[0]["name"].as_str().unwrap();
    for definition in definitions(name, rows[0]["text"].as_str().unwrap()) {
        let debug = format!("{definition:?}");
        assert!(debug.contains("BlockAlone"), "{debug}");
        assert!(!debug.contains("BlockSpecificAttacker"), "not an attacker filter: {debug}");
        assert!(!debug.contains("AttackOrBlockAlone"), "blocking only");
    }
}

#[test]
fn source_block_alone_reaches_the_direct_fact() {
    for text in ["This creature can't block alone.", "This creature can't attack alone."] {
        let source = format!("Mana cost: {{3}}\nType: Creature — Giant\nPower/Toughness: 4/4\n{text}");
        assert!(ironsmith_compiler_runtime::compile_to_runtime_definition("Alone probe", &source, false).is_ok(), "{text}");
    }
}
