//! "Target creature an opponent controls attacks during its controller's
//! next combat phase if able" (CR 508.1d). Source-authored, unrun.
use ironsmith::cards::CardDefinition;
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::effects::{EffectContext, MustAttackPlayerThisTurnEffect, execute_effect};
use ironsmith::effect::Effect;
use ironsmith::target::{ChooseSpec, PlayerFilter};
use ironsmith::{GameState, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
const TRENCH: &str = "Mana cost: {5}{U}{U}\nType: Creature — Salamander Horror\nPower/Toughness: 7/7\nReturn a land you control to its owner's hand: Untap this creature. It gains hexproof until end of turn.\nLandfall — Whenever a land you control enters, target creature an opponent controls attacks during its controller's next combat phase if able.";

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
fn trench_behemoth_requirement_waits_for_the_controllers_combat() {
    for definition in definitions("Trench Behemoth", TRENCH) {
        let debug = format!("{definition:?}");
        assert!(debug.contains("controllers_next_combat: true"), "{debug}");
    }
}

#[test]
fn requirement_applies_on_the_controllers_turn_and_is_spent_after_combat() {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    game.turn.active_player = A;
    let bear = compile_to_runtime_definition("Bear", "Type: Creature — Bear\nPower/Toughness: 2/2", false).unwrap();
    let source = game.create_object_from_definition(&bear, A, Zone::Battlefield);
    let victim = game.create_object_from_definition(&bear, B, Zone::Battlefield);
    let mut dm = SelectFirstDecisionMaker;
    let effect = MustAttackPlayerThisTurnEffect::new(
        ChooseSpec::SpecificObject(victim),
        ChooseSpec::Player(PlayerFilter::Any),
    )
    .with_controllers_next_combat(true);
    execute_effect(&mut game, &Effect::new(effect), &mut EffectContext::new(source, A, &mut dm)).unwrap();
    // Not on A's turn.
    assert!(!game.has_next_combat_attack_requirement(victim));
    game.cleanup_effects_end_of_combat();
    // B's turn: the requirement applies until B's combat ends.
    game.turn.active_player = B;
    assert!(game.has_next_combat_attack_requirement(victim));
    game.cleanup_effects_end_of_combat();
    assert!(!game.has_next_combat_attack_requirement(victim));
}
