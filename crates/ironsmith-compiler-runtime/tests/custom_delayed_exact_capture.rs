//! Delayed cast bodies import exact parent objects through artifact transport.
use ironsmith::{CardDefinition,Effect,GameState,PlayerId,Zone};
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::effects::{EffectContext,execute_effect};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A:PlayerId=PlayerId(0);
fn definitions(timing:&str)->[CardDefinition;2]{
 let text=format!("Mana cost: {{R}}\nType: Instant\nExile this spell. {timing}, you may cast the card exiled this way without paying its mana cost.");
 let (result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact("Delayed parent reference",&text,false)));
 assert!(!loss.is_lossy(),"{}",loss.reasons_text());let (artifact,direct)=result.unwrap();let restored=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();assert_eq!(artifact,restored);[direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
fn cast_tag(effect:&Effect)->Option<ironsmith::tag::TagKey>{
 if let Some(cast)=effect.downcast_ref::<ironsmith::effects::CastTaggedEffect>(){return Some(cast.tag.clone())}
 let mut found=None;effect.0.visit_child_effects(&mut |child|{if found.is_none(){found=cast_tag(child);}});found
}
#[test]
fn delayed_body_cannot_revive_a_card_that_left_and_reentered_exile(){
 for timing in ["When you next cast a blue spell","At the beginning of your next upkeep"]{for definition in definitions(timing){for reenter in [false,true]{
  let effects=definition.spell_effect.as_ref().unwrap().all_effects_owned();
  let schedule=effects.iter().find_map(|effect|effect.downcast_ref::<ironsmith::effects::ScheduleDelayedTriggerEffect>()).unwrap();
  let read=schedule.effects.all_effects_owned().iter().find_map(cast_tag).unwrap();
  assert_eq!(schedule.effects.object_reference_imports,vec![read],"import metadata must name the same lowered reference the cast body reads");
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=game.create_object_from_definition(&definition,A,Zone::Stack);let stable=game.object(source).unwrap().stable_id;
  let mut dm=SelectFirstDecisionMaker;let mut ctx=EffectContext::new_default(source,A).with_decision_maker(&mut dm);for effect in definition.spell_effect.as_ref().unwrap().all_effects_owned(){execute_effect(&mut game,&effect,&mut ctx).unwrap();}
  let exiled=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(exiled).unwrap().zone,Zone::Exile);assert_eq!(game.effect_store.delayed_triggers.len(),1);
  assert!(!game.effect_store.delayed_triggers[0].effects.captured_object_references.is_empty(),"the delayed body retains explicit per-registration object imports");
  if reenter{for zone in [Zone::Hand,Zone::Exile]{let current=game.find_object_by_stable_id(stable).unwrap();execute_effect(&mut game,&Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::target::ChooseSpec::SpecificObject(current),zone,false)),&mut EffectContext::new_default(current,A)).unwrap();}assert_ne!(game.find_object_by_stable_id(stable).unwrap(),exiled);}
  let blue=ironsmith_compiler_runtime::compile_to_runtime_definition("Blue cast","Mana cost: {U}\nType: Instant\nYou gain 1 life.",false).unwrap();let spell=game.create_object_from_definition(&blue,A,Zone::Stack);
  game.turn.turn_number+=2;
  let event=if timing.starts_with("When") {ironsmith::triggers::TriggerEvent::new(ironsmith::events::SpellCastEvent::new(spell,A,Zone::Hand),Default::default())}else{ironsmith::triggers::TriggerEvent::new(ironsmith::events::phase::BeginningOfUpkeepEvent::new(A),Default::default())};let mut queue=ironsmith::triggers::TriggerQueue::new();for trigger in ironsmith::triggers::check_delayed_triggers(&mut game,&event){queue.add(trigger);}
  ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();assert_eq!(game.stack.len(),1);ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();
  assert_eq!(game.stack.len(),usize::from(!reenter),"only the original exact exile arrival can be cast");
  let current=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(current).unwrap().zone,if reenter{Zone::Exile}else{Zone::Stack});
 }}}
}
#[test]
fn independent_registrations_and_missing_imports_have_separate_exact_capture_slots(){
 let card=ironsmith_compiler_runtime::compile_to_runtime_definition("Imported parent","Type: Instant\nYou gain 1 life.",false).unwrap();
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);let registrar=ironsmith_compiler_runtime::compile_to_runtime_definition("Registration source","Type: Artifact",false).unwrap();let source=game.create_object_from_definition(&registrar,A,Zone::Battlefield);let first=game.create_object_from_definition(&card,A,Zone::Exile);let second=game.create_object_from_definition(&card,A,Zone::Exile);
 for parent in [Some(first),Some(second),None]{
  let mut program=ironsmith::resolution::ResolutionProgram::from_effects(vec![Effect::new(ironsmith::effects::CastTaggedEffect::new("parent",ironsmith::target::PlayerFilter::You).from_zone(Zone::Exile).without_paying_mana_cost())]);program.object_reference_imports.push("parent".into());
  let schedule=ironsmith::effects::ScheduleDelayedTriggerEffect::new(ironsmith::triggers::Trigger::beginning_of_end_step(ironsmith::target::PlayerFilter::Any),program,true,Vec::new(),ironsmith::target::PlayerFilter::You);
  let mut ctx=EffectContext::new_default(source,A);if let Some(parent)=parent{ctx.set_tagged_objects("parent",vec![ironsmith::snapshot::ObjectSnapshot::from_object_id(&game,parent).unwrap()]);}
  execute_effect(&mut game,&Effect::new(schedule),&mut ctx).unwrap();
 }
 let registrations=&game.effect_store.delayed_triggers;assert_eq!(registrations.len(),3);
 assert_eq!(registrations[0].effects.captured_object_references[0].1,vec![first]);
 assert_eq!(registrations[1].effects.captured_object_references[0].1,vec![second]);
 assert!(registrations[2].effects.captured_object_references[0].1.is_empty(),"a missing import is an explicitly empty exact slot, never a future lookup");
}
