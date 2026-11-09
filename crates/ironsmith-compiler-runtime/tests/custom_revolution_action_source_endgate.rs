//! Exact physical scope, independent strict native/artifact routes and actual source programs.
#[path="custom_final_next_wave_support.rs"] mod source_support;
#[path="cf8_p08/play.rs"] mod play;
use ironsmith::{CardDefinition,PlayerId,Zone};
use ironsmith::effects::{EffectContext,execute_effect};
use ironsmith::decision::{compute_legal_actions,LegalAction};
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
fn fixture()->serde_json::Value {serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/revolution-actions-source-endgate.json")).unwrap()}
fn routes(name:&str)->[CardDefinition;2] {
 let fixture=fixture();let card=fixture["mechanics"].as_array().unwrap().iter().flat_map(|row|row["representative_cards"].as_array().unwrap()).find(|card|card["name"]==name).unwrap();
 source_support::definitions(card).unwrap_or_else(|error|panic!("{name}: {error}"))
}
fn ordinary(name:&str,text:&str)->CardDefinition {source_support::definitions(&serde_json::json!({"name":name,"executable_text":text})).unwrap()[0].clone()}
#[test]
fn all_143_exact_physical_sources_and_144_faces_admit_independent_strict_routes() {
 let fixture=fixture();let mut keys=std::collections::HashSet::new();let mut faces=0;
 for row in fixture["mechanics"].as_array().unwrap() {
  let expected=match row["keyword"].as_str().unwrap(){"Acclimate"=>13,"Dare"=>27,"Flow"=>13,"Revered"=>15,"Wander"=>40,"Crystallize"=>35,_=>panic!("unowned family")};
  let cards=row["representative_cards"].as_array().unwrap();assert_eq!(cards.len(),expected);
  for card in cards {
   let key=card["source_card_key"].as_str().unwrap();assert!(keys.insert(key.to_string()),"duplicate physicalsource {key}");
   assert_eq!(card["original_source_record"]["key"],key);assert!(!card["original_source_record"]["rulesText"].as_str().unwrap().is_empty());
   faces+=card["source_faces"].as_array().unwrap().len();
   for definition in source_support::definitions(card).unwrap_or_else(|error|panic!("{} {key}: {error}",row["keyword"])) {
    assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
    if card["source_faces"].as_array().unwrap().len()==2 {assert!(definition.card.other_face.is_some(),"physical linked back retained");}
   }
  }
 }
 assert_eq!(keys.len(),143);assert_eq!(faces,144);
}
#[test]
fn actual_capricious_cat_acclimates_twice_and_fallback_belongs_to_source() {
 for definition in routes("Capricious Cat") {for accept in [false,true] {
  let mut game=play::game();let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
  let other=game.create_object_from_definition(&ordinary("Other","Type: Creature\nPower/Toughness: 1/1"),A,Zone::Battlefield);
  let weather=source_support::definitions(&serde_json::json!({"name":"Weather witness","executable_text":"Type: Enchantment — Weather","custom_subtypes":[{"singular":"weather","plural":"weather","family":"Enchantment"}]})).unwrap()[0].clone();
  let choice=game.create_object_from_definition(&weather,A,Zone::Graveyard);let stable=game.object(choice).unwrap().stable_id;
  let trigger=definition.abilities.iter().find_map(|ability|match &ability.kind {ironsmith::ability::AbilityKind::Triggered(trigger)=>Some(trigger),_=>None}).unwrap();
  let mut dm=play::Script{decline:!accept,..Default::default()};let mut ctx=EffectContext::new(source,A,&mut dm);
  for effect in trigger.effects.all_effects(){execute_effect(&mut game,effect,&mut ctx).unwrap();}
  assert_eq!(game.counter_count(source,ironsmith::CounterType::PlusOnePlusOne),if accept {1}else{2});
  assert_eq!(game.counter_count(other,ironsmith::CounterType::PlusOnePlusOne),0);
  assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone,if accept {Zone::Hand}else{Zone::Graveyard});
 }}
}
#[test]
fn actual_gato_dare_pays_its_activation_and_preserves_other_characteristics_until_cleanup() {
 for definition in routes("Gato Vigilante") {
  let mut game=play::game();let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);let before=game.current_characteristics(source).unwrap();
  let old_power=game.calculated_power(source);let old_subtypes=before.subtypes.clone();
  play::give_mana(&mut game,A,ironsmith::mana::ManaSymbol::White,1);play::give_mana(&mut game,A,ironsmith::mana::ManaSymbol::Colorless,1);
  play::activate(&mut game,source,play::activated_at(&definition,0),&mut play::Script::default());assert_eq!(game.player(A).unwrap().mana_pool.total(),0);
  play::resolve_all(&mut game,&mut play::Script::default());
  let view=game.current_characteristics(source).unwrap();assert!(view.supertypes.contains(&ironsmith::types::Supertype::Legendary));assert!(view.subtypes.contains(&ironsmith::types::Subtype::Rebel));
  assert!(old_subtypes.iter().all(|kind|view.subtypes.contains(kind)));assert_eq!(game.calculated_power(source),Some(4));assert_eq!(game.calculated_toughness(source),Some(4));
  ironsmith::turn::execute_cleanup_step(&mut game);assert_eq!(game.calculated_power(source),old_power);assert!(!game.current_characteristics(source).unwrap().subtypes.contains(&ironsmith::types::Subtype::Rebel));
 }
}
#[test]
fn actual_uiki_flow_uses_native_creature_taps_only_for_generic_mana() {
 for definition in routes("Uiki Initiate") {for white in [false,true] {
  let mut game=play::game();let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
  let body=ordinary("Tapper","Type: Creature\nPower/Toughness: 1/1");let own=game.create_object_from_definition(&body,A,Zone::Battlefield);
  let opposing=game.create_object_from_definition(&body,B,Zone::Battlefield);
  if white{play::give_mana(&mut game,A,ironsmith::mana::ManaSymbol::White,1);}
  let index=play::activated_at(&definition,0);let action=LegalAction::ActivateAbility{source,ability_index:index};
  assert_eq!(compute_legal_actions(&game,A).unwrap().contains(&action),white,"creature tap cannot pay white");
  if !white {continue;}
  play::activate(&mut game,source,index,&mut play::Script::default());
  assert!(game.is_tapped(source));assert!(game.is_tapped(own));assert!(!game.is_tapped(opposing));assert_eq!(game.player(A).unwrap().mana_pool.total(),0);
  play::resolve_all(&mut game,&mut play::Script::default());assert_eq!(game.calculated_power(source),Some(4));assert_eq!(game.calculated_toughness(source),Some(4));
 }}
}
#[test]
fn actual_basking_kitsune_grant_tracks_control_counters_and_provider_departure() {
 for definition in routes("Basking Kitsune") {
  let mut game=play::game();let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
  let body=ordinary("Recipient","Type: Creature\nPower/Toughness: 2/2");let own=game.create_object_from_definition(&body,A,Zone::Battlefield);let other=game.create_object_from_definition(&body,B,Zone::Battlefield);
  assert!(!game.current_has_static_ability_id(own,ironsmith_core::StaticAbilityId::Trample));
  game.add_counters(own,ironsmith::CounterType::Level,1).unwrap();game.add_counters(other,ironsmith::CounterType::Level,1).unwrap();
  assert!(game.current_has_static_ability_id(own,ironsmith_core::StaticAbilityId::Trample));assert!(!game.current_has_static_ability_id(other,ironsmith_core::StaticAbilityId::Trample));
  game.move_object_by_effect(source,Zone::Graveyard).unwrap();assert!(!game.current_has_static_ability_id(own,ironsmith_core::StaticAbilityId::Trample));
 }
}
#[test]
fn actual_breaching_jaxl_wander_is_a_paid_battlefield_cast_with_fresh_entry() {
 use ironsmith::game_loop::{PriorityLoopState,PriorityResponse,apply_priority_response_with_dm,apply_decision_context_with_dm,put_triggers_on_stack_with_dm};
 for definition in routes("Breaching Jaxl") {
  let mut game=play::game();let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);let stable=game.object(source).unwrap().stable_id;
  let target=game.create_object_from_definition(&ordinary("Untap witness","Type: Artifact"),A,Zone::Battlefield);game.tap(target);
  play::give_mana(&mut game,A,ironsmith::mana::ManaSymbol::Blue,1);play::give_mana(&mut game,A,ironsmith::mana::ManaSymbol::Colorless,2);
  let action=compute_legal_actions(&game,A).unwrap().into_iter().find(|action|matches!(action,LegalAction::CastSpell{spell_id,from_zone:Zone::Battlefield,..}if *spell_id==source)).expect("actual printed battlefield price");
  let mut queue=ironsmith::triggers::TriggerQueue::new();let mut state=PriorityLoopState::new(3);let mut dm=play::Script{targets:vec![ironsmith::Target::Object(target)],..Default::default()};
  let mut progress=apply_priority_response_with_dm(&mut game,&mut queue,&mut state,&PriorityResponse::PriorityAction(action),&mut dm).unwrap();
  for _ in 0..60 {if !state.has_pending_action(){break;}let ironsmith::GameProgress::NeedsDecisionCtx(ctx)=progress else {panic!("{progress:?}")};progress=apply_decision_context_with_dm(&mut game,&mut queue,&mut state,&ctx,&mut dm).unwrap();}
  assert!(!state.has_pending_action());put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut dm).unwrap();
  let spell=game.find_object_by_stable_id(stable).unwrap();assert_ne!(spell,source);assert_eq!(game.object(spell).unwrap().zone,Zone::Stack);assert_eq!(game.player(A).unwrap().mana_pool.total(),0);
  for _ in 0..30{put_triggers_on_stack_with_dm(&mut game,&mut ironsmith::triggers::TriggerQueue::new(),&mut dm).unwrap();if game.stack_is_empty(){break;}ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();}let returned=game.find_object_by_stable_id(stable).unwrap();assert_ne!(returned,spell);assert_eq!(game.object(returned).unwrap().zone,Zone::Battlefield);assert!(!game.is_tapped(target));
 }
}

#[test]
fn actual_crystal_lion_chooses_counter_or_treasure_without_targeting() {
 for definition in routes("Crystal Lion") {for decline in [false,true] {
  let mut game=play::game();let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
  let trigger=definition.abilities.iter().find_map(|ability|match &ability.kind {ironsmith::ability::AbilityKind::Triggered(trigger)=>Some(trigger),_=>None}).unwrap();
  let mut dm=play::Script{decline,..Default::default()};let mut ctx=EffectContext::new(source,A,&mut dm);
  for effect in trigger.effects.all_effects(){execute_effect(&mut game,effect,&mut ctx).unwrap();}
  assert_eq!(game.counter_count(source,ironsmith::CounterType::PlusOnePlusOne),u32::from(!decline));
  assert_eq!(game.battlefield.iter().filter(|id|game.object(**id).unwrap().name.as_ref()=="Treasure").count(),usize::from(decline));
 }}
}
#[test]
fn actual_voidform_source_exile_cost_preserves_both_crystallize_actions_and_mill_sibling() {
 for definition in routes("Voidform Stonekin") {
  let mut game=play::game();let source=game.create_object_from_definition(&definition,A,Zone::Graveyard);let stable=game.object(source).unwrap().stable_id;
  for _ in 0..3 {game.create_object_from_definition(&ordinary("Mill witness","Type: Artifact"),A,Zone::Library);}
  play::give_mana(&mut game,A,ironsmith::mana::ManaSymbol::Black,1);play::give_mana(&mut game,A,ironsmith::mana::ManaSymbol::Colorless,3);
  play::activate(&mut game,source,play::activated_at(&definition,0),&mut play::Script{decline:true,..Default::default()});
  assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone,Zone::Exile);assert_eq!(game.player(A).unwrap().mana_pool.total(),0);
  play::resolve_all(&mut game,&mut play::Script{decline:true,..Default::default()});
  assert_eq!(game.battlefield.iter().filter(|id|game.object(**id).unwrap().name.as_ref()=="Treasure").count(),2);
  assert_eq!(game.player(A).unwrap().graveyard.len(),2);assert_eq!(game.player(A).unwrap().library.len(),1);
 }
}
