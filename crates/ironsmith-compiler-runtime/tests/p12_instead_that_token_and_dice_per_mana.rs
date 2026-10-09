//! "If you rolled 6 or higher, instead create that token and a Treasure
//! token" (CR 614.1a) and "Roll a six-sided die plus an additional six-sided
//! die for each mana from Treasures spent to activate this ability"
//! (CR 706.1). Source-authored, unrun.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const MR_HOUSE: &str = "Mana cost: {2}{R}{W}\nType: Legendary Artifact Creature — Human Robot\nPower/Toughness: 4/4\nWhenever you roll a 4 or higher, create a 3/3 colorless Robot artifact creature token. If you rolled 6 or higher, instead create that token and a Treasure token.\n{4}, {T}: Roll a six-sided die plus an additional six-sided die for each mana from Treasures spent to activate this ability.";

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
fn mr_house_upgrades_the_robot_and_rolls_extra_dice() {
    for definition in definitions("Mr. House, President and CEO", MR_HOUSE) {
        let debug = format!("{definition:?}");
        assert!(debug.contains("Treasure"), "{debug}");
        assert!(debug.contains("Robot"), "{debug}");
        assert!(debug.contains("ManaFromSourceSpentToCastThisSpell"), "dice per Treasure mana: {debug}");
        assert!(debug.contains("ThisAbility"), "{debug}");
    }
}
