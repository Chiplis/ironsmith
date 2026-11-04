//! MSEM action rewrites compose ordinary choices, extrema, counters and zone moves.
use ironsmith::{CardDefinition, Effect, GameState, ObjectId, PlayerId, Zone};
use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::object::CounterType;
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A: PlayerId = PlayerId(0);
fn definitions(text: &str) -> [CardDefinition; 2] {
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| ironsmith_compiler::word_claims::with_enforcement_options(true, true, || ironsmith_compiler_runtime::compile_to_artifact("Composed action", text, false)));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, direct) = result.unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(restored, artifact);
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
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
fn optional_complete_program_keeps_outer_choice_independent_of_inner_counter_choice() {
    for definition in definitions("Type: Sorcery\nYou may do the following: You may put a +1/+1 counter on a creature you control; If you don't, create a Treasure token. You gain 1 life.") {
        for (outer, inner) in [(false, false), (true, false), (true, true)] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let body = game.create_object_from_definition(&creature(1), A, Zone::Battlefield);
            let mut dm = Choices {yes: [outer, inner].into(), observed: vec![]};
            apply(&mut game, &definition, &mut dm);
            assert_eq!(game.counter_count(body, CounterType::PlusOnePlusOne), u32::from(outer && inner));
            let treasures = game.battlefield.iter().filter_map(|id| game.object(*id)).filter(|object| object.subtypes.contains(&ironsmith::types::Subtype::Treasure)).count();
            assert_eq!(treasures, usize::from(outer && !inner));
            assert_eq!(game.player(A).unwrap().life, 21, "effect following colon-bounded program is unconditional");
            assert_eq!(dm.yes.len(), usize::from(!outer), "inner choice is not asked when outer program is declined");
        }
    }
}
#[test]
fn bounded_repetition_keeps_conditional_followups_inside_each_iteration() {
    for definition in definitions("Type: Sorcery\nRepeat the following process 2 times: You may gain 1 life; If you do, draw a card. You lose 1 life.") {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        game.create_object_from_definition(&creature(1), A, Zone::Library);
        game.create_object_from_definition(&creature(2), A, Zone::Library);
        let mut dm = Choices {yes: [true, false].into(), observed: vec![]};
        apply(&mut game, &definition, &mut dm);
        assert_eq!(game.player(A).unwrap().life, 20);
        assert_eq!(game.player(A).unwrap().hand.len(), 1);
        assert!(dm.yes.is_empty());
    }
}
