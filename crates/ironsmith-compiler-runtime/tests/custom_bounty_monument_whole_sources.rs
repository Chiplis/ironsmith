//! Exact physical source admission on independent native and artifact routes.
mod custom_final_next_wave_support;
use ironsmith::{CardDefinition,Effect,GameState,ObjectId,PlayerId,Zone};
use ironsmith::decision::{LegalAction,SelectFirstDecisionMaker,compute_legal_actions};
use ironsmith::effects::{EffectContext,ResolvedTarget,execute_effect};
use serde_json::Value;
use std::collections::BTreeSet;
const A:PlayerId=PlayerId(0); const B:PlayerId=PlayerId(1); const C:PlayerId=PlayerId(2);
fn fixture()->Value {serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-bounty-monument-source-complete.json")).unwrap()}
fn source(key:&str)->Value {fixture()["mechanics"].as_array().unwrap().iter().flat_map(|m|m["representative_cards"].as_array().unwrap()).find(|c|c["source_card_key"]==key).unwrap().clone()}
fn source_routes(key:&str)->Vec<[CardDefinition;2]> {custom_final_next_wave_support::linked_definitions(&source(key)).unwrap_or_else(|error|panic!("{key}: {error}"))}
#[test]
fn every_bounty_and_monument_physical_source_preserves_all_faces_and_compiles_strictly_both_routes(){
 let fixture=fixture(); let mut errors=Vec::new(); let mut total=0;let mut face_count=0;
 for (family,expected) in [("post a bounty", &["Bounty Collector_HI12","Deputy Gibem_HI12","Ghastly Character_HI12","Graygulch Turncoat_HI12","Heavy Moneybag_HI12","Howler Sellout_HI12","Injustice_HI12","Mudslinging_HI12","Nightrail Forger_HI12","On Eli's Orders_OTH","Spectral Slinger_HI12","Spirit of Infamy_HI12","Sunset Cliffs Lawman_HI12","Sunset Cliffs Patron_HI12","Vengeful Agent_HI12","Wanted_HI12","Wild Bounties_HI12"][..]),("monument", &["A Slave's Vengeance//Crown of the Urchin King_OTH","Blessing of Golna//Golna's Heart_ALK","Contemplate the Cosmos//Orrey of the Cosmos_ALK","Ethersketch Blueprints//Etherwrought Figurehead_OTH","Falconry//Skythrone Aviary_ALK","Fight for Succession//Chalice of Fallen Foes_ALK","Fury of Hadak//Hadak's Shrine_ALK","Gathering of Minds//Repository of Thoughts_ALK","Ghulish Unearthing//Obelisk of Absolute Rule_ALK","Golna, the First Shah//The Path of Greatness_ALK","Hidden Cache//Hadak's Guardian_ALK","Hidden Garden Entrance//Cosmic Contraption_ALK","Jahan, the Third Shah//Cup of Jahan_ALK","Peek into the Future//Seer's Astrolabe_ALK","Peer into the Cosmos//Bauble of Infinites_ALK","Piruz, the Fourth Shah//The Spire of Undeath_ALK","Rapid Transformation//Vortex of Change_ALK","Rite of Ends//Mausoleum of Ends_ALK","Sacred Heirloom//Blade of Ancestors_ALK","Shapur, the Fifth Shah//Lamp of Endless Possibilities_ALK","Shift the Sands//Hourglass of Decay_ALK","Shomi's Journey//Shrine to the Sailor_ALK","Zareen, the Second Shah//Throne of Soli_ALK"][..])] {
  let mut observed=BTreeSet::new();
  for row in fixture["mechanics"].as_array().unwrap().iter().filter(|m|m["keyword"].as_str().unwrap().eq_ignore_ascii_case(family)) {
   for card in row["representative_cards"].as_array().unwrap() {
    let key=card["source_card_key"].as_str().unwrap();assert!(observed.insert(key.to_string()),"duplicatephysical:{family}/{key}");total+=1;
    let originals=card["source_faces"].as_array().unwrap();let expected_faces=if family=="monument"{2}else{1};assert_eq!(originals.len(),expected_faces,"{family}/{key}: sourcefacecount");face_count+=originals.len();
    match custom_final_next_wave_support::linked_definitions(card) {
     Err(error)=>errors.push(format!("row={}/card={key}/faces={:?}: {error}",row["id"],originals.iter().map(|f|f["name"].clone()).collect::<Vec<_>>())),
     Ok(routes)=>{assert_eq!(routes.len(),originals.len(),"{key}: no siblingbody may disappear");for (face,pair)in routes.iter().enumerate(){assert_eq!(pair[0].card.name.as_str(),originals[face]["name"].as_str().unwrap());assert_eq!(pair[1].card.name,pair[0].card.name);}}
    }
   }
  }
  assert_eq!(observed,expected.iter().map(|s|s.to_string()).collect::<BTreeSet<_>>(),"exactphysicalcohort:{family}");
 }
 assert_eq!(total,40);assert_eq!(face_count,63);assert!(errors.is_empty(),"all-source strict failures:\n{}",errors.join("\n"));
}
fn reserve()->CardDefinition {ironsmith_compiler_runtime::compile_to_runtime_definition("Independent reserve","Type: Creature\nPower/Toughness: 2/2",false).unwrap()}
fn flush(game:&mut GameState){let mut queue=ironsmith::triggers::TriggerQueue::new();ironsmith::game_loop::drain_pending_trigger_events_with_dm(game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();ironsmith::game_loop::put_triggers_on_stack_with_dm(game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();while !game.stack.is_empty(){ironsmith::game_loop::resolve_stack_entry_with(game,&mut SelectFirstDecisionMaker).unwrap();}}
#[test]
fn actual_wanted_repeated_postings_are_occurrences_not_counter_quantity_and_use_recipient_opponents(){
 for route in 0..2 {let definition=source_routes("Wanted_HI12")[0][route].clone();
  for (postings,additional_counters,removed,expected_draws) in [(1,1,0,1),(2,0,1,2),(2,0,2,0)] {
   let mut game=GameState::new(vec!["A".into(),"B".into(),"C".into()],20);let reserve=reserve();
   let marked=game.create_object_from_definition(&reserve,B,Zone::Battlefield);
   for player in [A,B,C]{for _ in 0..12{game.create_object_from_definition(&reserve,player,Zone::Library);}}
   for _ in 0..postings {
    let spell=game.create_object_from_definition(&definition,A,Zone::Stack);
    let program=Effect::new(ironsmith::effects::SequenceEffect::new(definition.spell_effect.as_ref().unwrap().all_effects_owned()));
    execute_effect(&mut game,&program,&mut EffectContext::new_default(spell,A).with_targets(vec![ResolvedTarget::Object(marked)])).unwrap();
    game.move_object_by_effect(spell,Zone::Graveyard).unwrap();
   }
   let bounty=ironsmith::object::CounterType::Named("bounty".into());
   if additional_counters>0{game.object_mut(marked).unwrap().counters.insert(bounty.clone(),(postings+additional_counters)as u32);}
   if removed>0{game.object_mut(marked).unwrap().remove_counters(bounty,removed);}
   let before=[A,B,C].map(|player|game.player(player).unwrap().hand.len());
   game.move_object(marked,Zone::Graveyard,ironsmith::events::EventCause::effect()).unwrap();flush(&mut game);
   let after=[A,B,C].map(|player|game.player(player).unwrap().hand.len());
   assert_eq!(after,[before[0]+expected_draws,before[1],before[2]+expected_draws],"postings={postings},extra={additional_counters},removed={removed}; opponent is relative to marked B");
  }
 }
}
fn announce(game:&mut GameState,action:LegalAction){
 let mut queue=ironsmith::triggers::TriggerQueue::new();let mut state=ironsmith::game_loop::PriorityLoopState::new(2);let mut dm=SelectFirstDecisionMaker;
 let mut progress=ironsmith::game_loop::apply_priority_response_with_dm(game,&mut queue,&mut state,&ironsmith::game_loop::PriorityResponse::PriorityAction(action),&mut dm).unwrap();
 for _ in 0..96 {if !state.has_pending_action(){return}let ironsmith::GameProgress::NeedsDecisionCtx(ctx)=progress else{panic!("{progress:?}")};progress=ironsmith::game_loop::apply_decision_context_with_dm(game,&mut queue,&mut state,&ctx,&mut dm).unwrap();}panic!("cast did not finish")
}
fn move_effect(game:&mut GameState,id:ObjectId,to:Zone)->ObjectId {let stable=game.object(id).unwrap().stable_id;execute_effect(game,&Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::target::ChooseSpec::SpecificObject(id),to,false)),&mut EffectContext::new_default(id,A)).unwrap();game.find_object_by_stable_id(stable).unwrap()}
#[test]
fn actual_falconry_monument_casts_back_face_on_stack_and_exiles_only_selected_method_incarnation(){
 let pairs=source_routes("Falconry//Skythrone Aviary_ALK");assert_eq!(pairs.len(),2);
 for route in 0..2{let front=&pairs[0][route];let back=&pairs[1][route];
  for monument in [false,true]{
   let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.register_linked_face_definition(front);game.register_linked_face_definition(back);
   game.turn.active_player=A;game.turn.priority_player=Some(A);game.turn.phase=ironsmith::game_state::Phase::FirstMain;game.turn.step=None;
   let source=game.create_object_from_definition(front,A,if monument{Zone::Graveyard}else{Zone::Hand});let stable=game.object(source).unwrap().stable_id;
   game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::White,1);game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,2);
   let action=compute_legal_actions(&game,A).unwrap().into_iter().find(|a|matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==source)).expect("actual source has chosen-zone cast");announce(&mut game,action);
   let stack=game.stack.last().unwrap().object_id;assert_eq!(game.object(stack).unwrap().name.as_str(),if monument{"Skythrone Aviary"}else{"Falconry"},"702.255 casts transformed, distinct from front-program-then-transform Memento");
   if monument{
    for zone in [Zone::Hand,Zone::Graveyard,Zone::Library]{let mut interrupted=game.clone();let arrival=move_effect(&mut interrupted,stack,zone);assert_eq!(interrupted.object(arrival).unwrap().zone,Zone::Exile);}
   }
   ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();let result=game.find_object_by_stable_id(stable).unwrap();
   if monument{
    assert_eq!(game.object(result).unwrap().name.as_str(),"Skythrone Aviary");assert_eq!(game.object(result).unwrap().zone,Zone::Battlefield);assert!(game.battlefield.iter().all(|id|game.object(*id).unwrap().kind != ironsmith::object::ObjectKind::Token),"front Bird instruction never ran");
    let exile=move_effect(&mut game,result,Zone::Hand);assert_eq!(game.object(exile).unwrap().zone,Zone::Exile);
    let hand=move_effect(&mut game,exile,Zone::Hand);let grave=move_effect(&mut game,hand,Zone::Graveyard);assert_eq!(game.object(grave).unwrap().zone,Zone::Graveyard,"newincarnation has no stale methodreplacement");
   }else{
    assert_eq!(game.object(result).unwrap().zone,Zone::Graveyard);assert_eq!(game.battlefield.iter().filter(|id|game.object(**id).unwrap().kind == ironsmith::object::ObjectKind::Token).count(),1,"normalfront creates Bird");
   }
  }
 }
}
