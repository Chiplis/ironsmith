//! "choose a nonlegendary creature that saddled it this turn and create a
//! tapped and attacking token that's a copy of it" — "tapped and attacking"
//! is one token entry, not two actions (CR 508.4, CR 707.2).
//! Source-authored, unrun; "Repeat this process once" relies on p11.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const CALAMITY: &str = "Mana cost: {4}{R}{R}\nType: Legendary Creature — Horse Mount\nPower/Toughness: 4/6\nHaste\nWhenever Calamity attacks while saddled, choose a nonlegendary creature that saddled it this turn and create a tapped and attacking token that's a copy of it. Sacrifice that token at the beginning of the next end step. Repeat this process once.\nSaddle 1";

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
fn calamity_creates_a_tapped_attacking_copy_of_a_saddler() {
    for definition in definitions("Calamity, Galloping Inferno", CALAMITY) {
        let debug = format!("{definition:?}");
        assert!(debug.contains("CreateTokenCopy"), "{debug}");
        assert!(debug.contains("tapped: true") || debug.contains("enters_tapped: true"), "{debug}");
        assert!(debug.contains("attacking"), "{debug}");
    }
}
