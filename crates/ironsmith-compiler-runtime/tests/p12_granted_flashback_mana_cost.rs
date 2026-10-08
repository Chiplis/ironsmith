//! Static grant "... has flashback. The flashback cost is equal to that
//! card's mana cost." (CR 702.34a). Source-authored, unrun.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const LIER: &str = "Mana cost: {3}{U}{U}\nType: Legendary Creature — Human Wizard\nPower/Toughness: 3/4\nSpells can't be countered.\nEach instant and sorcery card in your graveyard has flashback. The flashback cost is equal to that card's mana cost.";
const IROH: &str = "Mana cost: {3}{G}{U}{R}\nType: Legendary Creature — Human Noble Ally\nPower/Toughness: 5/5\nFirebending 2\nDuring your turn, each non-Lesson instant and sorcery card in your graveyard has flashback. The flashback cost is equal to that card's mana cost.\nDuring your turn, each Lesson card in your graveyard has flashback {1}.";

fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let (direct, loss) = parse_loss::capture(|| compile_to_runtime_definition(name, text, false));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let direct = direct.unwrap_or_else(|error| panic!("{name}: {error}"));
    let (artifact, _) = compile_to_artifact(name, text, false).unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}

fn statics(definition: &CardDefinition) -> Vec<String> {
    definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(ability) => Some(format!("{ability:?}")),
            _ => None,
        })
        .collect()
}

#[test]
fn lier_and_iroh_grant_mana_cost_flashback() {
    for (name, text) in [("Lier, Disciple of the Drowned", LIER), ("Iroh, Grand Lotus", IROH)] {
        for definition in definitions(name, text) {
            let statics = statics(&definition);
            assert!(
                statics.iter().any(|ability| ability.contains("FlashbackFromCardManaCost")),
                "{name}: {statics:?}"
            );
            if name.starts_with("Iroh") {
                assert!(statics.iter().any(|ability| ability.contains("Lesson")), "{statics:?}");
            }
        }
    }
}
