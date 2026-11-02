//! Rapport-like source-plus-other conditions inspect one captured tap action.
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::effects::{EffectContext, EffectExecutor, TapEffect};
use ironsmith::game_loop::{put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::object::CounterType;
use ironsmith::target::{ChooseSpec, ObjectFilter};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardDefinition, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);

fn definitions() -> [CardDefinition; 2] {
    let text = "Mana cost: {R}{W}\nType: Creature — Human Scout\nPower/Toughness: 2/1\nRapport — Whenever Herald of the Hunt and at least one other creature you control become tapped, You may put 1 +1/+1 counters on this creature. If you don't, create 1 1/1 white Citizen creature tokens.";
    let (result, loss) = ironsmith_compiler::parse_loss::capture(||
        ironsmith_compiler::word_claims::with_enforcement_options(true, true, ||
            ironsmith_compiler_runtime::compile_to_artifact("Herald of the Hunt", text, false)));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, direct) = result.unwrap();
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}

fn tap(game: &mut GameState, source: ObjectId, effect: TapEffect) {
    let mut dm = SelectFirstDecisionMaker;
    let mut context = EffectContext::new(source, A, &mut dm);
    let outcome = effect.execute(game, &mut context).unwrap();
    for event in outcome.events { game.queue_trigger_event(Default::default(), event); }
}

#[test]
fn source_and_another_must_transition_in_the_same_action_and_trigger_once() {
    for definition in definitions() {
        for scenario in 0..6 {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let body = ironsmith_compiler_runtime::compile_to_runtime_definition("Body", "Type: Creature\nPower/Toughness: 2/2", false).unwrap();
            let owner = if scenario == 2 { B } else { A };
            let other = game.create_object_from_definition(&body, owner, Zone::Battlefield);
            if scenario == 5 { game.create_object_from_definition(&body, A, Zone::Battlefield); }
            match scenario {
                1 => {
                    tap(&mut game, source, TapEffect::with_spec(ChooseSpec::SpecificObject(source)));
                    tap(&mut game, source, TapEffect::with_spec(ChooseSpec::SpecificObject(other)));
                }
                3 => tap(&mut game, source, TapEffect::with_spec(ChooseSpec::SpecificObject(other))),
                4 => {
                    game.tap(source);
                    tap(&mut game, source, TapEffect::all(ObjectFilter::creature()));
                }
                _ => tap(&mut game, source, TapEffect::all(ObjectFilter::creature())),
            }
            let mut queue = TriggerQueue::new();
            put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
            let expected = usize::from(scenario == 0 || scenario == 5);
            assert_eq!(game.stack.len(), expected, "scenario {scenario}");
            if expected == 1 {
                resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
                assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 1);
            }
        }
    }
}
