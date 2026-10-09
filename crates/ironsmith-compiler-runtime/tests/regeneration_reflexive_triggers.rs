//! "When it regenerates this way, ..." is a reflexive trigger created when
//! the shield replaces a destruction (CR 701.19, 603.12), controlled by the
//! shield's controller. Source-authored, deliberately unrun.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::effect::Effect;
use ironsmith::effects::{ReflexiveTriggerEffect, RegenerateEffect};
use ironsmith_compiled_artifact::CompiledCardArtifact;

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/regeneration_reflexive_triggers.json.fixture"))
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

fn collect(effect: &Effect, all: &mut Vec<Effect>) {
    all.push(effect.clone());
    effect.visit_child_effects(&mut |child| collect(child, all));
}

#[test]
fn regeneration_follow_up_is_a_reflexive_trigger_on_both_routes() {
    let rows = fixtures();
    assert_eq!(rows.len(), 2);
    for row in &rows {
        let name = row["name"].as_str().unwrap();
        for definition in definitions(name, row["text"].as_str().unwrap()) {
            let mut all = Vec::new();
            for ability in &definition.abilities {
                if let AbilityKind::Activated(activated) = &ability.kind {
                    for effect in activated.effects.all_effects() {
                        collect(effect, &mut all);
                    }
                }
            }
            let regenerate = all
                .iter()
                .find_map(|effect| effect.downcast_ref::<RegenerateEffect>())
                .expect("regeneration shield");
            assert_eq!(regenerate.follow_up_effects.len(), 1, "{name}");
            let reflexive = regenerate.follow_up_effects[0]
                .downcast_ref::<ReflexiveTriggerEffect>()
                .expect("the follow-up is a trigger, not an inline instruction");
            assert_eq!(reflexive.condition, RegenerateEffect::SHIELD_USED_ID, "{name}");
            let body = format!("{:?}", reflexive.effects);
            assert!(body.contains("PutCounters"), "{name}: {body}");
        }
    }
}

#[test]
fn matopi_golem_gets_its_counter_from_a_stack_trigger_after_regenerating() {
    use ironsmith::decision::SelectFirstDecisionMaker;
    use ironsmith::effects::{EffectContext, execute_effect};
    use ironsmith::events::processing::{DestroyResult, process_destroy_full};
    use ironsmith::game_loop::{put_triggers_on_stack_with_dm, resolve_stack_entry_with};
    use ironsmith::triggers::TriggerQueue;
    use ironsmith::{GameState, PlayerId, Zone};

    const A: PlayerId = PlayerId::from_index(0);
    let row = fixtures().into_iter().find(|row| row["name"] == "Matopi Golem").unwrap();
    for definition in definitions("Matopi Golem", row["text"].as_str().unwrap()) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let golem = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let AbilityKind::Activated(activated) = &definition.abilities[0].kind else {
            panic!("regeneration ability");
        };
        let mut dm = SelectFirstDecisionMaker;
        for effect in activated.effects.flattened_default_effects() {
            execute_effect(&mut game, effect, &mut EffectContext::new(golem, A, &mut dm)).unwrap();
        }
        assert_eq!(process_destroy_full(&mut game, golem, None).unwrap(), DestroyResult::Replaced);
        // The counter is not placed inside the replacement.
        assert_eq!(game.counter_count(golem, ironsmith::CounterType::MinusOneMinusOne), 0);
        let mut queue = TriggerQueue::new();
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        assert_eq!(game.stack.len(), 1, "the reflexive trigger waits for the stack");
        assert_eq!(game.stack.last().unwrap().controller, A, "the shield's controller");
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.counter_count(golem, ironsmith::CounterType::MinusOneMinusOne), 1);
    }
}
