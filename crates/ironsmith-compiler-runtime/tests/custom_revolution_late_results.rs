//! Exact local payment and counter-kind result composition for source programs.
use ironsmith::{CardDefinition,GameState,PlayerId,Zone,Effect};
use ironsmith::effects::{EffectContext,execute_effect};
use ironsmith::decision::DecisionMaker;
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A:PlayerId=PlayerId(0);
fn definitions(text:&str)->[CardDefinition;2] {
 let (result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact("Local result witness",text,false)));
 assert!(!loss.is_lossy(),"{}",loss.reasons_text());let (artifact,direct)=result.unwrap();
 let restored=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();assert_eq!(artifact,restored);
 [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
struct Pay(u32);
impl DecisionMaker for Pay {
 fn decide_number(&mut self,_:&GameState,ctx:&ironsmith::decisions::context::NumberContext)->u32 {assert!(self.0>=ctx.min&&self.0<=ctx.max);self.0}
}
#[test]
fn token_size_reads_exact_completed_life_payment_across_intervening_life_loss() {
 for extra in ["","You lose 2 life. "] {
  let text=format!("Type: Sorcery\nPay any amount of life. {extra}Create an X/X black Demon creature token, where X is the amount of life paid.");
  for definition in definitions(&text) {for paid in [0,3,7] {
   let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=game.create_object_from_definition(&definition,A,Zone::Stack);let mut dm=Pay(paid);
   let effects=definition.spell_effect.as_ref().unwrap().all_effects_owned();execute_effect(&mut game,&Effect::new(ironsmith::effects::SequenceEffect::new(effects)),&mut EffectContext::new_default(source,A).with_decision_maker(&mut dm)).unwrap();
   let token=*game.battlefield.first().unwrap();assert_eq!(game.current_power(token),Some(paid as i32));assert_eq!(game.current_toughness(token),Some(paid as i32));assert_eq!(game.player(A).unwrap().life,20-paid as i32-if extra.is_empty(){0}else{2});
  }}
 }
}
#[test]
fn amount_life_paid_never_binds_ordinary_loss_or_unrelated_x() {
 for prefix in ["You lose 3 life.","Choose a number.","You gain 3 life."] {
  let text=format!("Type: Sorcery\n{prefix} Create an X/X black Demon creature token, where X is the amount of life paid.");
  assert!(ironsmith_compiler_runtime::compile_to_runtime_definition("Unpaid source",&text,false).is_err(),"{prefix}");
 }
}
#[test]
fn each_source_counter_places_same_kind_on_exact_created_token_in_separate_actions() {
 use ironsmith::CounterType;
 for definition in definitions("Type: Creature\nPower/Toughness: 2/2\nWhenever this creature attacks, create a 1/1 green Spirit creature token. For each counter on this creature, put a counter of that kind on that token.") {for departed in [false,true] {for (plus,charge) in [(0,0),(3,2)] {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
  let plain=ironsmith_compiler_runtime::compile_to_runtime_definition("Unrelated","Type: Creature\nPower/Toughness: 1/1",false).unwrap();let unrelated=game.create_object_from_definition(&plain,A,Zone::Battlefield);
  game.add_counters(source,CounterType::PlusOnePlusOne,plus);game.add_counters(source,CounterType::Charge,charge);
  let snapshot=ironsmith::snapshot::ObjectSnapshot::from_object(game.object(source).unwrap(),&game);
  let added=if plus>0 {2}else{0};game.add_counters(source,CounterType::PlusOnePlusOne,added);
  if departed {game.move_object_by_effect(source,Zone::Graveyard).unwrap();}
  let effects=definition.abilities.iter().find_map(|ability|match &ability.kind {ironsmith::ability::AbilityKind::Triggered(trigger)=>Some(trigger.effects.all_effects_owned()),_=>None}).unwrap();
  let mut ctx=EffectContext::new_default(source,A);ctx.source_snapshot=Some(snapshot);
  let outcome=execute_effect(&mut game,&Effect::new(ironsmith::effects::SequenceEffect::new(effects)),&mut ctx).unwrap();
  let token=*game.battlefield.iter().find(|id|**id!=source&&**id!=unrelated).unwrap();
  assert_eq!(game.counter_count(token,CounterType::PlusOnePlusOne),plus+added);assert_eq!(game.counter_count(token,CounterType::Charge),charge);
  assert_eq!(game.counter_count(unrelated,CounterType::PlusOnePlusOne),0);assert_eq!(game.counter_count(unrelated,CounterType::Charge),0);
  assert_eq!(outcome.events_of_type::<ironsmith::events::other::CounterPlacedEvent>().count(),(plus+added+charge) as usize,"one placement instruction for each individual source counter");
 }}}
}
#[test]
fn generated_saga_quoted_chapter_and_live_resolution_instruction_repeat() {
 use ironsmith::decision::SelectFirstDecisionMaker;
 use ironsmith::game_loop::{add_saga_lore_counters,put_triggers_on_stack,resolve_stack_entry_with};
 for definition in definitions("Type: Sorcery\nCreate a colorless Saga enchantment token with \"As this Saga's final chapter ability resolves, remove all lore counters from it.\" and \"r3— You gain 3 life.\".") {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=game.create_object_from_definition(&definition,A,Zone::Stack);
  let effects=definition.spell_effect.as_ref().unwrap().all_effects_owned();execute_effect(&mut game,&Effect::new(ironsmith::effects::SequenceEffect::new(effects)),&mut EffectContext::new_default(source,A)).unwrap();
  let saga=*game.battlefield.first().unwrap();let mut queue=ironsmith::triggers::TriggerQueue::new();
  for cycle in 0..2 {
   for _ in 0..3 {if game.counter_count(saga,ironsmith::CounterType::Lore)>=3 {break;}add_saga_lore_counters(&mut game,&mut queue).unwrap();}
   put_triggers_on_stack(&mut game,&mut queue).unwrap();assert_eq!(game.stack.len(),1);
   resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();
   assert_eq!(game.counter_count(saga,ironsmith::CounterType::Lore),0);assert_eq!(game.player(A).unwrap().life,23+cycle*3);assert_eq!(game.object(saga).unwrap().zone,Zone::Battlefield);
  }
 }
}
