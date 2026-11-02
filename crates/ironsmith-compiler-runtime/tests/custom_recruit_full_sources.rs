//! Exact sourced Recruit programs retain all sibling abilities and linked faces.
//! The executable name is the explicit custom alias Muster ranks.
use ironsmith::{CardDefinition, CardId, GameState, ObjectId, PlayerId, Zone};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::object::CounterType;
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn cohort() -> serde_json::Value { serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/tokens-complete.json")).unwrap() }
fn source(name: &str) -> serde_json::Value {
    let fixture = cohort();
    fixture["mechanics"].as_array().unwrap().iter().find(|row| row["id"] == "msem-370").unwrap()["representative_cards"]
        .as_array().unwrap().iter().find(|card| card["name"] == name).unwrap().clone()
}
fn pairs(card: &serde_json::Value) -> Vec<[CardDefinition; 2]> {
    let vocabulary: serde_json::Value = serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/source-subtypes.json")).unwrap();
    let vocabulary: Vec<ironsmith_core::custom_subtypes::CustomSubtypeDefinition> = vocabulary["definitions"].as_array().unwrap().iter()
        .filter(|definition| definition["scope"] == "msem").map(|definition| serde_json::from_value(definition.clone()).unwrap()).collect();
    ironsmith_compiler::with_custom_subtypes(&vocabulary, || {
        let faces = card["executable_faces"].as_array().unwrap();
        assert_eq!(faces.len(), card["source_faces"].as_array().unwrap().len(), "every original source face must remain represented");
        let ids: Vec<_> = faces.iter().map(|_| CardId::new()).collect();
        faces.iter().enumerate().map(|(index, face)| {
            let name = face["name"].as_str().unwrap();
            let mut builder = ironsmith_compiler::CardDefinitionBuilder::new(ids[index], name);
            if faces.len() == 2 {
                builder = builder.other_face(ids[1-index]).other_face_name(faces[1-index]["name"].as_str().unwrap())
                    .linked_face_layout(ironsmith::card::LinkedFaceLayout::TransformLike);
            }
            for identity in face["custom_keyword_identities"].as_array().into_iter().flatten() {
                assert_ne!(identity.as_str().unwrap(), "recruit", "official Recruit must not be declared custom");
                builder = builder.with_custom_keyword(identity.as_str().unwrap());
            }
            let text = face["executable_text"].as_str().unwrap();
            let (result, loss) = ironsmith_compiler::parse_loss::capture(||
                ironsmith_compiler::word_claims::with_enforcement_options(true, true, ||
                    ironsmith_compiler_runtime::compile_builder_to_artifact(builder, text, false)));
            assert!(!loss.is_lossy(), "{} / {name}: {}", card["source_card_key"], loss.reasons_text());
            let (artifact, direct) = result.unwrap_or_else(|error| panic!("{} / {name}: {error}", card["source_card_key"]));
            let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
            assert_eq!(artifact, decoded);
            let indirect = ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap();
            assert_eq!(ironsmith_text::compiled_text_lines(&direct), ironsmith_text::compiled_text_lines(&indirect));
            [direct, indirect]
        }).collect()
    }).unwrap()
}
fn army() -> CardDefinition {
    ironsmith_compiler_runtime::compile_to_runtime_definition("Existing army", "Type: Creature — Army\nPower/Toughness: 2/2", false).unwrap()
}
fn armies(game: &GameState, player: PlayerId) -> Vec<ObjectId> {
    game.battlefield.iter().copied().filter(|id| game.current_characteristics(*id).is_some_and(|chars|
        chars.controller == player && chars.subtypes.contains(&ironsmith::types::Subtype::Army))).collect()
}
fn settle(game: &mut GameState) {
    for _ in 0..16 {
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::drain_pending_trigger_events_with_dm(game, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
        ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
        if game.stack_is_empty() { return; }
        ironsmith::game_loop::resolve_stack_entry_with(game, &mut SelectFirstDecisionMaker).unwrap();
    }
    panic!("unexpected recurring recruit trigger");
}
#[test]
fn every_exact_recruit_source_and_linked_face_compiles_without_loss_and_round_trips() {
    let fixture = cohort();
    let row = fixture["mechanics"].as_array().unwrap().iter().find(|row| row["id"] == "msem-370").unwrap();
    assert_eq!(row["executable_definitions"][0]["keyword"], "Muster ranks");
    assert_eq!(row["executable_definitions"][0]["sourceKeyword"], "Recruit");
    let cards = row["representative_cards"].as_array().unwrap();
    assert_eq!(cards.len(), 30);
    let mut faces = 0;
    for card in cards {
        assert!(card["source_card_key"].as_str().is_some());
        assert!(card["source_faces"].as_array().unwrap().iter().any(|face|
            face["original_text"].as_str().unwrap().to_lowercase().contains("recruit")));
        faces += pairs(card).len();
    }
    assert_eq!(faces, 32);
}
#[test]
fn full_call_on_reinforcements_creates_army_increments_all_allied_armies_and_grants_trample() {
    for definition in pairs(&source("Call on Reinforcements")).remove(0) {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let allied = game.create_object_from_definition(&army(), A, Zone::Battlefield);
        let enemy = game.create_object_from_definition(&army(), B, Zone::Battlefield);
        let spell = game.create_object_from_definition(&definition, A, Zone::Stack);
        let mut ctx = EffectContext::new_default(spell, A);
        for effect in definition.spell_effect.as_ref().unwrap().all_effects() { execute_effect(&mut game, effect, &mut ctx).unwrap(); }
        assert_eq!(armies(&game, A).len(), 2);
        for id in armies(&game, A) {
            assert_eq!(game.counter_count(id, CounterType::PlusOnePlusOne), 1);
            assert!(game.current_characteristics(id).unwrap().abilities.iter().any(|ability| matches!(&ability.kind,
                ironsmith::ability::AbilityKind::Static(ability) if ability.has_trample())));
        }
        assert_eq!(game.counter_count(enemy, CounterType::PlusOnePlusOne), 0);
        assert_eq!(game.counter_count(allied, CounterType::PlusOnePlusOne), 1);
    }
}
#[test]
fn full_major_benjamin_stout_enters_and_dies_each_execute_complete_recruit_program() {
    for definition in pairs(&source("Major Benjamin Stout")).remove(0) {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let enemy = game.create_object_from_definition(&army(), B, Zone::Battlefield);
        let hand = game.create_object_from_definition(&definition, A, Zone::Hand);
        let major = game.move_object_by_effect(hand, Zone::Battlefield).unwrap();
        settle(&mut game);
        assert_eq!(armies(&game, A).len(), 1);
        let first = armies(&game, A)[0];
        assert_eq!(game.counter_count(first, CounterType::PlusOnePlusOne), 1);
        game.move_object_by_effect(major, Zone::Graveyard).unwrap();
        settle(&mut game);
        let mut counts: Vec<_> = armies(&game, A).into_iter().map(|id| game.counter_count(id, CounterType::PlusOnePlusOne)).collect();
        counts.sort();
        assert_eq!(counts, vec![1, 2]);
        assert_eq!(game.counter_count(enemy, CounterType::PlusOnePlusOne), 0);
    }
}
