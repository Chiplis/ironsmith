//! "It deals 5 damage instead if that target is white and/or blue": the
//! replacement amount keys on the announced target (CR 614.1a, CR 115.1).
//! Source-authored, unrun.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const BARRAGE: &str = "Mana cost: {R}\nType: Instant\nThis spell can't be countered.\nLithomantic Barrage deals 1 damage to target creature or planeswalker. It deals 5 damage instead if that target is white and/or blue.";

fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let (direct, loss) = parse_loss::capture(|| compile_to_runtime_definition(name, text, false));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let direct = direct.unwrap_or_else(|error| panic!("{name}: {error}"));
    let (artifact, _) = compile_to_artifact(name, text, false).unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    let decoded =
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
    for definition in [&direct, &decoded] {
        assert!(
            !ironsmith::cards::generated_definition_has_unimplemented_content(definition),
            "{name}: unimplemented content"
        );
    }
    [direct, decoded]
}

#[test]
fn barrage_upgrades_damage_only_for_a_white_or_blue_target() {
    for definition in definitions("Lithomantic Barrage", BARRAGE) {
        let debug = format!("{definition:?}");
        assert!(debug.contains("TargetMatches"), "condition reads the target: {debug}");
        assert!(debug.contains("Fixed(5)"), "{debug}");
        assert!(debug.contains("Fixed(1)"), "{debug}");
        assert!(debug.contains("White") && debug.contains("Blue"), "{debug}");
    }
}
