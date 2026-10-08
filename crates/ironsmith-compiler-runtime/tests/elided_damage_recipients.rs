//! Source-authored and deliberately unrun (cf8 p04): paired damage fanouts
//! whose later recipient part elides "deals" — "and 1 additional damage to
//! ...", "and 1 damage to you and each creature you control", "and 1 damage
//! to its controller", "3 damage to that player and 1 damage to each creature
//! they control", and a leading "instead" pair (CR 120.3, CR 614.1a).
use ironsmith::cards::CardDefinition;
use ironsmith::game_loop::{extract_target_requirements_from_program_with_modes, resolve_stack_entry};
use ironsmith::game_state::{StackEntry, TargetAssignment};
use ironsmith::{GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);

fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/elided_damage_recipients.json.fixture"
    ))
    .unwrap()
}

fn row(name: &str) -> serde_json::Value {
    rows()
        .into_iter()
        .find(|row| row["name"] == name)
        .unwrap_or_else(|| panic!("missing fixture row {name}"))
}

fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = row(name);
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
        assert_eq!(definition.card.name, name);
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition));
    }
    [direct, decoded]
}

fn creature(game: &mut GameState, owner: PlayerId, name: &str, text: &str) -> ObjectId {
    let definition = compile_to_runtime_definition(name, text, false).unwrap();
    game.create_object_from_definition(&definition, owner, Zone::Battlefield)
}

/// Resolve `definition` as a spell cast by Alice, one listed target per
/// target requirement in declaration order.
fn resolve(game: &mut GameState, definition: &CardDefinition, x: Option<u32>, targets: Vec<Target>) {
    let spell = game.create_object_from_definition(definition, A, Zone::Stack);
    let requirements = extract_target_requirements_from_program_with_modes(
        game,
        definition.spell_effect.as_ref().unwrap(),
        A,
        Some(spell),
        None,
    );
    assert_eq!(requirements.len(), targets.len(), "one target per requirement");
    let mut assignments = Vec::new();
    for (index, (requirement, target)) in requirements.iter().zip(&targets).enumerate() {
        assert!(requirement.legal_targets.contains(target), "{target:?}");
        assignments.push(TargetAssignment {
            spec: requirement.spec.clone(),
            range: index..index + 1,
        });
    }
    let mut entry = StackEntry::new(spell, A)
        .with_targets(targets)
        .with_target_assignments(assignments);
    if let Some(x) = x {
        entry = entry.with_x(x);
    }
    game.push_to_stack(entry);
    resolve_stack_entry(game).unwrap();
}

fn fixture_identity() {
    let expected = [
        ("Tropical Storm", "9b3b0bef-cdbb-41c2-b8f1-ddda9903b43a"),
        ("Hail Storm", "73eacb1f-3d35-4ead-bead-f85e6cfc17ab"),
        ("Neonate's Rush", "721aeedc-1de5-40d0-a04c-a2ebc056d06a"),
        ("The Fall of Kroog", "bfd8d447-7253-40d0-a153-78e9700045d7"),
        ("Wildfire Howl", "04fd8c1f-80e5-4b3d-b63e-7af6ccafbbaa"),
    ];
    let rows = rows();
    assert_eq!(rows.len(), expected.len());
    for (name, oracle_id) in expected {
        assert_eq!(row(name)["oracle_id"], oracle_id, "{name}");
    }
}

#[test]
fn frozen_bodies_compile_strictly_on_both_routes() {
    fixture_identity();
    for row in rows() {
        definitions(row["name"].as_str().unwrap());
    }
}

#[test]
fn tropical_storm_deals_x_to_flyers_and_one_more_to_blue_creatures() {
    for definition in definitions("Tropical Storm") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let blue_flyer = creature(&mut game, B, "Blue flyer", "Mana cost: {U}\nType: Creature — Bird\nPower/Toughness: 1/9\nFlying");
        let blue_ground = creature(&mut game, B, "Blue walker", "Mana cost: {U}\nType: Creature — Merfolk\nPower/Toughness: 1/9");
        let red_flyer = creature(&mut game, B, "Red flyer", "Mana cost: {R}\nType: Creature — Dragon\nPower/Toughness: 1/9\nFlying");
        let green_ground = creature(&mut game, B, "Green walker", "Mana cost: {G}\nType: Creature — Bear\nPower/Toughness: 1/9");
        resolve(&mut game, &definition, Some(3), vec![]);
        assert_eq!(game.damage_on(blue_flyer), 4, "X plus the additional 1");
        assert_eq!(game.damage_on(blue_ground), 1);
        assert_eq!(game.damage_on(red_flyer), 3);
        assert_eq!(game.damage_on(green_ground), 0);
        assert_eq!(game.player(A).unwrap().life, 20);
        assert_eq!(game.player(B).unwrap().life, 20);
    }
}

#[test]
fn hail_storm_shares_the_second_amount_between_you_and_your_creatures() {
    for definition in definitions("Hail Storm") {
        let debug = format!("{:?}", definition.spell_effect);
        assert_eq!(debug.matches("DealDamageEachEffect").count(), 2, "{debug}");
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let mine = creature(&mut game, A, "My bear", "Mana cost: {G}\nType: Creature — Bear\nPower/Toughness: 2/9");
        let theirs = creature(&mut game, B, "Their bear", "Mana cost: {G}\nType: Creature — Bear\nPower/Toughness: 2/9");
        resolve(&mut game, &definition, None, vec![]);
        assert_eq!(game.player(A).unwrap().life, 19, "1 damage to you");
        assert_eq!(game.player(B).unwrap().life, 20);
        assert_eq!(game.damage_on(mine), 1, "each creature you control");
        assert_eq!(game.damage_on(theirs), 0, "not attacking, not yours");
    }
}

#[test]
fn neonates_rush_damages_the_target_and_its_controller() {
    for definition in definitions("Neonate's Rush") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let target = creature(&mut game, B, "Their bear", "Mana cost: {G}\nType: Creature — Bear\nPower/Toughness: 2/9");
        let hand_before = game.player(A).unwrap().hand.len();
        for n in 0..2 {
            game.create_object_from_definition(
                &compile_to_runtime_definition(&format!("Library {n}"), "Type: Land", false).unwrap(),
                A,
                Zone::Library,
            );
        }
        resolve(&mut game, &definition, None, vec![Target::Object(target)]);
        assert_eq!(game.damage_on(target), 1);
        assert_eq!(game.player(B).unwrap().life, 19, "its controller is the target's controller");
        assert_eq!(game.player(A).unwrap().life, 20);
        assert_eq!(game.player(A).unwrap().hand.len(), hand_before + 1);
    }
}

#[test]
fn fall_of_kroog_binds_that_player_and_their_creatures_to_the_chosen_opponent() {
    for definition in definitions("The Fall of Kroog") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let land = game.create_object_from_definition(
            &compile_to_runtime_definition("Their land", "Type: Land", false).unwrap(),
            B,
            Zone::Battlefield,
        );
        let theirs = creature(&mut game, B, "Their bear", "Mana cost: {G}\nType: Creature — Bear\nPower/Toughness: 2/9");
        let mine = creature(&mut game, A, "My bear", "Mana cost: {G}\nType: Creature — Bear\nPower/Toughness: 2/9");
        resolve(
            &mut game,
            &definition,
            None,
            vec![Target::Player(B), Target::Object(land)],
        );
        assert!(game.object(land).is_none_or(|object| object.zone != Zone::Battlefield));
        assert_eq!(game.player(B).unwrap().life, 17);
        assert_eq!(game.player(A).unwrap().life, 20);
        assert_eq!(game.damage_on(theirs), 1);
        assert_eq!(game.damage_on(mine), 0);
    }
}

#[test]
fn wildfire_howl_keeps_the_promised_gift_pair_as_its_replacement() {
    for definition in definitions("Wildfire Howl") {
        let debug = format!("{:?}", definition.spell_effect);
        assert!(debug.contains("leading_instead_surface: true"), "{debug}");
        // Default branch plus both replacement arms.
        assert_eq!(debug.matches("DealDamageEachEffect").count(), 2, "{debug}");
        assert_eq!(debug.matches("DealDamageEffect").count(), 1, "{debug}");
    }
}
