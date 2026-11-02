//! Source-defined player counter meanings use ordinary persistent rules.
use ironsmith::{CardDefinition,Effect,GameState,PlayerId,Zone};
use ironsmith::effects::{EffectContext,ResolvedTarget,execute_effect};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
fn definitions(kind:&str)->[CardDefinition;2] {
 let text=format!("Type: Sorcery\nTarget player gets three {kind} counters. Register a persistent rule named \"{kind} player counter meaning\" for that player with event condition \"You have one or more {kind} counters and it's the first main phase of the turn\" and ability \"At the beginning of each of your main phases, For each {kind} counter on you, do the following: Mill a card. If a nonland card was milled this way, you lose 1 life and remove a {kind} counter from yourself.\"");
 let (result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact("Tavo",&text,false)));
 assert!(!loss.is_lossy(),"{}",loss.reasons_text());let (artifact,direct)=result.unwrap();let restored=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();assert_eq!(artifact,restored);
 [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
fn event(player:PlayerId)->ironsmith::triggers::TriggerEvent {
 ironsmith::triggers::TriggerEvent::new_with_provenance(ironsmith::events::phase::BeginningOfPrecombatMainPhaseEvent::new(player),Default::default())
}
fn ready(game:&mut GameState,player:PlayerId,ordinal:u32) {game.turn.active_player=player;game.turn.phase=ironsmith::game_state::Phase::FirstMain;game.turn.step=None;game.turn_store.main_phases_started_this_turn=ordinal;}
#[test]
fn each_counter_mills_separately_and_only_its_nonland_result_loses_life_and_one_counter() {
 for name in ["corruption","restlessness"] {for def in definitions(name) {for scenario in 0..3 {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=game.create_object_from_definition(&def,A,Zone::Stack);let effects=def.spell_effect.as_ref().unwrap().all_effects_owned();
  execute_effect(&mut game,&Effect::new(ironsmith::effects::SequenceEffect::new(effects)),&mut EffectContext::new_default(source,A).with_targets(vec![ResolvedTarget::Player(B)])).unwrap();
  let kind=ironsmith::CounterType::Named(name.into());assert_eq!(game.player(B).unwrap().counter_count(kind),3);assert_eq!(game.effect_store.delayed_triggers.len(),1);assert_eq!(game.effect_store.delayed_triggers[0].controller,B);
  game.move_object_by_effect(source,Zone::Graveyard).unwrap();
  let creature=ironsmith_compiler_runtime::compile_to_runtime_definition("Nonland library object","Type: Creature\nPower/Toughness: 1/1",false).unwrap();let land=ironsmith_compiler_runtime::compile_to_runtime_definition("Land library object","Type: Land",false).unwrap();
  if scenario==0 {game.create_object_from_definition(&land,B,Zone::Library);for _ in 0..2 {game.create_object_from_definition(&creature,B,Zone::Library);}}
  if scenario==1 {game.create_object_from_definition(&creature,B,Zone::Library);}
  ready(&mut game,A,1);assert!(ironsmith::triggers::check_delayed_triggers(&mut game,&event(A)).is_empty());
  ready(&mut game,B,2);assert!(ironsmith::triggers::check_delayed_triggers(&mut game,&event(B)).is_empty(),"extra precombat phase is not first main");
  ready(&mut game,B,1);let entries=ironsmith::triggers::check_delayed_triggers(&mut game,&event(B));assert_eq!(entries.len(),1);assert_eq!(entries[0].source_kind,ironsmith::triggers::TriggeredAbilitySourceKind::GameRule);
  let mut queue=ironsmith::triggers::TriggerQueue::new();for entry in entries {queue.add(entry);}
  ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut ironsmith::decision::SelectFirstDecisionMaker).unwrap();ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut ironsmith::decision::SelectFirstDecisionMaker).unwrap();
  let nonlands=match scenario {0=>2,1=>1,_=>0};let milled=match scenario {0=>3,1=>1,_=>0};
  assert_eq!(game.player(B).unwrap().graveyard.len(),milled);assert_eq!(game.player(B).unwrap().life,20-nonlands);assert_eq!(game.player(B).unwrap().counter_count(kind),3-nonlands as u32);assert_eq!(game.player(A).unwrap().life,20);
  assert!(game.player(B).unwrap().library.is_empty());
 }}}
}
#[test]
fn repeated_counter_gains_share_one_meaning_and_permanent_counters_do_not_install_it() {
 for def in definitions("restlessness") {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=game.create_object_from_definition(&def,A,Zone::Stack);let program=Effect::new(ironsmith::effects::SequenceEffect::new(def.spell_effect.as_ref().unwrap().all_effects_owned()));
  for _ in 0..2 {execute_effect(&mut game,&program,&mut EffectContext::new_default(source,A).with_targets(vec![ResolvedTarget::Player(B)])).unwrap();}
  assert_eq!(game.effect_store.delayed_triggers.len(),1);assert_eq!(game.player(B).unwrap().counter_count(ironsmith::CounterType::Named("restlessness".into())),6);
  let permanent=ironsmith_compiler_runtime::compile_to_runtime_definition("Unrelated land","Type: Land",false).unwrap();let permanent=game.create_object_from_definition(&permanent,A,Zone::Battlefield);game.add_counters(permanent,ironsmith::CounterType::Named("corruption".into()),1).unwrap();assert_eq!(game.effect_store.delayed_triggers.len(),1,"ordinary permanent counter names carry no player meaning");
 }
}

#[test]
fn first_actual_next_main_phase_is_admitted_and_later_next_main_is_rejected() {
 for def in definitions("restlessness") {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=game.create_object_from_definition(&def,A,Zone::Stack);
  let program=Effect::new(ironsmith::effects::SequenceEffect::new(def.spell_effect.as_ref().unwrap().all_effects_owned()));execute_effect(&mut game,&program,&mut EffectContext::new_default(source,A).with_targets(vec![ResolvedTarget::Player(B)])).unwrap();
  ready(&mut game,B,1);game.turn.phase=ironsmith::game_state::Phase::NextMain;
  let event=|ordinal| ironsmith::triggers::TriggerEvent::new_with_provenance(ironsmith::events::phase::BeginningOfPostcombatMainPhaseEvent::new(B).with_main_phase_ordinal(ordinal),Default::default());
  let entries=ironsmith::triggers::check_delayed_triggers(&mut game,&event(1));assert_eq!(entries.len(),1,"the first main that actually began can be NextMain");
  game.turn_store.main_phases_started_this_turn=2;assert!(ironsmith::triggers::check_delayed_triggers(&mut game,&event(2)).is_empty());
 }
}
