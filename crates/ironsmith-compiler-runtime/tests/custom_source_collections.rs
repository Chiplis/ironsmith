//! Repeated registrations pool exact source-linked cards under one ordinary watcher.
use ironsmith::{Effect,GameState,ObjectId,PlayerId,Zone};
use ironsmith::effects::{EffectContext,execute_effect};
use ironsmith::target::{ChooseSpec,ObjectFilter,PlayerFilter};
use ironsmith::decision::SelectFirstDecisionMaker;
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);const C:PlayerId=PlayerId(2);
fn body()->ironsmith::CardDefinition{ironsmith_compiler_runtime::compile_to_runtime_definition("Carrier","Type: Creature\nPower/Toughness: 3/3",false).unwrap()}
fn schedule()->ironsmith::effects::ScheduleDelayedTriggerEffect{
 let tag:ironsmith::tag::TagKey="source_collection:cargo".into();let chosen:ironsmith::tag::TagKey="returned_cargo".into();
 let filter=ObjectFilter::default().in_zone(Zone::Exile).match_tagged(tag.clone(),ironsmith::filter::TaggedOpbjectRelation::SameObjectId);
 let choose=ironsmith::effects::ChooseObjectsEffect::new(filter,ironsmith::ChoiceCount::exactly(1),PlayerFilter::You,chosen.clone());
 ironsmith::effects::ScheduleDelayedTriggerEffect::new(ironsmith::triggers::Trigger::this_deals_combat_damage_to_player(PlayerFilter::Any),vec![Effect::new(choose),Effect::move_to_zone(ChooseSpec::Tagged(chosen),Zone::Hand,false)],false,vec![],PlayerFilter::You).watch_ability_source().with_source_collection_scope(Some(ironsmith_core::effect::SourceCollectionScope{name:"cargo".into(),tag,zone:Zone::Exile,capture_from:ironsmith::effect::EffectId(101),capture_action:ironsmith_core::PriorEffectAction::Exiled}))
}
fn store(game:&mut GameState,source:ObjectId,card:ObjectId){
 let mut ctx=EffectContext::new_default(source,A);
 execute_effect(game,&Effect::with_id(101,Effect::new(ironsmith::effects::ExileUntilEffect::source_leaves(ChooseSpec::SpecificObject(card)))),&mut ctx).unwrap();
 execute_effect(game,&Effect::new(schedule()),&mut ctx).unwrap();
}
fn hit(game:&mut GameState,source:ObjectId,victim:PlayerId){
 let controller=game.controller_of_id(source).unwrap();execute_effect(game,&Effect::new(ironsmith::effects::DealDamageEffect::new(1,ChooseSpec::SpecificPlayer(victim)).with_combat(true)),&mut EffectContext::new_default(source,controller)).unwrap();
}
#[test]
fn collection_pools_registrations_excludes_other_exiles_and_returns_only_one_card_per_hit(){
 let mut game=GameState::new(vec!["A".into(),"B".into(),"C".into()],20);let source=game.create_object_from_definition(&body(),A,Zone::Battlefield);
 let first=game.create_object_from_definition(&body(),A,Zone::Graveyard);let second=game.create_object_from_definition(&body(),A,Zone::Graveyard);let other=game.create_object_from_definition(&body(),A,Zone::Graveyard);let other_stable=game.object(other).unwrap().stable_id;
 store(&mut game,source,first);store(&mut game,source,second);
 execute_effect(&mut game,&Effect::new(ironsmith::effects::ExileUntilEffect::source_leaves(ChooseSpec::SpecificObject(other))),&mut EffectContext::new_default(source,A)).unwrap();
 assert_eq!(game.effect_store.delayed_triggers.len(),1);assert_eq!(game.source_collection_objects(source,"cargo").len(),2);
 hit(&mut game,source,B);let mut queue=ironsmith::triggers::TriggerQueue::new();ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();assert_eq!(queue.entries.len(),1);ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();
 assert_eq!(game.player(A).unwrap().hand.len(),1);assert_eq!(game.source_collection_objects(source,"cargo").len(),1);assert_eq!(game.object(game.find_object_by_stable_id(other_stable).unwrap()).unwrap().zone,Zone::Exile);
}
#[test]
fn collection_uses_current_controller_and_does_not_revive_a_card_after_it_leaves_and_returns(){
 let mut game=GameState::new(vec!["A".into(),"B".into(),"C".into()],20);let source=game.create_object_from_definition(&body(),A,Zone::Battlefield);let first=game.create_object_from_definition(&body(),A,Zone::Graveyard);let second=game.create_object_from_definition(&body(),A,Zone::Graveyard);
 store(&mut game,source,first);store(&mut game,source,second);let departed=game.source_collection_objects(source,"cargo")[0];let hand=game.move_object(departed,Zone::Hand,ironsmith::events::EventCause::effect()).unwrap();game.move_object(hand,Zone::Exile,ironsmith::events::EventCause::effect()).unwrap();assert_eq!(game.source_collection_objects(source,"cargo").len(),1);
 game.set_current_controller(source,B).unwrap();hit(&mut game,source,C);let mut queue=ironsmith::triggers::TriggerQueue::new();ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();assert_eq!(queue.entries.len(),1);assert_eq!(queue.entries[0].controller,B);ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();assert_eq!(game.player(A).unwrap().hand.len(),1);assert_eq!(game.player(B).unwrap().hand.len(),0);
}
#[test]
fn collection_duration_returns_to_original_graveyard_and_blink_does_not_inherit_the_old_relation(){
 let mut game=GameState::new(vec!["A".into(),"B".into(),"C".into()],20);let source=game.create_object_from_definition(&body(),A,Zone::Battlefield);let card=game.create_object_from_definition(&body(),A,Zone::Graveyard);let stable=game.object(card).unwrap().stable_id;store(&mut game,source,card);
 let hand=game.move_object(source,Zone::Hand,ironsmith::events::EventCause::effect()).unwrap();let returned=game.move_object(hand,Zone::Battlefield,ironsmith::events::EventCause::effect()).unwrap();let mut duration_queue=ironsmith::triggers::TriggerQueue::new();ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game,&mut duration_queue,&mut SelectFirstDecisionMaker).unwrap();
 assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone,Zone::Graveyard);assert!(game.source_collection_objects(source,"cargo").is_empty());assert!(game.source_collection_objects(returned,"cargo").is_empty());
 hit(&mut game,returned,B);let mut queue=ironsmith::triggers::TriggerQueue::new();ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();assert!(queue.entries.is_empty());
}

#[test]
fn authored_collection_registration_and_membership_compile_together(){
 let text="Type: Creature\nPower/Toughness: 3/3\n{1}: Exile target creature card from your graveyard until this creature leaves the battlefield. Maintain the cargo collection with cards exiled this way: Whenever this creature deals combat damage to a player, choose a card in exile from the cargo collection, then put it into its owner's hand.";
 let definition=ironsmith_compiler_runtime::compile_to_runtime_definition("Carrier",text,false).unwrap();
 let debug=format!("{definition:?}");
 assert!(debug.contains("SourceCollectionScope"),"{debug}");
 assert!(debug.contains("source_collection:cargo"),"{debug}");
}

#[test]
fn source_owned_collection_programs_preserve_count_random_and_quality_operands(){
 let mut failures=Vec::new();let mut passed=0;
 for name in ["Carrier","Collection producer"] {for selector in ["an artifact or creature card","a creature card with mana value 3 or less","up to one instant card and up to one sorcery card","up to eight noncreature, nonland cards","two cards at random","a card that was put into your graveyard this turn"] {
  let text=format!("Type: Creature\nPower/Toughness: 3/3\n{{1}}: Exile {selector} from your graveyard until this creature leaves the battlefield. Maintain the cargo collection with cards exiled this way: Whenever this creature deals combat damage to a player, choose a card in exile from the cargo collection, then put it into its owner's hand.");
  match ironsmith_compiler_runtime::compile_to_runtime_definition(name,&text,false){
   Ok(definition)=>{assert!(format!("{definition:?}").contains("SourceCollectionScope"));passed+=1;},
   Err(error)=>failures.push(format!("{name} / {selector}: {error:?}")),
  }
 }}
 assert!(failures.is_empty(),"{} authored source-collection failures ({passed}/12 passed):\n{}",failures.len(),failures.join("\n"));assert_eq!(passed,12);
}

#[test]
fn tagged_watched_recipient_owns_collection_independently_of_producing_ability_source(){
 let mut game=GameState::new(vec!["A".into(),"B".into(),"C".into()],20);
 let producer=game.create_object_from_definition(&body(),A,Zone::Battlefield);
 let host=game.create_object_from_definition(&body(),B,Zone::Battlefield);
 let card=game.create_object_from_definition(&body(),A,Zone::Graveyard);
 let card_stable=game.object(card).unwrap().stable_id;
 let mut ctx=EffectContext::new_default(producer,A);
 ctx.tagged_objects.insert("host".into(),vec![ironsmith::snapshot::ObjectSnapshot::from_object(game.object(host).unwrap(),&game)]);
 let exile=ironsmith::effects::ExileUntilEffect::source_leaves(ChooseSpec::SpecificObject(card)).with_leave_watcher(ChooseSpec::Tagged("host".into()));
 execute_effect(&mut game,&Effect::with_id(101,Effect::new(exile)),&mut ctx).unwrap();
 let mut watcher=schedule();watcher.target_tag=Some("host".into());
 execute_effect(&mut game,&Effect::new(watcher),&mut ctx).unwrap();
 assert_eq!(game.source_collection_objects(producer,"cargo"),vec![]);
 assert_eq!(game.source_collection_objects(host,"cargo").len(),1);
 hit(&mut game,producer,C);let mut queue=ironsmith::triggers::TriggerQueue::new();
 ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();assert!(queue.entries.is_empty());
 hit(&mut game,host,C);
 ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();assert_eq!(queue.entries.len(),1);assert_eq!(queue.entries[0].controller,B);
 ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();
 let card=game.find_object_by_stable_id(card_stable).unwrap();assert_eq!(game.object(card).unwrap().zone,Zone::Hand);assert_eq!(game.object(card).unwrap().owner,A);
}

#[test]
fn authored_recipient_departure_watcher_and_collection_share_the_same_chosen_creature(){
 let text="Type: Creature\nPower/Toughness: 3/3\n{1}: Choose target artifact creature you control. Exile a card from your graveyard until that creature leaves the battlefield. Maintain the cargo collection with cards exiled this way: Whenever that creature deals combat damage to a player, choose a card in exile from the cargo collection, then put it into its owner's hand.";
 let definition=ironsmith_compiler_runtime::compile_to_runtime_definition("Carrier",text,false).unwrap();
 let debug=format!("{definition:?}");assert!(debug.contains("SourceCollectionScope"),"{debug}");
 assert!(debug.contains("leave_watcher: Some"),"{debug}");assert!(debug.contains("target_tag: Some"),"{debug}");
}

#[test]
fn pending_collection_trigger_reads_live_pool_including_a_later_actual_exile(){
 use ironsmith::decision::DecisionMaker;
 struct Last;
 impl DecisionMaker for Last{
  fn decide_objects(&mut self,_:&GameState,c:&ironsmith::decisions::context::SelectObjectsContext)->Vec<ObjectId>{c.candidates.iter().filter(|candidate|candidate.legal).last().map(|candidate|vec![candidate.id]).unwrap_or_default()}
 }
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=game.create_object_from_definition(&body(),A,Zone::Battlefield);
 let first=game.create_object_from_definition(&body(),A,Zone::Graveyard);store(&mut game,source,first);
 hit(&mut game,source,B);let mut queue=ironsmith::triggers::TriggerQueue::new();ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();assert_eq!(queue.entries.len(),1);
 let later=game.create_object_from_definition(&body(),A,Zone::Graveyard);let stable=game.object(later).unwrap().stable_id;store(&mut game,source,later);
 ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut Last).unwrap();ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut Last).unwrap();
 assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone,Zone::Hand);assert_eq!(game.source_collection_objects(source,"cargo").len(),1);
}

#[test]
fn actual_exile_until_receipt_excludes_an_unrelated_replacement_added_exile(){
 use ironsmith::replacement::{ReplacementAction,ReplacementEffect};
 use ironsmith::events::zones::matchers::WouldBeExiledMatcher;
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=game.create_object_from_definition(&body(),A,Zone::Battlefield);
 let original=game.create_object_from_definition(&body(),A,Zone::Graveyard);let original_stable=game.object(original).unwrap().stable_id;
 let addition=game.create_object_from_definition(&body(),A,Zone::Graveyard);let addition_stable=game.object(addition).unwrap().stable_id;
 let mut filter=ObjectFilter::default();filter.specific=Some(original);
 game.effect_store.replacement_effects.add_resolution_effect(ReplacementEffect::with_matcher(source,A,WouldBeExiledMatcher::new(filter),ReplacementAction::Additionally(vec![Effect::new(ironsmith::effects::ExileEffect::with_spec(ChooseSpec::SpecificObject(addition)))])));
 store(&mut game,source,original);
 let first=game.find_object_by_stable_id(original_stable).unwrap();let extra=game.find_object_by_stable_id(addition_stable).unwrap();assert_eq!(game.object(first).unwrap().zone,Zone::Exile);assert_eq!(game.object(extra).unwrap().zone,Zone::Exile);
 assert_eq!(game.source_collection_objects(source,"cargo"),vec![first],"only the original exact arrival enters the declared collection");
}
#[test]
fn a_replacement_added_reexile_cannot_substitute_a_later_incarnation_for_the_original_arrival(){
 use ironsmith::replacement::{ReplacementAction,ReplacementEffect};
 use ironsmith::events::zones::matchers::WouldBeExiledMatcher;
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=game.create_object_from_definition(&body(),A,Zone::Battlefield);let original=game.create_object_from_definition(&body(),A,Zone::Graveyard);let stable=game.object(original).unwrap().stable_id;
 let mut filter=ObjectFilter::default();filter.specific=Some(original);
 game.effect_store.replacement_effects.add_resolution_effect(ReplacementEffect::with_matcher(source,A,WouldBeExiledMatcher::new(filter),ReplacementAction::Additionally(vec![Effect::new(ironsmith::effects::ExileEffect::with_spec(ChooseSpec::All(ObjectFilter::default().in_zone(Zone::Exile))))])));
 store(&mut game,source,original);assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone,Zone::Exile);
 assert!(game.source_collection_objects(source,"cargo").is_empty());assert!(game.effect_store.delayed_triggers.is_empty());
}
#[test]
fn collection_trigger_controller_is_sampled_at_combat_damage_before_queued_events_drain(){
 let mut game=GameState::new(vec!["A".into(),"B".into(),"C".into()],20);let source=game.create_object_from_definition(&body(),A,Zone::Battlefield);let card=game.create_object_from_definition(&body(),A,Zone::Graveyard);store(&mut game,source,card);
 hit(&mut game,source,C);game.set_current_controller(source,B).unwrap();let mut queue=ironsmith::triggers::TriggerQueue::new();
 ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();assert_eq!(queue.entries.len(),1);assert_eq!(queue.entries[0].controller,A,"event-time controller owns the already-triggered ability");
}
#[test]
fn repeated_scope_capture_grows_only_its_declared_slot_and_casts_each_original_arrival(){
 use ironsmith::resolution::ResolutionProgram;
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=game.create_object_from_definition(&body(),A,Zone::Battlefield);
 let first_definition=ironsmith_compiler_runtime::compile_to_runtime_definition("First cargo spell","Type: Sorcery\nYou gain 1 life.",false).unwrap();let second_definition=ironsmith_compiler_runtime::compile_to_runtime_definition("Second cargo spell","Type: Sorcery\nYou gain 2 life.",false).unwrap();
 let first=game.create_object_from_definition(&first_definition,A,Zone::Graveyard);let second=game.create_object_from_definition(&second_definition,A,Zone::Graveyard);let anchor_a=game.create_object_from_definition(&body(),A,Zone::Exile);let anchor_b=game.create_object_from_definition(&body(),A,Zone::Exile);
 let tag:ironsmith::tag::TagKey="source_collection:cargo".into();let anchor:ironsmith::tag::TagKey="fixed_parent".into();
 for (card,fixed) in [(first,anchor_a),(second,anchor_b)]{
  let mut ctx=EffectContext::new_default(source,A);ctx.set_tagged_objects(anchor.as_str(),vec![ironsmith::snapshot::ObjectSnapshot::from_object(game.object(fixed).unwrap(),&game)]);
  execute_effect(&mut game,&Effect::with_id(101,Effect::new(ironsmith::effects::ExileUntilEffect::source_leaves(ChooseSpec::SpecificObject(card)))),&mut ctx).unwrap();
  let mut watcher=schedule();watcher.effects=ResolutionProgram::from_effects(vec![Effect::new(ironsmith::effects::ForEachTaggedEffect::new(tag.clone(),vec![Effect::new(ironsmith::effects::CastTaggedEffect::new(ironsmith_core::tag::CURRENT_ITERATED_OBJECT_TAG,PlayerFilter::You).as_copy().without_paying_mana_cost())]))]);watcher.effects.object_reference_imports=vec![tag.clone(),anchor.clone()];
  execute_effect(&mut game,&Effect::new(watcher),&mut ctx).unwrap();
 }
 let watcher=&game.effect_store.delayed_triggers[0];assert_eq!(game.effect_store.delayed_triggers.len(),1);assert_eq!(watcher.effects.captured_object_references.iter().find(|(key,_)|key==&tag).unwrap().1.len(),2);assert_eq!(watcher.effects.captured_object_references.iter().find(|(key,_)|key==&anchor).unwrap().1,vec![anchor_a]);
 hit(&mut game,source,B);let mut queue=ironsmith::triggers::TriggerQueue::new();ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();assert_eq!(game.stack.len(),2);
 for _ in 0..2{ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();}assert_eq!(game.player(A).unwrap().life,23);
}
#[test]
fn authored_salvage_return_watcher_keeps_its_live_collection_out_of_cast_imports(){
 fn inspect(effect:&Effect,count:&mut usize){
  if let Some(schedule)=effect.downcast_ref::<ironsmith::effects::ScheduleDelayedTriggerEffect>(){
   if let Some(scope)=&schedule.source_collection_scope{*count+=1;assert!(!schedule.effects.object_reference_imports.contains(&scope.tag));}
  }
  effect.visit_child_effects(&mut|child|inspect(child,count));
 }
 let definition=ironsmith_compiler_runtime::compile_to_runtime_definition("Collection producer","Type: Creature\nPower/Toughness: 3/3\n{1}: Exile a card from your graveyard until this creature leaves the battlefield. Maintain the cargo collection with cards exiled this way: Whenever this creature deals combat damage to a player, choose a card in exile from the cargo collection, then put it into its owner's hand.",false).unwrap();
 let mut count=0;for ability in &definition.abilities{if let ironsmith::ability::AbilityKind::Activated(activated)=&ability.kind{for effect in activated.effects.all_effects(){inspect(effect,&mut count);}}}assert_eq!(count,1);
}

#[test]
fn authored_source_collection_exact_exile_receipt_survives_artifact_reload(){
 let text="Type: Creature\nPower/Toughness: 3/3\n{1}: Exile target creature card from your graveyard until this creature leaves the battlefield. Maintain the cargo collection with cards exiled this way: Whenever this creature deals combat damage to a player, choose a card in exile from the cargo collection, then put it into its owner's hand.";
 let (artifact,direct)=ironsmith_compiler_runtime::compile_to_artifact("Collection producer",text,false).unwrap();
 let restored=ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
 for definition in [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]{
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
  let material=game.create_object_from_definition(&body(),A,Zone::Graveyard);let stable=game.object(material).unwrap().stable_id;
  let ability=definition.abilities.iter().find_map(|ability|match &ability.kind{ironsmith::ability::AbilityKind::Activated(activated)=>Some(activated),_=>None}).unwrap();
  let mut ctx=EffectContext::new_default(source,A).with_targets(vec![ironsmith::effects::ResolvedTarget::Object(material)]);
  for effect in ability.effects.all_effects(){execute_effect(&mut game,effect,&mut ctx).unwrap();}
  drop(ctx);let arrival=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.source_collection_objects(source,"cargo"),vec![arrival]);assert_eq!(game.object(arrival).unwrap().zone,Zone::Exile);assert_eq!(game.effect_store.delayed_triggers.len(),1);
 }
}
#[test]
fn source_collection_registration_without_a_compatible_exile_producer_is_rejected(){
 for prefix in ["","You gain 1 life. "]{
  let text=format!("Type: Creature\nPower/Toughness: 3/3\n{{1}}: {prefix}Maintain the cargo collection with cards exiled this way: Whenever this creature deals combat damage to a player, choose a card in exile from the cargo collection, then put it into its owner's hand.");
  assert!(ironsmith_compiler_runtime::compile_to_runtime_definition("Unbound collection",&text,false).is_err(),"no unrelated result may stand in for the authored exile receipt");
 }
}
