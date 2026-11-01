//! Unregistered source-backed Colossal gameplay candidate; parent runs validation.
use ironsmith::{CardDefinition, Effect, GameProgress, GameState, ObjectId, PlayerId, Zone};
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm, apply_priority_response_with_dm, resolve_stack_entry_with};
use ironsmith::mana::ManaSymbol;
use ironsmith::object::CounterType;
use ironsmith::triggers::TriggerQueue;
use ironsmith_compiled_artifact::{ArtifactCardId, ArtifactCardIdentity, CompiledCardArtifact, CompiledCardPayload, wire_definition_from_serializable};
const A: PlayerId = PlayerId(0);
fn definitions()->[CardDefinition;2] {
 let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-colossal.json")).unwrap();
 assert_eq!(fixture["mechanics"].as_array().unwrap().len(),20);
 let row=fixture["mechanics"].as_array().unwrap().iter().find(|row|row["id"]=="msem-828").unwrap();
 let card=&row["representative_cards"][0];
 assert_eq!(card["custom_keyword_payments"][0],"colossal");
 compile_named(card["name"].as_str().unwrap(),card["executable_text"].as_str().unwrap())
}
fn compile_named(name: &str, text: &str) -> [CardDefinition; 2] {
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| ironsmith_compiler::word_claims::with_enforcement_options(true, true, || ironsmith_compiler::CompilerFacade::new().compile_definition(ironsmith_compiler::CardDefinitionBuilder::new(ironsmith_compiler::ids::CardId::new(), name).with_custom_keyword("colossal"), text.to_string(), ironsmith_compiler::CompilePolicy {allow_unsupported: false})));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let mut compiled = result.unwrap();
    assert!(compiled.definition.attribute_custom_keyword_payment("colossal"));
    let wire = wire_definition_from_serializable(&compiled.definition).unwrap();
    let direct = ironsmith_runtime_catalog::artifact_materializer::materialize_definition(wire.clone()).unwrap();
    assert!(direct.optional_costs.is_empty(), "the total alternative price must not become an additional cost");
    assert_eq!(direct.alternative_casts.len(), 1);
    assert!(direct.alternative_casts[0].keyword().is_none());
    let artifact = CompiledCardArtifact::new(ArtifactCardIdentity {local_id: ArtifactCardId(direct.card.id.0), name: direct.card.name.clone(), face_name: None, other_face: None, linked_face_layout: None}, CompiledCardPayload {definition: wire, canonical_text: ironsmith_text::compiled_text_lines(&direct).join("\n"), ability_labels: ironsmith_text::ability_surface_texts(&direct)}, "selected-price-copy-test", text.as_bytes());
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
fn setup() -> GameState {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.turn.step = None; game.turn.active_player = A; game.turn.priority_player = Some(A);
    game
}
fn cast(game: &mut GameState, definition: &CardDefinition, alternative: bool) -> ObjectId {
    cast_with_dm(game, definition, alternative, &mut SelectFirstDecisionMaker)
}
fn cast_with_dm(game: &mut GameState, definition: &CardDefinition, alternative: bool, dm: &mut impl ironsmith::decision::DecisionMaker) -> ObjectId {
    let source = game.create_object_from_definition(definition, A, Zone::Hand);
    let action = compute_legal_actions(game, A).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell {spell_id, casting_method, ..} if *spell_id == source && matches!(casting_method, CastingMethod::Alternative(_)) == alternative)).expect("requested price available");
    let mut queue = TriggerQueue::new(); let mut state = PriorityLoopState::new(2);
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), dm).unwrap();
    for _ in 0..64 {
        if !state.has_pending_action() {
            ironsmith::game_loop::drain_pending_trigger_events_with_dm(game, &mut queue, dm).unwrap();
            ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
            return game.stack.first().unwrap().object_id;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {panic!("{progress:?}")};
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    panic!("cast unfinished");
}

fn activate(game:&mut GameState,source:ObjectId) {
 let action=compute_legal_actions(game,A).unwrap().into_iter().find(|a|matches!(a,LegalAction::ActivateAbility{source:id,..} if *id==source)).expect("Colossal activation offered");
 let mut queue=TriggerQueue::new();let mut state=PriorityLoopState::new(2);let mut dm=SelectFirstDecisionMaker;
 let mut progress=apply_priority_response_with_dm(game,&mut queue,&mut state,&PriorityResponse::PriorityAction(action),&mut dm).unwrap();
 for _ in 0..64 {if !state.has_pending_action(){return;}let GameProgress::NeedsDecisionCtx(ctx)=progress else{panic!("{progress:?}")};progress=apply_decision_context_with_dm(game,&mut queue,&mut state,&ctx,&mut dm).unwrap();}panic!("unfinished activation");
}
fn fund(game:&mut GameState,n:u32){game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Red,1);game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless,n);}
#[test]
fn exact_source_total_price_and_counter_entry_are_replacement_not_trigger() {
 for definition in definitions(){for alternative in [false,true]{
 let mut game=setup();
 let probe=game.create_object_from_definition(&definition,A,Zone::Hand);fund(&mut game,3);
 let legal=compute_legal_actions(&game,A).unwrap();assert!(legal.iter().any(|a|matches!(a,LegalAction::CastSpell{spell_id,casting_method,..} if *spell_id==probe && !matches!(casting_method,CastingMethod::Alternative(_)))));assert!(!legal.iter().any(|a|matches!(a,LegalAction::CastSpell{spell_id,casting_method,..} if *spell_id==probe && matches!(casting_method,CastingMethod::Alternative(_)))));
 game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless,3);let spell=cast(&mut game,&definition,alternative);
 assert_eq!(game.stack.len(),1,"entry counters do not create cast trigger");
 assert_eq!(game.stack[0].optional_costs_paid.paid_custom_keyword("colossal"),alternative);
 assert_eq!(game.player(A).unwrap().mana_pool.total(),if alternative{0}else{3});
 let stable=game.object(spell).unwrap().stable_id;resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();
 let permanent=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(permanent).unwrap().zone,Zone::Battlefield);
 assert_eq!(game.counter_count(permanent,CounterType::PlusOnePlusOne),if alternative{3}else{0});
 assert!(game.stack.is_empty());let mut queue=TriggerQueue::new();ironsmith::game_loop::drain_pending_trigger_events_with_dm(&mut game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut SelectFirstDecisionMaker).unwrap();assert!(game.stack.is_empty(),"counters already exist without resolving an entry trigger");
 }}
}
#[test]
fn activation_checks_counter_absence_at_resolution_and_obeys_sorcery_timing() {
 for definition in definitions(){for added_in_response in [false,true]{
 let mut game=setup();let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);fund(&mut game,6);
 game.turn.phase=ironsmith::game_state::Phase::Combat;
 assert!(!compute_legal_actions(&game,A).unwrap().iter().any(|a|matches!(a,LegalAction::ActivateAbility{source:id,..} if *id==source)));
 game.turn.phase=ironsmith::game_state::Phase::FirstMain;activate(&mut game,source);assert_eq!(game.stack.len(),1);assert_eq!(game.player(A).unwrap().mana_pool.total(),0);
 if added_in_response {game.object_mut(source).unwrap().counters.insert(CounterType::PlusOnePlusOne,1);}
 resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();assert_eq!(game.counter_count(source,CounterType::PlusOnePlusOne),if added_in_response{1}else{3});
 fund(&mut game,6);activate(&mut game,source);assert_eq!(game.stack.len(),1,"existing counters are not an activation restriction");
 game.object_mut(source).unwrap().counters.remove(&CounterType::PlusOnePlusOne);
 resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();assert_eq!(game.counter_count(source,CounterType::PlusOnePlusOne),3,"absence is checked anew when the paid activation resolves");
 }}
}
