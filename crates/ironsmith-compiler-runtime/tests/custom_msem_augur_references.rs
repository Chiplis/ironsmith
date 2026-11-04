//! MSEM action rewrites compose ordinary choices, extrema, counters and zone moves.
use ironsmith::{CardDefinition, Effect, GameState, ObjectId, PlayerId, Zone};
use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::effects::{EffectContext, execute_effect};
const A: PlayerId = PlayerId(0);
#[path="custom_final_next_wave_support.rs"] mod source_routes;
fn definitions(text: &str) -> [CardDefinition; 2] {
    source_routes::definitions(&serde_json::json!({"name":"Composed action","executable_text":text}))
        .unwrap_or_else(|error|panic!("independent strict native/artifact action routes: {error}"))
}

fn creature(power: i32) -> CardDefinition {
    ironsmith_compiler_runtime::compile_to_runtime_definition("Body", &format!("Type: Creature — Fox\nPower/Toughness: {power}/5"), false).unwrap()
}
struct Choices { yes: std::collections::VecDeque<bool>, observed: Vec<Vec<ObjectId>> }
impl DecisionMaker for Choices {
    fn decide_boolean(&mut self, _: &GameState, _: &ironsmith::decisions::context::BooleanContext) -> bool { self.yes.pop_front().expect("independent optional decision") }
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
fn explicit_viewed_card_reference_survives_the_independent_optional_sacrifice() {
    for definition in definitions("Type: Sorcery\nLook at the top card of your library. You may sacrifice a creature. If you do, put the card you looked at into your hand. Otherwise, you may put the card you looked at into your graveyard.") {
        for (sacrifice, graveyard) in [(false, false), (false, true), (true, false)] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let body = game.create_object_from_definition(&creature(5), A, Zone::Battlefield);
            let viewed = game.create_object_from_definition(&creature(1), A, Zone::Library);
            let viewed_stable = game.object(viewed).unwrap().stable_id;
            let mut dm = Choices {yes: [sacrifice, graveyard].into(), observed: vec![]};
            apply(&mut game, &definition, &mut dm);
            let arrival = game.find_object_by_stable_id(viewed_stable).unwrap();
            let expected = if sacrifice {Zone::Hand} else if graveyard {Zone::Graveyard} else {Zone::Library};
            assert_eq!(game.object(arrival).unwrap().zone, expected);
            assert_eq!(game.object(body).is_none(), sacrifice);
        }
    }
}
