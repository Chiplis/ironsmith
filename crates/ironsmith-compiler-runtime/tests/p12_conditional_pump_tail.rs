//! "gets -2/-2 until end of turn if that opponent controls no other
//! creatures" — a resolution condition on the pump (CR 608.2c).
//! Source-authored, unrun.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const SKULKING_KILLER: &str = "Mana cost: {3}{B}\nType: Creature — Vampire Assassin\nPower/Toughness: 4/2\nWhen this creature enters, target creature an opponent controls gets -2/-2 until end of turn if that opponent controls no other creatures.";

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
fn skulking_killer_shrinks_only_a_lone_creature() {
    for definition in definitions("Skulking Killer", SKULKING_KILLER) {
        let debug = format!("{definition:?}");
        assert!(debug.contains("Fixed(-2)"), "{debug}");
        assert!(debug.contains("Conditional") || debug.contains("condition: Some"), "{debug}");
    }
}
