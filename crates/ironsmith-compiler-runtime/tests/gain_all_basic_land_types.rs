//! "Lands you control gain all basic land types until end of turn."
//! (Energybending): the five basic land types are added to each land
//! (CR 205.1b) until end of turn, which also gives each land the basic
//! types' intrinsic mana abilities (CR 305.6). Source-authored, unrun.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;
const ENERGYBENDING: &str = "Mana cost: {2}
Type: Instant — Lesson
Lands you control gain all basic land types until end of turn.
Draw a card.";

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
fn energybending_adds_every_basic_land_type_to_your_lands_until_end_of_turn() {
    for definition in routes("Energybending", ENERGYBENDING) {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let text = format!("{:?}", definition.spell_effect);
        assert!(text.contains("AddSubtypes"), "{text}");
        for land_type in ["Plains", "Island", "Swamp", "Mountain", "Forest"] {
            assert!(text.contains(land_type), "{land_type}: {text}");
        }
        assert!(text.contains("EndOfTurn"), "{text}");
        assert!(text.contains("DrawCardsEffect") || text.contains("Draw"), "{text}");
    }
}
