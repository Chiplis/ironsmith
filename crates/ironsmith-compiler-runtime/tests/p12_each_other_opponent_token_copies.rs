//! "Whenever ~ attacks a player, for each other opponent, create a token
//! that's a copy of ~ tapped and attacking that player, except it isn't
//! legendary" (CR 508.4, CR 707.9b). Source-authored, unrun.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const SHREDDER: &str = "Mana cost: {3}{B}{B}\nType: Legendary Creature — Human Ninja\nPower/Toughness: 5/5\nWhenever Shredder attacks a player, for each other opponent, create a token that's a copy of Shredder tapped and attacking that player, except it isn't legendary. Sacrifice those tokens at end of combat.\nWhenever Shredder deals combat damage to a player, that player loses half their life, rounded up.";

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
fn shredder_copies_attack_each_other_opponent() {
    for definition in definitions("Shredder, Shadow Master", SHREDDER) {
        let debug = format!("{definition:?}");
        assert!(debug.contains("ForEachPlayers") || debug.contains("ForEachPlayer"), "{debug}");
        assert!(debug.contains("Defending"), "excludes the attacked player: {debug}");
        assert!(debug.contains("IteratedPlayer"), "attacks the iterated opponent: {debug}");
        assert!(debug.contains("Legendary"), "removes legendary: {debug}");
    }
}
