//! Exact-source lifetime witnesses. Parent materializes the overlay after
//! frontend reader integration before registering this candidate target.
use ironsmith::{CardDefinition,GameState,ObjectId,PlayerId,Zone};
use ironsmith::decision::{DecisionMaker,LegalAction,SelectFirstDecisionMaker,compute_legal_actions};
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
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
fn attack(game:&mut GameState,host:ObjectId,dm:&mut Choices)->usize{
    game.queue_trigger_event(Default::default(),ironsmith::triggers::TriggerEvent::new(ironsmith::events::CreatureAttackedEvent::new(host,ironsmith::events::AttackEventTarget::Player(B)),Default::default()));
    let mut queue=ironsmith::triggers::TriggerQueue::new();ironsmith::game_loop::drain_pending_trigger_events_with_dm(game,&mut queue,dm).unwrap();ironsmith::game_loop::put_triggers_on_stack_with_dm(game,&mut queue,dm).unwrap();game.stack.len()
}
fn move_to(game:&mut GameState,object:ObjectId,zone:Zone)->ObjectId{
    let stable=game.object(object).unwrap().stable_id;let effect=ironsmith::Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::target::ChooseSpec::SpecificObject(object),zone,false));ironsmith::effects::execute_effect(game,&effect,&mut ironsmith::effects::EffectContext::new_default(object,A)).unwrap();game.find_object_by_stable_id(stable).unwrap()
}
fn setup(grantor:&CardDefinition)->(GameState,ObjectId,ObjectId,ObjectId,Choices){
    let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.active_player=A;game.turn.priority_player=Some(A);game.turn.phase=ironsmith::Phase::FirstMain;game.turn.step=None;
    let body=ironsmith_compiler_runtime::compile_to_runtime_definition("Static grant host","Type: Creature\nPower/Toughness: 2/2",false).unwrap();let host=game.create_object_from_definition(&body,A,Zone::Battlefield);
    let spell=ironsmith_compiler_runtime::compile_to_runtime_definition("Source-price spell","Mana cost: {U}\nType: Sorcery\nYou gain 1 life.",false).unwrap();let source=game.create_object_from_definition(&spell,A,Zone::Hand);
    let grantor=game.create_object_from_definition(grantor,A,Zone::Battlefield);(game,grantor,source,host,Choices{host})
}
#[test]
fn palace_static_exile_grant_tracks_keyword_presence_without_destroying_retained_relationship(){
    for definition in definitions("msem-422"){
        let (mut game,grantor,source,host,mut dm)=setup(&definition);let stable=game.object(source).unwrap().stable_id;
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Blue,1);activate(&mut game,source,&mut dm);let exiled=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(exiled).unwrap().zone,Zone::Exile);
        assert_eq!(attack(&mut game,host,&mut dm),1);ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
        let hand=move_to(&mut game,grantor,Zone::Hand);assert_eq!(attack(&mut game,host,&mut dm),0,"represented exile static ability is absent when no grantor gives the card its keyword");
        move_to(&mut game,hand,Zone::Battlefield);assert_eq!(attack(&mut game,host,&mut dm),1,"restoring keyword while exact exiled card/host remain restores static grant");
        assert_eq!(game.find_object_by_stable_id(stable),Some(exiled));
    }
}
#[test]
fn shae_literal_hand_only_keyword_grant_does_not_survive_into_exile(){
    for definition in definitions("msem-443"){
        let (mut game,_grantor,source,host,mut dm)=setup(&definition);let stable=game.object(source).unwrap().stable_id;
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Blue,2);game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,1);activate(&mut game,source,&mut dm);
        let exiled=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(exiled).unwrap().zone,Zone::Exile);assert_eq!(game.player(A).unwrap().mana_pool.total(),0,"source mana cost plus exact generic/hybrid increase");
        assert_eq!(attack(&mut game,host,&mut dm),0,"source 702.201a exile static ability is not supplied by a hand-only keyword grant");
    }
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
