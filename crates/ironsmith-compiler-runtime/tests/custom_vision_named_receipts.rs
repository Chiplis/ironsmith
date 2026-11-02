//! Generic named-action receipts preserve Vision's revealed-card attribution.
//! Parent registers and runs this target after the paired scratch patches.
use ironsmith::{CardDefinition, Effect, GameState, ObjectId, PlayerId, Zone};
use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A: PlayerId = PlayerId(0);
fn definitions(id: &str) -> [CardDefinition; 2] {
    let fixture: serde_json::Value = serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-casting-middle.json")).unwrap();
    let row = fixture["mechanics"].as_array().unwrap().iter().find(|row|row["id"]==id).unwrap();
    let card = &row["representative_cards"][0];
    let text = card["executable_text"].as_str().unwrap();
    let name = card["name"].as_str().unwrap();
    let builder = || ironsmith_compiler::CardDefinitionBuilder::new(ironsmith::CardId::new(), name).with_custom_keyword(if id=="msem-576" {"Artifact vision"} else {"Vision"});
    let (compiled, direct_loss) = ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler::CompilerFacade::new().compile_definition(builder(),text.to_string(),ironsmith_compiler::CompilePolicy{allow_unsupported:false})));
    assert!(!direct_loss.is_lossy(),"direct: {}",direct_loss.reasons_text());
    let direct=ironsmith_compiler_runtime::into_runtime_definition(compiled.unwrap().definition).unwrap();
    let (result, loss) = ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_builder_to_artifact(builder(),text,false)));
    assert!(!loss.is_lossy(),"artifact: {}",loss.reasons_text());
    let (artifact,_artifact_materialized)=result.unwrap();
    let restored=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(restored,artifact);
    [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
fn card(game:&mut GameState, zone:Zone, text:&str)->ObjectId {
    let definition=ironsmith_compiler_runtime::compile_to_runtime_definition("Independent card",text,false).unwrap();
    game.create_object_from_definition(&definition,A,zone)
}
struct Choices(bool);
impl DecisionMaker for Choices {
    fn decide_boolean(&mut self,_:&GameState,_:&ironsmith::decisions::context::BooleanContext)->bool {self.0}
    fn decide_targets(&mut self,game:&GameState,ctx:&ironsmith::decisions::context::TargetsContext)->Vec<ironsmith::Target> {
        SelectFirstDecisionMaker.decide_targets(game,ctx)
    }
}
fn settle(game:&mut GameState,dm:&mut Choices)->usize {
    let mut queue=ironsmith::triggers::TriggerQueue::new();
    ironsmith::game_loop::drain_pending_trigger_events_with_dm(game,&mut queue,dm).unwrap();
    let matched=queue.entries.len();
    ironsmith::game_loop::put_triggers_on_stack_with_dm(game,&mut queue,dm).unwrap();
    while !game.stack.is_empty() {ironsmith::game_loop::resolve_stack_entry_with(game,dm).unwrap();}
    matched
}
fn publish_reported_events(game:&mut GameState,outcome:ironsmith::effect::EffectOutcome) {
    // Raw execution returns its completion observations. The ordinary stack
    // owner publishes those; this direct-executor scenario must do the same.
    for event in outcome.events {
        // Physical moves already have a native pending-event owner. Publish
        // only the completion receipt missing from this direct-executor path.
        if event.downcast::<ironsmith::events::NamedActionEvent>().is_some() {
            game.queue_trigger_event(event.provenance(),event);
        }
    }
}
fn vision(game:&mut GameState,definition:&CardDefinition,source:ObjectId,dm:&mut Choices,expected_match:bool) {
    let top=game.player(A).unwrap().library.last().copied();
    let program=definition.abilities.iter().find_map(|ability|match &ability.kind {
        ironsmith::ability::AbilityKind::Triggered(triggered) if !matches!(triggered.trigger.compiled_model().map(|model|&model.kind),Some(ironsmith_core::trigger_model::TriggerKind::NamedAction {..}))=>Some(triggered.effects.all_effects_owned()),_=>None,
    }).unwrap();
    let outcome=execute_effect(game,&Effect::new(ironsmith::effects::SequenceEffect::new(program)),&mut EffectContext::new_default(source,A).with_decision_maker(dm)).unwrap();
    let events=outcome.events.iter().filter(|event|event.downcast::<ironsmith::events::NamedActionEvent>().is_some()).collect::<Vec<_>>();
    assert_eq!(events.len(),1,"the producer must publish exactly one named operation receipt: {outcome:?}");
    let event=events[0];let action=event.downcast::<ironsmith::events::NamedActionEvent>().unwrap();
    assert_eq!(action.label,"envision");assert_eq!(action.source,source);assert_eq!(action.amount,usize::from(top.is_some()) as u32);
    assert_eq!(action.objects.iter().map(|object|object.object_id).collect::<Vec<_>>(),top.into_iter().collect::<Vec<_>>(),"receipt must retain the revealed incarnation even if later moved");
    let watcher=definition.abilities.iter().find_map(|ability|match &ability.kind{ironsmith::ability::AbilityKind::Triggered(triggered) if matches!(triggered.trigger.compiled_model().map(|model|&model.kind),Some(ironsmith_core::trigger_model::TriggerKind::NamedAction {..}))=>Some(triggered),_=>None}).unwrap();
    let ctx=ironsmith::triggers::TriggerContext::for_source(source,A,game);
    assert_eq!(watcher.trigger.matches(event,&ctx),expected_match,"receipt/model stage: watcher={:?}, action={action:?}",watcher.trigger.compiled_model());
    publish_reported_events(game,outcome);
}
#[test]
fn envisioning_a_creature_is_independent_of_optional_graveyard_move_and_unrelated_reveals() {
    for definition in definitions("msem-575") {for discard in [false,true] {
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
        let top=card(&mut game,Zone::Library,"Type: Creature\nPower/Toughness: 1/1");
        let top_stable=game.object(top).unwrap().stable_id;
        let returned=card(&mut game,Zone::Graveyard,"Type: Instant\nYou gain 1 life.");
        let returned_stable=game.object(returned).unwrap().stable_id;
        let mut dm=Choices(discard);
        // An ordinary reveal by the same permanent is not an envision action.
        let ordinary=execute_effect(&mut game,&Effect::new(ironsmith::effects::RevealTopEffect::new(ironsmith::target::PlayerFilter::You,None)),&mut EffectContext::new_default(source,A).with_decision_maker(&mut dm)).unwrap();
        for event in ordinary.events {
            if event.downcast::<ironsmith::events::CardRevealedEvent>().is_some(){game.queue_trigger_event(event.provenance(),event);}
        }
        settle(&mut game,&mut dm);
        assert_eq!(game.object(game.find_object_by_stable_id(returned_stable).unwrap()).unwrap().zone,Zone::Graveyard);
        vision(&mut game,&definition,source,&mut dm,true);
        assert_eq!(game.object(game.find_object_by_stable_id(top_stable).unwrap()).unwrap().zone,if discard {Zone::Graveyard}else {Zone::Library});
        assert_eq!(settle(&mut game,&mut dm),1,"the creature receipt must reach its exact source watcher");
        assert_eq!(game.object(game.find_object_by_stable_id(returned_stable).unwrap()).unwrap().zone,Zone::Hand,"discard={discard}");
    }}
}
#[test]
fn artifact_vision_filters_the_exact_revealed_card_and_does_not_trigger_on_empty_library() {
    for definition in definitions("msem-576") {for matching in [false,true] {
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
        let mut dm=Choices(true);
        vision(&mut game,&definition,source,&mut dm,false);
        settle(&mut game,&mut dm);
        assert_eq!(game.counter_count(source,ironsmith::CounterType::PlusOnePlusOne),0);
        card(&mut game,Zone::Library,if matching {"Type: Artifact"}else{"Type: Instant\nYou gain 1 life."});
        vision(&mut game,&definition,source,&mut dm,matching);
        assert_eq!(settle(&mut game,&mut dm),usize::from(matching),"the exact artifact snapshot controls the watcher");
        assert_eq!(game.counter_count(source,ironsmith::CounterType::PlusOnePlusOne),u32::from(matching));
    }}
}
