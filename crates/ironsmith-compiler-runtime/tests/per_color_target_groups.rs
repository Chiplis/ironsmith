//! "For each color, return up to one target card of that color from your
//! graveyard to your hand." One independent "target" instance per color
//! (CR 115.3, 105.1). Source-authored, deliberately unrun.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;

const ALL_SUNS_DAWN: &str = "Mana cost: {4}{G}\nType: Sorcery\nFor each color, return up to one target card of that color from your graveyard to your hand. Exile All Suns' Dawn.";
const ROGUES_GALLERY: &str = "Mana cost: {2}{B}\nType: Sorcery\nFor each color, return up to one target creature card of that color from your graveyard to your hand.";

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
fn each_color_gets_its_own_up_to_one_graveyard_target() {
    for (name, text) in [("All Suns' Dawn", ALL_SUNS_DAWN), ("Rogues' Gallery", ROGUES_GALLERY)] {
        for definition in routes(name, text) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
            let effects = definition.spell_effect.as_ref().unwrap().all_effects_owned();
            let returns: Vec<String> = effects
                .iter()
                .map(|effect| format!("{effect:?}"))
                .filter(|text| text.contains("Hand") && text.contains("Graveyard"))
                .collect();
            assert_eq!(returns.len(), 5, "{name}: one return per color: {returns:#?}");
            for color in ["WHITE", "BLUE", "BLACK", "RED", "GREEN"] {
                assert!(
                    returns.iter().any(|text| text.to_ascii_uppercase().contains(color)),
                    "{name}: {color}"
                );
            }
            if name == "Rogues' Gallery" {
                assert!(returns.iter().all(|text| text.contains("Creature")), "{returns:#?}");
            } else {
                let text = format!("{effects:?}");
                assert!(text.contains("Exile"), "self-exile retained: {text}");
            }
        }
    }
}
