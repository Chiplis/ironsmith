//! Actual Trial sources through generic announcement programs/payment receipts.
#[path="custom_final_next_wave_support.rs"] mod source_support;
use ironsmith::{CardDefinition,GameState,ObjectId,PlayerId,Zone};
use ironsmith::decision::{DecisionMaker,SelectFirstDecisionMaker,compute_legal_actions,LegalAction};
use ironsmith::game_loop::{PriorityLoopState,PriorityResponse,apply_priority_response_with_dm,apply_decision_context_with_dm};
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);const C:PlayerId=PlayerId(2);
fn definitions(id:&str)->[CardDefinition;2] {
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../fixtures/custom_mechanics/msem-casting-middle.json")).unwrap();
    let row=fixture["mechanics"].as_array().unwrap().iter().find(|row|row["id"]==id).unwrap();let card=&row["representative_cards"][0];
    source_support::definitions(card).unwrap()
}
fn game()->GameState {
    let mut game=GameState::new(vec!["A".into(),"B".into(),"C".into()],20);
    game.turn.active_player=A;game.turn.priority_player=Some(A);game.turn.phase=ironsmith::game_state::Phase::FirstMain;
    for symbol in [ironsmith::mana::ManaSymbol::White,ironsmith::mana::ManaSymbol::Blue,ironsmith::mana::ManaSymbol::Black,ironsmith::mana::ManaSymbol::Red,ironsmith::mana::ManaSymbol::Green,ironsmith::mana::ManaSymbol::Colorless] {game.player_mut(A).unwrap().mana_pool.add(symbol,10);}
    game
}
struct Choices {pay:bool,target:Option<ObjectId>,target_seen:bool,require_targets:bool}
impl Choices {fn new(pay:bool)->Self {Self {pay,target:None,target_seen:false,require_targets:false}}}
impl DecisionMaker for Choices {
    fn decide_boolean(&mut self,_:&GameState,ctx:&ironsmith::decisions::context::BooleanContext)->bool {
        if ctx.player!=A && self.require_targets {assert!(self.target_seen,"opponent offer follows actual target announcement");}self.pay
    }
    fn decide_options(&mut self,game:&GameState,ctx:&ironsmith::decisions::context::SelectOptionsContext)->Vec<usize>{SelectFirstDecisionMaker.decide_options(game,ctx)}
    fn decide_objects(&mut self,game:&GameState,ctx:&ironsmith::decisions::context::SelectObjectsContext)->Vec<ObjectId>{SelectFirstDecisionMaker.decide_objects(game,ctx)}
    fn decide_targets(&mut self,game:&GameState,ctx:&ironsmith::decisions::context::TargetsContext)->Vec<ironsmith::Target>{self.target_seen=true;self.target.map(|target|vec![ironsmith::Target::Object(target)]).unwrap_or_else(||SelectFirstDecisionMaker.decide_targets(game,ctx))}
}
fn begin(game:&mut GameState,card:ObjectId,dm:&mut Choices)->(PriorityLoopState,ironsmith::triggers::TriggerQueue,ironsmith::GameProgress) {
    let action=compute_legal_actions(game,A).unwrap().into_iter().find(|action|matches!(action,LegalAction::CastSpell {spell_id,..} if *spell_id==card)).unwrap();
    let mut state=PriorityLoopState::new(3);let mut queue=ironsmith::triggers::TriggerQueue::new();
    let progress=apply_priority_response_with_dm(game,&mut queue,&mut state,&PriorityResponse::PriorityAction(action),dm).unwrap();(state,queue,progress)
}
fn finish(game:&mut GameState,state:&mut PriorityLoopState,queue:&mut ironsmith::triggers::TriggerQueue,mut progress:ironsmith::GameProgress,dm:&mut Choices) {
    for _ in 0..64 {if !state.has_pending_action(){return;}let ironsmith::GameProgress::NeedsDecisionCtx(ctx)=progress else{panic!("{progress:?}")};progress=apply_decision_context_with_dm(game,queue,state,&ctx,dm).unwrap();}panic!("casting did not finish");
}
fn cast(game:&mut GameState,card:ObjectId,dm:&mut Choices) {
    let (mut state,mut queue,progress)=begin(game,card,dm);finish(game,&mut state,&mut queue,progress,dm);
}
fn settle(game:&mut GameState,dm:&mut Choices) {
    let mut queue=ironsmith::triggers::TriggerQueue::new();ironsmith::game_loop::drain_pending_trigger_events_with_dm(game,&mut queue,dm).unwrap();ironsmith::game_loop::put_triggers_on_stack_with_dm(game,&mut queue,dm).unwrap();
}
#[test]
fn source_trial_payment_is_immediate_full_and_controls_only_its_own_result() {
    for definition in definitions("msem-509") {for pay in [false,true] {
        let mut game=game();let card=game.create_object_from_definition(&definition,A,Zone::Hand);let stable=game.object(card).unwrap().stable_id;let mut dm=Choices::new(pay);
        cast(&mut game,card,&mut dm);let spell=game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.player(B).unwrap().life,if pay {16}else{20});assert_eq!(game.player(C).unwrap().life,20);
        let receipt=&game.object(spell).unwrap().named_payment_outcomes;assert_eq!(receipt.len(),1);assert_eq!(receipt[0].payer,B);assert_eq!(receipt[0].paid,pay);
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();assert_eq!(game.battlefield.len(),if pay {1}else{3});
        let grave=game.find_object_by_stable_id(stable).unwrap();assert!(game.object(grave).unwrap().named_payment_outcomes.is_empty(),"independent next incarnation has no old payment outcome");
    }}
}
#[test]
fn source_trial_controller_check_uses_chosen_failed_player_after_targets() {
    for definition in definitions("msem-510") {for owner in [B,C] {
        let mut game=game();let body=ironsmith_compiler_runtime::compile_to_runtime_definition("Target body","Type: Creature\nPower/Toughness: 6/6",false).unwrap();let target=game.create_object_from_definition(&body,owner,Zone::Battlefield);
        let card=game.create_object_from_definition(&definition,A,Zone::Hand);let mut dm=Choices::new(false);dm.target=Some(target);dm.require_targets=true;
        cast(&mut game,card,&mut dm);ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();assert_eq!(game.calculated_toughness(target),Some(if owner==B {2}else{5}));
    }}
}
#[test]
fn source_trial_permanent_retains_outcome_only_for_spell_to_entry() {
    for definition in definitions("msem-511") {for pay in [false,true] {
        let mut game=game();let card=game.create_object_from_definition(&definition,A,Zone::Hand);let stable=game.object(card).unwrap().stable_id;let mut dm=Choices::new(pay);cast(&mut game,card,&mut dm);
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();settle(&mut game,&mut dm);while !game.stack.is_empty(){ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();}
        let permanent=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(permanent).unwrap().named_payment_outcomes.len(),1);assert_eq!(game.current_has_static_ability_id(permanent,ironsmith_core::StaticAbilityId::Haste),!pay);
        let hand=game.move_object_by_effect(permanent,Zone::Hand).unwrap();assert!(game.object(hand).unwrap().named_payment_outcomes.is_empty());
        let fresh=game.move_object_by_effect(hand,Zone::Battlefield).unwrap();settle(&mut game,&mut dm);assert!(game.object(fresh).unwrap().named_payment_outcomes.is_empty());assert!(game.stack.is_empty(),"uncast entry cannot borrow an earlier failed payment");
    }}
}
#[test]
fn departed_opponents_are_not_offered_and_unpayable_life_is_not_lost() {
    for definition in definitions("msem-509") {
        let mut game=game();game.player_mut(B).unwrap().has_left_game=true;game.player_mut(C).unwrap().life=3;
        let card=game.create_object_from_definition(&definition,A,Zone::Hand);let stable=game.object(card).unwrap().stable_id;cast(&mut game,card,&mut Choices::new(true));
        let spell=game.find_object_by_stable_id(stable).unwrap();let receipt=&game.object(spell).unwrap().named_payment_outcomes[0];assert_eq!(receipt.payer,C);assert!(!receipt.paid);assert_eq!(game.player(C).unwrap().life,3);assert_eq!(game.player(B).unwrap().life,20);
    }
}
#[test]
fn cancelling_after_an_opponent_paid_rolls_back_the_entire_cast() {
    for definition in definitions("msem-509") {
        let mut game=game();let card=game.create_object_from_definition(&definition,A,Zone::Hand);let stable=game.object(card).unwrap().stable_id;let mut dm=Choices::new(true);
        let (mut state,mut queue,mut progress)=begin(&mut game,card,&mut dm);
        for _ in 0..32 {
            let current=game.find_object_by_stable_id(stable).unwrap();if !game.object(current).unwrap().named_payment_outcomes.is_empty(){break;}
            let ironsmith::GameProgress::NeedsDecisionCtx(ctx)=progress else{panic!("{progress:?}")};progress=apply_decision_context_with_dm(&mut game,&mut queue,&mut state,&ctx,&mut dm).unwrap();
        }
        assert_eq!(game.player(B).unwrap().life,16);assert!(state.has_pending_action());assert!(state.rollback_action(&mut game));assert_eq!(game.player(B).unwrap().life,20);
        let original=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(original).unwrap().zone,Zone::Hand);assert!(game.object(original).unwrap().named_payment_outcomes.is_empty());assert!(game.stack.is_empty());
    }
}

#[test]
fn source_dragon_entry_trigger_uses_frozen_receipt_after_source_and_payer_depart() {
    for definition in definitions("msem-512") {
        let mut game=game();let body=ironsmith_compiler_runtime::compile_to_runtime_definition("Opponent body","Type: Creature\nPower/Toughness: 4/4",false).unwrap();let victim=game.create_object_from_definition(&body,C,Zone::Battlefield);
        let card=game.create_object_from_definition(&definition,A,Zone::Hand);let stable=game.object(card).unwrap().stable_id;let mut dm=Choices::new(false);cast(&mut game,card,&mut dm);
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();settle(&mut game,&mut dm);assert_eq!(game.stack.len(),1);
        let dragon=game.find_object_by_stable_id(stable).unwrap();game.move_object_by_effect(dragon,Zone::Graveyard).unwrap();game.player_mut(B).unwrap().has_left_game=true;
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();assert_eq!(game.damage_on(victim),3,"a completed failed-payment receipt does not depend on source or payer remaining in game");
    }
}
#[test]
fn source_granted_trial_failure_observers_use_actual_spell_type_and_success() {
    for definition in definitions("msem-513") {for creature in [false,true] {for pay in [false,true] {
        let mut game=game();game.create_object_from_definition(&definition,A,Zone::Battlefield);
        let body=ironsmith_compiler_runtime::compile_to_runtime_definition("Independent granted spell",if creature {"Type: Creature\nPower/Toughness: 1/1"}else{"Type: Instant\nYou gain 1 life."},false).unwrap();
        let card=game.create_object_from_definition(&body,A,Zone::Hand);let stable=game.object(card).unwrap().stable_id;let mut dm=Choices::new(pay);cast(&mut game,card,&mut dm);settle(&mut game,&mut dm);
        while game.stack.last().is_some_and(|entry|entry.is_ability) {ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();}
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();
        if creature {let permanent=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.counter_count(permanent,ironsmith::CounterType::PlusOnePlusOne),if pay {0}else{1});}
        let treasures=game.battlefield.iter().filter(|id|game.object(**id).is_some_and(|object|object.subtypes.contains(&ironsmith::Subtype::Treasure))).copied().collect::<Vec<_>>();
        assert_eq!(treasures.len(),if !creature && !pay {1}else{0});for treasure in treasures {assert!(game.is_tapped(treasure));}
    }}}
}

fn failure_count(events:&[ironsmith::triggers::TriggerEvent])->usize {
    events.iter().filter(|event|event.downcast::<ironsmith::events::NamedActionEvent>().is_some_and(|event|event.label=="fail trial")).count()
}
fn copy_trial_spell(game:&mut GameState,spell:ObjectId,dm:&mut Choices)->ObjectId {
    let outcome=ironsmith::effects::execute_effect(game,&ironsmith::Effect::copy_spell(ironsmith::target::ChooseSpec::SpecificObject(spell)),&mut ironsmith::effects::EffectContext::new(spell,A,dm)).unwrap();
    assert_eq!(failure_count(&outcome.events),0,"copying does not repeat a failed-payment action");
    for event in outcome.events {game.queue_trigger_event(event.provenance(),event);}
    assert_eq!(failure_count(&game.take_pending_trigger_events()),0,"copying publishes no new payment outcome");
    let copy=game.stack.last().unwrap().object_id;assert_ne!(copy,spell);assert_eq!(game.object(copy).unwrap().named_payment_outcomes,game.object(spell).unwrap().named_payment_outcomes);copy
}
#[test]
fn copied_trial_instant_keeps_paid_or_failed_casting_decision_without_repeating_payment() {
    for definition in definitions("msem-509") {for pay in [false,true] {
        let mut game=game();let card=game.create_object_from_definition(&definition,A,Zone::Hand);let stable=game.object(card).unwrap().stable_id;let mut dm=Choices::new(pay);cast(&mut game,card,&mut dm);
        let spell=game.find_object_by_stable_id(stable).unwrap();assert_eq!(failure_count(&game.take_pending_trigger_events()),usize::from(!pay));
        let life=game.player(B).unwrap().life;copy_trial_spell(&mut game,spell,&mut dm);assert_eq!(game.player(B).unwrap().life,life,"an opponent pays only for the original casting");
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();assert_eq!(game.battlefield.len(),if pay {1}else{3},"copy resolves with the original trial decision");assert_eq!(game.stack.len(),1);
    }}
}
#[test]
fn copied_trial_permanent_keeps_announcement_through_entry_but_ordinary_token_copy_does_not() {
    for definition in definitions("msem-511") {for pay in [false,true] {
        let mut game=game();let card=game.create_object_from_definition(&definition,A,Zone::Hand);let stable=game.object(card).unwrap().stable_id;let mut dm=Choices::new(pay);cast(&mut game,card,&mut dm);
        let spell=game.find_object_by_stable_id(stable).unwrap();assert_eq!(failure_count(&game.take_pending_trigger_events()),usize::from(!pay));let life=game.player(B).unwrap().life;
        let copy=copy_trial_spell(&mut game,spell,&mut dm);let copy_stable=game.object(copy).unwrap().stable_id;
        ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();settle(&mut game,&mut dm);while game.stack.last().is_some_and(|entry|entry.is_ability){ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut dm).unwrap();}
        let permanent=game.find_object_by_stable_id(copy_stable).unwrap();assert_eq!(game.object(permanent).unwrap().zone,Zone::Battlefield);assert_eq!(game.object(permanent).unwrap().named_payment_outcomes.len(),1);assert_eq!(game.current_has_static_ability_id(permanent,ironsmith_core::StaticAbilityId::Haste),!pay);assert_eq!(game.player(B).unwrap().life,life);
        let before=game.battlefield.clone();let outcome=ironsmith::effects::execute_effect(&mut game,&ironsmith::Effect::create_token_copy(ironsmith::target::ChooseSpec::SpecificObject(permanent)),&mut ironsmith::effects::EffectContext::new(permanent,A,&mut dm)).unwrap();for event in outcome.events{game.queue_trigger_event(event.provenance(),event);}settle(&mut game,&mut dm);
        let token=*game.battlefield.iter().find(|id|!before.contains(id)).unwrap();assert!(game.object(token).unwrap().named_payment_outcomes.is_empty(),"an ordinary permanent copy doesn't copy a casting announcement");assert!(!game.current_has_static_ability_id(token,ironsmith_core::StaticAbilityId::Haste));assert!(game.stack.last().is_some_and(|entry|!entry.is_ability),"ordinary token entry cannot use the source's failed trial");
    }}
}
