//! Unregistered full-source ordinary Improve scenarios. Parent owns execution.
use ironsmith::{CardDefinition,GameState,ObjectId,PlayerId,Zone};
use ironsmith::decision::{LegalAction,SelectFirstDecisionMaker,compute_legal_actions};
use ironsmith::mana::ManaSymbol;
use ironsmith::object::CounterType;
use ironsmith_compiled_artifact::{ArtifactCardId,ArtifactCardIdentity,CompiledCardArtifact,CompiledCardPayload,wire_definition_from_serializable};
const A:PlayerId=PlayerId(0);
fn definitions(id:&str)->[CardDefinition;2]{
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-improve.json")).unwrap();
 assert_eq!(fixture["mechanics"].as_array().unwrap().len(),20);
 let row=fixture["mechanics"].as_array().unwrap().iter().find(|row|row["id"]==id).unwrap();let card=&row["representative_cards"][0];
 assert_eq!(card["custom_keyword_identities"][0],"improve");assert!(card["custom_keyword_payments"].as_array().unwrap().is_empty());
 compile_named(card["name"].as_str().unwrap(),card["executable_text"].as_str().unwrap())
}
fn compile_named(name: &str, text: &str) -> [CardDefinition; 2] {
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| ironsmith_compiler::word_claims::with_enforcement_options(true, true, || ironsmith_compiler::CompilerFacade::new().compile_definition(ironsmith_compiler::CardDefinitionBuilder::new(ironsmith_compiler::ids::CardId::new(), name).with_custom_keyword("improve"), text.to_string(), ironsmith_compiler::CompilePolicy {allow_unsupported: false})));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let compiled = result.unwrap();
    let wire = wire_definition_from_serializable(&compiled.definition).unwrap();
    let direct = ironsmith_runtime_catalog::artifact_materializer::materialize_definition(wire.clone()).unwrap();
    let artifact = CompiledCardArtifact::new(ArtifactCardIdentity {local_id: ArtifactCardId(direct.card.id.0), name: direct.card.name.clone(), face_name: None, other_face: None, linked_face_layout: None}, CompiledCardPayload {definition: wire, canonical_text: ironsmith_text::compiled_text_lines(&direct).join("\n"), ability_labels: ironsmith_text::ability_surface_texts(&direct)}, "source-improve-test", text.as_bytes());
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}

fn legal(game:&GameState,source:ObjectId)->Option<LegalAction>{compute_legal_actions(game,A).unwrap().into_iter().find(|a|matches!(a,LegalAction::ActivateAbility{source:id,..}if *id==source))}
fn activate(game:&mut GameState,action:LegalAction) {
    use ironsmith::game_loop::{PriorityLoopState,PriorityResponse,apply_priority_response_with_dm,apply_decision_context_with_dm};
    let mut queue=ironsmith::triggers::TriggerQueue::new();let mut state=PriorityLoopState::new(2);let mut dm=SelectFirstDecisionMaker;
    let mut progress=apply_priority_response_with_dm(game,&mut queue,&mut state,&PriorityResponse::PriorityAction(action),&mut dm).unwrap();
    for _ in 0..64 {if !state.has_pending_action(){ironsmith::game_loop::resolve_stack_entry_with(game,&mut dm).unwrap();return;}let ironsmith::GameProgress::NeedsDecisionCtx(ctx)=progress else{panic!("{progress:?}")};progress=apply_decision_context_with_dm(game,&mut queue,&mut state,&ctx,&mut dm).unwrap();}panic!("activation unfinished");
}

#[test]
fn exact_familiar_uses_source_counter_count_for_each_paid_sorcery_activation(){scenario("msem-727",ManaSymbol::Blue,1);}
#[test]
fn exact_corym_replacement_adds_extra_counter_and_next_price_uses_actual_result(){scenario("msem-729",ManaSymbol::Green,2);}
fn scenario(id:&str,colored:ManaSymbol,increment:u32){
 for definition in definitions(id){for initial in [0u32,2]{
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.active_player=A;game.turn.priority_player=Some(A);game.turn.phase=ironsmith::game_state::Phase::FirstMain;game.turn.step=None;
 let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
 if initial>0 {game.object_mut(source).unwrap().counters.insert(CounterType::PlusOnePlusOne,initial);}
 for round in 0..2u32 {
 let counters=initial+round*increment;let generic=1+counters;
 game.player_mut(A).unwrap().mana_pool.add(colored,1);game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless,generic-1);
 assert!(legal(&game,source).is_none(),"one generic mana short at actual source counter count {counters}");
 game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless,1);
 game.turn.phase=ironsmith::game_state::Phase::Combat;assert!(legal(&game,source).is_none(),"Improve is sorcery-only");game.turn.phase=ironsmith::game_state::Phase::FirstMain;
 game.turn.active_player=PlayerId(1);assert!(legal(&game,source).is_none(),"not during opponent's main phase");game.turn.active_player=A;
 let action=legal(&game,source).expect("exact complete source-dependent price available");activate(&mut game,action);
 assert_eq!(game.counter_count(source,CounterType::PlusOnePlusOne),counters+increment);
 assert_eq!(game.player(A).unwrap().mana_pool.total(),0,"activation pays base colored+generic plus actual source counters");
 }
 }}
}
