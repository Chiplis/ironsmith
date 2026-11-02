//! Actual source graveyard permission composes normal mana and an aggregate tap price.
#[path="custom_final_next_wave_support.rs"] mod source_support;
use ironsmith::{CardDefinition,GameState,ObjectId,PlayerId,Zone};
use ironsmith::decision::{DecisionMaker,SelectFirstDecisionMaker,LegalAction,compute_legal_actions};
use ironsmith::game_loop::{PriorityLoopState,PriorityResponse,apply_priority_response_with_dm,apply_decision_context_with_dm,resolve_stack_entry_with};
use ironsmith::triggers::TriggerQueue;
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
fn definitions()->[CardDefinition;2]{
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-final-real-source-next-wave.json")).unwrap();
    let source=fixture["mechanics"].as_array().unwrap().iter().flat_map(|row|row["representative_cards"].as_array().unwrap()).find(|source|source["source_card_key"]=="Heir's Legacy_MC25").unwrap();
    source_support::definitions(source).unwrap()
}
fn creature(power:i32)->CardDefinition{ironsmith_compiler_runtime::compile_to_runtime_definition("Tap resource",&format!("Type: Creature\nPower/Toughness: {power}/3"),false).unwrap()}
fn game(blue:u32)->GameState{
    let mut game=GameState::new(vec!["A".into(),"B".into()],20);
    game.turn.active_player=A;game.turn.priority_player=Some(A);game.turn.phase=ironsmith::game_state::Phase::FirstMain;game.turn.step=None;
    game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Blue,blue);
    game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,1);game
}
struct PayAllResources;
impl DecisionMaker for PayAllResources{
    fn decide_objects(&mut self,_:&GameState,ctx:&ironsmith::decisions::context::SelectObjectsContext)->Vec<ObjectId>{ctx.candidates.iter().filter(|candidate|candidate.legal).map(|candidate|candidate.id).collect()}
}
fn cast_action(game:&GameState,id:ObjectId)->Option<LegalAction>{compute_legal_actions(game,A).unwrap().into_iter().find(|action|matches!(action,LegalAction::CastSpell{spell_id,..}if *spell_id==id))}
fn cast(game:&mut GameState,id:ObjectId){
    let action=cast_action(game,id).expect("the complete printed mana plus tap price is affordable");
    let mut state=PriorityLoopState::new(2);let mut queue=TriggerQueue::new();let mut dm=PayAllResources;
    let mut progress=apply_priority_response_with_dm(game,&mut queue,&mut state,&PriorityResponse::PriorityAction(action),&mut dm).unwrap();
    for _ in 0..64{if !state.has_pending_action(){return}let ironsmith::GameProgress::NeedsDecisionCtx(ctx)=progress else{panic!("untyped casting pause")};progress=apply_decision_context_with_dm(game,&mut queue,&mut state,&ctx,&mut dm).unwrap();}
    panic!("casting did not complete");
}
#[test]
fn actual_graveyard_source_pays_printed_mana_and_combines_two_subthreshold_creatures(){
    for definition in definitions(){
        let mut game=game(2);let source=game.create_object_from_definition(&definition,A,Zone::Graveyard);let stable=game.object(source).unwrap().stable_id;
        let one=game.create_object_from_definition(&creature(1),A,Zone::Battlefield);let two=game.create_object_from_definition(&creature(2),A,Zone::Battlefield);
        cast(&mut game,source);assert!(game.is_tapped(one));assert!(game.is_tapped(two));assert_eq!(game.object(game.stack.last().unwrap().object_id).unwrap().zone,Zone::Stack,"paying does not prematurely exile the spell");
        assert_eq!(game.player(A).unwrap().mana_pool.total(),0,"normal {{1}}{{U}}{{U}} remains due in addition to the tap price");
        resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone,Zone::Exile);assert_eq!(game.battlefield.len(),3,"ordinary source copy effect resolves");
    }
}
#[test]
fn source_permission_rejects_insufficient_power_tapped_opponent_resources_and_missing_mana(){
    for definition in definitions(){for scenario in 0..4{
        let mut game=game(if scenario==3{1}else{2});let source=game.create_object_from_definition(&definition,A,Zone::Graveyard);
        game.create_object_from_definition(&creature(1),A,Zone::Battlefield);
        let other=game.create_object_from_definition(&creature(if scenario==0{1}else{2}),if scenario==2{B}else{A},Zone::Battlefield);
        if scenario==1{game.tap(other);}assert!(cast_action(&game,source).is_none(),"scenario {scenario} must not waive part of the complete cost");assert_eq!(game.object(source).unwrap().zone,Zone::Graveyard);
    }}
}
#[test]
fn selected_graveyard_price_exiles_on_counter_or_return_without_resolving_copy(){
    for definition in definitions(){for counter in [false,true]{
        let mut game=game(2);let source=game.create_object_from_definition(&definition,A,Zone::Graveyard);let stable=game.object(source).unwrap().stable_id;
        game.create_object_from_definition(&creature(1),A,Zone::Battlefield);game.create_object_from_definition(&creature(2),A,Zone::Battlefield);cast(&mut game,source);let spell=game.stack.last().unwrap().object_id;
        let witness=game.create_object_from_definition(&creature(0),B,Zone::Battlefield);
        let effect=if counter{ironsmith::Effect::new(ironsmith::effects::CounterEffect::new(ironsmith::target::ChooseSpec::SpecificObject(spell)))}else{ironsmith::Effect::new(ironsmith::effects::MoveToZoneEffect::new(ironsmith::target::ChooseSpec::SpecificObject(spell),Zone::Hand,false))};
        ironsmith::effects::execute_effect(&mut game,&effect,&mut ironsmith::effects::EffectContext::new_default(witness,B)).unwrap();
        assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone,Zone::Exile);assert_eq!(game.battlefield.len(),3,"counter/bounce does not resolve the copy instruction");
    }}
}
#[test]
fn normal_hand_cast_does_not_inherit_the_graveyard_tap_price_or_exile_rider(){
    for definition in definitions(){
        let mut game=game(2);let source=game.create_object_from_definition(&definition,A,Zone::Hand);let stable=game.object(source).unwrap().stable_id;
        let target=game.create_object_from_definition(&creature(0),A,Zone::Battlefield);game.tap(target);
        cast(&mut game,source);assert!(game.is_tapped(target));resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone,Zone::Graveyard);assert_eq!(game.battlefield.len(),2);
    }
}
