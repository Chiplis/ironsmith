//! Source749 submodules invoke actual retained activated occurrences.
use ironsmith::{CardDefinition,GameState,ObjectId,Zone};
use ironsmith::decision::{compute_legal_actions,LegalAction};
use ironsmith::game_loop::{drain_pending_trigger_events_with_dm,put_triggers_on_stack_with_dm,resolve_stack_entry_with};
#[path="cf8_p08/support.rs"] mod support;
#[path="cf8_p08/play.rs"] mod play;
fn fixture()->serde_json::Value{serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-module.json")).unwrap()}
#[path="custom_final_next_wave_support.rs"] mod source_routes;
fn definitions(card:&serde_json::Value)->[CardDefinition;2]{
 source_routes::definitions(card).unwrap_or_else(|error|panic!("{}: {error}",card["name"]))
}
fn source(name:&str)->[CardDefinition;2]{definitions(fixture()["mechanics"][0]["representative_cards"].as_array().unwrap().iter().find(|card|card["name"]==name).unwrap())}
fn legal(game:&GameState,source:ObjectId,index:usize)->bool{compute_legal_actions(game,play::A).unwrap().iter().any(|action|matches!(action,LegalAction::ActivateAbility{source:id,ability_index}|LegalAction::ActivateManaAbility{source:id,ability_index} if *id==source&&*ability_index==index))}
fn enter_existing(game:&mut GameState,card:ObjectId,dm:&mut play::Script)->ObjectId{
 let stable=game.object(card).unwrap().stable_id;
 let entry={let mut ctx=ironsmith::effects::EffectContext::new(card,play::A,dm);
  ironsmith::effects::execute_effect(game,&ironsmith::Effect::put_onto_battlefield(ironsmith::target::ChooseSpec::SpecificObject(card),false,ironsmith::target::PlayerFilter::You),&mut ctx).unwrap()};
 for event in entry.events {game.queue_trigger_event(Default::default(),event);}
 let id=game.find_object_by_stable_id(stable).unwrap();
 let mut queue=ironsmith::triggers::TriggerQueue::new();
 drain_pending_trigger_events_with_dm(game,&mut queue,dm).unwrap();put_triggers_on_stack_with_dm(game,&mut queue,dm).unwrap();assert_eq!(game.stack.len(),1);id
}
fn enter(game:&mut GameState,definition:&CardDefinition,dm:&mut play::Script)->ObjectId{
 let card=game.create_object_from_definition(definition,play::A,Zone::Hand);enter_existing(game,card,dm)
}
#[test]
fn all_sixteen_complete_source_module_actor_and_consumer_faces_compile_directly_and_through_artifacts(){
 for card in fixture()["mechanics"][0]["representative_cards"].as_array().unwrap(){let _=definitions(card);}
}
#[test]
fn entry_activates_a_real_submodule_for_free_on_an_opponents_turn_and_counts_its_use(){
 for definition in source("Morality Module"){
  let mut game=play::game();game.turn.active_player=play::B;game.turn.phase=ironsmith::game_state::Phase::Combat;
  let mut dm=play::Script::default();let id=enter(&mut game,&definition,&mut dm);
  resolve_stack_entry_with(&mut game,&mut dm).unwrap();
  assert_eq!(game.stack.len(),1,"entry resolution announces a separate activated stack entry");
  assert_eq!(game.player(play::A).unwrap().life,20,"the chosen ability waits on the stack");
  game.stack.pop(); // Countering the activation still consumes its once receipt.
  game.turn.active_player=play::A;game.turn.phase=ironsmith::game_state::Phase::FirstMain;game.turn.priority_player=Some(play::A);
  for color in [ironsmith::mana::ManaSymbol::White,ironsmith::mana::ManaSymbol::Black]{play::give_mana(&mut game,play::A,color,10);}
  assert!(!legal(&game,id,play::activated_at(&definition,0)));assert!(legal(&game,id,play::activated_at(&definition,1)));
 }
}
#[test]
fn optional_entry_can_be_declined_and_normal_activation_still_pays_its_printed_cost(){
 for definition in source("Morality Module"){
  let mut game=play::game();let mut dm=play::Script{decline:true,..Default::default()};let id=enter(&mut game,&definition,&mut dm);
  resolve_stack_entry_with(&mut game,&mut dm).unwrap();assert!(game.stack.is_empty());assert_eq!(game.player(play::A).unwrap().life,20);
  let first=play::activated_at(&definition,0);assert!(!legal(&game,id,first),"ordinary activation is not made free by the intrinsic entry ability");
  play::give_mana(&mut game,play::A,ironsmith::mana::ManaSymbol::White,3);assert!(legal(&game,id,first));
  dm.decline=false;play::activate(&mut game,id,first,&mut dm);resolve_stack_entry_with(&mut game,&mut dm).unwrap();assert_eq!(game.player(play::A).unwrap().life,26);assert!(!legal(&game,id,first));
 }
}
#[test]
fn entry_chooses_legal_actual_targets_and_preserves_source_incarnation_boundaries(){
 for definition in source("Logic Module"){
  let mut game=play::game();let body=play::vanilla("Witness","{0}","Soldier",2,2);let target=game.create_object_from_definition(&body,play::B,Zone::Battlefield);
  let mut dm=play::Script{targets:vec![ironsmith::Target::Object(target)],..Default::default()};let id=enter(&mut game,&definition,&mut dm);
  resolve_stack_entry_with(&mut game,&mut dm).unwrap();assert_eq!(game.stack.len(),1);assert!(dm.offered_targets.iter().any(|offered|offered.contains(&ironsmith::Target::Object(target))));
  resolve_stack_entry_with(&mut game,&mut dm).unwrap();assert_eq!(game.battlefield.len(),3);
  let exiled=game.move_object_by_effect(id,Zone::Exile).unwrap();dm.decline=true;let returned=enter_existing(&mut game,exiled,&mut dm);resolve_stack_entry_with(&mut game,&mut dm).unwrap();dm.decline=false;assert_ne!(id,returned);
  play::give_mana(&mut game,play::A,ironsmith::mana::ManaSymbol::Blue,3);game.turn.priority_player=Some(play::A);assert!(legal(&game,returned,play::activated_at(&definition,0)));
 }
}

#[test]
fn comma_separated_spell_types_filter_the_exact_exiled_pool_on_both_routes(){
 let body="Type: Sorcery\nMana cost: {0}\nExile the top three cards of your library. You may cast an enchantment, instant, or sorcery spell exiled this way until the end of your next turn.";
 for definition in support::definitions("Filtered exile permission",body){
  let mut game=play::game();let source=game.create_object_from_definition(&definition,play::A,Zone::Stack);
  let mut originals=Vec::new();
  for (name,text) in [("Eligible enchantment","Type: Enchantment\nMana cost: {0}"),("Eligible instant","Type: Instant\nMana cost: {0}\nYou gain 1 life."),("Excluded creature","Type: Creature\nMana cost: {0}\nPower/Toughness: 1/1")]{
   let card=ironsmith_compiler_runtime::compile_to_runtime_definition(name,text,false).unwrap();
   let id=game.create_object_from_definition(&card,play::A,Zone::Library);originals.push((name,game.object(id).unwrap().stable_id));
  }
  let mut ctx=ironsmith::effects::EffectContext::new_default(source,play::A);
  let program=ironsmith::Effect::new(ironsmith::effects::SequenceEffect::new(definition.spell_effect.as_ref().unwrap().all_effects_owned()));
  ironsmith::effects::execute_effect(&mut game,&program,&mut ctx).unwrap();
  let actions=compute_legal_actions(&game,play::A).unwrap();
  for (name,stable) in originals{
   let id=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(id).unwrap().zone,Zone::Exile);
   let cast=actions.iter().any(|action|matches!(action,LegalAction::CastSpell{spell_id,from_zone:Zone::Exile,..}if *spell_id==id));
   assert_eq!(cast,name!="Excluded creature","complete type list must filter the actual exiled packet: {name}");
  }
 }
}

#[test]
fn actual_pyp_resolution_chooses_one_real_free_cast_or_submodule_announcement(){
 use ironsmith::decision::{DecisionMaker,SelectFirstDecisionMaker};
 struct PypChoice{mode:usize,accept:bool}
 impl DecisionMaker for PypChoice{
  fn decide_options(&mut self,game:&GameState,ctx:&ironsmith::decisions::context::SelectOptionsContext)->Vec<usize>{
   if ctx.options.len()==2&&ctx.options.iter().any(|option|option.description.to_lowercase().contains("designation module")){vec![self.mode]}else{SelectFirstDecisionMaker.decide_options(game,ctx)}
  }
  fn decide_boolean(&mut self,_:&GameState,_:&ironsmith::decisions::context::BooleanContext)->bool{self.accept}
  fn decide_objects(&mut self,game:&GameState,ctx:&ironsmith::decisions::context::SelectObjectsContext)->Vec<ObjectId>{SelectFirstDecisionMaker.decide_objects(game,ctx)}
  fn decide_targets(&mut self,game:&GameState,ctx:&ironsmith::decisions::context::TargetsContext)->Vec<ironsmith::Target>{SelectFirstDecisionMaker.decide_targets(game,ctx)}
 }
 for (route,pyp) in source("Pyp, the Indexer").into_iter().enumerate(){
  assert_eq!(pyp.abilities.iter().filter(|ability|matches!(&ability.kind,ironsmith::AbilityKind::Static(_))).count(),1,"complete draw replacement sibling must survive compilation");
  let morality=source("Morality Module")[route].clone();
  for mode in [0,1]{for accept in [false,true]{
   let mut game=play::game();game.turn.phase=ironsmith::game_state::Phase::Combat;game.turn.priority_player=Some(play::B);
   let module=game.create_object_from_definition(&morality,play::A,Zone::Battlefield);let source=game.create_object_from_definition(&pyp,play::A,Zone::Battlefield);
   let spell=ironsmith_compiler_runtime::compile_to_runtime_definition("Eligible enchantment","Mana cost: {9}\nType: Enchantment",false).unwrap();let spell=game.create_object_from_definition(&spell,play::A,Zone::Hand);let stable=game.object(spell).unwrap().stable_id;
   let excluded=play::vanilla("Excluded hand creature","{0}","Fox",1,1);let excluded=game.create_object_from_definition(&excluded,play::A,Zone::Hand);
   let ironsmith::AbilityKind::Activated(ability)=&pyp.abilities[play::activated_at(&pyp,0)].kind else{panic!("actual source activation")};
   let effects=ability.effects.all_effects_owned();let mut dm=PypChoice{mode,accept};
   ironsmith::effects::execute_effect(&mut game,&ironsmith::Effect::new(ironsmith::effects::SequenceEffect::new(effects)),&mut ironsmith::effects::EffectContext::new_default(source,play::A).with_decision_maker(&mut dm)).unwrap();
   assert_eq!(game.stack.len(),usize::from(accept));assert_eq!(game.player(play::A).unwrap().mana_pool.total(),0);assert_eq!(game.player(play::A).unwrap().life,20,"inner action waits on its own stack entry");assert_eq!(game.object(excluded).unwrap().zone,Zone::Hand);
   let spell=game.find_object_by_stable_id(stable).unwrap();
   if !accept{assert_eq!(game.object(spell).unwrap().zone,Zone::Hand);continue;}
   if mode==0{assert!(!game.stack[0].is_ability);assert_eq!(game.object(spell).unwrap().zone,Zone::Stack);}else{assert!(game.stack[0].is_ability);assert_eq!(game.stack[0].object_id,module);assert_eq!(game.stack[0].ability_index,Some(play::activated_at(&morality,0)));assert_eq!(game.object(spell).unwrap().zone,Zone::Hand);}
   resolve_stack_entry_with(&mut game,&mut dm).unwrap();
   if mode==0{let spell=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(spell).unwrap().zone,Zone::Battlefield);assert_eq!(game.player(play::A).unwrap().life,20);}else{assert_eq!(game.player(play::A).unwrap().life,26);}
  }}
 }
}

#[test]
fn actual_fury_discount_starts_only_in_the_next_actual_turn_and_is_consumed_once() {
 for definition in source("Fury Module") { for consume in [false,true] {
  let mut game=play::game();let mut dm=play::Script::default();
  let issuer=game.create_object_from_definition(&definition,play::A,Zone::Battlefield);
  play::give_mana(&mut game,play::A,ironsmith::ManaSymbol::Red,1);
  play::give_mana(&mut game,play::A,ironsmith::ManaSymbol::Colorless,3);
  play::activate(&mut game,issuer,play::activated_at(&definition,0),&mut dm);
  resolve_stack_entry_with(&mut game,&mut dm).unwrap();
  let probe=ironsmith_compiler_runtime::compile_to_runtime_definition("Independent future payment","Mana cost: {3}\nType: Instant\nYou gain 1 life.",false).unwrap();
  let candidate=game.create_object_from_definition(&probe,play::A,Zone::Hand);
  let can_cast=|game:&GameState|compute_legal_actions(game,play::A).unwrap().iter().any(|action|matches!(action,LegalAction::CastSpell{spell_id,..} if *spell_id==candidate));
  assert!(!can_cast(&game),"a future discount cannot pay today's spell");
  game.move_object_by_effect(issuer,Zone::Graveyard).unwrap();
  game.cleanup_temporary_spell_cost_reductions_end_of_turn();
  assert_eq!(game.effect_store.temporary_spell_cost_reductions.len(),1,"pending window survives ordinary cleanup and source departure");
  for _ in 0..3 {
   game.next_turn();
   game.turn.priority_player=Some(play::A);
   if game.is_active_player(play::A) {break;}
   assert!(!can_cast(&game),"the first opponent turn is not the requested window");
  }
  assert!(game.is_active_player(play::A));
  game.turn.phase=ironsmith::Phase::FirstMain;game.turn.step=None;
  assert!(can_cast(&game),"the reduction starts at the actual next own-turn boundary");
  let unrelated=play::vanilla("Nonmatching future body","{3}","Elf",2,2);
  let unrelated=game.create_object_from_definition(&unrelated,play::A,Zone::Hand);
  assert!(!compute_legal_actions(&game,play::A).unwrap().iter().any(|action|matches!(action,LegalAction::CastSpell{spell_id,..} if *spell_id==unrelated)),"nonmatching casts cannot borrow the instant/sorcery modifier");
  if consume {
   play::cast(&mut game,play::A,&probe,&mut dm);
   assert!(!can_cast(&game),"the completed matching cast consumes the one use");
   game.stack.pop(); // Countering that spell does not restore the spent discount.
   assert!(!can_cast(&game));
  }
  game.next_turn();game.turn.priority_player=Some(play::A);
  assert!(!can_cast(&game),"an unused window also expires when that turn ends");
 }}
}
