//! "Flashback—{3}{R}, Remove X loyalty counters from among planeswalkers you
//! control. If you cast this spell this way, X can't be 0." — an X minimum
//! that binds only the flashback method (CR 107.3, CR 702.34a).
//! Source-authored, unrun.
use ironsmith::alternative_cast::AlternativeCastingMethod;
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const LIGHT_UP_THE_NIGHT: &str = "Mana cost: {X}{R}\nType: Sorcery\nLight Up the Night deals X damage to any target. It deals X plus 1 damage instead if that target is a creature or planeswalker.\nFlashback—{3}{R}, Remove X loyalty counters from among planeswalkers you control. If you cast this spell this way, X can't be 0. (You may cast this card from your graveyard for its flashback cost. Then exile it.)";

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
fn light_up_the_night_flashback_requires_nonzero_x() {
    for definition in definitions("Light Up the Night", LIGHT_UP_THE_NIGHT) {
        let flashback = definition
            .alternative_casts
            .iter()
            .find_map(|method| match method {
                AlternativeCastingMethod::Flashback { total_cost, x_minimum } => {
                    Some((total_cost.clone(), *x_minimum))
                }
                _ => None,
            })
            .expect("flashback");
        assert_eq!(flashback.1, 1, "X can't be 0 when cast this way");
        let debug = format!("{:?}", flashback.0);
        assert!(debug.contains("Loyalty"), "remove-loyalty cost: {debug}");
        assert!(
            !definition.abilities.iter().any(|ability| format!("{ability:?}").contains("ThisSpellXMinimum")),
            "the minimum must not apply to the ordinary cast"
        );
    }
}
