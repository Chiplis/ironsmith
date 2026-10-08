//! "Put the rest of the cards / the rest of those cards on the bottom" is the
//! same looked-at remainder as "the rest". Source-authored, deliberately unrun.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/rest_of_those_cards_remainder.json.fixture"
    ))
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

fn triggered_effects(definition: &CardDefinition) -> Vec<String> {
    definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(format!("{:?}", triggered.effects.all_effects())),
            _ => None,
        })
        .collect()
}

#[test]
fn rest_of_those_cards_lowers_exactly_like_the_rest_on_both_routes() {
    let rows = fixtures();
    assert_eq!(rows.len(), 3);
    for row in &rows {
        let name = row["name"].as_str().unwrap();
        let text = row["text"].as_str().unwrap();
        let shorthand = text
            .replace("the rest of those cards", "the rest")
            .replace("the rest of the cards", "the rest");
        assert_ne!(shorthand, text, "{name}: fixture must use the long remainder");
        // Compare each route with the same route of the shorthand body.
        for (expected, definition) in definitions(name, &shorthand)
            .iter()
            .zip(definitions(name, text).iter())
        {
            let expected = triggered_effects(expected);
            assert_eq!(expected.len(), 1, "{name}");
            assert_eq!(triggered_effects(definition), expected, "{name}");
        }
    }
}
