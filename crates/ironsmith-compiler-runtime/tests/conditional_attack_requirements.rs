//! Source-authored and deliberately unrun (cf8 p04): "If <creature> attacks,
//! <creatures> attack if able" (CR 508.1d). The requirement exists only for a
//! declaration in which a matching creature attacks: Viashino Bey need not
//! attack, but once it does every other creature you control must attack if
//! able. Unconditional requirements keep their per-attacker scoring.
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::combat_state::{AttackTarget, CombatState};
use ironsmith::decision::AttackerDeclaration;
use ironsmith::game_state::{Phase, Step};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardId, CardType, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);

fn definitions(index: usize) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/conditional_attack_requirements.json.fixture"
    ))
    .unwrap();
    let row = &rows[index];
    let name = row["name"].as_str().unwrap();
    let text = row["text"].as_str().unwrap();
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| {
        compile_to_runtime_definition(name, text, false)
    });
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (compiled, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, _) = compiled.unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [direct.unwrap(), materialize_artifact(&restored).unwrap()]
}

fn creature(game: &mut GameState, owner: PlayerId, name: &str) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let id = game.create_object_from_card(&card, owner, Zone::Battlefield);
    game.remove_summoning_sickness(id);
    id
}

fn declares(game: &GameState, attackers: &[ObjectId]) -> bool {
    let mut copy = game.clone();
    copy.turn.phase = Phase::Combat;
    copy.turn.step = Some(Step::DeclareAttackers);
    let declarations = attackers
        .iter()
        .map(|creature| AttackerDeclaration {
            creature: *creature,
            target: AttackTarget::Player(B),
        })
        .collect::<Vec<_>>();
    ironsmith::game_loop::apply_attacker_declarations(
        &mut copy,
        &mut CombatState::default(),
        &mut TriggerQueue::new(),
        &declarations,
    )
    .is_ok()
}

#[test]
fn viashino_bey_obliges_the_rest_only_when_it_attacks() {
    for definition in definitions(0) {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        assert!(format!("{definition:?}").contains("ConditionalAttackRequirement"));
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let bey = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.remove_summoning_sickness(bey);
        let other = creature(&mut game, A, "Bear");
        game.refresh_continuous_state().unwrap();
        // No attack at all, or the other creature alone: no requirement exists.
        assert!(declares(&game, &[]));
        assert!(declares(&game, &[other]));
        // Bey attacks: the other creature must attack too.
        assert!(!declares(&game, &[bey]));
        assert!(declares(&game, &[bey, other]));
    }
}

#[test]
fn wars_toll_obliges_the_attacking_opponents_creatures() {
    for definition in definitions(1) {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        // War's Toll is controlled by the defending player (B); A attacks.
        game.create_object_from_definition(&definition, B, Zone::Battlefield);
        let first = creature(&mut game, A, "First");
        let second = creature(&mut game, A, "Second");
        game.refresh_continuous_state().unwrap();
        assert!(declares(&game, &[]));
        assert!(!declares(&game, &[first]));
        assert!(declares(&game, &[first, second]));
    }
}

#[test]
fn magnetic_web_compiles_a_counter_conditioned_requirement() {
    for definition in definitions(2) {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
        let debug = format!("{definition:?}");
        assert!(debug.contains("ConditionalAttackRequirement"), "{debug}");
        assert!(debug.contains("Magnet"), "{debug}");
    }
}
