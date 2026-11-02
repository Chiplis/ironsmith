//! MSEM action rewrites compose ordinary choices, extrema, counters and zone moves.
use ironsmith::{CardDefinition, Effect, GameState, ObjectId, PlayerId, Zone};
use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::object::CounterType;
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
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
fn bounded_foreach_runs_complete_body_per_recipient_and_followup_once_even_for_empty_set() {
    for definition in definitions("Type: Sorcery\nFor each creature you control, do the following: [Put a +1/+1 counter on that creature; You gain 1 life]. You gain 3 life.") {
        for count in [0,2] {
            let mut game=GameState::new(vec!["A".into(),"B".into()],20);
            let own=(0..count).map(|_|game.create_object_from_definition(&creature(2),A,Zone::Battlefield)).collect::<Vec<_>>();
            let opponent=game.create_object_from_definition(&creature(2),B,Zone::Battlefield);
            apply(&mut game,&definition,&mut Choices{yes:true,observed:vec![]});
            assert_eq!(game.player(A).unwrap().life,23+count);
            for id in own {assert_eq!(game.counter_count(id,CounterType::PlusOnePlusOne),1);}
            assert_eq!(game.counter_count(opponent,CounterType::PlusOnePlusOne),0);
        }
    }
}
#[test]
fn optional_complete_program_can_nest_bounded_iteration_without_flattening_it() {
    for definition in definitions("Type: Sorcery\nYou may do the following: [For each creature you control, do the following: [Put a +1/+1 counter on that creature; You gain 1 life]]. You gain 3 life.") {
        for yes in [false,true] {
            let mut game=GameState::new(vec!["A".into(),"B".into()],20);
            let own=(0..2).map(|_|game.create_object_from_definition(&creature(2),A,Zone::Battlefield)).collect::<Vec<_>>();
            apply(&mut game,&definition,&mut Choices{yes,observed:vec![]});
            assert_eq!(game.player(A).unwrap().life,23+if yes{2}else{0});
            for id in own {assert_eq!(game.counter_count(id,CounterType::PlusOnePlusOne),u32::from(yes));}
        }
    }
}
#[test]
fn targeted_bounded_iteration_declares_one_target_set_and_two_body_effects() {
    for definition in definitions("Type: Sorcery\nFor each of up to two target creatures you control, do the following: [Put a +1/+1 counter on that creature; You gain 1 life]. You gain 3 life.") {
        let effects=definition.spell_effect.as_ref().unwrap().all_effects_owned();
        let body=effects.iter().find_map(|effect|effect.downcast_ref::<ironsmith::effects::ForEachTaggedEffect>()).unwrap();
        assert_eq!(body.effects.len(),2);
        assert_eq!(effects.len(),3);
    }
}
