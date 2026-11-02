//! MSEM action rewrites compose ordinary choices, extrema, counters and zone moves.
use ironsmith::{CardDefinition, Effect, GameState, ObjectId, PlayerId, Zone};
use ironsmith::decision::DecisionMaker;
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
    fn decide_distribute(&mut self, _: &GameState, ctx: &ironsmith::decisions::context::DistributeContext) -> Vec<(ironsmith::game_state::Target, u32)> {
        assert_eq!(ctx.total, 2);
        ctx.targets.iter().take(2).map(|target| (target.target, 1)).collect()
    }
    fn decide_objects(&mut self, _game: &GameState, ctx: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ObjectId> {
        self.observed.push(ctx.candidates.iter().filter(|o| o.legal).map(|o| o.id).collect());
        ctx.candidates.iter().filter(|candidate| candidate.legal).take(2).map(|candidate| candidate.id).collect()
    }
}
fn apply(game: &mut GameState, definition: &CardDefinition, dm: &mut Choices) {
    let source = game.create_object_from_definition(definition, A, Zone::Stack);
    let effects = definition.spell_effect.as_ref().unwrap().all_effects_owned();
    execute_effect(game, &Effect::new(ironsmith::effects::SequenceEffect::new(effects)), &mut EffectContext::new_default(source, A).with_decision_maker(dm)).unwrap();
}
#[test]
fn rouse_distributes_counters_without_targeting_and_animates_only_noncreature_recipients() {
    for definition in definitions("Type: Sorcery\nDistribute two +1/+1 counters among permanents you control. If they aren't creatures, tap them and they become 0/0 creatures in addition to their other types.") {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let body = game.create_object_from_definition(&creature(3), A, Zone::Battlefield);
        let land_definition = ironsmith_compiler_runtime::compile_to_runtime_definition("Land", "Type: Land", false).unwrap();
        let artifact_definition = ironsmith_compiler_runtime::compile_to_runtime_definition("Artifact", "Type: Artifact", false).unwrap();
        let land = game.create_object_from_definition(&land_definition, A, Zone::Battlefield);
        let untouched = game.create_object_from_definition(&artifact_definition, A, Zone::Battlefield);
        let enemy = game.create_object_from_definition(&land_definition, B, Zone::Battlefield);
        apply(&mut game, &definition, &mut Choices {yes: true, observed: vec![]});
        assert_eq!(game.counter_count(body, CounterType::PlusOnePlusOne), 1);
        assert_eq!(game.counter_count(land, CounterType::PlusOnePlusOne), 1);
        assert_eq!(game.calculated_power(body), Some(4));
        assert!(!game.is_tapped(body));
        assert_eq!(game.calculated_power(land), Some(1));
        assert!(game.is_tapped(land));
        assert!(game.calculated_card_types(land).contains(&ironsmith::types::CardType::Land));
        assert!(game.calculated_card_types(land).contains(&ironsmith::types::CardType::Creature));
        for id in [untouched, enemy] {
            assert_eq!(game.counter_count(id, CounterType::PlusOnePlusOne), 0);
            assert!(!game.is_tapped(id));
            assert!(!game.calculated_card_types(id).contains(&ironsmith::types::CardType::Creature));
        }
    }
}
