//! MSEM action rewrites compose ordinary choices, extrema, counters and zone moves.
use ironsmith::{CardDefinition, Effect, GameState, ObjectId, PlayerId, Zone};
use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::effects::{EffectContext, execute_effect};
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
fn purge_uses_graveyards_of_both_players_and_draws_only_for_two_completed_exiles() {
    for definition in definitions("Type: Sorcery\nExile two cards that share a card type from graveyards. If you exile two cards this way, draw a card.") {
        for count in [1, 2] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let first = game.create_object_from_definition(&creature(1), A, Zone::Graveyard);
            let second = (count == 2).then(|| game.create_object_from_definition(&creature(2), B, Zone::Graveyard));
            game.create_object_from_definition(&creature(3), A, Zone::Library);
            let mut dm = Choices {yes: true, observed: vec![]};
            apply(&mut game, &definition, &mut dm);
            assert_eq!(game.player(A).unwrap().hand.len(), usize::from(count == 2));
            if let Some(second) = second {
                assert!(dm.observed.iter().any(|ids| ids.contains(&first) && ids.contains(&second)), "both graveyards supply one non-target choice: {:?}", dm.observed);
            }
        }
    }
}
