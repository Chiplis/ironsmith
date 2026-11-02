//! CR107.3b applies to nonzero selected zone prices as well as free ones.
#[path = "custom_final_next_wave_support.rs"] mod source;
use ironsmith::{CardDefinition,GameState,ObjectId,PlayerId,Zone};
use ironsmith::decision::{DecisionMaker,SelectFirstDecisionMaker,LegalAction,compute_legal_actions};
const A:PlayerId=PlayerId(0);
struct ChooseX(u32);
impl DecisionMaker for ChooseX {
    fn decide_number(&mut self,_:&GameState,ctx:&ironsmith::decisions::context::NumberContext)->u32 {self.0.max(ctx.min).min(ctx.max)}
    fn decide_boolean(&mut self,_:&GameState,ctx:&ironsmith::decisions::context::BooleanContext)->bool {ctx.can_accept}
    fn decide_objects(&mut self,game:&GameState,ctx:&ironsmith::decisions::context::SelectObjectsContext)->Vec<ObjectId> {SelectFirstDecisionMaker.decide_objects(game,ctx)}
    fn decide_options(&mut self,game:&GameState,ctx:&ironsmith::decisions::context::SelectOptionsContext)->Vec<usize> {SelectFirstDecisionMaker.decide_options(game,ctx)}
}
fn definitions(printed:&str,price:&str,minimum:bool)->[CardDefinition;2] {
    source::definitions(&serde_json::json!({"name":"Selected zone X witness","executable_text":format!(
        "Mana cost: {printed}\nType: Sorcery\nAs an additional cost to cast this spell, exile X cards from your graveyard.\nYou may cast this spell from your graveyard for {price} rather than pay its mana cost.\nYou draw X cards and you lose X life.{}",if minimum {"\nX can't be 0."} else {""})})).unwrap()
}
fn setup(definition:&CardDefinition)->(GameState,ObjectId,Vec<ironsmith::ids::StableId>) {
    let mut game=GameState::new(vec!["A".into(),"B".into()],20);
    game.turn.active_player=A;game.turn.priority_player=Some(A);game.turn.phase=ironsmith::Phase::FirstMain;game.turn.step=None;
    let source=game.create_object_from_definition(definition,A,Zone::Graveyard);
    let filler=ironsmith_compiler_runtime::compile_to_runtime_definition("Independent X payment control","Type: Artifact",false).unwrap();
    let materials=(0..3).map(|_| {let id=game.create_object_from_definition(&filler,A,Zone::Graveyard);game.object(id).unwrap().stable_id}).collect();
    for _ in 0..3 {game.create_object_from_definition(&filler,A,Zone::Library);}
    game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,2);
    (game,source,materials)
}
fn announce(game:&mut GameState,source:ObjectId,dm:&mut impl DecisionMaker)->Result<ObjectId,String> {
    use ironsmith::game_loop::{PriorityLoopState,PriorityResponse,apply_priority_response_with_dm,apply_decision_context_with_dm};
    let action=compute_legal_actions(game,A).map_err(|e|format!("{e:?}"))?.into_iter().find(|action|matches!(action,LegalAction::CastSpell {spell_id,..}if *spell_id==source)).ok_or("no legal selected price")?;
    let mut state=PriorityLoopState::new(2);let mut queue=ironsmith::triggers::TriggerQueue::new();
    let mut progress=apply_priority_response_with_dm(game,&mut queue,&mut state,&PriorityResponse::PriorityAction(action),dm).map_err(|e|format!("{e:?}"))?;
    for _ in 0..64 {
        if !state.has_pending_action() {break;}
        let ironsmith::GameProgress::NeedsDecisionCtx(context)=progress else {return Err(format!("unfinished {progress:?}"));};
        progress=apply_decision_context_with_dm(game,&mut queue,&mut state,&context,dm).map_err(|e|format!("{e:?}"))?;
    }
    if state.has_pending_action() {return Err("cast did not settle".into());}
    game.stack.last().map(|entry|entry.object_id).ok_or("cast did not commit".into())
}
#[test]
fn nonzero_fixed_zone_price_forces_printed_x_to_zero_despite_additional_x_cost() {
    for definition in definitions("{X}{B}","{2}",false) {
        let(mut game,source,materials)=setup(&definition);
        let stack=announce(&mut game,source,&mut ChooseX(2)).unwrap();
        assert_eq!(game.object(stack).unwrap().x_value,Some(0));
        assert!(materials.iter().all(|stable|game.object(game.find_object_by_stable_id(*stable).unwrap()).unwrap().zone==Zone::Graveyard));
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(game.player(A).unwrap().life,20);assert!(game.player(A).unwrap().hand.is_empty());assert_eq!(game.player(A).unwrap().library.len(),3);
    }
}
#[test]
fn selected_x_price_and_additional_only_x_remain_announced_independently() {
    for (printed,price) in [("{X}{B}","{X}"),("{B}","{2}")] {
        for definition in definitions(printed,price,false) {
            let(mut game,source,materials)=setup(&definition);
            let stack=announce(&mut game,source,&mut ChooseX(2)).unwrap();assert_eq!(game.object(stack).unwrap().x_value,Some(2));
            assert_eq!(materials.iter().filter(|stable|game.object(game.find_object_by_stable_id(**stable).unwrap()).unwrap().zone==Zone::Exile).count(),2);
            ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();
            assert_eq!(game.player(A).unwrap().life,18);assert_eq!(game.player(A).unwrap().hand.len(),2);assert_eq!(game.player(A).unwrap().library.len(),1);
        }
    }
}
#[test]
fn minimum_x_rejects_forced_zero_without_payment_or_incarnation_changes() {
    for definition in definitions("{X}{B}","{2}",true) {
        let(mut game,source,materials)=setup(&definition);
        assert!(announce(&mut game,source,&mut ChooseX(2)).is_err());
        assert!(game.stack.is_empty());assert_eq!(game.object(source).unwrap().zone,Zone::Graveyard);assert_eq!(game.player(A).unwrap().mana_pool.total(),2);
        assert!(materials.iter().all(|stable|game.object(game.find_object_by_stable_id(*stable).unwrap()).unwrap().zone==Zone::Graveyard));
        assert_eq!(game.player(A).unwrap().life,20);
    }
}
#[test]
fn actual_twelve_thousand_eyes_fixed_coalesce_price_keeps_x_zero_and_still_enters() {
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/coalesce-complete-source-fulfillment.json")).unwrap();
    let card=fixture["mechanics"].as_array().unwrap().iter().flat_map(|row|row["representative_cards"].as_array().unwrap()).find(|card|card["source_card_key"]=="Twelve Thousand Eyes_RWR").unwrap();
    for definition in source::linked_definitions(card).unwrap().pop().unwrap() {
        let(mut game,source,materials)=setup(&definition);
        game.player_mut(A).unwrap().mana_pool=Default::default();game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Black,9);
        let stack=announce(&mut game,source,&mut ChooseX(2)).unwrap();let stable=game.object(stack).unwrap().stable_id;assert_eq!(game.object(stack).unwrap().x_value,Some(0));
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();
        let permanent=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(permanent).unwrap().zone,Zone::Battlefield);assert_eq!(game.current_power(permanent),Some(6));
        assert_eq!(game.player(A).unwrap().life,20);assert!(game.player(A).unwrap().hand.is_empty());assert_eq!(game.player(A).unwrap().library.len(),3);
        assert!(materials.iter().all(|stable|game.object(game.find_object_by_stable_id(*stable).unwrap()).unwrap().zone==Zone::Graveyard));
    }
}
