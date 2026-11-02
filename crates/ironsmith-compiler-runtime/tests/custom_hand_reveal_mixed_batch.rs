//! All event-time hand authorities survive a mixed simultaneous departure.
#[path="custom_final_next_wave_support.rs"] mod source_support;
use ironsmith::{CardDefinition,GameState,ObjectId,PlayerId,Zone};
use ironsmith::decision::{DecisionMaker,SelectFirstDecisionMaker};
use ironsmith::effects::{EffectContext,execute_effect};
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
fn definitions(name:&str,text:&str)->[CardDefinition;2]{source_support::definitions(&serde_json::json!({"name":name,"executable_text":text,"custom_keyword_identities":[],"custom_keyword_payments":[]})).unwrap()}
struct Choices { openings:Vec<(PlayerId,Vec<ObjectId>)> }
impl DecisionMaker for Choices {
 fn decide_boolean(&mut self,_:&GameState,_:&ironsmith::decisions::context::BooleanContext)->bool{true}
 fn decide_objects(&mut self,game:&GameState,ctx:&ironsmith::decisions::context::SelectObjectsContext)->Vec<ObjectId>{
  if ctx.reveal_policy==ironsmith::decisions::context::SelectionRevealPolicy::Public {
   let ids=ctx.candidates.iter().filter(|c|c.legal).map(|c|c.id).collect::<Vec<_>>();self.openings.push((ctx.player,ids.clone()));ids
  }else{SelectFirstDecisionMaker.decide_objects(game,ctx)}
 }
 fn decide_targets(&mut self,game:&GameState,ctx:&ironsmith::decisions::context::TargetsContext)->Vec<ironsmith::Target>{SelectFirstDecisionMaker.decide_targets(game,ctx)}
}
fn wipe(game:&mut GameState,definition:&CardDefinition,dm:&mut Choices){
 let source=game.create_object_from_definition(definition,A,Zone::Stack);let mut ctx=EffectContext::new_default(source,A).with_decision_maker(dm);
 for effect in definition.spell_effect.as_ref().unwrap().all_effects(){execute_effect(game,effect,&mut ctx).unwrap();}
 assert!(!ctx.decision_maker.awaiting_choice());
}
fn collect(game:&mut GameState,dm:&mut Choices){let mut queue=ironsmith::triggers::TriggerQueue::new();ironsmith::game_loop::drain_pending_trigger_events_with_dm(game,&mut queue,dm).unwrap();ironsmith::game_loop::put_triggers_on_stack_with_dm(game,&mut queue,dm).unwrap();}
const REVEAL:&str="You may reveal this card from your hand as a permanent you control leaves the battlefield. When you reveal this card this way, you may cast it by paying {1} rather than its mana cost.";
#[test]
fn simultaneous_different_controllers_preserve_both_intrinsic_hand_listeners_and_ordinary_death_listeners(){
 let hands=definitions("Intrinsic hand observer",&format!("Mana cost: {{8}}\nType: Sorcery\n{REVEAL}\nYou gain 1 life."));
 let bodies=definitions("Departing body","Type: Creature\nPower/Toughness: 1/1");let observers=definitions("Ordinary death observer","Type: Enchantment\nWhenever a creature dies, you gain 1 life.");let wipes=definitions("Simultaneous departure","Type: Sorcery\nDestroy all creatures.");
 for index in 0..2 {for reverse_creation in [false,true]{
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let observer=game.create_object_from_definition(&observers[index],A,Zone::Battlefield);
  for owner in if reverse_creation{[B,A]}else{[A,B]} {game.create_object_from_definition(&bodies[index],owner,Zone::Battlefield);}
  let first=game.create_object_from_definition(&hands[index],A,Zone::Hand);let second=game.create_object_from_definition(&hands[index],B,Zone::Hand);
  let mut dm=Choices{openings:vec![]};wipe(&mut game,&wipes[index],&mut dm);collect(&mut game,&mut dm);
  assert_eq!(game.stack.iter().filter(|entry|entry.object_id==first).count(),1,"A's event-time authority survives either batch order");
  assert_eq!(game.stack.iter().filter(|entry|entry.object_id==second).count(),1,"B's authority cannot be prefiltered by A's singular departure");
  assert_eq!(game.stack.iter().filter(|entry|entry.object_id==observer).count(),2,"hand windows cannot drop ordinary listeners");assert_eq!(game.stack.len(),4);
 }}
}
#[test]
fn private_granted_permissions_survive_departing_grantors_without_merging_identical_grant_instances(){
 let grant_text=format!("Type: Creature\nPower/Toughness: 1/1\nCreature cards in your hand have custom keyword \"departure permission\", and \"{REVEAL}\".");
 let grantors=definitions("Private departure grantor",&grant_text);let hands=definitions("Private hand observer","Mana cost: {8}\nType: Creature\nPower/Toughness: 2/2");let observers=definitions("Ordinary death observer","Type: Enchantment\nWhenever a creature dies, you gain 1 life.");let wipes=definitions("Simultaneous departure","Type: Sorcery\nDestroy all creatures.");
 for index in 0..2 {for shared_owner in [false,true]{
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let observer=game.create_object_from_definition(&observers[index],A,Zone::Battlefield);
  let owners=if shared_owner{vec![A]}else{vec![A,B]};let mut cards=Vec::new();
  for owner in owners {let card=game.create_hidden_card_placeholder(owner,Zone::Hand,0,format!("private-hand-{}",owner.0));game.reveal_hidden_card_with_definition(card,&hands[index]).unwrap();cards.push((owner,card));}
  // Two equal-valued grants are separate abilities. Both must remain in the
  // frozen authority packet when both grantors leave in this one instruction.
  let first=game.create_object_from_definition(&grantors[index],A,Zone::Battlefield);let second=game.create_object_from_definition(&grantors[index],if shared_owner{A}else{B},Zone::Battlefield);
  let mut dm=Choices{openings:vec![]};wipe(&mut game,&wipes[index],&mut dm);collect(&mut game,&mut dm);assert!(game.object(first).is_none()&&game.object(second).is_none());
  for (owner,card) in &cards {assert!(dm.openings.iter().any(|(player,ids)|player==owner&&ids.contains(card)),"the owning player answers each private hand window");assert_eq!(game.stack.iter().filter(|entry|entry.object_id==*card).count(),if shared_owner{4}else{1},"grant instances and matching departure occurrences are independent");}
  assert_eq!(game.stack.iter().filter(|entry|entry.object_id==observer).count(),2);assert_eq!(game.stack.len(),if shared_owner{6}else{4});
 }}
}
