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
fn invigorate_chooses_only_tied_least_power_friendly_creatures_without_targeting() {
    for definition in definitions("Type: Sorcery\nChoose a creature you control with the least power among creatures you control. Put two +1/+1 counters on it.") {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let strong = game.create_object_from_definition(&creature(4), A, Zone::Battlefield);
        let weak = game.create_object_from_definition(&creature(1), A, Zone::Battlefield);
        let tied = game.create_object_from_definition(&creature(1), A, Zone::Battlefield);
        let opponent = game.create_object_from_definition(&creature(0), B, Zone::Battlefield);
        let mut dm = Choices {yes: true, observed: vec![]};
        apply(&mut game, &definition, &mut dm);
        assert!(dm.observed.iter().any(|ids| ids.len() == 2 && ids.contains(&weak) && ids.contains(&tied)), "{:?}", dm.observed);
        assert_eq!(game.counter_count(strong, CounterType::PlusOnePlusOne), 0);
        assert_eq!(game.counter_count(opponent, CounterType::PlusOnePlusOne), 0);
        assert_eq!(game.counter_count(weak, CounterType::PlusOnePlusOne) + game.counter_count(tied, CounterType::PlusOnePlusOne), 2);
    }
}
#[test]
fn crystallize_preserves_optional_counter_and_treasure_fallback() {
    for definition in definitions("Type: Sorcery\nYou may put a +1/+1 counter on a creature you control. If you don't, create a Treasure token.") {
        for yes in [false, true] { for available in [false, true] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let body = available.then(|| game.create_object_from_definition(&creature(1), A, Zone::Battlefield));
            let mut dm = Choices {yes, observed: vec![]};
            apply(&mut game, &definition, &mut dm);
            assert_eq!(body.map(|id| game.counter_count(id, CounterType::PlusOnePlusOne)).unwrap_or(0), u32::from(yes && available));
            let treasures = game.battlefield.iter().filter_map(|id| game.object(*id)).filter(|o| o.subtypes.contains(&ironsmith::types::Subtype::Treasure)).count();
            assert_eq!(treasures, usize::from(!yes || !available));
        } }
    }
}
#[test]
fn augur_keeps_the_looked_card_separate_from_the_sacrificed_creature() {
    for definition in definitions("Type: Sorcery\nLook at the top card of your library. You may sacrifice a creature. If you do, put the card you looked at into your hand. Otherwise, you may put that card into your graveyard.") {
        for yes in [false, true] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let body = game.create_object_from_definition(&creature(1), A, Zone::Battlefield);
            let viewed = game.create_object_from_definition(&creature(2), A, Zone::Library);
            let stable = game.object(viewed).unwrap().stable_id;
            let mut dm = Choices {yes, observed: vec![]};
            apply(&mut game, &definition, &mut dm);
            let arrival = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(game.object(arrival).unwrap().zone, if yes {Zone::Hand} else {Zone::Library});
            assert_eq!(game.object(body).is_none(), yes);
        }
    }
}
#[test]
fn soar_checks_flying_before_choosing_the_counter_branch() {
    for definition in definitions("Type: Sorcery\nChoose a creature you control. If that creature has flying, put a +1/+1 counter on it. Otherwise, put a flying counter on it.") {
        for has_flying in [false, true] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let text = format!("Type: Creature — Bird\nPower/Toughness: 1/1{}", if has_flying {"\nFlying"} else {""});
            let body_definition = ironsmith_compiler_runtime::compile_to_runtime_definition("Bird", &text, false).unwrap();
            let body = game.create_object_from_definition(&body_definition, A, Zone::Battlefield);
            apply(&mut game, &definition, &mut Choices {yes: true, observed: vec![]});
            assert_eq!(game.counter_count(body, CounterType::PlusOnePlusOne), u32::from(has_flying));
            assert_eq!(game.counter_count(body, CounterType::Flying), u32::from(!has_flying));
        }
    }
}
#[test]
fn cackle_checks_menace_before_dealing_damage_or_adding_the_counter() {
    for definition in definitions("Type: Sorcery\nChoose a creature you control. If that creature has menace, it deals 1 damage to each opponent. Otherwise, put a menace counter on it.") {
        for has_menace in [false, true] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let text = format!("Type: Creature — Demon\nPower/Toughness: 1/1{}", if has_menace {"\nMenace"} else {""});
            let body_definition = ironsmith_compiler_runtime::compile_to_runtime_definition("Demon", &text, false).unwrap();
            let body = game.create_object_from_definition(&body_definition, A, Zone::Battlefield);
            apply(&mut game, &definition, &mut Choices {yes: true, observed: vec![]});
            assert_eq!(game.counter_count(body, CounterType::Menace), u32::from(!has_menace));
            assert_eq!(game.player(A).unwrap().life, 20);
            assert_eq!(game.player(B).unwrap().life, if has_menace {19} else {20});
        }
    }
}
