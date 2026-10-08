//! "Roll to visit your Attractions" as an instruction (CR 701.52; Line
//! Cutter) shares the turn-based action's owner, and a die named like the
//! card ("Roll a six-sided die" on Six-Sided Die) is the die rolled, not a
//! self-reference. Source-authored, deliberately unrun.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;

const A: ironsmith::PlayerId = ironsmith::PlayerId::from_index(0);
const B: ironsmith::PlayerId = ironsmith::PlayerId::from_index(1);

fn definitions(name: &str) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/attraction_visit_rolls.json.fixture"
    ))
    .unwrap();
    let row = rows.into_iter().find(|row| row["name"] == name).unwrap();
    let text = row["text"].as_str().unwrap();
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
fn line_cutter_enters_and_rolls_to_visit_through_the_shared_action() {
    for definition in definitions("Line Cutter") {
        let debug = format!("{:?}", definition.abilities);
        assert!(debug.contains("RollToVisitAttractionsEffect"), "{debug}");
        assert!(debug.contains("player: You"), "{debug}");
    }
}

#[test]
fn line_cutter_visits_the_lit_attraction_and_its_visit_triggers() {
    use ironsmith::decision::SelectFirstDecisionMaker;
    use ironsmith::effects::{EffectContext, EffectExecutor, execute_effect};
    use ironsmith::game_loop::{put_triggers_on_stack_with_dm, resolve_stack_entry_with};
    use ironsmith::triggers::TriggerQueue;
    use ironsmith::{GameState, Zone};

    for definition in definitions("Line Cutter") {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let attraction = ironsmith::cards::builders::CardDefinitionBuilder::new(
            ironsmith::CardId::new(),
            "Visit on six",
        )
        .card_types(vec![ironsmith::CardType::Artifact])
        .subtypes(vec![ironsmith::types::Subtype::Attraction])
        .attraction_lights(vec![6])
        .with_spell_effect(vec![ironsmith::Effect::gain_life(1)])
        .build();
        game.enable_attractions(vec![(
            A,
            ironsmith::game_state::AttractionDeckFormat::Limited,
            vec![attraction.clone(), attraction.clone(), attraction],
        )])
        .unwrap();
        let cutter = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        ironsmith::effects::OpenAttractionEffect::new()
            .execute(&mut game, &mut EffectContext::new_default(cutter, A))
            .unwrap();
        let triggered = definition
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Triggered(triggered) => Some(triggered),
                _ => None,
            })
            .expect("enters trigger");
        let life = game.player(A).unwrap().life;
        let rolls = game.turn_store.turn_history.completed_die_roll_count(A);
        game.force_next_die_roll(6);
        let mut dm = SelectFirstDecisionMaker;
        {
            let mut ctx = EffectContext::new(cutter, A, &mut dm);
            for effect in triggered.effects.flattened_default_effects() {
                let outcome = execute_effect(&mut game, effect, &mut ctx).unwrap();
                for event in outcome.events {
                    game.queue_trigger_event(Default::default(), event);
                }
            }
        }
        assert_eq!(game.turn_store.turn_history.completed_die_roll_count(A), rolls + 1);
        let mut queue = TriggerQueue::new();
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        assert_eq!(game.stack.len(), 1, "the lit Attraction's Visit ability triggers");
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.player(A).unwrap().life, life + 1);
        assert_eq!(game.player(B).unwrap().life, 20);
    }
}

#[test]
fn six_sided_die_rolls_a_die_and_its_table_acts_on_the_chosen_creature() {
    for definition in definitions("Six-Sided Die") {
        let debug = format!("{:?}", definition.spell_effect);
        assert!(debug.contains("RollDie"), "{debug}");
        assert!(debug.contains("sides: 6"), "{debug}");
        assert!(debug.contains("Exile"), "{debug}");
        assert!(debug.contains("Destroy"), "{debug}");
        assert!(debug.contains("MinusOneMinusOne"), "{debug}");
        assert!(!debug.contains("roll a this"), "{debug}");
    }
}
