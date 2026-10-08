//! "Whenever an ability of equipped creature is activated, if it isn't a mana
//! ability, [you may pay {1}. If you do,] copy that ability" (CR 707.10).
//! Source-authored, unrun.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const CARDS: [(&str, &str); 2] = [
    ("Battlemage's Bracers", "Mana cost: {2}{R}\nType: Artifact — Equipment\nEquipped creature has haste.\nWhenever an ability of equipped creature is activated, if it isn't a mana ability, you may pay {1}. If you do, copy that ability. You may choose new targets for the copy.\nEquip {2}"),
    ("Illusionist's Bracers", "Mana cost: {2}\nType: Artifact — Equipment\nWhenever an ability of equipped creature is activated, if it isn't a mana ability, copy that ability. You may choose new targets for the copy.\nEquip {3}"),
];

fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let (direct, loss) = parse_loss::capture(|| compile_to_runtime_definition(name, text, false));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let direct = direct.unwrap_or_else(|error| panic!("{name}: {error}"));
    let (artifact, _) = compile_to_artifact(name, text, false).unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}

#[test]
fn bracers_copy_non_mana_abilities_of_the_equipped_creature() {
    for (name, text) in CARDS {
        for definition in definitions(name, text) {
            let trigger = definition
                .abilities
                .iter()
                .find_map(|ability| match &ability.kind {
                    AbilityKind::Triggered(triggered) => Some(format!("{triggered:?}")),
                    _ => None,
                })
                .expect("activation trigger");
            assert!(trigger.contains("\"equipped\""), "{trigger}");
            assert!(trigger.contains("non_mana_only: true"), "{trigger}");
            assert!(trigger.contains("Copy"), "{trigger}");
        }
    }
}
