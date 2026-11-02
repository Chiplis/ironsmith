//! Full physical Reverberate corpus and intrinsic mana/nonmana/permanent spell witnesses.
mod custom_final_next_wave_support;
use custom_final_next_wave_support::definitions;
fn sources()->Vec<serde_json::Value>{
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-reverberate-emblem-source.json")).unwrap();
 let sources=fixture["mechanics"][0]["representative_cards"].as_array().unwrap().clone();
 let actual:std::collections::BTreeSet<_>=sources.iter().map(|s|s["source_card_key"].as_str().unwrap().split(':').next_back().unwrap().to_owned()).collect();
 let expected:std::collections::BTreeSet<_>=EXPECTED.iter().map(|s|s.split(':').next_back().unwrap().to_owned()).collect();assert_eq!(actual,expected);assert_eq!(sources.len(),13);sources
}
fn source(name:&str)->[ironsmith::CardDefinition;2]{let sources=sources();definitions(sources.iter().find(|s|s["name"]==name).unwrap()).unwrap()}
#[test]
fn all_thirteen_exact_sources_compile_strictly_without_losing_secondary_rules(){
 let mut errors=Vec::new();for source in sources(){if let Err(error)=definitions(&source){errors.push(format!("{}: {error}",source["source_card_key"]));}}
 assert!(errors.is_empty(),"{} full-source failures:\n{}",errors.len(),errors.join("\n"));
}
const EXPECTED:&[&str]=&["msem:Lafterii Ingenuity_AFM", "msem:Controlled Decisions_KZD", "msem:Elementalize_KZD", "msem:Lafterii Trickery_KZD", "msem:Memories Reignited_KZD", "msem:Merkurio Zellurys_KZD", "msem:Mirror Image_KZD", "msem:Quiet Meditation_KZD", "msem:Searing Pulse_KZD", "msem:Unstable Experiment_KZD", "msem:Violent Spasm_KZD", "msem:Noxious Upwelling_HZN", "msem:Mystical Echo_OTH"];

use ironsmith::{GameState,PlayerId,Zone,ObjectId,GameProgress};
use ironsmith::decision::{DecisionMaker,SelectFirstDecisionMaker,LegalAction,compute_legal_actions};
use ironsmith::decisions::context::SelectOptionsContext;
use ironsmith::game_loop::{PriorityLoopState,PriorityResponse,apply_priority_response_with_dm,apply_decision_context_with_dm,resolve_stack_entry_with};
const A:PlayerId=PlayerId(0);
struct Choices{pay:bool}
impl DecisionMaker for Choices{
 fn decide_options(&mut self,game:&GameState,ctx:&SelectOptionsContext)->Vec<usize>{
  if ctx.description.starts_with("Choose optional costs"){return if self.pay{ctx.options.iter().filter(|o|o.legal).map(|o|{assert!(!o.repeatable);o.index}).collect()}else{vec![]};}
  SelectFirstDecisionMaker.decide_options(game,ctx)
 }
}
fn cast(game:&mut GameState,spell:ObjectId,dm:&mut Choices){
 let cast=compute_legal_actions(game,A).unwrap().into_iter().find(|a|matches!(a,LegalAction::CastSpell{spell_id,..}if *spell_id==spell)).unwrap();
 let mut queue=ironsmith::triggers::TriggerQueue::new();let mut state=PriorityLoopState::new(2);
 let mut progress=apply_priority_response_with_dm(game,&mut queue,&mut state,&PriorityResponse::PriorityAction(cast),dm).unwrap();
 for _ in 0..96{if !state.has_pending_action(){ironsmith::game_loop::put_triggers_on_stack(game,&mut queue).unwrap();return;}let GameProgress::NeedsDecisionCtx(ctx)=progress else{panic!("{progress:?}")};progress=apply_decision_context_with_dm(game,&mut queue,&mut state,&ctx,dm).unwrap();}panic!("cast did not finish");
}
fn game()->GameState{let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.phase=ironsmith::game_state::Phase::FirstMain;game.turn.step=None;game.turn.active_player=A;game.turn.priority_player=Some(A);for mana in [ironsmith::mana::ManaSymbol::Blue,ironsmith::mana::ManaSymbol::Black,ironsmith::mana::ManaSymbol::Green,ironsmith::mana::ManaSymbol::Red,ironsmith::mana::ManaSymbol::Colorless]{game.player_mut(A).unwrap().mana_pool.add(mana,10);}game}
fn resolve_copy_trigger(game:&mut GameState,paid:bool,dm:&mut Choices){assert_eq!(game.stack.len(),if paid{2}else{1});if paid{resolve_stack_entry_with(game,dm).unwrap();assert_eq!(game.stack.len(),2,"exactly one copied spell");}while !game.stack.is_empty(){resolve_stack_entry_with(game,dm).unwrap();}}
#[test]
fn intrinsic_mana_price_is_once_only_and_copies_the_complete_draw_spell(){
 for definition in source("Quiet Meditation"){for paid in [false,true]{
  let mut game=game();let filler=ironsmith_compiler_runtime::compile_to_runtime_definition("Library witness","Mana cost: {0}\nType: Artifact",false).unwrap();for _ in 0..4{game.create_object_from_definition(&filler,A,Zone::Library);}
  let spell=game.create_object_from_definition(&definition,A,Zone::Hand);let mut dm=Choices{pay:paid};cast(&mut game,spell,&mut dm);resolve_copy_trigger(&mut game,paid,&mut dm);
  assert_eq!(game.player(A).unwrap().hand.len(),if paid{2}else{1});
 }}
}
#[test]
fn intrinsic_nonmana_price_sacrifices_one_actual_swamp_before_cast_trigger(){
 for definition in source("Noxious Upwelling"){for paid in [false,true]{
  let mut game=game();let land=ironsmith_compiler_runtime::compile_to_runtime_definition("Swamp witness","Type: Basic Land — Swamp",false).unwrap();let swamp=game.create_object_from_definition(&land,A,Zone::Battlefield);let stable=game.object(swamp).unwrap().stable_id;
  let spell=game.create_object_from_definition(&definition,A,Zone::Hand);let mut dm=Choices{pay:paid};cast(&mut game,spell,&mut dm);
  let land=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(land).unwrap().zone,if paid{Zone::Graveyard}else{Zone::Battlefield});resolve_copy_trigger(&mut game,paid,&mut dm);
 }}
}
#[test]
fn intrinsic_permanent_spell_copy_resolves_as_token_without_a_second_payment(){
 for definition in source("Mystical Echo"){for paid in [false,true]{
  let mut game=game();let spell=game.create_object_from_definition(&definition,A,Zone::Hand);let mut dm=Choices{pay:paid};cast(&mut game,spell,&mut dm);resolve_copy_trigger(&mut game,paid,&mut dm);
  let copies:Vec<_>=game.battlefield.iter().copied().filter(|id|game.object(*id).unwrap().name.to_string()=="Mystical Echo").collect();assert_eq!(copies.len(),if paid{2}else{1});
  assert_eq!(copies.iter().filter(|id|matches!(game.object(**id).unwrap().kind,ironsmith::object::ObjectKind::Token)).count(),usize::from(paid));
  for copy in copies{assert_eq!(game.current_power(copy),Some(if paid{4}else{3}));}
 }}
}

fn external_copy(game:&mut GameState,original:ObjectId){
 use ironsmith::effects::EffectExecutor;
 ironsmith::effects::CopySpellEffect::single(ironsmith::target::ChooseSpec::SpecificObject(original)).execute(game,&mut ironsmith::effects::EffectContext::new(original,A,&mut SelectFirstDecisionMaker)).unwrap();
}
fn arrange_copies(game:&mut GameState,paid:bool,external:bool,counter_copy:bool,remove_identity:bool,dm:&mut Choices){
 let original=game.stack.first().unwrap().object_id;
 if paid{assert_eq!(game.stack.len(),2);resolve_stack_entry_with(game,dm).unwrap();}
 if external{external_copy(game,original);}
 if paid||external{
  let copy=game.stack.last().unwrap().object_id;assert_ne!(copy,original);
  assert_eq!(game.object(copy).unwrap().kind,ironsmith::object::ObjectKind::SpellCopy);
  if remove_identity{game.object_mut(copy).unwrap().abilities_mut().retain(|ability|!matches!(&ability.kind,ironsmith::ability::AbilityKind::Static(value)if value.custom_keyword_name().is_some_and(|name|name=="reverberate")));}
  if counter_copy{ironsmith::effects::execute_effect(game,&ironsmith::Effect::new(ironsmith::effects::CounterEffect::new(ironsmith::target::ChooseSpec::SpecificObject(copy))),&mut ironsmith::effects::EffectContext::new_default(original,A)).unwrap();}
 }
}
#[test]
fn actual_copy_status_excludes_paid_original_and_includes_unpaid_copy_with_exact_identity(){
 for definition in source("Unstable Experiment"){for (paid,external,counter_copy,remove_identity) in [(false,false,false,false),(true,false,false,false),(false,true,false,false),(true,false,true,false),(false,true,false,true)]{
  let mut game=game();let body=ironsmith_compiler_runtime::compile_to_runtime_definition("Durable witness","Type: Creature\nPower/Toughness: 3/5",false).unwrap();
  let own=game.create_object_from_definition(&body,A,Zone::Battlefield);let enemy=game.create_object_from_definition(&body,PlayerId(1),Zone::Battlefield);
  let spell=game.create_object_from_definition(&definition,A,Zone::Hand);let mut dm=Choices{pay:paid};cast(&mut game,spell,&mut dm);arrange_copies(&mut game,paid,external,counter_copy,remove_identity,&mut dm);
  while !game.stack.is_empty(){resolve_stack_entry_with(&mut game,&mut dm).unwrap();}
  let copy_resolved=(paid||external)&&!counter_copy;let status=copy_resolved&&!remove_identity;
  assert_eq!(game.current_power(own),Some(if status{4}else{3}),"paid original is never a reverberated spell");
  assert_eq!(game.current_power(enemy),Some(if copy_resolved{-1}else{1}),"both complete spells retain the opponent penalty regardless of status");
 }}
}
#[test]
fn violent_spasm_paid_original_without_its_copy_does_not_grant_flying(){
 for definition in source("Violent Spasm"){for (paid,external,counter_copy) in [(false,false,false),(true,false,false),(false,true,false),(true,false,true)]{
  let mut game=game();let body=ironsmith_compiler_runtime::compile_to_runtime_definition("Durable ground witness","Type: Creature\nPower/Toughness: 3/5",false).unwrap();
  let own=game.create_object_from_definition(&body,A,Zone::Battlefield);let enemy=game.create_object_from_definition(&body,PlayerId(1),Zone::Battlefield);
  let spell=game.create_object_from_definition(&definition,A,Zone::Hand);let mut dm=Choices{pay:paid};cast(&mut game,spell,&mut dm);arrange_copies(&mut game,paid,external,counter_copy,false,&mut dm);
  while !game.stack.is_empty(){resolve_stack_entry_with(&mut game,&mut dm).unwrap();}
  let copy_resolved=(paid||external)&&!counter_copy;
  assert_eq!(game.damage_on(own),if copy_resolved{0}else{1});assert_eq!(game.damage_on(enemy),if copy_resolved{2}else{1});
  let flying=game.current_characteristics(own).unwrap().abilities.iter().any(|a|matches!(&a.kind,ironsmith::ability::AbilityKind::Static(s)if s.id()==ironsmith::static_abilities::StaticAbilityId::Flying));
  assert_eq!(flying,copy_resolved,"a paid original alone never supplies the copy-only benefit");
 }}
}
