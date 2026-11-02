//! Complete physical source admission; native and artifact routes compile independently.
mod custom_final_next_wave_support;
use custom_final_next_wave_support::definitions;
use serde_json::Value;
fn full_family(fixture:&str,keyword:&str,expected:&[&str])->Vec<Value> {
 let fixture:Value=serde_json::from_str(fixture).unwrap();let mut sources=Vec::new();
 for row in fixture["mechanics"].as_array().unwrap().iter().filter(|r|r["keyword"]==keyword) {
  sources.extend(row["representative_cards"].as_array().unwrap().iter().cloned());
 }
 let canonical=|s:&str|s.split_once(':').map_or(s,|(_,key)|key).to_owned();
 let actual:std::collections::BTreeSet<_>=sources.iter().map(|s|canonical(s["source_card_key"].as_str().unwrap())).collect();
 let expected:std::collections::BTreeSet<_>=expected.iter().map(|s|canonical(s)).collect();
 assert_eq!(actual,expected,"exact physical family inventory");assert_eq!(sources.len(),actual.len(),"no duplicate source keys");sources
}
#[test]
fn every_malign_and_unite_physical_source_is_strict_and_lossless_on_both_routes() {
 let families=[
  (include_str!("../../../fixtures/custom_mechanics/msem-malign.json"),"Malign",MALIGN),
  (include_str!("../../../fixtures/custom_mechanics/revolution-abilities.json"),"Unite",UNITE),
 ];let mut failures=Vec::new();let mut checked=0;
 for (fixture,keyword,expected) in families {for source in full_family(fixture,keyword,expected) {
  checked+=1;let key=source["source_card_key"].as_str().unwrap();
  if source["executable_text"].as_str().is_none_or(|text|text.trim().is_empty()) {failures.push(format!("{keyword} {key}: missing full source executable body"));continue;}
  match definitions(&source) {
   Err(error)=>failures.push(format!("{keyword} {key}: {error}")),
   Ok([native,artifact])=>{
    let native=ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(native).map_err(|error|format!("{error:?}")).and_then(|value|serde_json::to_value(value).map_err(|error|error.to_string()));
    let artifact=ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(artifact).map_err(|error|format!("{error:?}")).and_then(|value|serde_json::to_value(value).map_err(|error|error.to_string()));
    match(native,artifact){(Ok(native),Ok(artifact))=>if native!=artifact{failures.push(format!("{keyword} {key}: complete typed native/artifact source differs"));},(left,right)=>failures.push(format!("{keyword} {key}: typed encoding errors {left:?} {right:?}")),}
   }
  }
 }}
 assert_eq!(checked,31);assert!(failures.is_empty(),"{} full-source errors:\n{}",failures.len(),failures.join("\n"));
}
const MALIGN:&[&str]=&["msem:Angelic Tactician_LIP", "msem:Charging Lance_LIP", "msem:Dogmatic Tactician_LIP", "msem:Fanatical Vanguard_LIP", "msem:Hateful Hunter_LIP", "msem:Instinctive Commander_LIP", "msem:Lurid Battlemaster_LIP", "msem:Rallied Purifier_LIP", "msem:Reckless Slayer_LIP", "msem:Vengeful Purifier_LIP", "msem:Vicious Orator_LIP", "msem:Vinza, Adamant Firebrand_LIP"];
const UNITE:&[&str]=&["revolution:Caller of the Great Bears_KUT", "revolution:Combat-Form Instructor_KUT", "revolution:Courtyard Sanctifier_KUT", "revolution:Cunning Pal Rai Yuk_KUT", "revolution:Guru of Tumbling Stone_KUT", "revolution:Monastery Flagdancers_KUT", "revolution:Paro Katsang Avenger_KUT", "revolution:Pass Pilgrim_KUT", "revolution:Rakshasa Initiate_KUT", "revolution:Seekers of Khembul_KUT", "revolution:Snow-Pass Guardian_KUT", "revolution:Spirit of the First Dawn_KUT", "revolution:Sunflare Disciple_KUT", "revolution:Sunrise Mentor_KUT", "revolution:Tender of the Heartglade_KUT", "revolution:Combat-Form Instructor (Avatar)_PRO_KUT", "revolution:Guru of Tumbling Stone (Avatar)_PRO_KUT", "revolution:Monastery Flagdancers (Avatar)_PRO_KUT", "revolution:Sunflare Disciple (Avatar)_PRO_KUT"];

use ironsmith::{CardDefinition,Effect,GameState,ObjectId,PlayerId,Zone};
use ironsmith::decision::AttackerDeclaration;
use ironsmith::combat_state::{AttackTarget,CombatState};
use ironsmith::effects::{EffectContext,execute_effect};
use ironsmith::object::CounterType;
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
fn attack(game:&mut GameState,player:PlayerId) {
    let d=ironsmith_compiler_runtime::compile_to_runtime_definition("Attack history witness","Type: Creature\nPower/Toughness: 1/1",false).unwrap();
    let attacker=game.create_object_from_definition(&d,player,Zone::Battlefield);game.remove_summoning_sickness(attacker);
    game.turn.active_player=player;game.turn.phase=ironsmith::game_state::Phase::Combat;game.turn.step=Some(ironsmith::game_state::Step::DeclareAttackers);game.mark_combat_phase_started();
    let mut combat=CombatState::default();let mut queue=ironsmith::triggers::TriggerQueue::new();
    ironsmith::game_loop::apply_attacker_declarations(game,&mut combat,&mut queue,&[AttackerDeclaration{creature:attacker,target:AttackTarget::Player(if player==A {B}else{A})}]).unwrap();
    assert!(game.turn_store.turn_history.players_attacked_this_turn.contains(&player));
    // The historical fact outlives every creature which declared the attack.
    execute_effect(game,&Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::target::ChooseSpec::SpecificObject(attacker),Zone::Graveyard,false)),&mut EffectContext::new_default(attacker,player)).unwrap();
}
fn enter(game:&mut GameState,definition:&CardDefinition,controller:PlayerId)->ObjectId {
    let source=game.create_object_from_definition(definition,controller,Zone::Hand);let stable=game.object(source).unwrap().stable_id;
    execute_effect(game,&Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::target::ChooseSpec::SpecificObject(source),Zone::Battlefield,false)),&mut EffectContext::new_default(source,controller)).unwrap();
    let arrived=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(arrived).unwrap().zone,Zone::Battlefield);arrived
}

#[test]
fn every_intrinsic_malign_source_uses_its_controller_history_and_ordinary_entry() {
 let fixture=include_str!("../../../fixtures/custom_mechanics/msem-malign.json");
 for source in full_family(fixture,"Malign",MALIGN) {
  // Fanatical Vanguard grants Malign to spells; it has no intrinsic instance.
  let original=source["original_text"].as_str().unwrap();
  let Some(line)=original.lines().find(|line|line.starts_with("Malign ")) else {assert_eq!(source["name"],"Fanatical Vanguard");continue};
  let amount:u32=line.split_whitespace().nth(1).unwrap().parse().unwrap();
  for definition in definitions(&source).unwrap() {for who in [None,Some(A),Some(B)] {
   let mut game=GameState::new(vec!["A".into(),"B".into()],20);if let Some(player)=who {attack(&mut game,player);}
   let arrived=enter(&mut game,&definition,A);
   assert_eq!(game.counter_count(arrived,CounterType::PlusOnePlusOne),if who==Some(A){amount}else{0},"{} history {who:?}",source["source_card_key"]);
  }}
 }
}
#[test]
fn full_source_malign_grant_adds_an_independent_entry_instance_and_ends_with_provider() {
 let sources=full_family(include_str!("../../../fixtures/custom_mechanics/msem-malign.json"),"Malign",MALIGN);
 let provider=sources.iter().find(|s|s["name"]=="Vengeful Purifier").unwrap();
 let recipient=sources.iter().find(|s|s["name"]=="Angelic Tactician").unwrap();
 for (provider,recipient) in definitions(provider).unwrap().into_iter().zip(definitions(recipient).unwrap()) {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);attack(&mut game,A);
  let grant=enter(&mut game,&provider,A);assert_eq!(game.counter_count(grant,CounterType::PlusOnePlusOne),1,"other excludes provider");
  let first=enter(&mut game,&recipient,A);assert_eq!(game.counter_count(first,CounterType::PlusOnePlusOne),2);
  execute_effect(&mut game,&Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::target::ChooseSpec::SpecificObject(grant),Zone::Exile,false)),&mut EffectContext::new_default(grant,A)).unwrap();
  let second=enter(&mut game,&recipient,A);assert_eq!(game.counter_count(second,CounterType::PlusOnePlusOne),1);
  assert_eq!(game.counter_count(first,CounterType::PlusOnePlusOne),2,"existing counters survive grant removal");
 }
}

struct UniteChoices { accept:bool, chosen:Vec<ObjectId> }
impl ironsmith::decision::DecisionMaker for UniteChoices {
 fn decide_boolean(&mut self,_:&GameState,_:&ironsmith::decisions::context::BooleanContext)->bool{self.accept}
 fn decide_objects(&mut self,_:&GameState,ctx:&ironsmith::decisions::context::SelectObjectsContext)->Vec<ObjectId>{ctx.candidates.iter().filter(|candidate|candidate.legal&&self.chosen.contains(&candidate.id)).map(|candidate|candidate.id).collect()}
}
#[test]
fn full_source_unite_action_exports_completed_taps_to_its_own_independent_watcher() {
 let sources=full_family(include_str!("../../../fixtures/custom_mechanics/revolution-abilities.json"),"Unite",UNITE);
 for source in sources.iter().filter(|s|s["source_card_key"].as_str().unwrap().contains("Combat-Form Instructor")) {
  for definition in definitions(source).unwrap() {for accept in [false,true] {for block_at_resolution in [false,true] {
   let mut game=GameState::new(vec!["A".into(),"B".into()],20);
   let id=game.create_object_from_definition(&definition,A,Zone::Battlefield);game.remove_summoning_sickness(id);
   let body=ironsmith_compiler_runtime::compile_to_runtime_definition("Ordinary participant","Type: Creature\nPower/Toughness: 1/1\nShroud",false).unwrap();
   let first=game.create_object_from_definition(&body,A,Zone::Battlefield);let second=game.create_object_from_definition(&body,A,Zone::Battlefield);
   let bystander=game.create_object_from_definition(&body,A,Zone::Battlefield);let enemy=game.create_object_from_definition(&body,B,Zone::Battlefield);
   game.turn.active_player=A;game.turn.phase=ironsmith::game_state::Phase::Combat;game.turn.step=Some(ironsmith::game_state::Step::DeclareAttackers);game.mark_combat_phase_started();
   let mut queue=ironsmith::triggers::TriggerQueue::new();let mut combat=CombatState::default();let mut dm=UniteChoices{accept,chosen:vec![first,second]};
   ironsmith::game_loop::apply_attacker_declarations(&mut game,&mut combat,&mut queue,&[AttackerDeclaration{creature:id,target:AttackTarget::Player(B)}]).unwrap();assert_eq!(queue.entries.len(),1);
   ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut dm).unwrap();
   if block_at_resolution{game.add_counters(id,CounterType::PlusOnePlusOne,1).unwrap();}
   ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
   let paid=accept&&!block_at_resolution;assert_eq!(game.is_tapped(first),paid);assert_eq!(game.is_tapped(second),paid);assert!(!game.is_tapped(bystander));assert!(!game.is_tapped(enemy));
   ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game,&mut queue,&mut dm).unwrap();assert_eq!(queue.entries.len(),usize::from(paid));
   if paid{let objects=&queue.entries[0].tagged_objects["__named_action_objects"];assert_eq!(objects.iter().map(|o|o.object_id).collect::<std::collections::HashSet<_>>(),[first,second].into_iter().collect());ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut dm).unwrap();ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();}
   for object in [first,second]{assert_eq!(game.counter_count(object,CounterType::PlusOnePlusOne),u32::from(paid));}
   assert_eq!(game.counter_count(id,CounterType::PlusOnePlusOne),u32::from(paid||block_at_resolution));assert_eq!(game.counter_count(bystander,CounterType::PlusOnePlusOne),0);
  }}}
 }
}
