//! "Whenever a creature you control attacking causes a triggered ability of
//! that creature to trigger, ... you may copy that ability" (CR 508.1m,
//! CR 707.10). Source-authored, unrun.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const FIREBENDER_ASCENSION: &str = "Mana cost: {1}{R}\nType: Enchantment\nWhen this enchantment enters, create a 2/2 red Soldier creature token with firebending 1.\nWhenever a creature you control attacking causes a triggered ability of that creature to trigger, put a quest counter on this enchantment. Then if it has four or more quest counters on it, you may copy that ability. You may choose new targets for the copy.";

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
fn firebender_ascension_watches_attack_caused_triggers_and_copies_them() {
    for definition in definitions("Firebender Ascension", FIREBENDER_ASCENSION) {
        let debug = format!("{definition:?}");
        assert!(debug.contains("caused_by_source_attacking: true"), "{debug}");
        assert!(debug.contains("Quest"), "{debug}");
        assert!(debug.contains("CopySpell"), "{debug}");
    }
}
