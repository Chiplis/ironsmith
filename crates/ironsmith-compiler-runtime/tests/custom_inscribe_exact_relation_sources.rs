//! Exact-source lifetime witnesses. Parent materializes the overlay after
//! frontend reader integration before registering this candidate target.
use ironsmith::{CardDefinition,GameState,ObjectId,PlayerId,Zone};
use ironsmith::decision::{DecisionMaker,LegalAction,SelectFirstDecisionMaker,compute_legal_actions};
const A:PlayerId=PlayerId(0);
fn definitions(id:&str)->[CardDefinition;2]{
    let fixture=read_source_fixture();
    let row=fixture["mechanics"].as_array().unwrap().iter().find(|row|row["id"]==id).unwrap();let source=&row["representative_cards"][0];
    let text=source["executable_text"].as_str().expect("materialize exact-source overlay after reader integration");
    let (result,loss)=with_vocabulary(||ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_builder_to_artifact(source_builder(source),text,false))));
    assert!(!loss.is_lossy(),"{}",loss.reasons_text());let (artifact,direct)=result.unwrap();let decoded=ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();assert_eq!(artifact,decoded);
    [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}
struct Choices{host:ObjectId}
impl DecisionMaker for Choices{
    fn decide_boolean(&mut self,_:&GameState,_:&ironsmith::decisions::context::BooleanContext)->bool{false}
    fn decide_objects(&mut self,game:&GameState,ctx:&ironsmith::decisions::context::SelectObjectsContext)->Vec<ObjectId>{if ctx.candidates.iter().any(|candidate|candidate.id==self.host&&candidate.legal){vec![self.host]}else{SelectFirstDecisionMaker.decide_objects(game,ctx)}}
}
fn activate(game:&mut GameState,source:ObjectId,dm:&mut Choices){
    use ironsmith::game_loop::{PriorityLoopState,PriorityResponse,apply_priority_response_with_dm,apply_decision_context_with_dm};
    let action=compute_legal_actions(game,A).unwrap().into_iter().find(|action|matches!(action,LegalAction::ActivateAbility{source:id,..}if *id==source)).expect("actual granted hand activation");
    let mut queue=ironsmith::triggers::TriggerQueue::new();let mut state=PriorityLoopState::new(2);let mut progress=apply_priority_response_with_dm(game,&mut queue,&mut state,&PriorityResponse::PriorityAction(action),dm).unwrap();
    for _ in 0..64{if !state.has_pending_action(){break;}let ironsmith::GameProgress::NeedsDecisionCtx(ctx)=progress else{panic!("{progress:?}")};progress=apply_decision_context_with_dm(game,&mut queue,&mut state,&ctx,dm).unwrap();}
    assert!(!state.has_pending_action());ironsmith::game_loop::resolve_stack_entry_with(game,dm).unwrap();
}
fn move_to(game:&mut GameState,object:ObjectId,zone:Zone)->ObjectId{
    let stable=game.object(object).unwrap().stable_id;let effect=ironsmith::Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::target::ChooseSpec::SpecificObject(object),zone,false));ironsmith::effects::execute_effect(game,&effect,&mut ironsmith::effects::EffectContext::new_default(object,A)).unwrap();game.find_object_by_stable_id(stable).unwrap()
}
fn read_source_fixture()->serde_json::Value{
    let path=std::env::var("IRONSMITH_INSCRIBE_SOURCE_FIXTURE").unwrap_or_else(|_|format!("{}/../../fixtures/custom_mechanics/msem-inscribe-all-sources-candidate.json",env!("CARGO_MANIFEST_DIR")));
    serde_json::from_str(&std::fs::read_to_string(path).expect("parent must materialize full-source Inscribe candidate fixture first")).unwrap()
}
fn with_vocabulary<T>(body:impl FnOnce()->T)->T{
    use ironsmith_core::{SubtypeFamily,custom_subtypes::CustomSubtypeDefinition};
    let vocabulary=[CustomSubtypeDefinition{singular:"Inscription".into(),plural:"Inscriptions".into(),family:SubtypeFamily::Spell},CustomSubtypeDefinition{singular:"Inscription".into(),plural:"Inscriptions".into(),family:SubtypeFamily::Artifact}];
    ironsmith_compiler::with_custom_subtypes(&vocabulary,body).unwrap()
}
fn source_builder(source:&serde_json::Value)->ironsmith_compiler::CardDefinitionBuilder{
    let mut builder=ironsmith_compiler::CardDefinitionBuilder::new(ironsmith_compiler::ids::CardId::new(),source["name"].as_str().unwrap());
    if let Some(identities)=source.get("custom_keyword_identities").or_else(||source.get("executable_faces").and_then(|faces|faces.get(0)).and_then(|face|face.get("custom_keyword_identities"))).and_then(|value|value.as_array()){
        for identity in identities{builder=builder.with_custom_keyword(identity.as_str().unwrap());}
    }
    builder
}

fn source_setup(definition:&CardDefinition)->(GameState,ObjectId,ObjectId,Choices){
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.active_player=A;game.turn.priority_player=Some(A);game.turn.phase=ironsmith::Phase::FirstMain;game.turn.step=None;
 let body=ironsmith_compiler_runtime::compile_to_runtime_definition("Source relation host","Type: Creature\nPower/Toughness: 2/2",false).unwrap();let host=game.create_object_from_definition(&body,A,Zone::Battlefield);let source=game.create_object_from_definition(definition,A,Zone::Hand);
 for symbol in [ironsmith::mana::ManaSymbol::White,ironsmith::mana::ManaSymbol::Blue,ironsmith::mana::ManaSymbol::Black,ironsmith::mana::ManaSymbol::Red,ironsmith::mana::ManaSymbol::Green,ironsmith::mana::ManaSymbol::Colorless]{game.player_mut(A).unwrap().mana_pool.add(symbol,20);}
 (game,source,host,Choices{host})
}
fn drain(game:&mut GameState,dm:&mut Choices){let mut queue=ironsmith::triggers::TriggerQueue::new();ironsmith::game_loop::drain_pending_trigger_events_with_dm(game,&mut queue,dm).unwrap();ironsmith::game_loop::put_triggers_on_stack_with_dm(game,&mut queue,dm).unwrap();}
#[test]
fn exact_katana_source_retains_relation_during_ability_loss_and_never_revives_after_reexile(){
 for definition in definitions("msem-442"){
  let (mut game,source,host,mut dm)=source_setup(&definition);let stable=game.object(source).unwrap().stable_id;activate(&mut game,source,&mut dm);let exile=game.find_object_by_stable_id(stable).unwrap();
  assert_eq!(game.source_collection_objects(exile,"inscribed host"),vec![host]);assert_eq!(game.calculated_power(host),Some(4));assert_eq!(game.calculated_toughness(host),Some(3));
  let abilities=game.object(exile).unwrap().abilities.clone();game.object_mut(exile).unwrap().abilities_mut().clear();assert_eq!(game.calculated_power(host),Some(2));assert_eq!(game.source_collection_objects(exile,"inscribed host"),vec![host],"relationship is noncopiable state independent of ability presence");
  game.object_mut(exile).unwrap().abilities=abilities;assert_eq!(game.calculated_power(host),Some(4));
  let hand=move_to(&mut game,exile,Zone::Hand);let new_exile=move_to(&mut game,hand,Zone::Exile);assert_ne!(new_exile,exile);assert!(game.source_collection_objects(new_exile,"inscribed host").is_empty());assert_eq!(game.calculated_power(host),Some(2));
 }
}
#[test]
fn exact_curse_source_observes_linked_hosts_death_in_exile(){
 for definition in definitions("msem-429"){
  let (mut game,source,host,mut dm)=source_setup(&definition);let stable=game.object(source).unwrap().stable_id;activate(&mut game,source,&mut dm);let exile=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.source_collection_objects(exile,"inscribed host"),vec![host]);
  move_to(&mut game,host,Zone::Graveyard);drain(&mut game,&mut dm);assert_eq!(game.stack.len(),1,"printed source-owned death trigger looks back at departed membership");ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
  assert_eq!(game.battlefield.iter().filter_map(|id|game.object(*id)).filter(|object|object.subtypes.contains(&ironsmith::types::Subtype::Spirit)).count(),1,"only the death instruction created its Spirit; inscription did not cast the base spell");
 }
}
#[test]
fn exact_intrusive_source_applies_mill_bonus_to_linked_host_only(){
 for definition in definitions("msem-444"){
  let (mut game,source,host,mut dm)=source_setup(&definition);activate(&mut game,source,&mut dm);
  let library=ironsmith_compiler_runtime::compile_to_runtime_definition("Mill witness","Type: Land",false).unwrap();game.create_object_from_definition(&library,A,Zone::Library);
  let mill=ironsmith::Effect::new(ironsmith::effects::MillEffect::you(1));ironsmith::effects::execute_effect(&mut game,&mill,&mut ironsmith::effects::EffectContext::new_default(host,A)).unwrap();drain(&mut game,&mut dm);assert_eq!(game.stack.len(),1);ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();assert_eq!(game.calculated_power(host),Some(3));assert_eq!(game.calculated_toughness(host),Some(3));
 }
}
