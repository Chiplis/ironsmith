//! Exact whole-corpus source inputs, strict independent native/wire routes.
#[path="custom_final_next_wave_support.rs"] mod source_support;
use ironsmith::{CardDefinition,GameState,ObjectId,PlayerId,Zone};
use ironsmith::decision::{LegalAction,SelectFirstDecisionMaker,compute_legal_actions};
use ironsmith::mana::ManaSymbol;
use ironsmith::object::CounterType;
const A:PlayerId=PlayerId(0); const B:PlayerId=PlayerId(1);
fn fixture()->serde_json::Value {serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-growth-whole-source.json")).unwrap()}
fn source(key:&str)->serde_json::Value {let f=fixture();f["mechanics"].as_array().unwrap().iter().flat_map(|r|r["representative_cards"].as_array().unwrap()).find(|c|c["source_card_key"]==key).unwrap().clone()}
fn definitions(key:&str)->[CardDefinition;2]{source_support::definitions(&source(key)).unwrap()}
fn game()->GameState {let mut g=GameState::new(vec!["A".into(),"B".into()],20);g.turn.active_player=A;g.turn.priority_player=Some(A);g.turn.phase=ironsmith::game_state::Phase::FirstMain;g.turn.step=None;g}
fn announce(g:&mut GameState,action:LegalAction){
 use ironsmith::game_loop::{PriorityLoopState,PriorityResponse,apply_priority_response_with_dm,apply_decision_context_with_dm};
 let mut s=PriorityLoopState::new(2);let mut q=ironsmith::triggers::TriggerQueue::new();let mut dm=SelectFirstDecisionMaker;
 let mut progress=apply_priority_response_with_dm(g,&mut q,&mut s,&PriorityResponse::PriorityAction(action),&mut dm).unwrap();
 for _ in 0..64 {if !s.has_pending_action(){return}let ironsmith::GameProgress::NeedsDecisionCtx(ctx)=progress else{panic!("{progress:?}")};progress=apply_decision_context_with_dm(g,&mut q,&mut s,&ctx,&mut dm).unwrap();}panic!("announcement unfinished");
}
fn activate(g:&mut GameState,id:ObjectId){let a=compute_legal_actions(g,A).unwrap().into_iter().find(|a|matches!(a,LegalAction::ActivateAbility{source,..}if *source==id)).expect("actual source activation legal");announce(g,a);ironsmith::game_loop::resolve_stack_entry_with(g,&mut SelectFirstDecisionMaker).unwrap();}
fn settle(g:&mut GameState){let mut q=ironsmith::triggers::TriggerQueue::new();let mut dm=SelectFirstDecisionMaker;for _ in 0..32 {ironsmith::game_loop::drain_pending_trigger_events_with_dm(g,&mut q,&mut dm).unwrap();ironsmith::game_loop::put_triggers_on_stack_with_dm(g,&mut q,&mut dm).unwrap();if g.stack.is_empty(){return}ironsmith::game_loop::resolve_stack_entry_with(g,&mut dm).unwrap();}panic!("trigger loop unfinished");}
#[test]
fn all_seventy_six_physical_sources_retain_all_faces_and_compile_on_independent_routes(){
 let f=fixture();let mut failures=Vec::new();let mut physical=0;let mut faces=0;
 for row in f["mechanics"].as_array().unwrap(){let keyword=row["keyword"].as_str().unwrap();let expected=match keyword{"Acquiesce"=>22,"Ascend"=>10,"Improve"=>41,"Infamy"|"Simulacrum"|"Successor"=>1,_=>panic!("unexpected family")};assert_eq!(row["representative_cards"].as_array().unwrap().len(),expected);
 for c in row["representative_cards"].as_array().unwrap(){physical+=1;assert_eq!(c["source_card_key"],c["original_source_record"]["key"]);if c["rewrite_status"]!="frontend_materialized_unverified"{failures.push(format!("{}: {}",c["name"],c["rewrite_status"]));continue}
 match source_support::linked_definitions(c){Ok(pair)=>{assert_eq!(pair.len(),c["source_faces"].as_array().unwrap().len());for [native,wire] in pair{assert_eq!(ironsmith_text::compiled_text_lines(&native),ironsmith_text::compiled_text_lines(&wire));faces+=1;}},Err(e)=>failures.push(format!("{}: {e}",c["source_card_key"]))}
 }}assert_eq!(physical,76);assert!(failures.is_empty(),"{}",failures.join("\n"));assert_eq!(faces,86);
}
#[test]
fn actual_ascend_exiles_old_incarnation_and_returns_linked_back_with_its_abilities(){
 let pairs=source_support::linked_definitions(&source("Bundan Firebrand//Kanlan Jaded_KLC")).unwrap();assert_eq!(pairs.len(),2);
 for route in 0..2 {let mut g=game();for p in &pairs{g.register_linked_face_definition(&p[route]);}let id=g.create_object_from_definition(&pairs[0][route],A,Zone::Battlefield);let stable=g.object(id).unwrap().stable_id;
 g.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Black,1);g.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless,1);activate(&mut g,id);
 let returned=g.find_object_by_stable_id(stable).unwrap();assert_ne!(returned,id);assert_eq!(g.object(returned).unwrap().zone,Zone::Battlefield);assert_eq!(g.object(returned).unwrap().name.to_string(),"Kanlan Jaded");assert_eq!(g.calculated_power(returned),Some(2));assert_eq!(g.calculated_toughness(returned),Some(3));assert!(g.current_has_static_ability_id(returned,ironsmith_core::StaticAbilityId::Deathtouch));assert_eq!(g.player(A).unwrap().mana_pool.total(),0);
 }
}
#[test]
fn actual_successor_pays_graveyard_exile_and_creates_the_sourced_one_one_without_mana_cost(){
 for d in definitions("Imitator of Success_MSC"){let mut g=game();let id=g.create_object_from_definition(&d,A,Zone::Graveyard);let stable=g.object(id).unwrap().stable_id;g.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Blue,2);g.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless,1);activate(&mut g,id);let old=g.find_object_by_stable_id(stable).unwrap();assert_eq!(g.object(old).unwrap().zone,Zone::Exile);assert_eq!(g.player(A).unwrap().mana_pool.total(),0);let tokens:Vec<_>=g.battlefield.iter().copied().filter(|id|g.object(*id).unwrap().kind==ironsmith::ObjectKind::Token).collect();assert_eq!(tokens.len(),1);let token=tokens[0];assert_eq!(g.calculated_power(token),Some(1));assert_eq!(g.calculated_toughness(token),Some(1));assert!(g.object(token).unwrap().mana_cost.is_none());}
}
#[test]
fn actual_simulacrum_requires_the_exact_dead_card_still_in_your_graveyard(){
 for d in definitions("Voidtouched Nightblade_OTH"){for removed in [false,true]{let mut g=game();let id=g.create_object_from_definition(&d,A,Zone::Battlefield);g.take_pending_trigger_events();let dead=g.move_object(id,Zone::Graveyard,ironsmith::events::EventCause::effect()).unwrap();if removed{g.move_object(dead,Zone::Hand,ironsmith::events::EventCause::effect()).unwrap();}settle(&mut g);let tokens:Vec<_>=g.battlefield.iter().copied().filter(|id|g.object(*id).unwrap().kind==ironsmith::ObjectKind::Token).collect();assert_eq!(tokens.len(),usize::from(!removed));if !removed {let token=tokens[0];assert_eq!(g.object(dead).map(|o|o.zone),None);assert_eq!(g.calculated_power(token),Some(1));assert_eq!(g.calculated_toughness(token),Some(1));assert!(g.object(token).unwrap().mana_cost.is_none());assert!(g.object(token).unwrap().card_types.contains(&ironsmith::types::CardType::Artifact));assert!(g.object(token).unwrap().card_types.contains(&ironsmith::types::CardType::Creature));}}}
}
#[test]
fn actual_improve_still_uses_recipient_source_counters_and_only_its_own_price(){
 for key in ["Artificer's Familiar_SWR","Corym, Gyrewood Ranger_SWR"]{for d in definitions(key){let mut g=game();let id=g.create_object_from_definition(&d,A,Zone::Battlefield);g.object_mut(id).unwrap().counters.insert(CounterType::PlusOnePlusOne,2);let colored=if key.starts_with("Corym"){ManaSymbol::Green}else{ManaSymbol::Blue};g.player_mut(A).unwrap().mana_pool.add(colored,1);g.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless,2);assert!(!compute_legal_actions(&g,A).unwrap().iter().any(|a|matches!(a,LegalAction::ActivateAbility{source,..}if *source==id)),"one generic short");g.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless,1);activate(&mut g,id);assert_eq!(g.counter_count(id,CounterType::PlusOnePlusOne),if key.starts_with("Corym"){4}else{3});assert_eq!(g.player(A).unwrap().mana_pool.total(),0);}}
}
fn damage(g:&mut GameState,source:ObjectId,combat:bool){use ironsmith::effects::{DealDamageEffect,EffectExecutor,EffectContext};let result=DealDamageEffect::new(1,ironsmith::target::ChooseSpec::SpecificPlayer(B)).with_combat(combat).execute(g,&mut EffectContext::new_default(source,A)).unwrap();for event in result.events{g.queue_trigger_event(event.provenance(),event);}settle(g);}
#[test]
fn actual_infamy_follows_formal_status_gate_independently_of_external_and_removed_counters(){
 for d in definitions("Alpha Scout_OTH"){let mut g=game();let id=g.create_object_from_definition(&d,A,Zone::Battlefield);g.object_mut(id).unwrap().counters.insert(CounterType::PlusOnePlusOne,2);g.take_pending_trigger_events();damage(&mut g,id,false);assert_eq!(g.counter_count(id,CounterType::PlusOnePlusOne),2);damage(&mut g,id,true);assert_eq!(g.counter_count(id,CounterType::PlusOnePlusOne),3);assert!(g.has_custom_status(id,"infamous"));assert_eq!(g.battlefield.len(),2,"source's independent becomes-infamous watcher creates exactly one Wolf");g.object_mut(id).unwrap().counters.remove(&CounterType::PlusOnePlusOne);damage(&mut g,id,true);assert_eq!(g.counter_count(id,CounterType::PlusOnePlusOne),0);assert_eq!(g.battlefield.len(),2,"removing counters never erases the formal designation");}
}
#[test]
fn actual_acquiesce_cast_from_hand_exiles_then_combat_recasts_without_mana_and_does_not_reexile(){
 for d in definitions("Crooked Growth_BYK"){let mut g=game();let spell=g.create_object_from_definition(&d,A,Zone::Hand);let stable=g.object(spell).unwrap().stable_id;let witness=ironsmith_compiler_runtime::compile_to_runtime_definition("Ordinary creature","Type: Creature\nPower/Toughness: 1/1",false).unwrap();let attacker=g.create_object_from_definition(&witness,A,Zone::Battlefield);for _ in 0..3{g.create_object_from_definition(&witness,A,Zone::Library);}g.take_pending_trigger_events();g.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Green,1);g.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless,2);let action=compute_legal_actions(&g,A).unwrap().into_iter().find(|a|matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==spell)).unwrap();announce(&mut g,action);ironsmith::game_loop::resolve_stack_entry_with(&mut g,&mut SelectFirstDecisionMaker).unwrap();let exiled=g.find_object_by_stable_id(stable).unwrap();assert_eq!(g.object(exiled).unwrap().zone,Zone::Exile);assert_eq!(g.player(A).unwrap().hand.len(),1,"the independent draw clause resolves before the hand-cast exile");assert_eq!(g.player(A).unwrap().mana_pool.total(),0);damage(&mut g,attacker,false);assert_eq!(g.object(g.find_object_by_stable_id(stable).unwrap()).unwrap().zone,Zone::Exile);damage(&mut g,attacker,true);let final_id=g.find_object_by_stable_id(stable).unwrap();assert_eq!(g.object(final_id).unwrap().zone,Zone::Graveyard,"the second cast came from exile, not hand");assert_eq!(g.player(A).unwrap().hand.len(),2,"actual triggered free cast resolves the independent draw again");assert_eq!(g.player(A).unwrap().mana_pool.total(),0);}
}
