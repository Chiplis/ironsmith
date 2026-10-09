//! "Target player chooses a card name, then reveals the top card of their
//! library. If that card has the chosen name, ..." (Petra Sphinx, Vexing
//! Arcanix): the declared target player names the card. Source-authored,
//! deliberately unrun.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::effect::Effect;
use ironsmith::effects::ChooseCardNameEffect;
use ironsmith_compiled_artifact::CompiledCardArtifact;

fn definitions(name: &str) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/target_player_name_choice.json.fixture"
    ))
    .unwrap();
    let row = rows.into_iter().find(|row| row["name"] == name).unwrap();
    let text = row["text"].as_str().unwrap();
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
    for definition in [&direct, &decoded] {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition));
    }
    [direct, decoded]
}

fn collect(effect: &Effect, all: &mut Vec<Effect>) {
    all.push(effect.clone());
    effect.visit_child_effects(&mut |child| collect(child, all));
}

#[test]
fn the_target_player_names_the_card_and_moves_the_revealed_top_card() {
    for name in ["Petra Sphinx", "Vexing Arcanix"] {
        for definition in definitions(name) {
            let AbilityKind::Activated(activated) = &definition.abilities[0].kind else {
                panic!("{name}: activated ability");
            };
            let mut all = Vec::new();
            for effect in activated.effects.all_effects() {
                collect(effect, &mut all);
            }
            let choose = all
                .iter()
                .find_map(|effect| effect.downcast_ref::<ChooseCardNameEffect>())
                .unwrap_or_else(|| panic!("{name}: name choice"));
            assert!(
                matches!(choose.chooser, ironsmith::target::PlayerFilter::Target(_)),
                "{name}: {:?}",
                choose.chooser
            );
            let debug = format!("{all:?}");
            assert!(debug.contains("Hand"), "{name}: {debug}");
            assert!(debug.contains("Graveyard"), "{name}: {debug}");
        }
    }
}
