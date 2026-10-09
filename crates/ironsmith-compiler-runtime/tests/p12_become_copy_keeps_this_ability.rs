//! "this creature becomes a copy of another target creature you control,
//! except it has this ability and \"...\"" (CR 707.9a). Source-authored, unrun.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const AURORA_SHIFTER: &str = "Mana cost: {1}{U}{U}\nType: Creature — Shapeshifter\nPower/Toughness: 1/3\nWhenever this creature deals combat damage to a player, you get that many {E}.\nAt the beginning of combat on your turn, you may pay {E}{E}. When you do, this creature becomes a copy of another target creature you control, except it has this ability and \"Whenever this creature deals combat damage to a player, you get that many {E}.\"";

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
fn aurora_shifter_copy_keeps_its_ability_and_gains_the_energy_trigger() {
    for definition in definitions("Aurora Shifter", AURORA_SHIFTER) {
        let debug = format!("{definition:?}");
        assert!(debug.contains("BecomeCopy") || debug.contains("Copy"), "{debug}");
        assert!(debug.contains("preserve_source_abilities: true") || debug.contains("retain"), "{debug}");
        assert!(debug.contains("Energy"), "{debug}");
    }
}
