//! Actual source preparation -> actual spell resolution -> delayed cast.
//! Authored/unrun. No synthetic SpellResolvedEvent or source body substitution.
#[path="custom_final_next_wave_support.rs"] mod source_routes;
use ironsmith::{GameState,PlayerId,ObjectId,Zone};
use ironsmith::decision::{LegalAction,SelectFirstDecisionMaker,compute_legal_actions};
use ironsmith::special_actions::{SpecialAction,perform};
use ironsmith::mana::ManaSymbol;
const A:PlayerId=PlayerId(0);
fn sources()->Vec<serde_json::Value>{let f:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-orchestrate-full-19.json")).unwrap();f["mechanics"][0]["representative_cards"].as_array().unwrap().clone()}
fn source(name:&str)->serde_json::Value{sources().into_iter().find(|row|row["name"]==name).unwrap()}
fn ready()->GameState{let mut g=GameState::new(vec!["A".into(),"B".into()],20);g.turn.active_player=A;g.turn.priority_player=Some(A);g.turn.phase=ironsmith::game_state::Phase::FirstMain;g}
fn fund(g:&mut GameState){for symbol in [ManaSymbol::White,ManaSymbol::Blue,ManaSymbol::Black,ManaSymbol::Red,ManaSymbol::Green,ManaSymbol::Colorless]{g.player_mut(A).unwrap().mana_pool.add(symbol,20);}}
fn add(g:&mut GameState,name:&str,text:&str,zone:Zone)->ObjectId{let d=ironsmith_compiler_runtime::compile_to_runtime_definition(name,text,false).unwrap();g.create_object_from_definition(&d,A,zone)}
fn materials_and_targets(g:&mut GameState){
 for n in ["Price land one","Price land two","Target land"]{add(g,n,"Type: Basic Land — Forest",Zone::Battlefield);}
 let tapped=add(g,"Red price and target creature","Mana cost: {R}\nType: Creature\nPower/Toughness: 2/2",Zone::Battlefield);
 add(g,"White price and target creature","Mana cost: {W}\nType: Creature\nPower/Toughness: 2/2",Zone::Battlefield);
 add(g,"Second red price creature","Mana cost: {R}\nType: Creature\nPower/Toughness: 2/2",Zone::Battlefield);
 g.tap(tapped);
 add(g,"Destructible artifact","Type: Artifact",Zone::Battlefield);
 for n in ["Library one","Library two","Library three"]{add(g,n,"Type: Basic Land — Forest",Zone::Library);}
}
fn cast_zero_mana_witness(g:&mut GameState){
 let witness=add(g,"Real resolution witness","Mana cost: {0}\nType: Sorcery\nYou gain 1 life.",Zone::Hand);
 let action=compute_legal_actions(g,A).unwrap().into_iter().find(|action|matches!(action,LegalAction::CastSpell{spell_id,..}if *spell_id==witness)).unwrap();
 let mut queue=ironsmith::triggers::TriggerQueue::new();let mut state=ironsmith::game_loop::PriorityLoopState::new(g.players.len());let mut dm=SelectFirstDecisionMaker;
 let mut progress=ironsmith::game_loop::apply_priority_response_with_dm(g,&mut queue,&mut state,&ironsmith::game_loop::PriorityResponse::PriorityAction(action),&mut dm).unwrap();
 for _ in 0..32{if state.pending_cast.is_none(){break;}let ironsmith::decision::GameProgress::NeedsDecisionCtx(context)=progress else{panic!("cast stopped before actual witness was stacked: {progress:?}")};progress=ironsmith::game_loop::apply_decision_context_with_dm(g,&mut queue,&mut state,&context,&mut dm).unwrap();}
 assert!(state.pending_cast.is_none());assert!(!g.stack.last().unwrap().is_ability);
 ironsmith::game_loop::resolve_stack_entry_with(g,&mut dm).unwrap();
 // Native resolution emitted the event. Its actual trigger production owns
 // one-shot consumption and the delayed ability's normal response window.
 ironsmith::game_loop::drain_pending_trigger_events_with_dm(g,&mut queue,&mut dm).unwrap();
 ironsmith::game_loop::put_triggers_on_stack_with_dm(g,&mut queue,&mut dm).unwrap();
 assert!(g.stack.last().is_some_and(|entry|entry.is_ability),"real resolution must produce delayed trigger");
 assert!(g.effect_store.delayed_triggers.is_empty(),"next-resolution watcher is one-shot");
 ironsmith::game_loop::resolve_stack_entry_with(g,&mut dm).unwrap();
}
#[test]
fn every_eighteen_intrinsic_sources_prepare_and_cast_after_actual_resolution_on_both_routes(){
 let rows=sources();assert_eq!(rows.len(),19);let mut checked=0;
 for row in rows.into_iter().filter(|row|row["name"]!="Reytali, Song of the Sand"){
  for definition in source_routes::definitions(&row).unwrap(){
   let mut g=ready();fund(&mut g);materials_and_targets(&mut g);
   let source=g.create_object_from_definition(&definition,A,Zone::Hand);let stable=g.object(source).unwrap().stable_id;
   let preparation=compute_legal_actions(&g,A).unwrap().into_iter().find_map(|action|match action{LegalAction::SpecialAction(action@SpecialAction::PrepareExile{card_id,..})if card_id==source=>Some(action),_=>None}).unwrap_or_else(||panic!("{} actual preparation missing",row["name"]));
   perform(preparation,&mut g,A,&mut SelectFirstDecisionMaker).unwrap();
   assert!(g.stack.is_empty());assert_eq!(g.effect_store.delayed_triggers.len(),1);
   let exiled=g.find_object_by_stable_id(stable).unwrap();assert_eq!(g.object(exiled).unwrap().zone,Zone::Exile);
   cast_zero_mana_witness(&mut g);
   let spell=g.stack.iter().find(|entry|!entry.is_ability && g.object(entry.object_id).is_some_and(|object|object.stable_id==stable)).unwrap_or_else(||panic!("{} delayed cast missing",row["name"]));
   let object=g.object(spell.object_id).unwrap();
   assert!(object.optional_costs_paid.paid_custom_keyword("orchestrate"),"{} selected preparation identity must survive to actual cast",row["name"]);
   assert_eq!(object.zone,Zone::Stack);
   checked+=1;
  }
 }
 assert_eq!(checked,36,"all eighteen complete source definitions exercised independently");
}
#[test]
fn exact_reytali_grant_preserves_authored_preparation_identity_for_return_to_form(){
 for (provider,recipient) in source_routes::definitions(&source("Reytali, Song of the Sand")).unwrap().into_iter().zip(source_routes::definitions(&source("Return to Form")).unwrap()){
  let mut g=ready();fund(&mut g);materials_and_targets(&mut g);
  let donor=g.create_object_from_definition(&provider,A,Zone::Battlefield);
  let card=g.create_object_from_definition(&recipient,A,Zone::Hand);let stable=g.object(card).unwrap().stable_id;
  let action=compute_legal_actions(&g,A).unwrap().into_iter().find_map(|action|match action{LegalAction::SpecialAction(action) if matches!(&action,SpecialAction::PrepareGrantedExile{card_id,grant,..} if *card_id==card && grant.source==donor)=>Some(action),_=>None}).expect("exact source grants independently selectable preparation");
  perform(action,&mut g,A,&mut SelectFirstDecisionMaker).unwrap();
  // The preparation occurrence owns its completed grant; donor departure
  // cannot erase the delayed cast or substitute recipient intrinsic price.
  g.move_object_by_effect(donor,Zone::Graveyard).unwrap();
  cast_zero_mana_witness(&mut g);
  let entry=g.stack.iter().find(|entry|!entry.is_ability && g.object(entry.object_id).is_some_and(|object|object.stable_id==stable)).unwrap();
  assert!(g.object(entry.object_id).unwrap().optional_costs_paid.paid_custom_keyword("orchestrate"),"Return to Form's printed conditional must observe the granted authored payment");
 }
}
fn stack_normal_hand_spell(g:&mut GameState,source:ObjectId){
 let action=compute_legal_actions(g,A).unwrap().into_iter().find(|action|matches!(action,LegalAction::CastSpell{spell_id,..}if *spell_id==source)).unwrap();
 let mut queue=ironsmith::triggers::TriggerQueue::new();let mut state=ironsmith::game_loop::PriorityLoopState::new(g.players.len());let mut dm=SelectFirstDecisionMaker;
 let mut progress=ironsmith::game_loop::apply_priority_response_with_dm(g,&mut queue,&mut state,&ironsmith::game_loop::PriorityResponse::PriorityAction(action),&mut dm).unwrap();
 for _ in 0..32{if state.pending_cast.is_none(){return;}let ironsmith::decision::GameProgress::NeedsDecisionCtx(context)=progress else{panic!("native source cast stopped: {progress:?}")};progress=ironsmith::game_loop::apply_decision_context_with_dm(g,&mut queue,&mut state,&context,&mut dm).unwrap();}
 panic!("native source cast did not finish announcement");
}
fn stack_exact_source(g:&mut GameState,definition:&ironsmith::CardDefinition,from_exile:bool)->ironsmith::ids::StableId{
 let source=g.create_object_from_definition(definition,A,Zone::Hand);let stable=g.object(source).unwrap().stable_id;
 if from_exile{
  let action=compute_legal_actions(g,A).unwrap().into_iter().find_map(|action|match action{LegalAction::SpecialAction(action@SpecialAction::PrepareExile{card_id,..})if card_id==source=>Some(action),_=>None}).unwrap();
  perform(action,g,A,&mut SelectFirstDecisionMaker).unwrap();cast_zero_mana_witness(g);
 }else{stack_normal_hand_spell(g,source);}
 assert!(g.stack.iter().any(|entry|!entry.is_ability && g.object(entry.object_id).is_some_and(|object|object.stable_id==stable)));
 stable
}
#[test]
fn exact_shatter_facade_switches_mana_value_condition_only_when_actually_cast_from_exile(){
 for definition in source_routes::definitions(&source("Shatter Facade")).unwrap(){for from_exile in [false,true]{for high in [false,true]{
  let mut g=ready();fund(&mut g);
  let target=add(&mut g,"Only Shatter Facade target",if high{"Mana cost: {5}\nType: Creature\nPower/Toughness: 2/2"}else{"Mana cost: {1}\nType: Creature\nPower/Toughness: 2/2"},Zone::Battlefield);
  let target_stable=g.object(target).unwrap().stable_id;
  stack_exact_source(&mut g,&definition,from_exile);
  ironsmith::game_loop::resolve_stack_entry_with(&mut g,&mut SelectFirstDecisionMaker).unwrap();
  let target=g.find_object_by_stable_id(target_stable).unwrap();
  assert_eq!(g.object(target).unwrap().zone,if high!=from_exile{Zone::Graveyard}else{Zone::Battlefield},"printed threshold must invert only for actual exile cast");
 }}}
}
#[test]
fn exact_thunderous_charge_adds_four_power_and_toughness_only_for_actual_exile_cast(){
 for definition in source_routes::definitions(&source("Thunderous Charge")).unwrap(){for from_exile in [false,true]{
  let mut g=ready();fund(&mut g);
  let target=add(&mut g,"Only Thunderous Charge target","Mana cost: {2}\nType: Creature\nPower/Toughness: 2/2",Zone::Battlefield);
  stack_exact_source(&mut g,&definition,from_exile);
  ironsmith::game_loop::resolve_stack_entry_with(&mut g,&mut SelectFirstDecisionMaker).unwrap();
  assert_eq!(g.calculated_power(target),Some(if from_exile{6}else{2}));
  assert_eq!(g.calculated_toughness(target),Some(if from_exile{6}else{2}));
  assert!(g.current_has_static_ability_id(target,ironsmith_core::StaticAbilityId::Trample));
 }}
}
