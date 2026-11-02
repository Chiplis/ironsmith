//! MSEM action rewrites compose ordinary choices, extrema, counters and zone moves.
use ironsmith::{CardDefinition, Effect, GameState, ObjectId, PlayerId, Zone};
use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::object::CounterType;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
#[path="custom_final_next_wave_support.rs"] mod source_routes;
fn definitions(text: &str) -> [CardDefinition; 2] {
    source_routes::definitions(&serde_json::json!({"name":"Composed action","executable_text":text}))
        .unwrap_or_else(|error|panic!("independent strict native/artifact action routes: {error}"))
}

fn creature(power: i32) -> CardDefinition {
    ironsmith_compiler_runtime::compile_to_runtime_definition("Body", &format!("Type: Creature — Fox\nPower/Toughness: {power}/5"), false).unwrap()
}
struct Choices { yes: bool, observed: Vec<Vec<ObjectId>> }
impl DecisionMaker for Choices {
    fn decide_boolean(&mut self, _: &GameState, _: &ironsmith::decisions::context::BooleanContext) -> bool { self.yes }
    fn decide_counters(&mut self, _: &GameState, ctx: &ironsmith::decisions::context::CountersContext) -> Vec<(CounterType, u32)> {
        ctx.available_counters.iter().find(|(_, count)| *count > 0).map(|(kind, _)| vec![(*kind, 1)]).unwrap_or_default()
    }
    fn decide_objects(&mut self, game: &GameState, ctx: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ObjectId> {
        self.observed.push(ctx.candidates.iter().filter(|o| o.legal).map(|o| o.id).collect());
        SelectFirstDecisionMaker.decide_objects(game, ctx)
    }
}
fn apply(game: &mut GameState, definition: &CardDefinition, dm: &mut Choices) {
    let source = game.create_object_from_definition(definition, A, Zone::Stack);
    let effects = definition.spell_effect.as_ref().unwrap().all_effects_owned();
    execute_effect(game, &Effect::new(ironsmith::effects::SequenceEffect::new(effects)), &mut EffectContext::new_default(source, A).with_decision_maker(dm)).unwrap();
}
#[test]
fn permutate_removes_any_counter_kind_then_chooses_a_distinct_friendly_recipient() {
    for definition in definitions("Type: Sorcery\nYou may remove a counter from a creature you control. If you do, put two +1/+1 counters on a different creature you control.") {
        for accepted in [false, true] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let donor = game.create_object_from_definition(&creature(2), A, Zone::Battlefield);
            let recipient = game.create_object_from_definition(&creature(3), A, Zone::Battlefield);
            let enemy = game.create_object_from_definition(&creature(4), B, Zone::Battlefield);
            game.add_counters(donor, CounterType::Flying, 1);
            let mut dm = Choices {yes: accepted, observed: vec![]};
            apply(&mut game, &definition, &mut dm);
            assert_eq!(game.counter_count(donor, CounterType::Flying), u32::from(!accepted));
            assert_eq!(game.counter_count(donor, CounterType::PlusOnePlusOne), 0);
            assert_eq!(game.counter_count(enemy, CounterType::PlusOnePlusOne), 0);
            assert_eq!(game.counter_count(recipient, CounterType::PlusOnePlusOne), if accepted {2} else {0});
            if accepted { assert!(dm.observed.iter().any(|ids| ids == &[recipient]), "different excludes the counter donor: {:?}", dm.observed); }
        }
    }
}
