//! Existing-operation reminder abilities retain their runtime behavior through artifacts.
use ironsmith::{CardDefinition,GameState,PlayerId,Zone};
use ironsmith::object::CounterType;
use ironsmith::combat_state::{AttackTarget,CombatState};
use ironsmith::decision::{AttackerDeclaration,SelectFirstDecisionMaker};
use ironsmith::game_loop::{apply_attacker_declarations,put_triggers_on_stack_with_dm,resolve_stack_entry_with};
use ironsmith::triggers::TriggerQueue;
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A:PlayerId=PlayerId(0); const B:PlayerId=PlayerId(1);
fn definitions(name:&str,text:&str)->[CardDefinition;2] {
 let (result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact(name,text,false)));
 assert!(!loss.is_lossy(),"{}",loss.reasons_text());let (artifact,direct)=result.unwrap();
 let restored=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();assert_eq!(restored,artifact);
 [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
#[test]
fn experience_counter_gate_checks_attack_and_resolution_independently() {
 for counter in [CounterType::PlusOnePlusOne,CounterType::Level] {
  let noun=if counter==CounterType::Level {"level"}else{"+1/+1"};
  let text=format!("Type: Creature\nPower/Toughness: 2/2\nWhenever this creature attacks, if it has fewer than three {noun} counters on it, put a {noun} counter on it.");
  for definition in definitions("Independent attack learner",&text) {for (initial,added) in [(0,0),(2,0),(3,0),(2,1)] {
   let mut game=GameState::new(vec!["A".into(),"B".into()],20);
   game.turn.active_player=A;game.turn.phase=ironsmith::game_state::Phase::Combat;game.turn.step=Some(ironsmith::game_state::Step::DeclareAttackers);game.mark_combat_phase_started();
   let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);game.remove_summoning_sickness(source);
   if initial>0 {game.add_counters(source,counter,initial).unwrap();}
   let mut queue=TriggerQueue::new();let mut combat=CombatState::default();
   apply_attacker_declarations(&mut game,&mut combat,&mut queue,&[AttackerDeclaration{creature:source,target:AttackTarget::Player(B)}]).unwrap();
   assert_eq!(queue.entries.len(),usize::from(initial<3));
   put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();
   if added>0 {game.add_counters(source,counter,added).unwrap();}
   while !game.stack.is_empty() {resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();}
   assert_eq!(game.counter_count(source,counter),initial+added+u32::from(initial+added<3));
  }}
 }
}
#[test]
fn every_materialized_existing_operation_source_compiles_losslessly() {
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-abilities-tail.json")).unwrap();
 let mut failures=Vec::new(); let mut checked=0;
 for mechanic in fixture["mechanics"].as_array().unwrap() {for card in mechanic["representative_cards"].as_array().unwrap() {
  if card["rewrite_status"]!="frontend_materialized_unverified" {continue;}
  let mut faces=vec![("representative".to_string(),card)];
  if let Some(extra)=card["executable_faces"].as_array() {faces.extend(extra.iter().enumerate().map(|(i,f)|(format!("face[{i}]"),f)));}
  for (face_key,face) in faces {
   checked+=1;
   let key=format!("row={} card={} source={} {} name={}",mechanic["id"],card["id"],card["source_card_key"],face_key,face["name"]);
   let result=std::panic::catch_unwind(|| {
    let vocabulary:Vec<ironsmith_compiler::CustomSubtypeDefinition>=serde_json::from_value(face.get("custom_subtypes").or_else(||card.get("custom_subtypes")).cloned().unwrap_or_else(||serde_json::json!([]))).expect("source subtype metadata");
    ironsmith_compiler::with_custom_subtypes(&vocabulary,|| {
     definitions(face["name"].as_str().unwrap(),face["executable_text"].as_str().expect("every materialized face needs its full body"));
    }).expect("valid source subtype scope");
   });
   if let Err(error)=result {
    let message=error.downcast_ref::<String>().map(String::as_str).or_else(||error.downcast_ref::<&str>().copied()).unwrap_or("non-text compilation panic");
    failures.push(format!("{key}: {message}"));
   }
  }
 }}
 assert!(checked>0,"source cohort must not be empty");
 assert!(failures.is_empty(),"{} full-source strict compilation failures:\n{}",failures.len(),failures.join("\n"));
}
#[test]
fn optional_mill_payment_requires_the_exact_number_and_is_not_kicker() {
 for definition in definitions("Optional mill witness","Type: Creature\nPower/Toughness: 1/1\nYou may mill three cards as you cast this spell.") {
  assert_eq!(definition.optional_costs.len(),1);
  assert!(!format!("{:?}",definition.optional_costs[0].kind).contains("Kicker"));
  assert!(!definition.optional_costs[0].repeatable);
  for library_size in [0,2,3,4] {
   let mut game=GameState::new(vec!["A".into(),"B".into()],20);
   let source=game.create_object_from_definition(&definition,A,Zone::Stack);
   for _ in 0..library_size {game.create_object_from_definition(&definition,A,Zone::Library);}
   let mut dm=SelectFirstDecisionMaker;
   let mut context=ironsmith::costs::CostContext::new(source,A,&mut dm);
   let costs=definition.optional_costs[0].cost.costs();assert_eq!(costs.len(),1);
   assert_eq!(costs[0].can_pay(&game,&context).is_ok(),library_size>=3,"library size {library_size}");
   if library_size>=3 {costs[0].pay(&mut game,&mut context).unwrap();assert_eq!(game.player(A).unwrap().library.len(),library_size-3);assert_eq!(game.player(A).unwrap().graveyard.len(),3);}
  }
 }
}
#[test]
fn husk_creates_decayed_zombie_and_attaches_equipment_only_to_that_token() {
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-abilities-tail.json")).unwrap();
 let row=fixture["mechanics"].as_array().unwrap().iter().find(|m|m["id"]=="msem-944").unwrap();let card=&row["representative_cards"][0];
 for definition in definitions(card["name"].as_str().unwrap(),card["executable_text"].as_str().unwrap()) {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  let source=game.create_object_from_definition(&definition,A,Zone::Hand);let stable=game.object(source).unwrap().stable_id;
  let other=game.create_object_from_definition(&definition,A,Zone::Battlefield);
  let outcome=ironsmith::effects::execute_effect(&mut game,&ironsmith::Effect::move_to_zone(ironsmith::target::ChooseSpec::Source,Zone::Battlefield,false),&mut ironsmith::effects::EffectContext::new_default(source,A)).unwrap();
  let mut queue=TriggerQueue::new();for event in outcome.events {for entry in ironsmith::triggers::check_triggers(&game,&event){queue.add(entry);}}
  // Zone movement queues entry events on the game, drained by the real stack owner.
  put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();assert_eq!(game.stack.len(),1);while !game.stack.is_empty(){resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();}
  let source=game.find_object_by_stable_id(stable).unwrap();
  let zombies:Vec<_>=game.battlefield.iter().copied().filter(|id|game.object(*id).unwrap().subtypes.contains(&ironsmith::types::Subtype::Zombie)).collect();
  assert_eq!(zombies.len(),1);let token=zombies[0];
  assert_eq!(game.current_power(token),Some(3));assert_eq!(game.current_toughness(token),Some(2));
  assert_eq!(game.object(source).unwrap().attached_to,Some(ironsmith::object::AttachmentTarget::Object(token)));
  assert_eq!(game.object(other).unwrap().attached_to,None);
  assert!(game.object(token).unwrap().has_static_ability_id(ironsmith::static_abilities::StaticAbilityId::CantBlock));
 }
}
#[test]
fn attacker_scoped_tapped_blocking_permission_keeps_ordinary_evasion_and_other_attackers() {
 let text="Type: Creature\nPower/Toughness: 2/2\nTapped creatures can block this creature as though they were untapped.\nThis creature can't be blocked by untapped creatures.";
 for definition in definitions("Unusual evasion witness",text) {for tapped in [false,true] {for restricted in [false,true] {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  let ordinary=ironsmith_compiler_runtime::compile_to_runtime_definition("Ordinary body","Type: Creature\nPower/Toughness: 2/2",false).unwrap();
  let attacker=game.create_object_from_definition(if restricted {&definition}else{&ordinary},A,Zone::Battlefield);
  let blocker=game.create_object_from_definition(&ordinary,B,Zone::Battlefield);
  // Another attacker bearing the permission must not leak it to this attacker.
  game.create_object_from_definition(&definition,A,Zone::Battlefield);
  game.remove_summoning_sickness(attacker);
  game.turn.active_player=A;game.turn.phase=ironsmith::game_state::Phase::Combat;game.turn.step=Some(ironsmith::game_state::Step::DeclareAttackers);game.mark_combat_phase_started();
  let mut combat=CombatState::default();let mut queue=TriggerQueue::new();
  apply_attacker_declarations(&mut game,&mut combat,&mut queue,&[AttackerDeclaration{creature:attacker,target:AttackTarget::Player(B)}]).unwrap();
  if tapped {game.tap(blocker);}
  let expected=if restricted {tapped}else{!tapped};
  assert_eq!(ironsmith::rules::combat::can_block(game.object(attacker).unwrap(),game.object(blocker).unwrap(),&game),expected);
  let options=ironsmith::decision::compute_legal_blockers(&game,&combat,B);
  assert_eq!(options[0].valid_blockers.contains(&blocker),expected);
  assert_eq!(ironsmith::combat_state::declare_blockers(&game,&mut combat,vec![(blocker,attacker)]).is_ok(),expected);
  assert_eq!(game.is_tapped(blocker),tapped,"permission does not untap the blocker");
 }}}
}
#[test]
fn attacker_scoped_permission_respects_its_blocker_filter_and_flying() {
 for suffix in ["","\nFlying"] {
  for definition in definitions("Filtered evasion witness",&format!("Type: Creature\nPower/Toughness: 2/2\nTapped Zombie creatures can block this creature as though they were untapped.{suffix}")) {
   for zombie in [false,true] {
    let mut game=GameState::new(vec!["A".into(),"B".into()],20);
    let body=ironsmith_compiler_runtime::compile_to_runtime_definition("Blocker",if zombie {"Type: Creature — Zombie\nPower/Toughness: 2/2"}else{"Type: Creature — Human\nPower/Toughness: 2/2"},false).unwrap();
    let attacker=game.create_object_from_definition(&definition,A,Zone::Battlefield);let blocker=game.create_object_from_definition(&body,B,Zone::Battlefield);game.tap(blocker);
    assert_eq!(ironsmith::rules::combat::can_block(game.object(attacker).unwrap(),game.object(blocker).unwrap(),&game),zombie && suffix.is_empty());
   }
  }
 }
}
#[test]
fn practiced_full_source_grant_reduces_only_the_next_creature_by_distinct_attackers() {
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-abilities-tail.json")).unwrap();let row=fixture["mechanics"].as_array().unwrap().iter().find(|m|m["id"]=="msem-946").unwrap();let card=&row["representative_cards"][0];
 for definition in definitions(card["name"].as_str().unwrap(),card["executable_text"].as_str().unwrap()) {for count in [0,2] {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.active_player=A;
  if count>0 {
   let body=ironsmith_compiler_runtime::compile_to_runtime_definition("Historical attacker","Type: Creature\nPower/Toughness: 1/1",false).unwrap();
   let ids:Vec<_>=(0..count).map(|_|{let id=game.create_object_from_definition(&body,A,Zone::Battlefield);game.remove_summoning_sickness(id);id}).collect();
   game.turn.phase=ironsmith::game_state::Phase::Combat;game.turn.step=Some(ironsmith::game_state::Step::DeclareAttackers);game.mark_combat_phase_started();let mut combat=CombatState::default();let mut queue=TriggerQueue::new();
   let declarations:Vec<_>=ids.iter().map(|id|AttackerDeclaration{creature:*id,target:AttackTarget::Player(B)}).collect();ironsmith::game_loop::apply_attacker_declarations(&mut game,&mut combat,&mut queue,&declarations).unwrap();
   for id in ids {ironsmith::effects::execute_effect(&mut game,&ironsmith::Effect::move_to_zone(ironsmith::target::ChooseSpec::SpecificObject(id),Zone::Graveyard,false),&mut ironsmith::effects::EffectContext::new_default(id,A)).unwrap();}
  }
  game.turn.phase=ironsmith::game_state::Phase::FirstMain;game.turn.step=None;game.turn.priority_player=Some(A);game.combat=None;
  let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
  let effects=definition.abilities.iter().find_map(|a|match &a.kind{ironsmith::ability::AbilityKind::Activated(a)=>Some(a.effects.all_effects_owned()),_=>None}).unwrap();
  ironsmith::effects::execute_effect(&mut game,&ironsmith::Effect::new(ironsmith::effects::SequenceEffect::new(effects)),&mut ironsmith::effects::EffectContext::new_default(source,A)).unwrap();
  game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,3);
  let creature=ironsmith_compiler_runtime::compile_to_runtime_definition("Next creature","Mana cost: {5}\nType: Creature\nPower/Toughness: 2/2",false).unwrap();let spell=game.create_object_from_definition(&creature,A,Zone::Hand);
  let other=ironsmith_compiler_runtime::compile_to_runtime_definition("Next noncreature","Mana cost: {5}\nType: Sorcery\nDraw a card.",false).unwrap();let other=game.create_object_from_definition(&other,A,Zone::Hand);
  let actions=ironsmith::decision::compute_legal_actions(&game,A).unwrap();
  assert_eq!(actions.iter().any(|a|matches!(a,ironsmith::decision::LegalAction::CastSpell{spell_id,..} if *spell_id==spell)),count==2);
  assert!(!actions.iter().any(|a|matches!(a,ironsmith::decision::LegalAction::CastSpell{spell_id,..} if *spell_id==other)));
 }}
}
struct AcceptCopy(bool);
impl ironsmith::decision::DecisionMaker for AcceptCopy {
 fn decide_boolean(&mut self,_:&GameState,_:&ironsmith::decisions::context::BooleanContext)->bool{self.0}
 fn decide_options(&mut self,_:&GameState,ctx:&ironsmith::decisions::context::SelectOptionsContext)->Vec<usize>{ctx.options.iter().find(|o|o.legal && (o.description.starts_with("Enter as a copy of")==self.0)).map(|o|vec![o.index]).unwrap_or_default()}
}
#[test]
fn compound_copy_exceptions_keep_name_legendary_and_own_other_abilities() {
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-abilities-tail.json")).unwrap();let row=fixture["mechanics"].as_array().unwrap().iter().find(|m|m["id"]=="msem-937").unwrap();let card=&row["representative_cards"][0];
 for definition in definitions(card["name"].as_str().unwrap(),card["executable_text"].as_str().unwrap()) {for copy in [false,true] {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  let donor=ironsmith_compiler_runtime::compile_to_runtime_definition("Copy donor","Type: Creature — Beast\nPower/Toughness: 7/8\nVigilance",false).unwrap();game.create_object_from_definition(&donor,A,Zone::Battlefield);
  let source=game.create_object_from_definition(&definition,A,Zone::Hand);let stable=game.object(source).unwrap().stable_id;
  game.move_object_with_etb_processing_with_dm(source,Zone::Battlefield,&mut AcceptCopy(copy)).unwrap();let arrived=game.find_object_by_stable_id(stable).unwrap();
  let chars=game.calculated_characteristics_arc(arrived).unwrap();assert_eq!(chars.name.as_ref(),"Ambivalence");assert!(chars.supertypes.contains(&ironsmith::types::Supertype::Legendary));
  assert_eq!(game.current_power(arrived),Some(if copy{7}else{3}));
  assert_eq!(chars.static_abilities.iter().any(|a|a.id()==ironsmith::static_abilities::StaticAbilityId::Vigilance),copy);
  assert!(chars.abilities.iter().any(|a|matches!(a.kind,ironsmith::ability::AbilityKind::Triggered(_))),"own combat trigger survives the copy");
 }}
}
