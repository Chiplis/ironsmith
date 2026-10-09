//! Source-authored and deliberately unrun (cf8 p04): serial object
//! qualifiers stay inside one operand instead of becoming coordinated
//! effects — "each Pest, Bat, Insect, Snake, and Spider you control",
//! "target artifact, creature, or land you control", and "each attacking or
//! blocking creature target player controls".
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::game_loop::resolve_stack_entry_with;
use ironsmith::game_state::StackEntry;
use ironsmith::object::CounterType;
use ironsmith::{GameState, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

const A: PlayerId = PlayerId::from_index(0);

fn definitions(name: &str, oracle_id: &str) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/serial_object_qualifiers.json.fixture"
    ))
    .unwrap();
    let row = rows.iter().find(|row| row["name"] == name).unwrap();
    assert_eq!(row["oracle_id"], oracle_id);
    let text = row["text"].as_str().unwrap();
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| {
        compile_to_runtime_definition(name, text, false)
    });
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (compiled, loss) =
        ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (artifact, _) = compiled.unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    let decoded = materialize_artifact(&restored).unwrap();
    assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&decoded));
    [direct.unwrap(), decoded]
}

#[test]
fn blech_counters_every_listed_creature_type_you_control_only() {
    for definition in definitions("Blech, Loafing Pest", "c1bdd361-c7f2-4aa2-8d07-b582775551ff") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let blech = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let creature = |game: &mut GameState, name: &str, subtype: &str| {
            let text = format!("Mana cost: {{1}}\nType: Creature — {subtype}\nPower/Toughness: 1/1");
            let definition = compile_to_runtime_definition(name, &text, false).unwrap();
            game.create_object_from_definition(&definition, A, Zone::Battlefield)
        };
        let bat = creature(&mut game, "Bat probe", "Bat");
        let spider = creature(&mut game, "Spider probe", "Spider");
        let human = creature(&mut game, "Human probe", "Human");
        let program = definition
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Triggered(triggered) => Some(triggered.effects.clone()),
                _ => None,
            })
            .unwrap();
        game.push_to_stack(StackEntry::ability(blech, A, program));
        resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        let counters = |id| {
            game.object(id)
                .unwrap()
                .counters
                .get(&CounterType::PlusOnePlusOne)
                .copied()
                .unwrap_or(0)
        };
        assert_eq!(counters(blech), 1, "Blech is a Pest");
        assert_eq!(counters(bat), 1);
        assert_eq!(counters(spider), 1);
        assert_eq!(counters(human), 0);
    }
}

#[test]
fn tawnos_and_brigid_keep_one_target_operand() {
    for definition in definitions("Tawnos's Tinkering", "bd101a9b-4c1e-44c8-b9ef-cd79c4d28f36") {
        let debug = format!("{:?}", definition.spell_effect);
        assert!(debug.contains("Artifact") && debug.contains("Land"), "{debug}");
        assert_eq!(debug.matches("PutCountersEffect").count(), 1, "{debug}");
    }
    for definition in definitions("Brigid, Hero of Kinsbaile", "929ad5c3-4efe-44c3-93ac-eda432814385") {
        let debug = format!("{definition:?}");
        assert!(debug.contains("DealDamageEachEffect"), "{debug}");
        assert!(debug.contains("TargetOnlyEffect") || debug.contains("Target("), "{debug}");
    }
}
