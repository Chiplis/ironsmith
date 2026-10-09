//! "create a tapped colorless land token named Everywhere that is every basic
//! land type" (CR 111.4, CR 305.6). Source-authored, unrun.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const OVERLORD: &str = "Mana cost: {3}{G}{G}\nType: Enchantment Creature — Avatar Horror\nPower/Toughness: 6/5\nImpending 4—{1}{G}{G} (If you cast this spell for its impending cost, it enters with four time counters and isn't a creature until the last is removed. At the beginning of your end step, remove a time counter from it.)\nWhenever this permanent enters or attacks, create a tapped colorless land token named Everywhere that is every basic land type.";

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
fn overlord_creates_a_tapped_everywhere_land() {
    for definition in definitions("Overlord of the Hauntwoods", OVERLORD) {
        let debug = format!("{definition:?}");
        assert!(debug.contains("Everywhere"), "{debug}");
        for subtype in ["Plains", "Island", "Swamp", "Mountain", "Forest"] {
            assert!(debug.contains(subtype), "{subtype}: {debug}");
        }
        assert!(debug.contains("Land"), "{debug}");
    }
}
