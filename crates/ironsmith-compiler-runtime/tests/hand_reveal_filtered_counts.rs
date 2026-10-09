//! "<source> deals 3 damage to that player for each card of the chosen type /
//! with the chosen name revealed this way" (Blood Oath, Thought Hemorrhage):
//! the count reads the exact local hand reveal's revealed set, filtered.
//! Source-authored, deliberately unrun.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;

fn definitions(name: &str) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/hand_reveal_filtered_counts.json.fixture"
    ))
    .unwrap();
    let row = rows.into_iter().find(|row| row["name"] == name).unwrap();
    let text = row["text"].as_str().unwrap();
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
fn damage_counts_the_filtered_cards_of_the_revealed_hand() {
    for name in ["Blood Oath", "Thought Hemorrhage"] {
        for definition in definitions(name) {
            let debug = format!("{:?}", definition.spell_effect);
            assert!(debug.contains("LookAtHandEffect"), "{name}: {debug}");
            assert!(debug.contains("PriorEffectMetric"), "{name}: {debug}");
            assert!(debug.contains("Revealed"), "{name}: {debug}");
            assert!(debug.contains("DealDamageEffect"), "{name}: {debug}");
        }
    }
}
