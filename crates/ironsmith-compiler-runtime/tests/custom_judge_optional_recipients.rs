//! Full original optional Judge consumers; no detached flat-target execution.
#[path="custom_final_next_wave_support.rs"] mod routes;
use ironsmith::{CardDefinition,GameState,ObjectId,PlayerId,Target,Zone};
use ironsmith::decision::{DecisionMaker,LegalAction,SelectFirstDecisionMaker,compute_legal_actions};
use ironsmith::game_loop::{PriorityLoopState,PriorityResponse,apply_priority_response_with_dm,apply_decision_context_with_dm,resolve_stack_entry_with};
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
struct Choices{targets:Vec<Target>,mode:usize,resolution_prompts:usize}
impl DecisionMaker for Choices{
 fn decide_targets(&mut self,_:&GameState,_:&ironsmith::decisions::context::TargetsContext)->Vec<Target>{self.targets.clone()}
 fn decide_options(&mut self,_:&GameState,c:&ironsmith::decisions::context::SelectOptionsContext)->Vec<usize>{self.resolution_prompts+=1;vec![c.options.iter().filter(|o|o.legal).nth(self.mode).expect("one requested legal Judge branch").index]}
 fn decide_objects(&mut self,g:&GameState,c:&ironsmith::decisions::context::SelectObjectsContext)->Vec<ObjectId>{SelectFirstDecisionMaker.decide_objects(g,c)}
}
fn actual(name:&str)->[CardDefinition;2]{
 let input:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-actions-status-whole-source-complete.json")).unwrap();
 let card=input["mechanics"].as_array().unwrap().iter().flat_map(|r|r["representative_cards"].as_array().unwrap()).find(|c|c["name"]==name).unwrap();
 assert!(card["original_text"].as_str().unwrap().contains("Judge up to one target creature you control."));routes::definitions(card).unwrap()
}
fn cast(game:&mut GameState,definition:&CardDefinition,dm:&mut Choices){
 let card=game.create_object_from_definition(definition,A,Zone::Hand);
 let action=compute_legal_actions(game,A).unwrap().into_iter().find(|a|matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==card)).expect("the mandatory target admits the real spell with zero optional recipients");
 let mut state=PriorityLoopState::new(2);let mut queue=ironsmith::triggers::TriggerQueue::new();
 let mut progress=apply_priority_response_with_dm(game,&mut queue,&mut state,&PriorityResponse::PriorityAction(action),dm).unwrap();
 for _ in 0..64{if !state.has_pending_action(){return;}let ironsmith::GameProgress::NeedsDecisionCtx(c)=progress else{panic!("{progress:?}")};progress=apply_decision_context_with_dm(game,&mut queue,&mut state,&c,dm).unwrap();}panic!("cast did not complete");
}
#[test]fn full_source_zero_or_lost_optional_recipient_skips_complete_judge_but_keeps_mandatory_sibling(){run(false);}
#[test]fn full_source_selected_recipient_honors_or_condemns_once_and_keeps_mandatory_sibling(){run(true);}
fn run(selected:bool){
 let witness=routes::definitions(&serde_json::json!({"name":"Flying witness","executable_text":"Type: Creature — Bird\nPower/Toughness: 3/3\nFlying"})).unwrap();
 for name in ["Fall from the Heavens","Ordeal of Kaneh"]{for (route,definition) in actual(name).into_iter().enumerate(){
 // Empty with a legal friendly creature must not silently select it. The
 // lost case announces a recipient, then removes it before resolution.
 for variant in 0..2{
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.active_player=A;game.turn.priority_player=Some(A);game.turn.phase=ironsmith::game_state::Phase::FirstMain;game.turn.step=None;
 for symbol in [ironsmith::mana::ManaSymbol::Colorless,ironsmith::mana::ManaSymbol::Green,ironsmith::mana::ManaSymbol::Black]{game.player_mut(A).unwrap().mana_pool.add(symbol,20);}
 let enemy=game.create_object_from_definition(&witness[route],B,Zone::Battlefield);let enemy_stable=game.object(enemy).unwrap().stable_id;
 let friendly=game.create_object_from_definition(&witness[route],A,Zone::Battlefield);let friendly_stable=game.object(friendly).unwrap().stable_id;
 game.create_object_from_definition(&witness[route],A,Zone::Library);
 let announce=selected||variant==1;let mut targets=vec![Target::Object(enemy)];if announce{targets.push(Target::Object(friendly));}
 let mut dm=Choices{targets,mode:variant,resolution_prompts:0};cast(&mut game,&definition,&mut dm);
 let assignments=&game.stack.last().unwrap().target_assignments;assert_eq!(assignments.len(),2);assert_eq!(assignments[0].range.len(),1);assert_eq!(assignments[1].range.len(),usize::from(announce));
 if !selected&&announce{game.move_object_by_effect(friendly,Zone::Graveyard).unwrap();}
 dm.resolution_prompts=0;resolve_stack_entry_with(&mut game,&mut dm).unwrap();
 let enemy_now=game.find_object_by_stable_id(enemy_stable).unwrap();assert_eq!(game.object(enemy_now).unwrap().zone,Zone::Graveyard,"the mandatory source instruction resolves despite an empty/invalid optional group");
 assert_eq!(dm.resolution_prompts,usize::from(selected),"only a legal selected recipient offers a Judge mode");
 assert_eq!(game.player(A).unwrap().hand.len(),usize::from(selected&&variant==1),"draw belongs exclusively to one completed condemnation");
 let current=game.find_object_by_stable_id(friendly_stable).unwrap();
 assert_eq!(game.object(current).unwrap().zone,if variant==1{Zone::Graveyard}else{Zone::Battlefield});
 assert_eq!(game.counter_count(current,ironsmith::CounterType::PlusOnePlusOne),u32::from(selected&&variant==0));
 assert!(game.stack.is_empty());
 }}}
}
