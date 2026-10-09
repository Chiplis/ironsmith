//! "Each other planeswalker you control has the loyalty abilities of
//! Kasmina." (CR 613.1f): a grant copying this object's loyalty abilities.
//! Source-authored, unrun.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const KASMINA: &str = "Mana cost: {1}{G}{U}\nType: Legendary Planeswalker — Kasmina\nLoyalty: 2\nEach other planeswalker you control has the loyalty abilities of Kasmina.\n+2: Scry 1.\n−X: Create a 0/0 green and blue Fractal creature token. Put X +1/+1 counters on it.\n−8: Search your library for an instant or sorcery card that shares a color with this planeswalker, exile that card, then shuffle. You may cast that card without paying its mana cost.";

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
fn kasmina_grants_its_loyalty_abilities_to_other_planeswalkers() {
    for definition in definitions("Kasmina, Enigma Sage", KASMINA) {
        let debug = format!("{definition:?}");
        assert!(debug.contains("CopyActivatedAbilities"), "{debug}");
        assert!(debug.contains("only_loyalty: true"), "{debug}");
        assert!(debug.contains("Planeswalker"), "recipient filter: {debug}");
    }
}
