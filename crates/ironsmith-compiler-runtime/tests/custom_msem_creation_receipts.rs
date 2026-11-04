//! MSEM action rewrites compose ordinary choices, extrema, counters and zone moves.
use ironsmith::{CardDefinition, Effect, GameState, ObjectId, PlayerId, Zone};
use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::effects::{EffectContext, execute_effect};
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
fn created_this_way_queries_the_local_typed_creation_receipt() {
    for definition in definitions("Type: Sorcery\nYou may put a +1/+1 counter on a creature you control. If you don't, create a Treasure token. If you create a Treasure this way, draw a card.") {
        for accepted in [false, true] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            game.create_object_from_definition(&creature(2), A, Zone::Battlefield);
            game.create_object_from_definition(&creature(1), A, Zone::Library);
            let old = ironsmith_compiler_runtime::compile_to_runtime_definition("Earlier creation", "Type: Sorcery\nCreate a Treasure token.", false).unwrap();
            apply(&mut game, &old, &mut Choices {yes:true,observed:vec![]});
            apply(&mut game, &definition, &mut Choices {yes:accepted,observed:vec![]});
            assert_eq!(game.player(A).unwrap().hand.len(), usize::from(!accepted), "earlier Treasure creation cannot satisfy this local condition");
        }
    }
}
#[test]
fn token_creation_history_accepts_the_subtype_without_redundant_token_noun() {
    for definition in definitions("Type: Sorcery\nCreate a Treasure token. You gain 1 life for each Treasure you've created this turn.") {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        apply(&mut game, &definition, &mut Choices {yes:true,observed:vec![]});
        assert_eq!(game.player(A).unwrap().life, 21);
    }
}
