//! Level-tier triggered occurrences exist only at event time; queued abilities
//! retain their normal programs after the source leaves that tier.
use ironsmith::{GameState,PlayerId,Zone,CounterType};
#[path = "p01_support/mod.rs"] mod support;
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
#[test]
fn level_range_triggers_are_event_time_gated() {
 for (name,phrase,threshold) in [
  ("Lighthouse Chronologist","take an extra turn after this one",7),
  ("Lord of Shatterskull Pass","deals 6 damage to each creature defending player controls",6),
 ] {
  for definition in support::definitions(name) {
   let text=support::rendered(&definition);assert!(text.contains(phrase),"{name}: {text}");assert!(!text.contains("__ironsmith_level_range"),"{name}: {text}");
   let mut game=GameState::new(vec!["A".into(),"B".into()],20);
   game.turn.active_player=if name=="Lighthouse Chronologist"{B}else{A};
   let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
   let defender=ironsmith_compiler_runtime::compile_to_runtime_definition("Defending witness","Type: Creature\nPower/Toughness: 2/10",false).unwrap();let defender=game.create_object_from_definition(&defender,B,Zone::Battlefield);
   let event=if name=="Lighthouse Chronologist"{
    ironsmith::triggers::TriggerEvent::new(ironsmith::events::BeginningOfEndStepEvent::new(B),Default::default())
   }else{
    ironsmith::triggers::TriggerEvent::new(ironsmith::events::CreatureAttackedEvent::with_total_attackers(source,ironsmith::triggers::AttackEventTarget::Player(B),1),Default::default())
   };
   game.add_counters(source,CounterType::Level,threshold-1);game.refresh_continuous_state().unwrap();
   assert!(ironsmith::triggers::check_triggers(&game,&event).into_iter().all(|entry|entry.source!=source),"{name}: ability must not exist below its printed tier");
   game.add_counters(source,CounterType::Level,1);game.refresh_continuous_state().unwrap();
   let mut matched=ironsmith::triggers::check_triggers(&game,&event).into_iter().filter(|entry|entry.source==source).collect::<Vec<_>>();assert_eq!(matched.len(),1,"{name}: exactly one current tier occurrence");let retained=matched.pop().unwrap();
   game.remove_counters(source,CounterType::Level,threshold,None,None);game.refresh_continuous_state().unwrap();
   assert!(ironsmith::triggers::check_triggers(&game,&event).into_iter().all(|entry|entry.source!=source),"{name}: later events cannot find the lost tier ability");
   let mut queue=ironsmith::triggers::TriggerQueue::new();queue.add(retained);
   ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut ironsmith::decision::SelectFirstDecisionMaker).unwrap();assert_eq!(game.stack.len(),1);
   ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut ironsmith::decision::SelectFirstDecisionMaker).unwrap();
   if name=="Lighthouse Chronologist"{assert_eq!(game.turn_store.extra_turns,vec![A]);}else{assert_eq!(game.damage_on(defender),6);}
  }
 }
}
