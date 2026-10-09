//! "you gain 3 life and that creature fights up to one target creature you
//! don't control" — a subject + "fights" clause is its own action
//! (CR 701.14a). Source-authored, unrun.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const TOLSIMIR: &str = "Mana cost: {2}{G}{G}{W}\nType: Legendary Creature — Elf Scout\nPower/Toughness: 3/3\nWhen Tolsimir enters, create Voja, Friend to Elves, a legendary 3/3 green and white Wolf creature token.\nWhenever a Wolf you control enters, you gain 3 life and that creature fights up to one target creature you don't control.";

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
fn tolsimir_gains_life_and_the_wolf_fights() {
    for definition in definitions("Tolsimir, Friend to Wolves", TOLSIMIR) {
        let debug = format!("{definition:?}");
        assert!(debug.contains("GainLife"), "{debug}");
        assert!(debug.contains("Fight"), "{debug}");
    }
}
