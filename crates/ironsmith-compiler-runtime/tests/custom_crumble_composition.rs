//! Four independent abilities share only the named casting receipt for entry.
mod custom_final_next_wave_support;
use custom_final_next_wave_support::definitions;
use ironsmith::{CardDefinition,GameState,PlayerId,ObjectId,Zone,GameProgress};
use ironsmith::object::{CounterType,ObjectKind};
use ironsmith::decision::{SelectFirstDecisionMaker,LegalAction,compute_legal_actions};
const A:PlayerId=PlayerId(0);
fn actual()->[CardDefinition;2]{
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-final-real-source-next-wave.json")).unwrap();let row=fixture["mechanics"].as_array().unwrap().iter().find(|r|r["id"]=="msem-915").unwrap();let card=&row["representative_cards"][0];
 assert!(card["original_text"].as_str().unwrap().contains("Crumble {2}{U}"));definitions(card).unwrap()
}
fn game()->GameState{let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.active_player=A;game.turn.priority_player=Some(A);game.turn.phase=ironsmith::game_state::Phase::FirstMain;game.turn.step=None;game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,10);game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Blue,1);let filler=ironsmith_compiler_runtime::compile_to_runtime_definition("Draw witness","Type: Artifact",false).unwrap();for _ in 0..4{game.create_object_from_definition(&filler,A,Zone::Library);}game}
fn flush(game:&mut GameState){let mut queue=ironsmith::triggers::TriggerQueue::new();for _ in 0..32{ironsmith::game_loop::drain_pending_trigger_events_with_dm(game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();ironsmith::game_loop::put_triggers_on_stack_with_dm(game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();if game.stack.is_empty(){return}ironsmith::game_loop::resolve_stack_entry_with(game,&mut SelectFirstDecisionMaker).unwrap();}panic!("trigger pipeline did not settle")}
fn cast(game:&mut GameState,definition:&CardDefinition,alternative:bool)->ObjectId{
 let source=game.create_object_from_definition(definition,A,Zone::Hand);let stable=game.object(source).unwrap().stable_id;
 let action=compute_legal_actions(game,A).unwrap().into_iter().find(|a|matches!(a,LegalAction::CastSpell{spell_id,casting_method,..}if *spell_id==source&&matches!(casting_method,ironsmith::alternative_cast::CastingMethod::Alternative(_))==alternative)).expect("both ordinary and selected named prices available");
 let mut queue=ironsmith::triggers::TriggerQueue::new();let mut state=ironsmith::game_loop::PriorityLoopState::new(2);let mut dm=SelectFirstDecisionMaker;
 let mut progress=ironsmith::game_loop::apply_priority_response_with_dm(game,&mut queue,&mut state,&ironsmith::game_loop::PriorityResponse::PriorityAction(action),&mut dm).unwrap();
 for _ in 0..64{if !state.has_pending_action(){ironsmith::game_loop::put_triggers_on_stack_with_dm(game,&mut queue,&mut dm).unwrap();flush(game);let arrived=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(arrived).unwrap().zone,Zone::Battlefield);return arrived}let GameProgress::NeedsDecisionCtx(ctx)=progress else{panic!("{progress:?}")};progress=ironsmith::game_loop::apply_decision_context_with_dm(game,&mut queue,&mut state,&ctx,&mut dm).unwrap();}panic!("cast did not finish")
}
fn upkeep(game:&mut GameState){game.turn.phase=ironsmith::game_state::Phase::Beginning;game.turn.step=Some(ironsmith::game_state::Step::Upkeep);game.queue_trigger_event(Default::default(),ironsmith::triggers::TriggerEvent::new(ironsmith::events::BeginningOfUpkeepEvent::new(A),Default::default()));flush(game);}
#[test]
fn opaque_four_ability_program_reads_only_the_selected_named_price_for_entry(){
 let source=serde_json::json!({"name":"Independent clock","custom_keyword_identities":["dissolve"],"custom_keyword_payments":["dissolve"],"executable_text":"Mana cost: {1}\nType: Artifact\nYou may pay {3} rather than pay this spell's mana cost.\nIf its dissolve cost was paid, this permanent enters with two charge counters on it.\nAt the beginning of your upkeep, if this permanent has a charge counter on it, remove a charge counter from it.\nWhen the last charge counter is removed from this permanent, sacrifice it and draw a card."});
 for definition in definitions(&source).unwrap(){for alternative in [false,true]{let mut game=game();let id=cast(&mut game,&definition,alternative);let stable=game.object(id).unwrap().stable_id;assert_eq!(game.counter_count(id,CounterType::Charge),if alternative{2}else{0});assert_eq!(game.object(id).unwrap().optional_costs_paid.paid_custom_keyword("dissolve"),alternative);
  if !alternative{game.add_counters(id,CounterType::Charge,1).unwrap();}else{upkeep(&mut game);assert_eq!(game.counter_count(id,CounterType::Charge),1);}
  upkeep(&mut game);let id=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(id).unwrap().zone,Zone::Graveyard);assert_eq!(game.player(A).unwrap().hand.len(),1,"last-counter ability exists for either price");
 }}
}
#[test]
fn complete_potato_chip_swan_has_unconditional_countdown_and_copies_without_paid_receipts(){
 for definition in actual(){for alternative in [false,true]{let mut game=game();let original=cast(&mut game,&definition,alternative);let stable=game.object(original).unwrap().stable_id;
  assert_eq!(game.counter_count(original,CounterType::Time),if alternative{3}else{0});assert_eq!(game.object(original).unwrap().optional_costs_paid.paid_custom_keyword("crumble"),alternative);
  // A normal-cast Swan's other three abilities still function if it gets a
  // time counter later; the entry condition does not gate its whole text box.
  if !alternative{game.add_counters(original,CounterType::Time,1).unwrap();}
  let count=if alternative{3}else{1};for remaining in (0..count).rev(){upkeep(&mut game);let current=game.find_object_by_stable_id(stable).unwrap();if remaining>0{assert_eq!(game.counter_count(current,CounterType::Time),remaining);}else{assert_eq!(game.object(current).unwrap().zone,Zone::Graveyard);}}
  let copies:Vec<_>=game.battlefield.iter().copied().filter(|id|game.object(*id).unwrap().name.to_string()=="Potato Chip Swan").collect();assert_eq!(copies.len(),count as usize);
  for copy in copies{let object=game.object(copy).unwrap();assert!(matches!(object.kind,ObjectKind::Token));assert_eq!(game.counter_count(copy,CounterType::Time),0,"copy has no casting receipt and gains no conditional entry counters");assert!(!object.optional_costs_paid.paid_custom_keyword("crumble"));}
  assert_eq!(game.player(A).unwrap().hand.len(),1,"exactly the last-counter trigger draws");
 }}
}
#[test]
fn another_player_removing_the_last_counter_still_sacrifices_but_does_not_create_a_copy(){
 for definition in actual(){let mut game=game();let swan=cast(&mut game,&definition,false);let stable=game.object(swan).unwrap().stable_id;game.add_counters(swan,CounterType::Time,1).unwrap();
  let other=ironsmith_compiler_runtime::compile_to_runtime_definition("Counter removal witness","Type: Artifact",false).unwrap();let other=game.create_object_from_definition(&other,PlayerId(1),Zone::Battlefield);
  let effect=ironsmith::Effect::new(ironsmith::effects::RemoveCountersEffect::new(CounterType::Time,1,ironsmith::target::ChooseSpec::SpecificObject(swan)));
  ironsmith::effects::execute_effect(&mut game,&effect,&mut ironsmith::effects::EffectContext::new_default(other,PlayerId(1))).unwrap();flush(&mut game);
  let current=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(current).unwrap().zone,Zone::Graveyard);assert_eq!(game.player(A).unwrap().hand.len(),1);
  assert!(!game.battlefield.iter().any(|id|game.object(*id).unwrap().name.to_string()=="Potato Chip Swan"),"the source's separate you-remove watcher must read the actual remover");
 }
}
