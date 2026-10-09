//! "Choose one ... Create a [colors] creature token with those
//! characteristics." with characteristic bullets (CR 111.4, CR 700.2).
//! Source-authored, unrun.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const OUTLAWS_MERRIMENT: &str = "Mana cost: {1}{R}{W}{W}\nType: Enchantment\nAt the beginning of your upkeep, choose one at random. Create a red and white creature token with those characteristics.\n• 3/1 Human Warrior with trample and haste.\n• 2/1 Human Cleric with lifelink and haste.\n• 1/2 Human Rogue with haste and \"When this creature enters, it deals 1 damage to any target.\"";
const WILD_SHAPE: &str = "Mana cost: {G}\nType: Instant\nChoose one. Until end of turn, target creature you control has that base power and toughness, becomes that creature type, and gains that ability.\n• 1/3 Turtle with hexproof.\n• 1/5 Spider with reach.\n• 3/3 Elephant with trample.";
const GENKU: &str = "Mana cost: {3}{W}{U}\nType: Legendary Creature — Human Wizard\nPower/Toughness: 3/4\nWhenever another nontoken permanent you control leaves the battlefield, choose one that hasn't been chosen this turn. Create a creature token with those characteristics.\n• 2/2 white Fox with vigilance.\n• 1/2 blue Moonfolk with flying.\n• 1/1 black Rat with lifelink.\n{3}{W}{U}: Put a +1/+1 counter on each creature you control.";

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
fn outlaws_merriment_creates_one_random_red_and_white_token() {
    for definition in definitions("Outlaws' Merriment", OUTLAWS_MERRIMENT) {
        let debug = format!("{definition:?}");
        assert!(debug.contains("Warrior") && debug.contains("Cleric") && debug.contains("Rogue"), "{debug}");
        assert!(debug.contains("random: true") || debug.contains("Random"), "{debug}");
    }
}

#[test]
fn genku_offers_each_token_once_per_turn() {
    for definition in definitions("Genku, Future Shaper", GENKU) {
        let debug = format!("{definition:?}");
        assert!(debug.contains("Fox") && debug.contains("Moonfolk") && debug.contains("Rat"), "{debug}");
    }
}

#[test]
fn wild_shape_modes_set_body_type_and_keyword_until_end_of_turn() {
    for definition in definitions("Wild Shape", WILD_SHAPE) {
        let debug = format!("{definition:?}");
        assert!(debug.contains("Turtle") && debug.contains("Spider") && debug.contains("Elephant"), "{debug}");
        assert!(debug.contains("Hexproof") && debug.contains("Reach") && debug.contains("Trample"), "{debug}");
        assert!(debug.contains("EndOfTurn"), "{debug}");
    }
}
