//! "For any number of opponents, destroy target nonland permanent that player
//! controls." = any number of targets controlled by different opponents
//! (CR 601.2c, 115.1). Source-authored, deliberately unrun.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;

const WINDGRACES_JUDGMENT: &str = "Mana cost: {3}{B}{G}\nType: Instant\nFor any number of opponents, destroy target nonland permanent that player controls.";

fn routes(name: &str, text: &str) -> [CardDefinition; 2] {
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
    [direct, decoded]
}

#[test]
fn windgraces_judgment_targets_one_permanent_per_chosen_opponent() {
    for definition in routes("Windgrace's Judgment", WINDGRACES_JUDGMENT) {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let text = format!("{:?}", definition.spell_effect);
        assert!(text.contains("Destroy"), "{text}");
        assert!(text.contains("target_set_different_controllers: true"), "{text}");
        assert!(text.contains("controller: Some(Opponent)"), "{text}");
        assert!(text.contains("excluded_card_types: [Land]"), "{text}");
    }
}
