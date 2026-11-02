//! Complete quoted entry programs keep their own sacrifice-result scope.
#[path = "custom_final_next_wave_support.rs"] mod routes;
use ironsmith::{CardDefinition,GameState,ObjectId,PlayerId,Zone};
use ironsmith::decision::{DecisionMaker,LegalAction,compute_legal_actions};
use ironsmith::game_loop::{PriorityLoopState,PriorityResponse,apply_priority_response_with_dm,apply_decision_context_with_dm,resolve_stack_entry_with};
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
struct Choices { accept:bool, selected:Vec<ObjectId>, object_prompts:usize }
impl DecisionMaker for Choices {
 fn decide_boolean(&mut self,_:&GameState,_:&ironsmith::decisions::context::BooleanContext)->bool { self.accept }
 fn decide_objects(&mut self,_:&GameState,c:&ironsmith::decisions::context::SelectObjectsContext)->Vec<ObjectId> {
  self.object_prompts+=1;
  c.candidates.iter().filter(|candidate|candidate.legal&&self.selected.contains(&candidate.id)).map(|candidate|candidate.id).collect()
 }
}
fn cast(game:&mut GameState,card:ObjectId,dm:&mut Choices) {
 let action=compute_legal_actions(game,A).unwrap().into_iter().find(|a|matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==card)).expect("real spell must be castable");
 let mut state=PriorityLoopState::new(2);let mut queue=ironsmith::triggers::TriggerQueue::new();
 let mut progress=apply_priority_response_with_dm(game,&mut queue,&mut state,&PriorityResponse::PriorityAction(action),dm).unwrap();
 for _ in 0..64 {
  if !state.has_pending_action(){return;}
  let ironsmith::GameProgress::NeedsDecisionCtx(context)=progress else{panic!("cast stalled: {progress:?}")};
  progress=apply_decision_context_with_dm(game,&mut queue,&mut state,&context,dm).unwrap();
 }
 panic!("cast did not complete");
}
fn actual()->[CardDefinition;2] {
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-final-real-source-next-wave.json")).unwrap();
 let source=fixture["mechanics"].as_array().unwrap().iter().flat_map(|row|row["representative_cards"].as_array().unwrap()).find(|card|card["source_card_key"]=="Skittish Prey_EOW").unwrap();
 assert!(source["original_text"].as_str().unwrap().contains("gains voracious 2"));
 routes::definitions(source).unwrap()
}
fn creature()->[CardDefinition;2] { routes::definitions(&serde_json::json!({"name":"Entry receiver","executable_text":"Mana cost: {0}\nType: Creature — Beast\nPower/Toughness: 2/2"})).unwrap() }
#[test]
fn full_source_next_spell_entry_program_reads_only_its_own_actual_sacrifices() {
 let receivers=creature();
 for (route,source) in actual().into_iter().enumerate() { for (accept,amount) in [(false,0usize),(true,0),(true,1),(true,2)] {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.active_player=A;game.turn.priority_player=Some(A);game.turn.phase=ironsmith::game_state::Phase::FirstMain;game.turn.step=None;
  game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Green,10);
  let donor1=game.create_object_from_definition(&receivers[route],A,Zone::Battlefield);
  let donor2=game.create_object_from_definition(&receivers[route],A,Zone::Battlefield);
  let donor_stables=[donor1,donor2].map(|id|game.object(id).unwrap().stable_id);
  let spell=game.create_object_from_definition(&source,A,Zone::Hand);
  let mut dm=Choices{accept,selected:vec![donor1,donor2][..amount].to_vec(),object_prompts:0};
  cast(&mut game,spell,&mut dm);resolve_stack_entry_with(&mut game,&mut dm).unwrap();
  let squirrel_count=game.battlefield.len();assert_eq!(squirrel_count,3,"full original source creates its Squirrel before granting the next spell");
  let receiver=game.create_object_from_definition(&receivers[route],A,Zone::Hand);let stable=game.object(receiver).unwrap().stable_id;
  game.turn.active_player=B;game.turn.priority_player=Some(A);
  cast(&mut game,receiver,&mut dm);resolve_stack_entry_with(&mut game,&mut dm).unwrap();
  let entered=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(entered).unwrap().zone,Zone::Battlefield);
  assert_eq!(game.counter_count(entered,ironsmith::CounterType::PlusOnePlusOne),amount as u32,"entry counters are actual local sacrifices, not the earlier Squirrel creation or spell result");
  for (index,stable) in donor_stables.into_iter().enumerate() {
   let id=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(id).unwrap().zone,if index<amount{Zone::Graveyard}else{Zone::Battlefield});
  }
  assert!(dm.object_prompts<=1,"one bounded sacrifice selection, including an empty choice, must not be repeated");
  let later=game.create_object_from_definition(&receivers[route],A,Zone::Hand);
  assert!(!compute_legal_actions(&game,A).unwrap().iter().any(|action|matches!(action,LegalAction::CastSpell{spell_id,..}if *spell_id==later)),"the next-spell timing permission is consumed once");
 }}
}
#[test]
fn quoted_entry_counter_without_local_producer_does_not_borrow_enclosing_spell_result() {
 let source=serde_json::json!({"name":"Missing local producer","executable_text":"Mana cost: {G}\nType: Instant\nCreate a 1/1 green Squirrel creature token.\nThe next creature spell you cast this turn can be cast as though it had flash. As you cast that spell, it gains \"As this permanent enters, this permanent enters with that many +1/+1 counters on it.\"."});
 assert!(routes::definitions(&source).is_err(),"the outer token creation cannot satisfy a missing entry-program result");
}
