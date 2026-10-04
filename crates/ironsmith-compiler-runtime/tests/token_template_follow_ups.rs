//! Authored only; build, compile and execution remain deferred.
use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder};
use ironsmith::card::PowerToughness;
use ironsmith::decision::DecisionMaker;
use ironsmith::effects::{CreateTokenEffect, EffectContext, EffectExecutor, TurnFaceUpEffect};
use ironsmith::game_loop::{put_triggers_on_stack, resolve_stack_entry, resolve_stack_entry_with};
use ironsmith::object::CounterType;
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::TriggerQueue;
use ironsmith::types::Subtype;
use ironsmith::{CardId, CardType, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::compile_to_artifact;
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
fn a() -> PlayerId { PlayerId::from_index(0) }
fn b() -> PlayerId { PlayerId::from_index(1) }
fn game() -> GameState { GameState::new(vec!["Alice".into(), "Bob".into()], 20) }
fn definitions(name: &str) -> [CardDefinition; 2] {
    let cards: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../fixtures/token_template_follow_ups.json.fixture")).unwrap();
    let card = cards.iter().find(|card| card["name"] == name).unwrap();
    let text = format!("Mana cost: {}\nType: {}\n{}", card["mana_cost"].as_str().unwrap(), card["type_line"].as_str().unwrap(), card["oracle_text"].as_str().unwrap());
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, direct) = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text()); artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [direct, materialize_artifact(&restored).unwrap()]
}
fn creature(subtype: Subtype) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Transition creature").token().card_types(vec![CardType::Creature])
        .subtypes(vec![subtype]).power_toughness(PowerToughness::fixed(2, 2)).build()
}
fn settle(game: &mut GameState, queue: &mut TriggerQueue) {
    for _ in 0..16 {
        put_triggers_on_stack(game, queue).unwrap();
        if game.stack_is_empty() { return; }
        resolve_stack_entry(game).unwrap();
    }
    panic!("bounded transition triggers did not settle");
}
fn create(game: &mut GameState, host: ObjectId, owner: PlayerId, subtype: Subtype) -> Vec<ObjectId> {
    CreateTokenEffect::you(creature(subtype), 1).execute(game, &mut EffectContext::new_default(host, owner)).unwrap().result_objects().unwrap().to_vec()
}
fn counters(game: &GameState, object: ObjectId) -> u32 {
    game.object(object).unwrap().counters.get(&CounterType::PlusOnePlusOne).copied().unwrap_or(0)
}
#[test]
fn case_real_entry_and_face_up_each_tag_the_same_detective_not_the_case() {
    for definition in definitions("Case of the Pilfered Proof") {
        let mut game = game(); let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
        let mut queue = TriggerQueue::new();
        for (owner, subtype, expected) in [(a(), Subtype::Detective, 1), (b(), Subtype::Detective, 0), (a(), Subtype::Soldier, 0)] {
            let id = create(&mut game, host, owner, subtype)[0]; settle(&mut game, &mut queue);
            assert_eq!(counters(&game, id), expected); assert_eq!(counters(&game, host), 0);
            assert!(game.set_face_down(id));
            TurnFaceUpEffect::new(ChooseSpec::SpecificObject(id)).execute(&mut game, &mut EffectContext::new_default(host, a())).unwrap();
            settle(&mut game, &mut queue);
            assert_eq!(counters(&game, id), expected * 2); assert_eq!(counters(&game, host), 0);
            assert_eq!(game.current_controller(id), Some(owner));
        }
    }
}
#[test]
fn case_solve_runs_at_real_end_step_and_later_creations_get_one_clue() {
    for definition in definitions("Case of the Pilfered Proof") {
        for detectives in [2, 3] {
            let mut game = game(); let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
            let mut queue = TriggerQueue::new();
            for _ in 0..detectives { create(&mut game, host, a(), Subtype::Detective); }
            settle(&mut game, &mut queue); assert!(!game.is_case_solved(host));
            let mut runner = ironsmith::turn_runner::TurnRunner::from_state_for_sync(ironsmith::turn_runner::TurnState::EndStep);
            runner.advance(&mut game, &mut queue).unwrap(); settle(&mut game, &mut queue);
            assert_eq!(game.is_case_solved(host), detectives == 3);
            let ids = create(&mut game, host, a(), Subtype::Soldier);
            assert_eq!(ids.len(), if detectives == 3 {2} else {1});
            assert_eq!(ids.iter().filter(|id| game.current_has_subtype(**id, Subtype::Clue)).count(), if detectives == 3 {1} else {0});
        }
    }
}
#[test]
fn face_up_trigger_keeps_the_current_incarnation_after_its_subject_leaves() {
    for definition in definitions("Case of the Pilfered Proof") {
        let mut game = game(); let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
        let mut queue = TriggerQueue::new();
        let id = create(&mut game, host, a(), Subtype::Detective)[0]; settle(&mut game, &mut queue);
        assert!(game.set_face_down(id));
        TurnFaceUpEffect::new(ChooseSpec::SpecificObject(id)).execute(&mut game, &mut EffectContext::new_default(host, a())).unwrap();
        put_triggers_on_stack(&mut game, &mut queue).unwrap(); assert_eq!(game.stack.len(), 1);
        game.move_object_by_effect(id, Zone::Exile).unwrap();
        resolve_stack_entry(&mut game).unwrap(); assert_eq!(counters(&game, host), 0);
        assert!(game.battlefield.iter().all(|id| !game.current_has_subtype(*id, Subtype::Detective)));
    }
}
struct RevealChoice(bool);
impl DecisionMaker for RevealChoice {
    fn decide_boolean(&mut self, _: &GameState, _: &ironsmith::decisions::context::BooleanContext) -> bool { self.0 }
}
#[test]
fn fisher_real_upkeep_reveal_gate_draw_and_level_chain_remain_independent() {
    for definition in definitions("Fisher's Talent") {
        for level in [1, 2, 3] { for land in [false, true] { for reveal in [false, true] {
            let mut game = game(); let host = game.create_object_from_definition(&definition, a(), Zone::Battlefield);
            game.set_class_level(host, level);
            let card = CardDefinitionBuilder::new(CardId::new(), "Top card probe")
                .card_types(vec![if land { CardType::Land } else { CardType::Artifact }]).build();
            game.create_object_from_definition(&card, a(), Zone::Library);
            let mut queue = TriggerQueue::new();
            let mut runner = ironsmith::turn_runner::TurnRunner::from_state_for_sync(ironsmith::turn_runner::TurnState::Upkeep);
            runner.advance(&mut game, &mut queue).unwrap(); put_triggers_on_stack(&mut game, &mut queue).unwrap();
            assert_eq!(game.stack.len(), 1); let mut dm = RevealChoice(reveal);
            resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            assert_eq!(game.player(a()).unwrap().hand.len(), 1, "draw is outside the reveal/create condition");
            let tokens: Vec<_> = game.battlefield.iter().copied().filter(|id| game.object(*id).unwrap().kind == ironsmith::object::ObjectKind::Token).collect();
            assert_eq!(tokens.len(), usize::from(land && reveal));
            if let Some(&id) = tokens.first() {
                let (subtype, power) = match level {1 => (Subtype::Fish, 1), 2 => (Subtype::Shark, 3), _ => (Subtype::Octopus, 8)};
                assert!(game.current_has_subtype(id, subtype)); assert_eq!(game.current_power(id), Some(power));
            }
        } } }
    }
}
