//! Actual Abhorrent Exhumer entry mill/exile receipt and historical type gate.
use ironsmith::{CardDefinition,GameState,ObjectId,PlayerId,Zone};use ironsmith::decision::SelectFirstDecisionMaker;
const A:PlayerId=PlayerId(0);
fn definitions()->[CardDefinition;2]{
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-engorge-source-complete.json")).unwrap();let text=fixture["mechanics"][0]["representative_cards"][0]["executable_text"].as_str().unwrap();
 let compiled=ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler::CompilerFacade::new().compile_definition(ironsmith_compiler::CardDefinitionBuilder::new(ironsmith_compiler::ids::CardId::new(),"Abhorrent Exhumer").with_custom_keyword("Engorge"),text.to_string(),ironsmith_compiler::CompilePolicy{allow_unsupported:false})).unwrap();
 let direct=ironsmith_compiler_runtime::into_runtime_definition(compiled.definition.clone()).unwrap();
 let wire=ironsmith_compiled_artifact::wire_definition_from_serializable(&compiled.definition).unwrap();
 let artifact=ironsmith_compiled_artifact::CompiledCardArtifact::new(ironsmith_compiled_artifact::ArtifactCardIdentity{local_id:ironsmith_compiled_artifact::ArtifactCardId(compiled.definition.card.id.0),name:compiled.definition.card.name.clone(),face_name:None,other_face:None,linked_face_layout:None},ironsmith_compiled_artifact::CompiledCardPayload{definition:wire,canonical_text:ironsmith_text::compiled_text_lines(&direct).join("\n"),ability_labels:ironsmith_text::ability_surface_texts(&direct)},"engorge-entry-source",text.as_bytes());
 let restored=ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();assert_eq!(artifact,restored);
 [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
fn card(name:&str,creature:bool)->CardDefinition{ironsmith_compiler_runtime::compile_to_runtime_definition(name,if creature{"Mana cost: {1}\nType: Creature\nPower/Toughness: 1/1"}else{"Mana cost: {1}\nType: Artifact"},false).unwrap()}
fn enter(game:&mut GameState,source:ObjectId)->ObjectId{let stable=game.object(source).unwrap().stable_id;let effect=ironsmith::Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::ChooseSpec::SpecificObject(source),Zone::Battlefield,false));let result=ironsmith::effects::execute_effect(game,&effect,&mut ironsmith::effects::EffectContext::new_default(source,A)).unwrap();for event in result.events{game.queue_trigger_event(Default::default(),event);}game.find_object_by_stable_id(stable).unwrap()}
fn queue(game:&mut GameState)->ironsmith::triggers::TriggerQueue{let mut queue=ironsmith::triggers::TriggerQueue::new();ironsmith::game_loop::drain_pending_trigger_events_with_dm(game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();queue}
#[test]fn actual_source_retains_only_its_own_mill_exile_and_checks_frozen_type(){for definition in definitions(){for creature in [false,true]{for size in [0,1,3]{
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);let unrelated=game.create_object_from_definition(&card("Existing graveyard creature",true),A,Zone::Graveyard);let unrelated_stable=game.object(unrelated).unwrap().stable_id;
 for _ in 0..size{game.create_object_from_definition(&card("Milled resource",creature),A,Zone::Library);}
 let source=game.create_object_from_definition(&definition,A,Zone::Hand);let entered=enter(&mut game,source);assert_eq!(game.object(entered).unwrap().zone,Zone::Battlefield);assert_eq!(game.player(A).unwrap().library.len(),0);
 let exile=game.objects_in_zone(Zone::Exile);assert_eq!(exile.len(),usize::from(size>0));assert_eq!(game.object(game.find_object_by_stable_id(unrelated_stable).unwrap()).unwrap().zone,Zone::Graveyard);
 let snapshots=game.object(entered).unwrap().cast_tagged_objects.get(&ironsmith::tag::TagKey::from("__entry_action_exiled__")).cloned().unwrap_or_default();assert_eq!(snapshots.len(),usize::from(size>0));
 let mut pending=queue(&mut game);assert_eq!(pending.entries.len(),usize::from(creature&&size>0));
 if creature&&size>0{ironsmith::game_loop::put_triggers_on_stack(&mut game,&mut pending).unwrap();
 // Leaving exile after the successful action cannot rewrite the historical type.
 let exiled=exile[0];let movement=ironsmith::Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::ChooseSpec::SpecificObject(exiled),Zone::Hand,false));ironsmith::effects::execute_effect(&mut game,&movement,&mut ironsmith::effects::EffectContext::new_default(entered,A)).unwrap();
 let departure=ironsmith::Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::ChooseSpec::SpecificObject(entered),Zone::Hand,false));ironsmith::effects::execute_effect(&mut game,&departure,&mut ironsmith::effects::EffectContext::new_default(entered,A)).unwrap();
 ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();assert!(game.player(A).unwrap().hand.iter().any(|id|game.object(*id).is_some_and(|object|object.stable_id==unrelated_stable)));}
}}}}
#[test]fn entry_source_new_incarnation_does_not_inherit_previous_entry_receipt(){for definition in definitions(){
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.create_object_from_definition(&card("Creature to mill",true),A,Zone::Library);let source=game.create_object_from_definition(&definition,A,Zone::Hand);let source=enter(&mut game,source);assert!(game.object(source).unwrap().cast_tagged_objects.contains_key(&ironsmith::tag::TagKey::from("__entry_action_exiled__")));
 let stable=game.object(source).unwrap().stable_id;let movement=ironsmith::Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::ChooseSpec::SpecificObject(source),Zone::Hand,false));ironsmith::effects::execute_effect(&mut game,&movement,&mut ironsmith::effects::EffectContext::new_default(source,A)).unwrap();let hand=game.find_object_by_stable_id(stable).unwrap();let second=enter(&mut game,hand);assert!(game.object(second).unwrap().cast_tagged_objects.get(&ironsmith::tag::TagKey::from("__entry_action_exiled__")).is_none_or(|snapshots|snapshots.is_empty()));
}}

#[test]fn replacement_exiles_during_mill_are_not_eligible_authored_entry_exiles(){for definition in definitions(){
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.create_object_from_definition(&card("Existing target",true),A,Zone::Graveyard);
 let replacement=ironsmith_compiler_runtime::compile_to_runtime_definition("Mill destination replacement","Type: Enchantment\nIf a card would be put into your graveyard from your library, exile it instead.",false).unwrap();game.create_object_from_definition(&replacement,A,Zone::Battlefield);for _ in 0..3{game.create_object_from_definition(&card("Replaced mill",true),A,Zone::Library);}
 let source=game.create_object_from_definition(&definition,A,Zone::Hand);let source=enter(&mut game,source);assert_eq!(game.objects_in_zone(Zone::Exile).len(),3);assert!(game.object(source).unwrap().cast_tagged_objects.get(&ironsmith::tag::TagKey::from("__entry_action_exiled__")).is_none_or(|snapshots|snapshots.is_empty()));assert!(queue(&mut game).entries.is_empty());
}}
#[test]fn distinct_entry_sources_keep_independent_frozen_recipients(){for definition in definitions(){
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.create_object_from_definition(&card("First receipt",true),A,Zone::Library);let first=game.create_object_from_definition(&definition,A,Zone::Hand);let first=enter(&mut game,first);game.create_object_from_definition(&card("Second receipt",false),A,Zone::Library);let second=game.create_object_from_definition(&definition,A,Zone::Hand);let second=enter(&mut game,second);let key=ironsmith::tag::TagKey::from("__entry_action_exiled__");let one=&game.object(first).unwrap().cast_tagged_objects[&key];let two=&game.object(second).unwrap().cast_tagged_objects[&key];assert_eq!(one.len(),1);assert_eq!(two.len(),1);assert_ne!(one[0].stable_id,two[0].stable_id);assert!(one[0].card_types.contains(&ironsmith::CardType::Creature));assert!(!two[0].card_types.contains(&ironsmith::CardType::Creature));
}}
