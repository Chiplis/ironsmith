//! Source-scoped Accomplice: only the resolving attack ability creates its relation.
#[path="custom_final_next_wave_support.rs"] mod source_support;
use ironsmith::{GameState, PlayerId, Zone};
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::decision::DecisionMaker;
use ironsmith::decisions::context::BooleanContext;
fn fixture()->serde_json::Value{serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/revolution-accomplice-source-complete.json")).unwrap()}
fn card(name:&str)->serde_json::Value{fixture()["mechanics"].as_array().unwrap().iter().flat_map(|row|row["representative_cards"].as_array().unwrap()).find(|card|card["name"]==name).unwrap().clone()}
#[test]
fn every_exact_accomplice_source_compiles_through_independent_native_and_artifact_routes(){
 let f=fixture();let mut failures=vec![];let mut count=0;
 for row in f["mechanics"].as_array().unwrap(){for input in row["representative_cards"].as_array().unwrap(){
  count+=1;if let Err(error)=source_support::definitions(input){failures.push(format!("{}: {error}",input["name"]));}
 }}assert_eq!(count,22);assert!(failures.is_empty(),"{}",failures.join("\n"));
}
struct Answer(bool);
impl DecisionMaker for Answer{fn decide_boolean(&mut self,_:&GameState,_:&BooleanContext)->bool{self.0}}
fn drain(game:&mut GameState)->ironsmith::triggers::TriggerQueue{
 let mut queue=ironsmith::triggers::TriggerQueue::new();
 ironsmith::game_loop::drain_pending_trigger_events_with_dm(game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();queue
}
#[test]
fn actual_source_optional_tap_creates_one_relation_only_when_it_changes_the_creature(){
 let a=PlayerId(0);let b=PlayerId(1);
 let actor=source_support::definitions(&card("Dust Smugglers")).unwrap();
 let observer=source_support::definitions(&card("Angelic Embezzler")).unwrap();
 for actor in &actor{for observer in &observer{for accept in [false,true]{for already_tapped in [false,true]{
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  let source=game.create_object_from_definition(actor,a,Zone::Battlefield);
  game.create_object_from_definition(observer,a,Zone::Battlefield);
  let attacker=game.create_object_from_definition(&ironsmith_compiler_runtime::compile_to_runtime_definition("Attacker","Type: Creature\nPower/Toughness: 2/2",false).unwrap(),a,Zone::Battlefield);
  if already_tapped{game.tap(source);}
  let event=ironsmith::triggers::TriggerEvent::new_with_provenance(ironsmith::events::CreatureAttackedEvent::new(attacker,ironsmith::events::AttackEventTarget::Player(b)),Default::default());
  game.queue_trigger_event(Default::default(),event);
  let mut queue=drain(&mut game);assert_eq!(queue.entries.len(),1);
  ironsmith::game_loop::put_triggers_on_stack(&mut game,&mut queue).unwrap();
  ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut Answer(accept)).unwrap();
  let mut queue=drain(&mut game);let associated=accept&&!already_tapped;
  assert_eq!(queue.entries.len(),if associated{2}else{0},"declining or failing to tap cannot announce an association");
  ironsmith::game_loop::put_triggers_on_stack(&mut game,&mut queue).unwrap();
  while !game.stack.is_empty(){ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();}
  assert_eq!(game.player(a).unwrap().life,20+if associated{2}else{0});
  assert_eq!(game.is_tapped(source),already_tapped||accept);
 }}}}
}
#[test]
fn unrelated_effect_tap_does_not_create_the_named_relation(){
 let a=PlayerId(0);
 for observer in source_support::definitions(&card("Angelic Embezzler")).unwrap(){
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  let observer=game.create_object_from_definition(&observer,a,Zone::Battlefield);
  let source=game.create_object_from_definition(&source_support::definitions(&card("Dust Smugglers")).unwrap()[0],a,Zone::Battlefield);
  let spell=ironsmith_compiler_runtime::compile_to_runtime_definition("Tapping witness","Type: Instant\nTap target creature.",false).unwrap();
  let mut ctx=ironsmith::effects::EffectContext::new_default(observer,a).with_targets(vec![ironsmith::effects::ResolvedTarget::Object(source)]);
  let program=ironsmith::Effect::new(ironsmith::effects::SequenceEffect::new(spell.spell_effect.as_ref().unwrap().all_effects_owned()));
  let outcome=ironsmith::effects::execute_effect(&mut game,&program,&mut ctx).unwrap();
  for event in outcome.events{game.queue_trigger_event(Default::default(),event);}
  assert!(game.is_tapped(source));assert!(drain(&mut game).entries.is_empty());assert_eq!(game.player(a).unwrap().life,20);
 }
}

#[test]
fn prior_tap_relation_retains_exact_attacker_and_original_observers_after_additional_movement(){
 use ironsmith::effect::{EffectId,ObjectSnapshot};
 use ironsmith::effects::{EffectContext,execute_effect,PerformNamedActionEffect,TapEffect,MoveToZoneEffect};
 use ironsmith::target::ChooseSpec;
 let a=PlayerId(0);
 for observer_definition in source_support::definitions(&card("Angelic Embezzler")).unwrap(){
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  let observer=game.create_object_from_definition(&observer_definition,a,Zone::Battlefield);
  let body=ironsmith_compiler_runtime::compile_to_runtime_definition("Creature","Type: Creature\nPower/Toughness: 2/2",false).unwrap();
  let attacker=game.create_object_from_definition(&body,a,Zone::Battlefield);
  let participant=game.create_object_from_definition(&body,a,Zone::Battlefield);
  let stable=game.object(attacker).unwrap().stable_id;
  let mut ctx=EffectContext::new_default(participant,a);
  ctx.tag_object("retained performer",ObjectSnapshot::from_object_id(&game,attacker).unwrap());
  let tapped=execute_effect(&mut game,&ironsmith::Effect::new(TapEffect::with_spec(ChooseSpec::SpecificObject(participant))),&mut ctx).unwrap();
  ctx.store_outcome(EffectId(917),tapped);
  for object in [attacker,observer]{execute_effect(&mut game,&ironsmith::Effect::new(MoveToZoneEffect::new(ChooseSpec::SpecificObject(object),Zone::Exile,false)),&mut ctx).unwrap();}
  let exiled=game.find_object_by_stable_id(stable).unwrap();
  execute_effect(&mut game,&ironsmith::Effect::new(MoveToZoneEffect::new(ChooseSpec::SpecificObject(exiled),Zone::Battlefield,false)),&mut ctx).unwrap();
  let reentered=game.find_object_by_stable_id(stable).unwrap();assert_ne!(attacker,reentered);
  let marker=PerformNamedActionEffect{label:"accomplice association".into(),performer:Some(ChooseSpec::Tagged("retained performer".into())),objects:None,prior_objects:Some((EffectId(917),ironsmith_core::PriorEffectAction::Tapped))};
  let out=execute_effect(&mut game,&ironsmith::Effect::new(marker),&mut ctx).unwrap();
  let relation=out.events.iter().find_map(|event|event.downcast::<ironsmith::events::NamedActionEvent>()).unwrap();
  assert_eq!(relation.source,attacker,"the new incarnation is not the attacker's retained role");
  assert_eq!(relation.objects.len(),1);assert_eq!(relation.objects[0].object_id,participant);
  assert!(relation.participant_source_lookback.as_ref().unwrap().iter().any(|snapshot|snapshot.object_id==observer),"a listener present at completed tap survives additional movement");
  for event in out.events{game.queue_trigger_event(Default::default(),event);}
  let mut queue=drain(&mut game);assert_eq!(queue.entries.len(),1);
  ironsmith::game_loop::put_triggers_on_stack(&mut game,&mut queue).unwrap();
  ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();
  assert_eq!(game.player(a).unwrap().life,22);
 }
}

#[test]
fn actual_torch_singer_resolving_body_changes_control_then_taps_and_adds_the_same_permanent_to_combat() {
    use ironsmith::effects::{EffectContext,execute_effect};
    use ironsmith::effects::ResolvedTarget;
    let a=PlayerId(0);let b=PlayerId(1);
    for definition in source_support::definitions(&card("Torch Singer")).unwrap(){for in_combat in [false,true]{
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        game.turn.active_player=a;game.turn.phase=if in_combat{ironsmith::Phase::Combat}else{ironsmith::Phase::FirstMain};
        let source=game.create_object_from_definition(&definition,a,Zone::Battlefield);
        // Invoke the source's complete consumer body after its association;
        // the shared actual producer scenarios above own optional-tap matching.
        game.tap(source);
        let body=ironsmith_compiler_runtime::compile_to_runtime_definition("Controlled creature","Type: Creature\nPower/Toughness: 2/5",false).unwrap();
        let host=game.create_object_from_definition(&body,b,Zone::Battlefield);
        let stable=game.object(host).unwrap().stable_id;
        let effects=definition.abilities.iter().filter_map(|ability|match &ability.kind{ironsmith::ability::AbilityKind::Triggered(trigger)=>Some(trigger.effects.all_effects_owned()),_=>None}).nth(1).unwrap();
        let mut ctx=EffectContext::new_default(source,a).with_targets(vec![ResolvedTarget::Object(host)]);
        let outcome=execute_effect(&mut game,&ironsmith::Effect::new(ironsmith::effects::SequenceEffect::new(effects)),&mut ctx).unwrap();
        assert_eq!(game.find_object_by_stable_id(stable),Some(host),"joining combat is not a zone change");
        assert_eq!(game.controller_of(game.object(host).unwrap()),a);assert!(game.is_tapped(host));
        let attacked=game.combat.as_ref().is_some_and(|combat|combat.attackers.iter().any(|info|info.creature==host));
        assert_eq!(attacked,in_combat);
        assert!(!outcome.events.iter().any(|event|event.kind()==ironsmith::events::EventKind::CreatureAttacked),"it was never declared as an attacker");
    }}
}

#[derive(Default)]
struct WatchRideScry { menus:usize }
impl DecisionMaker for WatchRideScry {
    fn decide_partition(&mut self,_:&GameState,ctx:&ironsmith::decisions::context::PartitionContext)->Vec<ironsmith::ObjectId>{
        assert!(ctx.description.contains("Scry 2"));assert_eq!(ctx.cards.len(),2);self.menus+=1;vec![]
    }
}
#[test]
fn actual_ride_to_nowhere_checks_tapped_or_keyword_before_the_return_not_current_hand_state() {
    use ironsmith::effects::{EffectContext,execute_effect};use ironsmith::effects::ResolvedTarget;
    let a=PlayerId(0);let b=PlayerId(1);
    let rides=source_support::definitions(&card("Ride to Nowhere")).unwrap();
    let donors=source_support::definitions(&card("Dust Smugglers")).unwrap();
    let plain=ironsmith_compiler_runtime::compile_to_runtime_definition("Ordinary creature","Type: Creature\nPower/Toughness: 2/2",false).unwrap();
    for route in 0..2 {for has_keyword in [false,true] {for tapped in [false,true] {
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        let source=game.create_object_from_definition(&rides[route],a,Zone::Stack);
        let target=game.create_object_from_definition(if has_keyword{&donors[route]}else{&plain},b,Zone::Battlefield);
        if tapped{game.tap(target);}let physical=game.object(target).unwrap().stable_id;
        for _ in 0..2 {game.create_object_from_definition(&plain,a,Zone::Library);}
        let mut dm=WatchRideScry::default();
        let mut ctx=EffectContext::new_default(source,a).with_targets(vec![ResolvedTarget::Object(target)]).with_decision_maker(&mut dm);
        execute_effect(&mut game,&ironsmith::Effect::new(ironsmith::effects::SequenceEffect::new(rides[route].spell_effect.as_ref().unwrap().all_effects_owned())),&mut ctx).unwrap();
        let arrival=game.find_object_by_stable_id(physical).unwrap();assert_ne!(arrival,target);assert_eq!(game.object(arrival).unwrap().zone,Zone::Hand);
        drop(ctx);assert_eq!(dm.menus,usize::from(has_keyword||tapped));
    }}}
}
