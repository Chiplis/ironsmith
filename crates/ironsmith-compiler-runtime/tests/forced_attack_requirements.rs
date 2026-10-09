//! Source-authored and deliberately unrun (cf8 p04): "attacks this turn if
//! able" over a targeted player's creatures and over the enchanted creature,
//! and "during target player's next turn, each creature that player controls
//! attacks if able" (CR 508.1d attack requirements).
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::game_loop::{
    extract_target_requirements_from_program_with_modes, resolve_stack_entry_with,
};
use ironsmith::game_state::{StackEntry, TargetAssignment};
use ironsmith::object::AttachmentTarget;
use ironsmith::resolution::ResolutionProgram;
use ironsmith::rules::combat::must_attack_with_game;
use ironsmith::{GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);

fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/forced_attack_requirements.json.fixture"
    ))
    .unwrap()
}

fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = rows()
        .into_iter()
        .find(|row| row["name"] == name)
        .unwrap_or_else(|| panic!("missing fixture row {name}"));
    let text = row["text"].as_str().unwrap();
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| {
        compile_to_runtime_definition(name, text, false)
    });
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let direct = direct.unwrap_or_else(|error| panic!("{name}: {error}"));
    let (compiled, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (artifact, _) = compiled.unwrap_or_else(|error| panic!("{name}: {error}"));
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    let decoded = materialize_artifact(&restored).unwrap();
    for definition in [&direct, &decoded] {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition));
    }
    [direct, decoded]
}

fn bear(game: &mut GameState, owner: PlayerId) -> ObjectId {
    let definition = compile_to_runtime_definition(
        "Bear",
        "Mana cost: {1}{G}\nType: Creature — Bear\nPower/Toughness: 2/2",
        false,
    )
    .unwrap();
    game.create_object_from_definition(&definition, owner, Zone::Battlefield)
}

fn activated_program(definition: &CardDefinition, index: usize) -> ResolutionProgram {
    definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Activated(ability) => Some(ability.effects.clone()),
            _ => None,
        })
        .nth(index)
        .expect("activated ability")
}

fn resolve_ability(
    game: &mut GameState,
    source: ObjectId,
    program: ResolutionProgram,
    targets: Vec<Target>,
) {
    let requirements = extract_target_requirements_from_program_with_modes(
        game,
        &program,
        A,
        Some(source),
        None,
    );
    assert_eq!(requirements.len(), targets.len());
    let assignments = requirements
        .iter()
        .enumerate()
        .map(|(index, requirement)| {
            assert!(requirement.legal_targets.contains(&targets[index]));
            TargetAssignment {
                spec: requirement.spec.clone(),
                range: index..index + 1,
            }
        })
        .collect();
    game.push_to_stack(
        StackEntry::ability(source, A, program)
            .with_targets(targets)
            .with_target_assignments(assignments),
    );
    resolve_stack_entry_with(game, &mut SelectFirstDecisionMaker).unwrap();
}

#[test]
fn frozen_bodies_compile_strictly_on_both_routes() {
    let expected = [
        ("Incite War", "8a640369-59ab-4506-b473-1f804721414c"),
        ("Instigator", "0fe1a1f7-41c9-415e-a5e3-b5ba0c0271e5"),
        ("Nettling Curse", "93d575b3-0fff-4fc6-85c6-3a8a9d672d6f"),
        ("Rowan Kenrith", "e191976c-6ab1-4a23-aeb8-8a88bd4bd203"),
    ];
    let rows = rows();
    assert_eq!(rows.len(), expected.len());
    for (name, oracle_id) in expected {
        assert!(rows.iter().any(|row| row["name"] == name && row["oracle_id"] == oracle_id), "{name}");
        definitions(name);
    }
}

#[test]
fn instigator_forces_only_the_target_players_creatures_this_turn() {
    for definition in definitions("Instigator") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let theirs = bear(&mut game, B);
        let mine = bear(&mut game, A);
        let program = activated_program(&definition, 0);
        let debug = format!("{program:?}");
        assert!(debug.contains("MustAttack"), "{debug}");
        resolve_ability(&mut game, source, program, vec![Target::Player(B)]);
        assert!(must_attack_with_game(game.object(theirs).unwrap(), &game));
        assert!(!must_attack_with_game(game.object(mine).unwrap(), &game));
        game.next_turn();
        assert!(
            !must_attack_with_game(game.object(theirs).unwrap(), &game),
            "this turn only"
        );
    }
}

#[test]
fn incite_war_first_mode_declares_the_player_target() {
    for definition in definitions("Incite War") {
        let debug = format!("{:?}", definition.spell_effect);
        assert!(debug.contains("ChooseModeEffect"), "{debug}");
        assert!(debug.contains("MustAttack"), "{debug}");
        assert!(debug.contains("TargetOnlyEffect"), "{debug}");
        assert_eq!(definition.optional_costs.len(), 1, "entwine");
    }
}

#[test]
fn nettling_curse_forces_the_enchanted_creature_only() {
    for definition in definitions("Nettling Curse") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let host = bear(&mut game, B);
        let other = bear(&mut game, B);
        let aura = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        assert!(game.attach_object_to_target(aura, AttachmentTarget::Object(host)));
        resolve_ability(&mut game, aura, activated_program(&definition, 0), vec![]);
        assert!(must_attack_with_game(game.object(host).unwrap(), &game));
        assert!(!must_attack_with_game(game.object(other).unwrap(), &game));
    }
}

#[test]
fn rowan_requirement_waits_for_the_target_players_next_turn() {
    for definition in definitions("Rowan Kenrith") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let theirs = bear(&mut game, B);
        let mine = bear(&mut game, A);
        // The +2 is the first activated (loyalty) ability.
        resolve_ability(
            &mut game,
            source,
            activated_program(&definition, 0),
            vec![Target::Player(B)],
        );
        assert!(
            !must_attack_with_game(game.object(theirs).unwrap(), &game),
            "not during the current turn"
        );
        game.next_turn();
        assert_eq!(game.turn.active_player, B);
        let later = bear(&mut game, B);
        game.update_cant_effects();
        assert!(must_attack_with_game(game.object(theirs).unwrap(), &game));
        assert!(
            must_attack_with_game(game.object(later).unwrap(), &game),
            "the controlled set stays live during that turn"
        );
        assert!(!must_attack_with_game(game.object(mine).unwrap(), &game));
        game.next_turn();
        assert!(!must_attack_with_game(game.object(theirs).unwrap(), &game));
    }
}
