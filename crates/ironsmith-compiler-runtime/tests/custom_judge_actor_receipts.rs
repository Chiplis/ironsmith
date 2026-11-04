//! Exact actor and completed-program receipts; parent owns execution.
#[path="custom_final_next_wave_support.rs"]mod source_support;
use ironsmith::{CardDefinition,Effect,GameState,ObjectId,PlayerId,Zone};
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{DecisionMaker,SelectFirstDecisionMaker};
use ironsmith::effects::{EffectContext,ResolvedTarget,execute_effect};
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
const CHOICE:&str="You choose honor or condemn; If you chose honor, perform the honor action with that creature, then put a +1/+1 counter on that creature; If you chose condemn, perform the condemn action with that creature, then sacrifice that creature, then draw a card";
fn generic()->[CardDefinition;2]{
 let text=format!("Type: Sorcery\nChoose target creature. Its controller performs the verdict action as follows: [{CHOICE}]. If the preceding program included the honor action, you gain 3 life. If the preceding program included the condemn action, you gain 5 life.");
 source_support::definitions(&serde_json::json!({"name":"Scoped participant verdict","executable_text":text})).unwrap()
}
struct Options{index:usize,actors:Vec<PlayerId>}
impl DecisionMaker for Options{
 fn decide_options(&mut self,_:&GameState,context:&ironsmith::decisions::context::SelectOptionsContext)->Vec<usize>{self.actors.push(context.player);vec![context.options.iter().filter(|option|option.legal).nth(self.index).unwrap().index]}
 fn decide_objects(&mut self,game:&GameState,context:&ironsmith::decisions::context::SelectObjectsContext)->Vec<ObjectId>{SelectFirstDecisionMaker.decide_objects(game,context)}
}
fn body(name:&str)->CardDefinition{CardDefinitionBuilder::new(ironsmith::CardId::new(),name).card_types(vec![ironsmith::CardType::Creature]).power_toughness(ironsmith::card::PowerToughness::fixed(3,4)).build()}
fn setup()->GameState{GameState::new(vec!["A".into(),"B".into()],20)}
fn resolve(game:&mut GameState,definition:&CardDefinition,recipient:ObjectId,option:usize)->ironsmith::effect::EffectOutcome{
 let source=game.create_object_from_definition(definition,A,Zone::Stack);
 let effects=definition.spell_effect.as_ref().unwrap().all_effects_owned();
 let mut choices=Options{index:option,actors:Vec::new()};
 let outcome=execute_effect(game,&Effect::new(ironsmith::effects::SequenceEffect::new(effects)),
  &mut EffectContext::new_default(source,A).with_targets(vec![ResolvedTarget::Object(recipient)]).with_decision_maker(&mut choices)).unwrap();
 assert_eq!(choices.actors,vec![B],"the target's controller owns the one branch decision");outcome
}
#[test]fn explicit_actor_owns_complete_choice_sacrifice_draw_and_inner_receipts(){
 for definition in generic(){for option in 0..2{
  let mut game=setup();let recipient=game.create_object_from_definition(&body("B's recipient"),B,Zone::Battlefield);let stable=game.object(recipient).unwrap().stable_id;
  game.create_object_from_definition(&body("A's library witness"),A,Zone::Library);game.create_object_from_definition(&body("B's library witness"),B,Zone::Library);
  let result=resolve(&mut game,&definition,recipient,option);
  let name=if option==0{"honor"}else{"condemn"};
  let receipts=result.events.iter().filter_map(|event|event.downcast::<ironsmith::events::NamedActionEvent>()).filter(|event|event.label==name).collect::<Vec<_>>();
  assert_eq!(receipts.len(),1);assert_eq!(receipts[0].player,B);assert_eq!(receipts[0].objects.len(),1);assert_eq!(receipts[0].objects[0].object_id,recipient);
  assert_eq!(game.player(A).unwrap().hand.len(),0);assert_eq!(game.player(B).unwrap().hand.len(),usize::from(option==1));
  assert_eq!(game.player(A).unwrap().life,if option==0{23}else{25},"following instructions restore the spell's original actor and sibling result gates share the same producer");
  let current=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(current).unwrap().zone,if option==0{Zone::Battlefield}else{Zone::Graveyard});
  if option==0{assert_eq!(game.counter_count(current,ironsmith::CounterType::PlusOnePlusOne),1);}
 }}
}
#[test]fn named_receipt_predicate_reads_only_the_exact_supplied_program_result(){
 use ironsmith::effect::EffectPredicateRuntimeExt as _;
 for definition in generic(){
  let mut game=setup();let recipient=game.create_object_from_definition(&body("Prior receipt object"),B,Zone::Battlefield);
  let result=resolve(&mut game,&definition,recipient,0);
  let predicate=ironsmith::effect::EffectPredicate::NamedActionCompleted{label:"honor".into()};
  assert!(predicate.evaluate_outcome(&result));
  let receipt=result.events.iter().find_map(|event|event.downcast::<ironsmith::events::NamedActionEvent>().filter(|event|event.label=="honor")).unwrap().clone();
  let unverified=ironsmith::effect::EffectOutcome::count(1).with_event(ironsmith::triggers::TriggerEvent::new_with_provenance(receipt,Default::default()));
  assert!(!predicate.evaluate_outcome(&unverified),"equal event text without completion authority is not a receipt");
  assert!(!ironsmith::effect::EffectPredicate::NamedActionCompleted{label:"honorary".into()}.evaluate_outcome(&result));
  let unrelated=ironsmith::effect::EffectOutcome::aggregate_replacement_outcomes(ironsmith::effect::EffectOutcome::count(1),[result.clone()]);
  assert!(!predicate.evaluate_outcome(&unrelated),"replacement/addition child events are not the preceding authored program result");
  let retained=ironsmith::effect::EffectOutcome::aggregate_replacement_outcomes(result.clone(),[ironsmith::effect::EffectOutcome::count(0)]);
  assert!(predicate.evaluate_outcome(&retained));
  assert!(!predicate.evaluate_outcome(&ironsmith::effect::EffectOutcome::count(1)),"another successful instruction cannot borrow the earlier receipt or turn history");
  assert!(!ironsmith::effect::EffectPredicate::NamedActionCompleted{label:"condemn".into()}.evaluate_outcome(&result));
 }
}
fn actual(name:&str)->[CardDefinition;2]{
 let input:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-actions-status-whole-source-complete.json")).unwrap();
 let source=input["mechanics"].as_array().unwrap().iter().flat_map(|row|row["representative_cards"].as_array().unwrap()).find(|card|card["name"]==name).unwrap();
 assert!(source["original_text"].as_str().unwrap().contains("judge"));
 source_support::definitions(source).unwrap()
}
#[test]fn marianne_full_source_returns_to_original_controller_after_opponent_judges(){
 for definition in actual("Marianne's Mercies"){for option in 0..2{
  let mut game=setup();let recipient=game.create_object_from_definition(&body("Opponent recipient"),B,Zone::Battlefield);let stable=game.object(recipient).unwrap().stable_id;game.tap(recipient);
  game.create_object_from_definition(&body("Opponent draw"),B,Zone::Library);
  let result=resolve(&mut game,&definition,recipient,option);
  let current=game.find_object_by_stable_id(stable).unwrap();
  if option==0{assert_eq!(game.controller_of_id(current),Some(A));assert!(!game.is_tapped(current));assert_eq!(game.counter_count(current,ironsmith::CounterType::PlusOnePlusOne),1);}
  else{assert_eq!(game.object(current).unwrap().zone,Zone::Graveyard);assert_eq!(game.player(B).unwrap().hand.len(),1);}
  assert_eq!(game.player(A).unwrap().hand.len(),0);
  assert!(result.events.iter().filter_map(|event|event.downcast::<ironsmith::events::NamedActionEvent>()).any(|event|event.player==B&&event.label==if option==0{"honor"}else{"condemn"}));
 }}
}
#[test]fn change_of_station_full_source_carries_the_preceding_creators_actor(){
 for definition in actual("Change of Station"){for option in 0..2{
  let mut game=setup();let recipient=game.create_object_from_definition(&body("Exiled opponent creature"),B,Zone::Battlefield);let stable=game.object(recipient).unwrap().stable_id;
  game.create_object_from_definition(&body("Opponent draw"),B,Zone::Library);
  let result=resolve(&mut game,&definition,recipient,option);
  assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone,Zone::Exile);
  let receipt=result.events.iter().filter_map(|event|event.downcast::<ironsmith::events::NamedActionEvent>()).find(|event|event.label==if option==0{"honor"}else{"condemn"}).unwrap();
  assert_eq!(receipt.player,B);assert_eq!(receipt.objects.len(),1);assert!(receipt.objects[0].subtypes.contains(&ironsmith::Subtype::Soldier));
  assert_eq!(game.player(B).unwrap().hand.len(),usize::from(option==1));assert_eq!(game.player(A).unwrap().hand.len(),0);
 }}
}

#[test]fn actual_recipient_observer_reads_actor_and_departed_recipient_power_toughness(){
 for spell in actual("Marianne's Mercies"){for observer in actual("Suhra and Fori, Grand Viziers"){for option in 0..2{
  let mut game=setup();game.create_object_from_definition(&observer,B,Zone::Battlefield);
  let recipient=game.create_object_from_definition(&body("Observed recipient"),B,Zone::Battlefield);game.create_object_from_definition(&body("B draw"),B,Zone::Library);
  let result=resolve(&mut game,&spell,recipient,option);
  game.take_pending_trigger_events();
  // Enqueue authenticated actual resolution receipts, never invented action
  // events. The direct effect harness has no game-loop receipt publisher.
  for event in result.events{if event.downcast::<ironsmith::events::NamedActionEvent>().is_some(){game.queue_trigger_event(event.provenance(),event);}}
  let mut pending=ironsmith::triggers::TriggerQueue::new();ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game,&mut pending,&mut SelectFirstDecisionMaker).unwrap();
  assert_eq!(pending.entries.len(),1,"only the actual honor/condemn actor's observer fires");
  ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut pending,&mut SelectFirstDecisionMaker).unwrap();ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();
  if option==0{assert_eq!(game.battlefield.iter().filter(|id|game.controller_of_id(**id)==Some(B)&&game.object(**id).unwrap().subtypes.contains(&ironsmith::Subtype::Soldier)).count(),2);}
  else{assert_eq!(game.player(A).unwrap().life,17);assert_eq!(game.player(B).unwrap().life,24,"condemned recipient LKI, not operation source power/toughness");}
 }}}
}

#[test]fn explicit_owner_actor_is_distinct_from_current_controller(){
 let source=serde_json::json!({"name":"Owner-scoped operation","executable_text":"Type: Sorcery\nChoose target creature. Its owner performs the consent action as follows: [Draw a card]. You gain 1 life."});
 for definition in source_support::definitions(&source).unwrap(){
  let mut game=setup();let recipient=game.create_object_from_definition(&body("Owned by B"),B,Zone::Battlefield);game.object_mut(recipient).unwrap().initial_controller=A;game.refresh_continuous_state().unwrap();
  assert_eq!(game.controller_of_id(recipient),Some(A));game.create_object_from_definition(&body("B's draw"),B,Zone::Library);
  let actor=game.create_object_from_definition(&definition,A,Zone::Stack);let effects=definition.spell_effect.as_ref().unwrap().all_effects_owned();
  execute_effect(&mut game,&Effect::new(ironsmith::effects::SequenceEffect::new(effects)),&mut EffectContext::new_default(actor,A).with_targets(vec![ResolvedTarget::Object(recipient)])).unwrap();
  assert_eq!(game.player(B).unwrap().hand.len(),1);assert_eq!(game.player(A).unwrap().hand.len(),0);assert_eq!(game.player(A).unwrap().life,21);
 }
}
