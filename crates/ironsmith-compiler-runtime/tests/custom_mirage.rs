//! Price-specific instant timing and next-cleanup sacrifice of the resulting incarnation.
use ironsmith::{CardDefinition, GameState, ObjectId, PlayerId, Zone};
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::mana::ManaSymbol;
const A:PlayerId=PlayerId::from_index(0);const B:PlayerId=PlayerId::from_index(1);
const TEXT:&str="Mana cost: {3}{B}{B}\nType: Enchantment\nAny time you could cast an instant, you may cast this spell from your hand for {1}{B} rather than pay its mana cost. If you cast this spell this way, sacrifice it at the beginning of the next cleanup step.";
fn definitions()->Vec<CardDefinition>{definitions_for(TEXT)}
fn definitions_for(text:&str)->Vec<CardDefinition>{
    let mut compiled=ironsmith_compiler::CompilerFacade::new().compile_definition(
        ironsmith_compiler::CardDefinitionBuilder::new(ironsmith_compiler::ids::CardId::new(),"Selected timing price").with_custom_keyword("Mirage"),text.into(),ironsmith_compiler::CompilePolicy{allow_unsupported:false}).unwrap();
    assert!(compiled.definition.attribute_custom_keyword_payment("Mirage"),"selected generic price retains its own payment identity");
    let wire=ironsmith_compiled_artifact::wire_definition_from_serializable(&compiled.definition).unwrap();
    let direct=ironsmith_compiler_runtime::into_runtime_definition(compiled.definition.clone()).unwrap();
    let artifact=ironsmith_compiled_artifact::CompiledCardArtifact::new(ironsmith_compiled_artifact::ArtifactCardIdentity{
        local_id:ironsmith_compiled_artifact::ArtifactCardId(compiled.definition.card.id.0),name:compiled.definition.card.name.clone(),face_name:None,other_face:None,linked_face_layout:None,
    },ironsmith_compiled_artifact::CompiledCardPayload{definition:wire,canonical_text:ironsmith_text::compiled_text_lines(&direct).join("\n"),ability_labels:ironsmith_text::ability_surface_texts(&direct)},"selected-timing-test",text.as_bytes());
    let decoded=ironsmith_compiled_artifact::CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    let restored=ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap();
    vec![direct,restored]
}
fn game()->GameState{let mut g=GameState::new(vec!["A".into(),"B".into()],20);g.turn.active_player=A;g.turn.priority_player=Some(A);g.turn.phase=ironsmith::game_state::Phase::FirstMain;g.turn.step=None;g.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Black,5);g.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless,3);g}
fn announce(g:&mut GameState,action:LegalAction){
    use ironsmith::game_loop::{PriorityLoopState,PriorityResponse,apply_priority_response_with_dm,apply_decision_context_with_dm};
    let mut state=PriorityLoopState::new(2);let mut queue=ironsmith::triggers::TriggerQueue::new();let mut dm=SelectFirstDecisionMaker;
    let mut progress=apply_priority_response_with_dm(g,&mut queue,&mut state,&PriorityResponse::PriorityAction(action),&mut dm).unwrap();
    for _ in 0..64 {if !state.has_pending_action(){return;}let ironsmith::GameProgress::NeedsDecisionCtx(ctx)=progress else{panic!("{progress:?}")};progress=apply_decision_context_with_dm(g,&mut queue,&mut state,&ctx,&mut dm).unwrap();}panic!("casting incomplete");
}
fn action(g:&GameState,id:ObjectId,alternate:bool)->LegalAction{compute_legal_actions(g,A).unwrap().into_iter().find(|a|matches!(a,LegalAction::CastSpell{spell_id,casting_method,..} if *spell_id==id && matches!(casting_method,CastingMethod::Alternative(_))==alternate)).unwrap()}
fn cleanup(g:&mut GameState){let event=ironsmith::triggers::TriggerEvent::new_with_provenance(ironsmith::events::phase::BeginningOfCleanupStepEvent::new(B),Default::default());let mut q=ironsmith::triggers::TriggerQueue::new();for t in ironsmith::triggers::check_delayed_triggers(g,&event){q.add(t);}ironsmith::game_loop::put_triggers_on_stack(g,&mut q).unwrap();while !g.stack_is_empty(){ironsmith::game_loop::resolve_stack_entry_with(g,&mut SelectFirstDecisionMaker).unwrap();}}
#[test]
fn only_selected_alternative_price_grants_instant_timing(){for d in definitions(){let mut g=game();g.turn.active_player=B;g.turn.phase=ironsmith::game_state::Phase::Beginning;g.turn.step=Some(ironsmith::game_state::Step::Upkeep);let id=g.create_object_from_definition(&d,A,Zone::Hand);let actions=compute_legal_actions(&g,A).unwrap();assert!(actions.iter().any(|a|matches!(a,LegalAction::CastSpell{spell_id,casting_method:CastingMethod::Alternative(_),..} if *spell_id==id)));assert!(!actions.iter().any(|a|matches!(a,LegalAction::CastSpell{spell_id,casting_method:CastingMethod::Normal,..} if *spell_id==id)));}}
#[test]
fn selected_price_sacrifices_at_cleanup_after_control_change_and_normal_cast_does_not(){for d in definitions(){for alternate in [false,true]{let mut g=game();let id=g.create_object_from_definition(&d,A,Zone::Hand);let stable=g.object(id).unwrap().stable_id;let selected=action(&g,id,alternate);announce(&mut g,selected);ironsmith::game_loop::resolve_stack_entry_with(&mut g,&mut SelectFirstDecisionMaker).unwrap();let permanent=g.find_object_by_stable_id(stable).unwrap();assert_eq!(g.object(permanent).unwrap().zone,Zone::Battlefield);let end=ironsmith::triggers::TriggerEvent::new_with_provenance(ironsmith::events::phase::BeginningOfEndStepEvent::new(B),Default::default());assert!(ironsmith::triggers::check_delayed_triggers(&mut g,&end).is_empty());g.set_current_controller(permanent,B).unwrap();cleanup(&mut g);let current=g.find_object_by_stable_id(stable).unwrap();assert_eq!(g.object(current).unwrap().zone,if alternate{Zone::Graveyard}else{Zone::Battlefield});}}}
#[test]
fn countered_selected_spell_never_schedules_cleanup_sacrifice(){for d in definitions(){let mut g=game();let id=g.create_object_from_definition(&d,A,Zone::Hand);let selected=action(&g,id,true);announce(&mut g,selected);let stack_id=g.stack.last().unwrap().object_id;g.move_object_by_effect(stack_id,Zone::Graveyard);assert!(g.effect_store.delayed_triggers.is_empty());}}
#[test]
fn selected_cleanup_trigger_does_not_follow_a_blinked_permanent(){for d in definitions(){let mut g=game();let id=g.create_object_from_definition(&d,A,Zone::Hand);let stable=g.object(id).unwrap().stable_id;let selected=action(&g,id,true);announce(&mut g,selected);ironsmith::game_loop::resolve_stack_entry_with(&mut g,&mut SelectFirstDecisionMaker).unwrap();let id=g.find_object_by_stable_id(stable).unwrap();let exile=g.move_object_by_effect(id,Zone::Exile).unwrap();let returned=g.move_object_by_effect(exile,Zone::Battlefield).unwrap();cleanup(&mut g);assert_eq!(g.object(returned).unwrap().zone,Zone::Battlefield);}}

#[test]
fn printed_flash_keeps_normal_cast_legal_without_scheduling_selected_price_sacrifice(){
    let text=TEXT.replace("Type: Enchantment\n","Type: Enchantment\nFlash\n");
    for d in definitions_for(&text){
        assert_eq!(d.alternative_casts[0].name(),"Mirage");assert_eq!(d.alternative_casts[0].keyword(),None);
        let mut g=game();g.turn.active_player=B;g.turn.phase=ironsmith::game_state::Phase::Beginning;g.turn.step=Some(ironsmith::game_state::Step::Upkeep);
        let id=g.create_object_from_definition(&d,A,Zone::Hand);let stable=g.object(id).unwrap().stable_id;let normal=action(&g,id,false);announce(&mut g,normal);
        ironsmith::game_loop::resolve_stack_entry_with(&mut g,&mut SelectFirstDecisionMaker).unwrap();cleanup(&mut g);
        let permanent=g.find_object_by_stable_id(stable).unwrap();assert_eq!(g.object(permanent).unwrap().zone,Zone::Battlefield);assert!(g.effect_store.delayed_triggers.is_empty());
    }
}
#[test]
fn selected_aura_with_illegal_resolution_target_never_schedules_cleanup_sacrifice(){
    let text=TEXT.replace("Type: Enchantment\n","Type: Enchantment — Aura\nEnchant creature\n");
    for d in definitions_for(&text){
        let mut g=game();let target_definition=ironsmith_compiler_runtime::compile_to_runtime_definition("Aura target","Type: Creature\nPower/Toughness: 2/2",false).unwrap();let target=g.create_object_from_definition(&target_definition,A,Zone::Battlefield);
        let source=g.create_object_from_definition(&d,A,Zone::Hand);let stable=g.object(source).unwrap().stable_id;let selected=action(&g,source,true);announce(&mut g,selected);
        g.move_object_by_effect(target,Zone::Hand).unwrap();ironsmith::game_loop::resolve_stack_entry_with(&mut g,&mut SelectFirstDecisionMaker).unwrap();
        let finished=g.find_object_by_stable_id(stable).unwrap();assert_eq!(g.object(finished).unwrap().zone,Zone::Graveyard);assert!(g.effect_store.delayed_triggers.is_empty());
    }
}
