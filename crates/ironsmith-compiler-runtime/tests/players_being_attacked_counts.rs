//! "for each player being attacked" counts the players attacked directly in
//! the current combat. Source-authored, deliberately unrun.
use ironsmith::ability::AbilityKind;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::combat_state::{AttackTarget, AttackerInfo, CombatState};
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::{CardId, CardType, GameState, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
const C: PlayerId = PlayerId::from_index(2);

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/players_being_attacked_counts.json.fixture"
    ))
    .unwrap()
}

fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
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

#[test]
fn both_cards_count_players_being_attacked_on_both_routes() {
    let rows = fixtures();
    assert_eq!(rows.len(), 2);
    for row in &rows {
        let name = row["name"].as_str().unwrap();
        for definition in definitions(name, row["text"].as_str().unwrap()) {
            let debug = format!("{definition:?}");
            assert!(debug.contains("PlayersBeingAttacked"), "{name}");
            assert!(!debug.contains("CountPlayers("), "{name}: not every player");
        }
    }
}

#[test]
fn apothecary_white_creates_one_food_per_directly_attacked_player() {
    let row = fixtures()
        .into_iter()
        .find(|row| row["name"] == "Apothecary White")
        .unwrap();
    for definition in definitions("Apothecary White", row["text"].as_str().unwrap()) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let attacker = CardBuilder::new(CardId::new(), "Attacker")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        let first = game.create_object_from_card(&attacker, A, Zone::Battlefield);
        let second = game.create_object_from_card(&attacker, A, Zone::Battlefield);
        let third = game.create_object_from_card(&attacker, A, Zone::Battlefield);
        game.combat = Some(CombatState {
            attackers: vec![
                AttackerInfo { creature: first, target: AttackTarget::Player(B) },
                AttackerInfo { creature: second, target: AttackTarget::Player(B) },
                AttackerInfo { creature: third, target: AttackTarget::Player(C) },
            ],
            ..Default::default()
        });
        let triggered = definition
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Triggered(triggered) => Some(triggered),
                _ => None,
            })
            .unwrap();
        let before = game.battlefield.len();
        let mut dm = SelectFirstDecisionMaker;
        for effect in triggered.effects.flattened_default_effects() {
            execute_effect(&mut game, effect, &mut EffectContext::new(source, A, &mut dm)).unwrap();
        }
        // Bob and Cara are being attacked: two Foods, not three (attackers)
        // and not one per opponent in the game.
        assert_eq!(game.battlefield.len(), before + 2);
    }
}
