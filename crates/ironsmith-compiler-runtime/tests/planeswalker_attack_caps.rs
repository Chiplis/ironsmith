//! "No more than one creature can attack <this planeswalker> each combat."
//! (The Eternal Wanderer; granted by Tomik, Orzhov Lawmage): a cap on the
//! attackers declared against that planeswalker (CR 508.1c).
//! Source-authored, deliberately unrun.
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;

const THE_ETERNAL_WANDERER: &str = "Mana cost: {4}{W}{W}\nType: Legendary Planeswalker\nLoyalty: 5\nNo more than one creature can attack The Eternal Wanderer each combat.\n+1: Exile up to one target artifact or creature. Return that card to the battlefield under its owner's control at the beginning of that player's next end step.\n0: Create a 2/2 white Samurai creature token with double strike.\n−4: For each player, choose a creature that player controls. Each player sacrifices all creatures they control not chosen this way.";
const TOMIK_ORZHOV_LAWMAGE: &str = "Mana cost: {1}{W}\nType: Legendary Creature — Human Advisor\nPower/Toughness: 2/1\nFlying\nPlaneswalkers you control have \"No more than one creature can attack this planeswalker each combat.\"\n{T}: Target creature with a +1/+1 counter on it gains flying until end of turn.";

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
fn planeswalker_attack_caps_compile_on_the_planeswalker_and_through_grants() {
    for (name, text) in [
        ("The Eternal Wanderer", THE_ETERNAL_WANDERER),
        ("Tomik, Orzhov Lawmage", TOMIK_ORZHOV_LAWMAGE),
    ] {
        for definition in routes(name, text) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
            let text = format!("{:?}", definition.abilities);
            assert!(
                text.contains("MaxCreaturesCanAttackSourceEachCombat"),
                "{name}: {text}"
            );
        }
    }
}
