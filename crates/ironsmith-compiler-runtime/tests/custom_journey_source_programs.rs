//! Independent strict/artifact routes for the composed player-rule declaration.
use ironsmith::{CardDefinition,Effect,GameState,ObjectId,PlayerId,Zone};
use ironsmith::effects::{EffectContext,execute_effect};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
fn definitions(event_only:bool,witness:bool)->[CardDefinition;2] {
 let extra=if event_only {"event "} else {""};let after=if witness {" You gain 1 life."} else {""};
 let text=format!("Type: Sorcery\nExile the top card of your library face up. Give it the journeyed status while in exile. Register a persistent rule named \"journey retrieval\" for you with {extra}condition \"You own a card in exile with the journeyed status\" and ability \"Whenever a creature you control deals combat damage to an opponent, put a card you own in exile with the journeyed status into your hand.{after}\"");
 let (result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact("Rale",&text,false)));
 assert!(!loss.is_lossy(),"{}",loss.reasons_text());let (artifact,direct)=result.unwrap();let restored=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();assert_eq!(artifact,restored);
 [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
fn creature(game:&mut GameState,player:PlayerId,zone:Zone)->ObjectId {
 let def=ironsmith_compiler_runtime::compile_to_runtime_definition("Independent traveler","Type: Creature\nPower/Toughness: 2/2",false).unwrap();game.create_object_from_definition(&def,player,zone)
}
fn try_run(game:&mut GameState,def:&CardDefinition)->Result<ObjectId,ironsmith::effects::ExecutionError> {
 let source=game.create_object_from_definition(def,A,Zone::Stack);let program=Effect::new(ironsmith::effects::SequenceEffect::new(def.spell_effect.as_ref().unwrap().all_effects_owned()));execute_effect(game,&program,&mut EffectContext::new_default(source,A))?;Ok(source)
}
fn run(game:&mut GameState,def:&CardDefinition)->ObjectId {try_run(game,def).unwrap()}
fn damage(source:ObjectId)->ironsmith::triggers::TriggerEvent {
 ironsmith::triggers::TriggerEvent::new_with_provenance(ironsmith::events::DamageEvent::with_cause(source,ironsmith::events::DamageTarget::Player(B),2,true,ironsmith::events::EventCause::combat_damage(source)),Default::default())
}
#[test]
fn composed_exile_marks_exact_result_and_rule_survives_source_departure() {
 for event_only in [false,true] {for def in definitions(event_only,false) {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let physical=creature(&mut game,A,Zone::Library);let stable=game.object(physical).unwrap().stable_id;let source=run(&mut game,&def);
  let exiled=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(exiled).unwrap().zone,Zone::Exile);assert!(game.has_custom_status(exiled,"journeyed"));assert!(!game.has_custom_status(source,"journeyed"));assert_eq!(game.effect_store.delayed_triggers.len(),1);
  game.move_object_by_effect(source,Zone::Graveyard).unwrap();let attacker=creature(&mut game,A,Zone::Battlefield);let entries=ironsmith::triggers::check_delayed_triggers(&mut game,&damage(attacker));assert_eq!(entries.len(),1);let mut queue=ironsmith::triggers::TriggerQueue::new();for entry in entries{queue.add(entry);}
  ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut ironsmith::decision::SelectFirstDecisionMaker).unwrap();ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut ironsmith::decision::SelectFirstDecisionMaker).unwrap();
  assert_eq!(game.player(A).unwrap().hand.len(),1);let returned=game.player(A).unwrap().hand[0];assert_eq!(game.object(returned).unwrap().stable_id,stable);assert!(!game.has_custom_status(returned,"journeyed"));assert!(game.has_custom_status(exiled,"journeyed")==false);
 }}
}
#[test]
fn event_only_and_intervening_conditions_have_distinct_real_resolution_behavior() {
 // The extra ordinary gain-life instruction is a neutral witness for whether
 // the registered ability resolves; physical-source fixtures retain their body.
 for event_only in [false,true] {for def in definitions(event_only,true) {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);creature(&mut game,A,Zone::Library);run(&mut game,&def);let attacker=creature(&mut game,A,Zone::Battlefield);let entries=ironsmith::triggers::check_delayed_triggers(&mut game,&damage(attacker));assert_eq!(entries.len(),1);let mut queue=ironsmith::triggers::TriggerQueue::new();for entry in entries{queue.add(entry);}
  ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut ironsmith::decision::SelectFirstDecisionMaker).unwrap();let marked=game.exile.iter().copied().find(|id|game.has_custom_status(*id,"journeyed")).unwrap();game.move_object_by_effect(marked,Zone::Graveyard).unwrap();
  ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut ironsmith::decision::SelectFirstDecisionMaker).unwrap();assert_eq!(game.player(A).unwrap().life,if event_only{21}else{20});
 }}
}
#[test]
fn empty_library_never_marks_an_unrelated_exiled_card_or_the_originating_spell() {
 for event_only in [false,true] {for def in definitions(event_only,false) {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let unrelated=creature(&mut game,A,Zone::Exile);let source=run(&mut game,&def);
  assert_eq!(game.exile,vec![unrelated]);assert!(!game.has_custom_status(unrelated,"journeyed"));assert!(!game.has_custom_status(source,"journeyed"));
  let attacker=creature(&mut game,A,Zone::Battlefield);assert!(ironsmith::triggers::check_delayed_triggers(&mut game,&damage(attacker)).is_empty());
 }}
}
#[test]
fn repeated_composed_actions_mark_each_actual_card_and_keep_one_player_rule() {
 for event_only in [false,true] {for def in definitions(event_only,false) {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);creature(&mut game,A,Zone::Library);creature(&mut game,A,Zone::Library);run(&mut game,&def);run(&mut game,&def);
  assert_eq!(game.exile.len(),2);assert!(game.exile.iter().all(|id|game.has_custom_status(*id,"journeyed")));assert_eq!(game.effect_store.delayed_triggers.len(),1);
 }}
}

#[test]
fn identical_independently_compiled_player_rules_deduplicate_across_direct_and_artifact_routes() {
 for event_only in [false,true] {
  let first=definitions(event_only,false);let second=definitions(event_only,false);
  for left in &first {for right in &second {
   let mut game=GameState::new(vec!["A".into(),"B".into()],20);creature(&mut game,A,Zone::Library);creature(&mut game,A,Zone::Library);run(&mut game,left);run(&mut game,right);
   assert_eq!(game.effect_store.delayed_triggers.len(),1);assert_eq!(game.exile.len(),2);
  }}
 }
}
#[test]
fn incompatible_msem_rev_journey_rules_fail_in_both_registration_orders_and_preserve_first_definition() {
 for first_event_only in [false,true] {
  let first=definitions(first_event_only,false);let second=definitions(!first_event_only,false);
  for left in &first {for right in &second {
   let mut game=GameState::new(vec!["A".into(),"B".into()],20);creature(&mut game,A,Zone::Library);creature(&mut game,A,Zone::Library);run(&mut game,left);
   let condition=game.effect_store.delayed_triggers[0].intervening_if.clone();
   let error=try_run(&mut game,right).unwrap_err();assert!(matches!(error,ironsmith::effects::ExecutionError::InternalError(message) if message.contains("incompatible persistent player rule")));
   assert_eq!(game.effect_store.delayed_triggers.len(),1);assert_eq!(game.effect_store.delayed_triggers[0].intervening_if,condition);
  }}
 }
}
#[test]
fn same_named_rule_with_a_different_complete_body_is_rejected() {
 let first=definitions(false,false);let second=definitions(false,true);
 for left in &first {for right in &second {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);creature(&mut game,A,Zone::Library);creature(&mut game,A,Zone::Library);run(&mut game,left);
  assert!(matches!(try_run(&mut game,right),Err(ironsmith::effects::ExecutionError::InternalError(_))));assert_eq!(game.effect_store.delayed_triggers.len(),1);
 }}
}
