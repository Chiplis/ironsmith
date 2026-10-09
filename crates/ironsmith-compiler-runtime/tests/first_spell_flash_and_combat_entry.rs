//! "You may cast the first creature spell you cast each turn as though it had
//! flash." (CR 702.8a timing for the first matching spell) and "Whenever a
//! nontoken creature you control enters during combat" (an entering trigger
//! restricted to the combat phase, CR 506). Source-authored, unrun.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;
const THE_BLUE_SPIRIT: &str = "Mana cost: {3}{U}
Type: Legendary Creature — Human Rogue Ally
Power/Toughness: 2/4
You may cast the first creature spell you cast each turn as though it had flash.
Whenever a nontoken creature you control enters during combat, draw a card.";

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
fn the_blue_spirit_flashes_its_first_creature_and_draws_on_combat_entries() {
    for definition in routes("The Blue Spirit", THE_BLUE_SPIRIT) {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let text = format!("{:?}", definition.abilities);
        assert!(text.contains("first_spell_cast_each_turn: true"), "{text}");
        assert!(text.contains("cast_by: Some(You)"), "{text}");
        assert!(text.contains("zone: Stack"), "flash timing from any origin: {text}");
        assert!(text.contains("DuringCombat"), "{text}");
        assert!(text.contains("nontoken: true"), "{text}");
    }
}
