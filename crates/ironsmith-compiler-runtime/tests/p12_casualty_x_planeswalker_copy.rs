//! "Casualty X. The copy isn't legendary and has starting loyalty X."
//! (CR 702.153a, CR 707.10). Source-authored, unrun.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const OB_NIXILIS: &str = "Mana cost: {1}{B}{R}\nType: Legendary Planeswalker — Nixilis\nLoyalty: 3\nCasualty X. The copy isn't legendary and has starting loyalty X. (As you cast this spell, you may sacrifice a creature with power X. When you do, copy this spell. The copy becomes a token.)\n+1: Each opponent loses 2 life unless they discard a card. If you control a Demon or Devil, you gain 2 life.\n−2: Create a 1/1 red Devil creature token with \"When this token dies, it deals 1 damage to any target.\"\n−7: Target player draws seven cards and loses 7 life.";

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
fn ob_nixilis_casualty_copy_is_supported_and_rendered() {
    for definition in definitions("Ob Nixilis, the Adversary", OB_NIXILIS) {
        let debug = format!("{definition:?}");
        assert!(debug.contains("VariableCasualtyPlaneswalkerCopy"), "{debug}");
    }
}
