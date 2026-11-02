//! Battlefield merge reuses physical components while retaining owners and noncopiable state.
use ironsmith::{CardDefinition,Effect,GameState,ObjectId,PlayerId,Zone};
use ironsmith::ability::AbilityKind;
use ironsmith::decision::{DecisionMaker,SelectFirstDecisionMaker};
use ironsmith::effects::{EffectContext,execute_effect};
use ironsmith::object::{AttachmentTarget,CounterType,ObjectKind};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
struct Choices{accept:bool,incoming_top:bool}
impl DecisionMaker for Choices{
 fn decide_boolean(&mut self,_:&GameState,_:&ironsmith::decisions::context::BooleanContext)->bool{self.accept}
 fn decide_objects(&mut self,game:&GameState,ctx:&ironsmith::decisions::context::SelectObjectsContext)->Vec<ObjectId>{SelectFirstDecisionMaker.decide_objects(game,ctx)}
 fn decide_options(&mut self,game:&GameState,ctx:&ironsmith::decisions::context::SelectOptionsContext)->Vec<usize>{if ctx.description=="Choose the order of the merged permanent"{vec![usize::from(!self.incoming_top)]}else{SelectFirstDecisionMaker.decide_options(game,ctx)}}
}
fn definitions(name:&str,text:&str)->[CardDefinition;2]{let(result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact(name,text,false)));assert!(!loss.is_lossy(),"{}",loss.reasons_text());let(artifact,direct)=result.unwrap();let restored=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();assert_eq!(artifact,restored);[direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]}
fn merge_effect(definition:&CardDefinition)->ironsmith::effects::MergePermanentsEffect{definition.abilities.iter().find_map(|a|match&a.kind{AbilityKind::Activated(a)=>a.effects.all_effects().iter().find_map(|effect|effect.downcast_ref::<ironsmith::effects::MergePermanentsEffect>().cloned()),_=>None}).unwrap()}
fn merge(game:&mut GameState,prototype:&ironsmith::effects::MergePermanentsEffect,incoming:ObjectId,host:ObjectId,top:bool){let mut payload=prototype.clone();payload.host=ironsmith::target::ObjectFilter::specific(host);let mut dm=Choices{accept:true,incoming_top:top};let outcome=execute_effect(game,&Effect::new(payload),&mut EffectContext::new_default(incoming,A).with_decision_maker(&mut dm)).unwrap();assert_eq!(outcome.instruction_result().count_or_zero(),1);}
fn move_object(game:&mut GameState,id:ObjectId,zone:Zone){let result=execute_effect(game,&Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::target::ChooseSpec::SpecificObject(id),zone,false)),&mut EffectContext::new_default(id,A)).unwrap();for event in result.events{game.queue_trigger_event(Default::default(),event);}}
fn flush(game:&mut GameState,dm:&mut Choices){let mut queue=ironsmith::triggers::TriggerQueue::new();ironsmith::game_loop::drain_pending_trigger_events_with_dm(game,&mut queue,dm).unwrap();ironsmith::game_loop::put_triggers_on_stack_with_dm(game,&mut queue,dm).unwrap();}
const OPERATION:&str="Mana cost: {3}{R}\nType: Creature — Human Wizard\nPower/Toughness: 4/5\nFlying\n{0}: Merge this permanent with another creature you control, choosing which is on top and moving all counters and attachments to the merged permanent.";
#[test]
fn battlefield_merge_preserves_top_abilities_counters_attachments_and_component_owners(){
 let operation=format!("{OPERATION}\nIf one or more +1/+1 counters would be put on a creature you control, twice that many +1/+1 counters are put on it instead.");
 for incoming_definition in definitions("Incoming physical card",&operation){for top in [false,true]{
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let incoming=game.create_object_from_definition(&incoming_definition,B,Zone::Battlefield);game.set_current_controller(incoming,A).unwrap();let incoming_stable=game.object(incoming).unwrap().stable_id;
  let host_definition=ironsmith_compiler_runtime::compile_to_runtime_definition("Host physical card","Type: Creature — Human Cleric\nPower/Toughness: 2/3\nVigilance",false).unwrap();let host=game.create_object_from_definition(&host_definition,A,Zone::Battlefield);let host_stable=game.object(host).unwrap().stable_id;
  game.object_mut(incoming).unwrap().counters.insert(CounterType::PlusOnePlusOne,2);game.object_mut(incoming).unwrap().counters.insert(CounterType::Charge,1);game.object_mut(host).unwrap().counters.insert(CounterType::PlusOnePlusOne,1);game.object_mut(host).unwrap().counters.insert(CounterType::Charge,3);
  let aura=ironsmith_compiler_runtime::compile_to_runtime_definition("Attached aura","Type: Enchantment — Aura\nEnchant creature\nEnchanted creature gets +1/+0.",false).unwrap();let aura=game.create_object_from_definition(&aura,B,Zone::Battlefield);assert!(game.attach_object_to_target(aura,AttachmentTarget::Object(incoming)));
  let equipment=ironsmith_compiler_runtime::compile_to_runtime_definition("Attached equipment","Type: Artifact — Equipment\nEquipped creature gets +0/+1.\nEquip {1}",false).unwrap();let equipment=game.create_object_from_definition(&equipment,A,Zone::Battlefield);assert!(game.attach_object_to_target(equipment,AttachmentTarget::Object(incoming)));
  let watcher=ironsmith_compiler_runtime::compile_to_runtime_definition("Zone watcher","Type: Enchantment\nWhenever a creature enters, you gain 1 life.\nWhenever a creature leaves the battlefield, you gain 2 life.",false).unwrap();game.create_object_from_definition(&watcher,A,Zone::Battlefield);
  let mut dm=Choices{accept:true,incoming_top:top};flush(&mut game,&mut dm);assert!(game.stack.is_empty());merge(&mut game,&merge_effect(&incoming_definition),incoming,host,top);
  assert!(game.object(incoming).is_none());assert_eq!(game.merged_permanent(host_stable).unwrap().components.len(),2);assert_eq!(game.counter_count(host,CounterType::PlusOnePlusOne),5,"the incoming replacement ability is active exactly once on the merged recipient before actual counter movement");assert_eq!(game.counter_count(host,CounterType::Charge),4);
  assert_eq!(game.object(aura).unwrap().attached_to,Some(AttachmentTarget::Object(host)));assert_eq!(game.object(equipment).unwrap().attached_to,Some(AttachmentTarget::Object(host)));assert_eq!(game.current_power(host),Some(if top{10}else{8}));assert_eq!(game.current_toughness(host),Some(if top{11}else{9}));
  assert!(game.current_has_static_ability_id(host,ironsmith::static_abilities::StaticAbilityId::Flying));assert!(game.current_has_static_ability_id(host,ironsmith::static_abilities::StaticAbilityId::Vigilance));flush(&mut game,&mut dm);assert!(game.stack.is_empty(),"merging emits no artificial leave or enter event");assert_eq!(game.player(A).unwrap().life,20);
  move_object(&mut game,host,Zone::Graveyard);let incoming_card=game.find_object_by_stable_id(incoming_stable).unwrap();let host_card=game.find_object_by_stable_id(host_stable).unwrap();assert_eq!(game.object(incoming_card).unwrap().owner,B);assert_eq!(game.object(host_card).unwrap().owner,A);assert!(game.player(B).unwrap().graveyard.contains(&incoming_card));assert!(game.player(A).unwrap().graveyard.contains(&host_card));assert!(game.object(incoming_card).unwrap().counters.is_empty());assert!(game.object(host_card).unwrap().counters.is_empty());
 }}
}
#[test]
fn complete_source_has_optional_self_and_other_entry_merge_programs(){
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-gestalt-next.json")).unwrap();let card=&fixture["mechanics"][0]["representative_cards"][0];
 for source_definition in definitions(card["name"].as_str().unwrap(),card["executable_text"].as_str().unwrap()){for accept in [false,true]{
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let body=ironsmith_compiler_runtime::compile_to_runtime_definition("Existing host","Type: Creature — Bird\nPower/Toughness: 1/1\nFlying",false).unwrap();let host=game.create_object_from_definition(&body,A,Zone::Battlefield);let stable=game.object(host).unwrap().stable_id;
  let source=game.create_object_from_definition(&source_definition,A,Zone::Hand);let mut dm=Choices{accept,incoming_top:true};move_object(&mut game,source,Zone::Battlefield);flush(&mut game,&mut dm);assert_eq!(game.stack.len(),1);ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();assert_eq!(game.merged_permanent(stable).map(|group|group.components.len()),accept.then_some(2));
  let third=game.create_object_from_definition(&body,A,Zone::Hand);move_object(&mut game,third,Zone::Battlefield);flush(&mut game,&mut dm);assert_eq!(game.stack.len(),1,"the other-entry trigger remains available even under another component");ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();assert_eq!(game.merged_permanent(stable).map(|group|group.components.len()),accept.then_some(3));
 }}
}
#[test]
fn merging_existing_groups_flattens_components_once_and_token_copies_do_not_recreate_them(){
 for definition in definitions("Group component",OPERATION){
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let prototype=merge_effect(&definition);let mut cards=Vec::new();for owner in [A,B,A,B]{let id=game.create_object_from_definition(&definition,owner,Zone::Battlefield);game.set_current_controller(id,A).unwrap();cards.push((id,game.object(id).unwrap().stable_id,owner));}
  merge(&mut game,&prototype,cards[0].0,cards[1].0,true);merge(&mut game,&prototype,cards[2].0,cards[3].0,false);merge(&mut game,&prototype,cards[1].0,cards[3].0,true);let host=cards[3].0;assert_eq!(game.merged_permanent(cards[3].1).unwrap().components.len(),4);
  let producer=ironsmith_compiler_runtime::compile_to_runtime_definition("Token producer","Type: Sorcery\nCreate a 1/1 blue Bird creature token with flying.",false).unwrap();let source=game.create_object_from_definition(&producer,A,Zone::Stack);for effect in producer.spell_effect.as_ref().unwrap().all_effects_owned(){execute_effect(&mut game,&effect,&mut EffectContext::new_default(source,A)).unwrap();}let token=*game.battlefield.iter().find(|id|game.object(**id).unwrap().kind==ObjectKind::Token).unwrap();let token_stable=game.object(token).unwrap().stable_id;merge(&mut game,&prototype,token,host,true);assert_eq!(game.merged_permanent(cards[3].1).unwrap().components.len(),5);
  let copier=ironsmith_compiler_runtime::compile_to_runtime_definition("Token copier","Type: Sorcery\nCreate a token that's a copy of target creature.",false).unwrap();let copier_id=game.create_object_from_definition(&copier,A,Zone::Stack);let before=game.battlefield.clone();for effect in copier.spell_effect.as_ref().unwrap().all_effects_owned(){execute_effect(&mut game,&effect,&mut EffectContext::new_default(copier_id,A).with_targets(vec![ironsmith::effects::ResolvedTarget::Object(host)])).unwrap();}let copied=*game.battlefield.iter().find(|id|!before.contains(id)).unwrap();let copy_stable=game.object(copied).unwrap().stable_id;assert!(game.merged_permanent(copy_stable).is_none(),"copying copiable values does not duplicate physical components");
  move_object(&mut game,copied,Zone::Hand);ironsmith::rules::state_based::apply_state_based_actions_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();assert!(game.find_object_by_stable_id(copy_stable).is_none());assert!(game.player(A).unwrap().graveyard.is_empty());assert!(game.player(B).unwrap().graveyard.is_empty());
  move_object(&mut game,host,Zone::Graveyard);ironsmith::rules::state_based::apply_state_based_actions_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();assert!(game.find_object_by_stable_id(token_stable).is_none());for(_,stable,owner)in &cards{let id=game.find_object_by_stable_id(*stable).unwrap();assert_eq!(game.object(id).unwrap().owner,*owner);assert!(game.player(*owner).unwrap().graveyard.contains(&id));}assert_eq!(game.player(A).unwrap().graveyard.len(),2);assert_eq!(game.player(B).unwrap().graveyard.len(),2);
 }
}

#[test]
fn absorbed_component_is_not_an_independent_source_but_other_real_doublers_still_apply() {
 let replacement="If one or more +1/+1 counters would be put on a creature you control, twice that many +1/+1 counters are put on it instead.";
 for definition in definitions("Incoming replacement component",&format!("{OPERATION}\n{replacement}")) {
  for external in [false,true] {
   let mut game=GameState::new(vec!["A".into(),"B".into()],20);
   let incoming=game.create_object_from_definition(&definition,A,Zone::Battlefield);
   let host_definition=ironsmith_compiler_runtime::compile_to_runtime_definition("Independent merge host","Type: Creature — Human\nPower/Toughness: 1/1",false).unwrap();
   let host=game.create_object_from_definition(&host_definition,A,Zone::Battlefield);
   if external {
    let observer=ironsmith_compiler_runtime::compile_to_runtime_definition("Independent counter replacement",&format!("Type: Enchantment\n{replacement}"),false).unwrap();
    game.create_object_from_definition(&observer,A,Zone::Battlefield);
   }
   game.object_mut(incoming).unwrap().counters.insert(CounterType::PlusOnePlusOne,2);
   game.object_mut(host).unwrap().counters.insert(CounterType::PlusOnePlusOne,1);
   merge(&mut game,&merge_effect(&definition),incoming,host,false);
   assert_eq!(game.counter_count(host,CounterType::PlusOnePlusOne),if external{9}else{5},"each real replacement source applies once; absorbed tombstones do not count as another permanent");
   assert!(!game.object_ids_in_deterministic_order().contains(&incoming));
   assert!(game.object(incoming).is_none(),"state-transfer retention ends at merge completion");
  }
 }
}

#[test]
fn complete_gestalt_source_phases_out_with_attachments_until_cleanup_not_extra_untap() {
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-gestalt-next.json")).unwrap();let card=&fixture["mechanics"][0]["representative_cards"][0];
 for definition in definitions(card["name"].as_str().unwrap(),card["executable_text"].as_str().unwrap()) {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.active_player=A;
  let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);let physical=game.object(source).unwrap().stable_id;
  game.tap(source);game.object_mut(source).unwrap().counters.insert(CounterType::Charge,3);
  let body=ironsmith_compiler_runtime::compile_to_runtime_definition("Ordinary body","Type: Creature\nPower/Toughness: 2/2",false).unwrap();let normal=game.create_object_from_definition(&body,A,Zone::Battlefield);
  let equipment=ironsmith_compiler_runtime::compile_to_runtime_definition("Attachment","Type: Artifact — Equipment",false).unwrap();let attachment=game.create_object_from_definition(&equipment,A,Zone::Battlefield);
  game.object_mut(attachment).unwrap().attached_to=Some(AttachmentTarget::Object(source));game.object_mut(source).unwrap().attachments.push(attachment);game.mark_continuous_state_dirty();
  let cast=ironsmith_compiler_runtime::compile_to_runtime_definition("Opponent spell","Type: Instant\nYou gain 1 life.",false).unwrap();let spell=game.create_object_from_definition(&cast,B,Zone::Stack);
  let mut dm=Choices{accept:false,incoming_top:false};flush(&mut game,&mut dm);
  for _ in 0..8 {if game.stack.is_empty(){break;}ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();flush(&mut game,&mut dm);}assert!(game.stack.is_empty());
  game.queue_trigger_event(Default::default(),ironsmith::triggers::TriggerEvent::new_with_provenance(ironsmith::events::SpellCastEvent::new(spell,B,Zone::Hand),Default::default()));
  flush(&mut game,&mut dm);assert_eq!(game.stack.len(),1);
  ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();assert!(game.is_phased_out(source));assert!(game.is_phased_out(attachment));assert!(game.effect_store.delayed_triggers.is_empty(),"duration return is not a delayed stack trigger");
  execute_effect(&mut game,&Effect::new(ironsmith::effects::PhaseOutEffect::new(ironsmith::target::ChooseSpec::SpecificObject(normal))),&mut EffectContext::new_default(normal,A)).unwrap();
  ironsmith::turn::execute_untap_step(&mut game);assert!(game.is_phased_out(source));assert!(game.is_phased_out(attachment));assert!(!game.is_phased_out(normal),"normal phasing still returns at untap");
  ironsmith::turn::try_execute_cleanup_step(&mut game).unwrap();assert!(!game.is_phased_out(source));assert!(!game.is_phased_out(attachment));assert_eq!(game.find_object_by_stable_id(physical),Some(source));assert!(game.is_tapped(source));assert_eq!(game.counter_count(source,CounterType::Charge),3);assert_eq!(game.object(attachment).unwrap().attached_to,Some(AttachmentTarget::Object(source)));
  flush(&mut game,&mut dm);assert!(game.stack.is_empty(),"phasing creates no false Gestalt entry event");
  ironsmith::turn::try_execute_cleanup_step(&mut game).unwrap();assert!(!game.is_phased_out(source));
 }
}
#[test]
fn end_of_turn_phase_deadline_does_not_follow_a_new_incarnation() {
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);let body=ironsmith_compiler_runtime::compile_to_runtime_definition("Physical creature","Type: Creature\nPower/Toughness: 2/2",false).unwrap();let source=game.create_object_from_definition(&body,A,Zone::Battlefield);
 let mut phase=ironsmith::effects::PhaseOutEffect::new(ironsmith::target::ChooseSpec::SpecificObject(source));phase.duration=ironsmith::effects::PhaseOutDuration::UntilEndOfTurn;execute_effect(&mut game,&Effect::new(phase),&mut EffectContext::new_default(source,A)).unwrap();
 game.phase_in(source);let hand=game.move_object(source,Zone::Hand,Default::default()).unwrap();let new=game.move_object(hand,Zone::Battlefield,Default::default()).unwrap();assert_ne!(source,new);
 execute_effect(&mut game,&Effect::new(ironsmith::effects::PhaseOutEffect::new(ironsmith::target::ChooseSpec::SpecificObject(new))),&mut EffectContext::new_default(new,A)).unwrap();
 ironsmith::turn::try_execute_cleanup_step(&mut game).unwrap();assert!(game.is_phased_out(new),"the canceled old deadline cannot return a different incarnation");
 ironsmith::turn::execute_untap_step(&mut game);assert!(!game.is_phased_out(new));
}
