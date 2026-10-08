//! Token-copy exceptions that grant keywords: "except they're 3/3 creatures
//! in addition to their other types and they have vigilance and menace"
//! (CR 707.9a). Source-authored, unrun. Rebuild the City also depends on p01
//! retiring its ChooseLeadingSpell fail-loud guard.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const REBUILD_THE_CITY: &str = "Mana cost: {3}{R}{G}{W}\nType: Sorcery\nChoose target land. Create three tokens that are copies of it, except they're 3/3 creatures in addition to their other types and they have vigilance and menace. (They're affected by summoning sickness.)";

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
fn rebuild_the_city_copies_gain_vigilance_and_menace() {
    for definition in definitions("Rebuild the City", REBUILD_THE_CITY) {
        let debug = format!("{definition:?}");
        assert!(debug.contains("CreateTokenCopy"), "{debug}");
        assert!(debug.contains("Vigilance"), "{debug}");
        assert!(debug.contains("Menace"), "{debug}");
        assert!(debug.contains("Creature"), "{debug}");
    }
}
