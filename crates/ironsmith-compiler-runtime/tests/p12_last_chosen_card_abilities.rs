//! "Pay 1 life: Choose a creature card exiled with Koh. Koh has all activated
//! and triggered abilities of the last chosen card." — a per-source
//! remembered choice read by a copy grant (CR 613.1f). Source-authored, unrun.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const KOH: &str = "Mana cost: {4}{B}{B}\nType: Legendary Creature — Shapeshifter Spirit\nPower/Toughness: 6/6\nWhen Koh enters, exile up to one other target creature.\nWhenever another nontoken creature dies, you may exile it.\nPay 1 life: Choose a creature card exiled with Koh.\nKoh has all activated and triggered abilities of the last chosen card.";

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
fn koh_remembers_its_choice_and_copies_that_cards_abilities() {
    for definition in definitions("Koh, the Face Stealer", KOH) {
        let debug = format!("{definition:?}");
        assert!(debug.contains("remember_as_chosen_object: true"), "{debug}");
        assert!(debug.contains("CopyActivatedAbilities"), "{debug}");
        assert!(debug.contains("CopyTriggeredAbilities"), "{debug}");
        assert!(debug.contains("__chosen_objects__") || debug.contains("ChosenObjects"), "{debug}");
    }
}
