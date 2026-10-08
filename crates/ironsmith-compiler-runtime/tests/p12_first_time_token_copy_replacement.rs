//! "The first time you would create one or more tokens each turn, you may
//! instead create that many tokens that are copies of enchanted permanent."
//! (CR 614.1, CR 707.2). Source-authored, unrun.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const MOONLIT_MEDITATION: &str = "Mana cost: {2}{U}\nType: Enchantment — Aura\nEnchant artifact or creature you control\nThe first time you would create one or more tokens each turn, you may instead create that many tokens that are copies of enchanted permanent.";

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
fn moonlit_meditation_replaces_only_the_first_creation_with_copies() {
    for definition in definitions("Moonlit Meditation", MOONLIT_MEDITATION) {
        let debug = format!("{definition:?}");
        assert!(debug.contains("TokenCreationTemplates"), "{debug}");
        assert!(debug.contains("CreateTokenCopy"), "copy template: {debug}");
        assert!(debug.contains("TokensCreated"), "first-time gate: {debug}");
        assert!(debug.contains("optional: true"), "{debug}");
    }
}
