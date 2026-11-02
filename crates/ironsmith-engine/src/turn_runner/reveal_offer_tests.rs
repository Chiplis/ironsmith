//! Runner decisions must suspend the reveal boundary, including nested choices.
use super::*;
use crate::{Ability,CardDefinitionBuilder,CardId,CardType,Zone};
use crate::static_abilities::{CompiledStaticAbility,StaticAbility};
fn fixture(nested:bool)->(GameState,TurnRunner,TriggerQueue,PlayerId,crate::ids::StableId){
 let mut game=crate::tests::test_helpers::setup_two_player_game();let player=PlayerId::from_index(0);game.turn.active_player=player;game.turn.turn_number=2;game.turn.phase=Phase::Beginning;game.turn.step=Some(Step::Draw);
 let watcher=CardDefinitionBuilder::new(CardId::new(),"Runner reveal watcher").card_types(vec![CardType::Artifact]).with_ability(Ability::static_ability(StaticAbility::reveal_first_card_you_draw_each_turn(false,false))).build();game.create_object_from_definition(&watcher,player,Zone::Battlefield);
 let mut effects=vec![crate::Effect::new(crate::effects::ExileEffect::with_spec(crate::ChooseSpec::Source))];if nested{effects.push(crate::Effect::may(vec![crate::Effect::gain_life(4)]));}
 let offer=StaticAbility::from_model(CompiledStaticAbility::reveal_action_offer("entry offer",vec![Zone::Hand],Some(1),Zone::Exile,effects,"Exile this revealed card"));
 let card=CardDefinitionBuilder::new(CardId::new(),"Revealed offer card").card_types(vec![CardType::Instant]).with_ability(Ability::static_ability(offer)).build();
 // Leave another library card to prove resuming never performs a second draw.
 let other=CardDefinitionBuilder::new(CardId::new(),"Unrelated library card").card_types(vec![CardType::Instant]).build();game.create_object_from_definition(&other,player,Zone::Library);let card=game.create_object_from_definition(&card,player,Zone::Library);let stable=game.object(card).unwrap().stable_id;
 game.take_pending_trigger_events();let mut runner=TurnRunner::new();runner.state=TurnState::Draw;(game,runner,TriggerQueue::new(),player,stable)
}
fn expect_boolean(action:TurnAction,player:PlayerId){let TurnAction::Decision(DecisionContext::Boolean(prompt))=action else{panic!("expected real reveal offer decision, got {action:?}")};assert_eq!(prompt.player,player);}
#[test]fn actual_draw_offer_decline_is_explicit_and_does_not_consume_quota(){
 let(mut game,mut runner,mut queue,player,stable)=fixture(false);expect_boolean(runner.advance(&mut game,&mut queue).unwrap(),player);assert!(runner.has_pending_replay_choice());assert_eq!(game.player(player).unwrap().library.len(),2);assert!(game.player(player).unwrap().hand.is_empty());
 runner.respond_boolean(false);let action=runner.advance(&mut game,&mut queue).unwrap();assert!(matches!(action,TurnAction::RunPriority),"{action:?}");let arrival=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(arrival).unwrap().zone,Zone::Hand);assert_eq!(game.player(player).unwrap().library.len(),1);assert!(!game.turn_store.turn_history.projected_records().any(|record|record.event.downcast::<crate::events::NamedActionEvent>().is_some_and(|event|event.label=="entry offer")));
}
#[test]fn accepted_draw_offer_and_nested_choice_commit_once_before_priority(){
 let(mut game,mut runner,mut queue,player,stable)=fixture(true);expect_boolean(runner.advance(&mut game,&mut queue).unwrap(),player);runner.respond_boolean(true);expect_boolean(runner.advance(&mut game,&mut queue).unwrap(),player);assert_eq!(game.player(player).unwrap().library.len(),2);assert!(game.player(player).unwrap().hand.is_empty());assert_eq!(game.player(player).unwrap().life,20);
 runner.respond_boolean(true);let action=runner.advance(&mut game,&mut queue).unwrap();assert!(matches!(action,TurnAction::RunPriority),"{action:?}");let arrival=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(arrival).unwrap().zone,Zone::Exile);assert_eq!(game.player(player).unwrap().library.len(),1);assert!(game.player(player).unwrap().hand.is_empty());assert_eq!(game.player(player).unwrap().life,24);assert_eq!(game.turn_store.turn_history.projected_records().filter(|record|record.event.downcast::<crate::events::NamedActionEvent>().is_some_and(|event|event.label=="entry offer")).count(),1);
}

#[test]fn explicit_queued_reveal_continuation_replays_acceptance_and_nested_choice_once(){
 let(mut game,mut runner,_,player,stable)=fixture(true);let library=game.find_object_by_stable_id(stable).unwrap();let hand=game.move_object_by_game_rule(library,Zone::Hand).unwrap();game.take_pending_trigger_events();
 let progress=runner.finish_turn_draw(&mut game,player,vec![hand],0).unwrap();let RunnerProgress::NeedsDecision(DecisionContext::Boolean(prompt))=progress else{panic!("queued reveal must request a real offer decision")};assert_eq!(prompt.player,player);assert_eq!(game.object(hand).unwrap().zone,Zone::Hand);assert!(runner.has_pending_replay_choice());
 runner.respond_boolean(true);let pending=runner.pending_draw_reveal.take().unwrap();let progress=runner.finish_pending_draw_reveal_choices(&mut game,pending).unwrap();assert!(matches!(progress,RunnerProgress::NeedsDecision(DecisionContext::Boolean(_))));assert_eq!(game.object(hand).unwrap().zone,Zone::Hand);assert_eq!(game.player(player).unwrap().life,20);
 runner.respond_boolean(true);let pending=runner.pending_draw_reveal.take().unwrap();let progress=runner.finish_pending_draw_reveal_choices(&mut game,pending).unwrap();let RunnerProgress::Complete(events)=progress else{panic!("accepted queued reveal must finish its original boundary")};assert_eq!(events.iter().filter(|event|event.downcast::<crate::events::CardRevealedEvent>().is_some()).count(),1);let arrival=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(arrival).unwrap().zone,Zone::Exile);assert_eq!(game.player(player).unwrap().life,24);assert!(!runner.has_pending_replay_choice());assert_eq!(game.turn_store.turn_history.projected_records().filter(|record|record.event.downcast::<crate::events::NamedActionEvent>().is_some_and(|event|event.label=="entry offer")).count(),1);
}
