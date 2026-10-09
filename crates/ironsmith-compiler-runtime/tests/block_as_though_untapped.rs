//! "Tapped creatures you control can block as though they were untapped."
//! (Masako the Humorless, CR 509.1a): the permission is granted to each
//! matching creature and honored by every blocker-legality check.
//! Source-authored, deliberately unrun.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;

const MASAKO_THE_HUMORLESS: &str = "Mana cost: {2}{W}\nType: Legendary Creature — Human Advisor\nPower/Toughness: 2/1\nFlash\nTapped creatures you control can block as though they were untapped.";

fn routes(name: &str, text: &str) -> [CardDefinition; 2] {
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| {
        ironsmith_compiler_runtime::compile_to_runtime_definition(name, text, false)
    });
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let direct = direct.unwrap_or_else(|error| panic!("{name}: {error}"));
    let (compiled, loss) = ironsmith_compiler::parse_loss::capture(|| {
        ironsmith_compiler_runtime::compile_to_artifact(name, text, false)
    });
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (artifact, _) = compiled.unwrap_or_else(|error| panic!("{name}: {error}"));
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(artifact, restored);
    let decoded =
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
    [direct, decoded]
}

#[test]
fn masako_grants_tapped_creatures_the_untapped_blocking_permission() {
    for definition in routes("Masako the Humorless", MASAKO_THE_HUMORLESS) {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let text = format!("{:?}", definition.abilities);
        assert!(text.contains("CanBlockAsThoughUntapped"), "{text}");
        assert!(text.contains("tapped: true"), "{text}");
    }
}
