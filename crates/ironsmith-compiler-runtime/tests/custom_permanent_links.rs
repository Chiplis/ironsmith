//! Directed relationships preserve distinct local durations and shared bounds.
use ironsmith::{CardDefinition,Effect,GameState,ObjectId,PlayerId,Zone};
use ironsmith::effects::{EffectContext,execute_effect};
use ironsmith::target::ChooseSpec;
#[path="custom_final_next_wave_support.rs"] mod source_routes;
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
fn definitions(name:&str,text:&str)->[CardDefinition;2]{
 source_routes::definitions(&serde_json::json!({"name":name,"executable_text":text})).unwrap()
}
fn object(game:&mut GameState,owner:PlayerId,types:&str)->ObjectId{let def=ironsmith_compiler_runtime::compile_to_runtime_definition(types,&format!("Type: {types}\n{}",if types.contains("Creature"){"Power/Toughness: 2/2"}else{""}),false).unwrap();game.create_object_from_definition(&def,owner,Zone::Battlefield)}
// Use the ordinary public continuous-effect owner so current type changes
// and link-duration invalidation are observed in the same gameplay world.
fn set_creature_type(game:&mut GameState,target:ObjectId,present:bool){
 let modification=if present {ironsmith::continuous::Modification::AddCardTypes(vec![ironsmith::CardType::Creature])}
  else {ironsmith::continuous::Modification::RemoveCardTypes(vec![ironsmith::CardType::Creature])};
 let effect=Effect::new(ironsmith::effects::ApplyContinuousEffect::with_spec(ChooseSpec::SpecificObject(target),modification,ironsmith::effect::Until::Forever));
 execute_effect(game,&effect,&mut EffectContext::new_default(target,A)).unwrap();
 game.refresh_continuous_state().unwrap();
 assert_eq!(game.current_is_creature(target),present,"the actual layered type change must be visible");
}
fn link(game:&mut GameState,source:ObjectId,target:ObjectId,relation:&str,creatures:bool)->bool{
 let effect=Effect::new(ironsmith::effects::LinkPermanentsEffect{source:ChooseSpec::SpecificObject(source),target:ChooseSpec::SpecificObject(target),relation:relation.into(),require_creatures:creatures});execute_effect(game,&effect,&mut EffectContext::new_default(source,A)).unwrap().as_count()==Some(1)
}
#[test]fn directed_links_replace_only_the_same_sources_named_target_and_allow_shared_targets(){
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);let a=object(&mut game,A,"Creature");let b=object(&mut game,A,"Creature");let first=object(&mut game,A,"Artifact");let second=object(&mut game,A,"Artifact");let enemy=object(&mut game,B,"Artifact");
 assert!(link(&mut game,a,first,"sample",false));assert!(link(&mut game,b,first,"sample",false));assert!(link(&mut game,a,second,"other relation",false));
 assert_eq!(game.linked_permanent(a,"sample"),Some(first));assert_eq!(game.linked_permanent(b,"sample"),Some(first));
 assert!(link(&mut game,a,second,"sample",false));assert_eq!(game.linked_permanent(a,"sample"),Some(second));assert_eq!(game.linked_permanent(b,"sample"),Some(first));assert_eq!(game.linked_permanent(a,"other relation"),Some(second));
 assert!(!link(&mut game,a,enemy,"sample",false));assert_eq!(game.linked_permanent(a,"sample"),Some(second),"invalid replacement cannot erase the old valid link");
 let stable=game.object(second).unwrap().stable_id;let hand=game.move_object_by_effect(second,Zone::Hand).unwrap();let returned=game.move_object_by_effect(hand,Zone::Battlefield).unwrap();assert_eq!(game.object(returned).unwrap().stable_id,stable);assert_eq!(game.linked_permanent(a,"sample"),None);assert_eq!(game.linked_permanent(a,"other relation"),None);assert_eq!(game.linked_permanent(b,"sample"),Some(first));
}
#[test]fn literal_lifetime_variants_keep_their_distinct_creature_constraints(){
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=object(&mut game,A,"Creature");let artifact=object(&mut game,A,"Artifact");let artifact_creature=object(&mut game,A,"Artifact Creature");
 assert!(link(&mut game,source,artifact,"permanent variant",false));assert!(!link(&mut game,source,artifact,"creature variant",true));assert!(link(&mut game,source,artifact_creature,"creature variant",true));
 set_creature_type(&mut game,artifact_creature,false);assert_eq!(game.linked_permanent(source,"creature variant"),None);assert_eq!(game.linked_permanent(source,"permanent variant"),Some(artifact));
 set_creature_type(&mut game,artifact_creature,true);assert_eq!(game.linked_permanent(source,"creature variant"),None,"a lapsed link does not return when types return");
}
#[test]fn generic_link_reader_and_dynamic_grant_survive_artifact_transport(){
 let text="Type: Creature\nPower/Toughness: 2/3\n{0}: Choose an artifact you control. Link this permanent to it by the sample relation for as long as both remain permanents on the battlefield under your control.\nArtifact linked by sample to this permanent has flying.";
 for definition in definitions("Link source",text){let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);let target=object(&mut game,A,"Artifact");
 let effects=definition.abilities.iter().find_map(|ability|match &ability.kind{ironsmith::ability::AbilityKind::Activated(ability)=>Some(ability.effects.all_effects_owned()),_=>None}).unwrap();let sequence=Effect::new(ironsmith::effects::SequenceEffect::new(effects));execute_effect(&mut game,&sequence,&mut EffectContext::new_default(source,A)).unwrap();assert_eq!(game.linked_permanent(source,"sample"),Some(target));
 let has_flying=|game:&GameState|game.calculated_characteristics_arc(target).unwrap().static_abilities.iter().any(|ability|ability.id()==ironsmith::static_abilities::StaticAbilityId::Flying);assert!(has_flying(&game));game.move_object_by_effect(source,Zone::Hand).unwrap();game.refresh_continuous_state().unwrap();assert!(!has_flying(&game));}
}
#[test]fn full_source_candidate_preserves_both_formal_duration_branches(){
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-overclock-links.json")).unwrap();let row=&fixture["mechanics"][0];assert_eq!(row["semantic_equivalence"],"pending_native_verification");let card=&row["representative_cards"][0];source_routes::definitions(card).unwrap();
}
#[test]fn control_changes_end_links_permanently_even_if_the_old_controller_returns(){
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=object(&mut game,A,"Creature");let other=object(&mut game,A,"Creature");let target=object(&mut game,A,"Artifact");
 assert!(link(&mut game,source,target,"sample",false));assert!(link(&mut game,other,target,"sample",false));game.set_current_controller(source,B).unwrap();assert_eq!(game.linked_permanent(source,"sample"),None);assert_eq!(game.linked_permanent(other,"sample"),Some(target));game.set_current_controller(source,A).unwrap();assert_eq!(game.linked_permanent(source,"sample"),None);
 game.set_current_controller(target,B).unwrap();assert_eq!(game.linked_permanent(other,"sample"),None);game.set_current_controller(target,A).unwrap();assert_eq!(game.linked_permanent(other,"sample"),None);
}
#[test]fn phasing_preserves_a_creature_constrained_link_without_copying_its_state(){
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=object(&mut game,A,"Creature");let target=object(&mut game,A,"Artifact Creature");assert!(link(&mut game,source,target,"sample",true));game.phase_out(source);game.phase_out(target);game.refresh_continuous_state().unwrap();assert_eq!(game.linked_permanent(source,"sample"),Some(target));game.phase_in(source);game.phase_in(target);game.refresh_continuous_state().unwrap();assert_eq!(game.linked_permanent(source,"sample"),Some(target));let independent=object(&mut game,A,"Creature");assert_eq!(game.linked_permanent(independent,"sample"),None);
}

#[test]fn losing_source_creature_type_expires_only_the_creature_duration(){
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=object(&mut game,A,"Artifact Creature");let target=object(&mut game,A,"Artifact Creature");
 assert!(link(&mut game,source,target,"permanent duration",false));assert!(link(&mut game,source,target,"creature duration",true));
 set_creature_type(&mut game,source,false);
 assert_eq!(game.linked_permanent(source,"permanent duration"),Some(target));assert_eq!(game.linked_permanent(source,"creature duration"),None);
}
#[test]fn both_duration_branches_share_control_zone_and_relink_bounds(){
 for creatures in [false,true]{for event in 0..5{
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=object(&mut game,A,"Artifact Creature");let target=object(&mut game,A,"Artifact Creature");let next=object(&mut game,A,"Artifact Creature");
  assert!(link(&mut game,source,target,"sample",creatures));
  match event{0=>{game.set_current_controller(source,B).unwrap();},1=>{game.set_current_controller(target,B).unwrap();},2=>{game.move_object_by_effect(source,Zone::Hand).unwrap();},3=>{game.move_object_by_effect(target,Zone::Hand).unwrap();},_=>{assert!(link(&mut game,source,next,"sample",creatures));}}
  assert_eq!(game.linked_permanent(source,"sample"),if event==4{Some(next)}else{None},"mode={creatures} event={event}");
 }}
}

#[test]
fn phased_out_endpoints_cannot_form_a_new_link_or_erase_an_existing_link(){
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);
 let source=object(&mut game,A,"Creature");let original=object(&mut game,A,"Artifact");let other=object(&mut game,A,"Artifact");
 assert!(link(&mut game,source,original,"sample",false));
 game.phase_out(other);assert!(!link(&mut game,source,other,"sample",false));assert_eq!(game.linked_permanent(source,"sample"),Some(original));
 game.phase_out(source);assert!(!link(&mut game,source,original,"new relation",false));assert_eq!(game.linked_permanent(source,"sample"),Some(original));
 game.phase_in(source);game.phase_in(other);assert!(link(&mut game,source,other,"sample",false));
}
#[test]
fn typed_link_filter_words_rewrite_without_renaming_opaque_relation_identity(){
 let black=ironsmith::ObjectFilter{colors:Some(ironsmith::color::ColorSet::BLACK),..Default::default()};
 let original=Effect::new(ironsmith::effects::LinkPermanentsEffect{source:ChooseSpec::Object(black.clone()),target:ChooseSpec::Object(black),relation:"black relation".into(),require_creatures:true});
 let rewritten=original.with_text_change(ironsmith_core::TextChange::color(ironsmith::color::Color::Black,ironsmith::color::Color::Blue).unwrap()).unwrap();
 let link=rewritten.downcast_ref::<ironsmith::effects::LinkPermanentsEffect>().unwrap();
 for spec in [&link.source,&link.target]{let ChooseSpec::Object(filter)=spec else{panic!("object operand changed kind")};assert_eq!(filter.colors,Some(ironsmith::color::ColorSet::BLUE));}
 assert_eq!(link.relation,"black relation");assert!(link.require_creatures);
}
fn exact_source()->serde_json::Value{
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-overclock-links.json")).unwrap();fixture["mechanics"][0]["representative_cards"][0].clone()
}
fn enter_for_source(game:&mut GameState,card:&CardDefinition)->ObjectId{
 let hand=game.create_object_from_definition(card,A,Zone::Hand);let stable=game.object(hand).unwrap().stable_id;
 let outcome=execute_effect(game,&Effect::new(ironsmith::effects::MoveToZoneEffect::new(ChooseSpec::SpecificObject(hand),Zone::Battlefield,false)),&mut EffectContext::new_default(hand,A)).unwrap();
 for event in outcome.events{game.queue_trigger_event(Default::default(),event);}
 game.find_object_by_stable_id(stable).unwrap()
}
fn resolve_entry_triggers(game:&mut GameState){
 let mut queue=ironsmith::triggers::TriggerQueue::new();let mut dm=ironsmith::decision::SelectFirstDecisionMaker;
 ironsmith::game_loop::drain_pending_trigger_events_with_dm(game,&mut queue,&mut dm).unwrap();
 ironsmith::game_loop::put_triggers_on_stack_with_dm(game,&mut queue,&mut dm).unwrap();
 while !game.stack.is_empty(){ironsmith::game_loop::resolve_stack_entry_with(game,&mut dm).unwrap();ironsmith::game_loop::drain_pending_trigger_events_with_dm(game,&mut queue,&mut dm).unwrap();ironsmith::game_loop::put_triggers_on_stack_with_dm(game,&mut queue,&mut dm).unwrap();}
}
#[test]
fn exact_source_entry_links_its_actual_artifact_and_grants_the_printed_activated_ability(){
 for definition in source_routes::definitions(&exact_source()).unwrap(){
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let target=object(&mut game,A,"Artifact");
  let source=enter_for_source(&mut game,&definition);resolve_entry_triggers(&mut game);
  assert_eq!(game.linked_permanent(source,"overclock"),Some(target));
  assert!(game.current_characteristics(target).unwrap().abilities.iter().any(|ability|matches!(&ability.kind,ironsmith::AbilityKind::Activated(_))),"actual linked artifact receives the source's printed damage ability");
  game.move_object_by_effect(source,Zone::Hand).unwrap();game.refresh_continuous_state().unwrap();
  assert!(!game.current_characteristics(target).unwrap().abilities.iter().any(|ability|matches!(&ability.kind,ironsmith::AbilityKind::Activated(_))));
 }
}
#[test]
fn exact_source_artifact_entry_retains_the_formal_creature_only_local_duration(){
 for definition in source_routes::definitions(&exact_source()).unwrap(){for creature in [false,true]{
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
  let types=if creature{"Artifact Creature"}else{"Artifact"};let body=ironsmith_compiler_runtime::compile_to_runtime_definition("Incoming artifact",&format!("Type: {types}\n{}",if creature{"Power/Toughness: 2/2"}else{""}),false).unwrap();
  let target=enter_for_source(&mut game,&body);resolve_entry_triggers(&mut game);
  assert_eq!(game.linked_permanent(source,"overclock"),creature.then_some(target));
 }}
}
