//! "switch its power and toughness" reads the trigger's own antecedent, like
//! "it". Source-authored, deliberately unrun.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/possessive_switch_pt.json.fixture")).unwrap()
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

fn triggered(definition: &CardDefinition) -> String {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(format!("{:?}", triggered.effects.all_effects())),
            _ => None,
        })
        .expect("attack trigger")
}

#[test]
fn valakut_fireboar_switches_its_own_power_and_toughness_without_a_target() {
    let rows = fixtures();
    assert_eq!(rows.len(), 1);
    let name = rows[0]["name"].as_str().unwrap();
    for definition in definitions(name, rows[0]["text"].as_str().unwrap()) {
        let effects = triggered(&definition);
        assert!(effects.contains("Switch"), "{effects}");
        assert!(!effects.contains("Target(Object"), "no declared target: {effects}");
        assert!(effects.contains("EndOfTurn"), "{effects}");
    }
}
