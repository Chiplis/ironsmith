//! where-X values from this turn's history: opponents being attacked
//! (CR 506.2) and damage dealt to you this turn by artifacts (CR 120).
//! Source-authored, unrun.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const DIMIR_STRANDCATCHER: &str = "Mana cost: {2}{U/B}{U/B}\nType: Creature — Faerie Rogue\nPower/Toughness: 3/3\nFlying\nWhenever you attack, surveil X, where X is the number of opponents being attacked.\nAt the beginning of each end step, if three or more cards were put into your graveyard from anywhere other than the battlefield this turn, draw a card.";
const REVERSE_POLARITY: &str = "Mana cost: {W}{W}\nType: Instant\nYou gain X life, where X is twice the damage dealt to you so far this turn by artifacts.";

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
fn strandcatcher_surveils_per_attacked_opponent() {
    for definition in definitions("Dimir Strandcatcher", DIMIR_STRANDCATCHER) {
        let debug = format!("{definition:?}");
        assert!(debug.contains("PlayersBeingAttacked"), "{debug}");
    }
}

#[test]
fn reverse_polarity_counts_artifact_damage_to_you_twice() {
    for definition in definitions("Reverse Polarity", REVERSE_POLARITY) {
        let debug = format!("{definition:?}");
        assert!(debug.contains("DamageHistory"), "{debug}");
        assert!(debug.contains("Players(You)"), "{debug}");
        assert!(debug.contains("Artifact"), "{debug}");
    }
}
