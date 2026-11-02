//! Historical object names gate global candidate spell prices, never the granting source alone.
#[path="custom_final_next_wave_support.rs"] mod support;
use ironsmith::{GameState,ObjectId,PlayerId,Zone};
use ironsmith::decision::{LegalAction,compute_legal_actions};
use ironsmith::effects::{EffectContext,execute_effect};
use ironsmith::target::{ChooseSpec,ObjectFilter,PlayerFilter};
use ironsmith::filter::ObjectFilterExt;
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
fn definition(name:&str,text:&str)->ironsmith::CardDefinition{ironsmith_compiler_runtime::compile_to_runtime_definition(name,text,false).unwrap()}
fn complete(game:&mut GameState,source:ObjectId,object:ObjectId,label:&str,player:PlayerId){
 let program=ironsmith::effects::NamedActionProgramEffect{completion:ironsmith_core::effect::NamedActionProgramCompletion::ProgramCompleted,objects:Some(ChooseSpec::SpecificObject(object)),label:label.into(),player:PlayerFilter::Specific(player),effects:vec![]};
 let outcome=execute_effect(game,&ironsmith::Effect::new(program),&mut EffectContext::new_default(source,player)).unwrap();
 assert!(outcome.events.iter().any(|event|event.downcast::<ironsmith::events::NamedActionEvent>().is_some()));
 for event in outcome.events{game.queue_trigger_event(event.provenance(),event);}
}
fn can_cast(game:&GameState,id:ObjectId,player:PlayerId)->bool{compute_legal_actions(game,player).unwrap().into_iter().any(|action|matches!(action,LegalAction::CastSpell{spell_id,..}if spell_id==id))}
#[test]
fn global_discount_uses_exact_completed_names_actor_and_turn_before_a_spell_is_cast(){
 let text="Type: Artifact\nSpells you cast sharing a name with a card from your sample forge action this turn cost {1} less to cast.";
 let card=serde_json::json!({"name":"Independent history price","executable_text":text});
 for discount in support::definitions(&card).unwrap(){
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.active_player=A;game.turn.priority_player=Some(A);game.turn.phase=ironsmith::game_state::Phase::FirstMain;game.turn.step=None;
  let observer=game.create_object_from_definition(&discount,A,Zone::Battlefield);
  let maker=game.create_object_from_definition(&definition("Independent performer","Type: Artifact"),A,Zone::Battlefield);
  let twin=definition("Historical Twin","Mana cost: {2}\nType: Sorcery\nYou gain 1 life.");
  let object=game.create_object_from_definition(&twin,A,Zone::Exile);let candidate=game.create_object_from_definition(&twin,A,Zone::Hand);
  game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless,1);
  assert!(!can_cast(&game,candidate,A),"history predicate must hold before the cast is proposed");
  complete(&mut game,maker,object,"sample forge",B);assert!(!can_cast(&game,candidate,A),"opponent's completed action is separate");
  complete(&mut game,maker,object,"different operation",A);assert!(!can_cast(&game,candidate,A),"other action labels are separate");
  // An event-shaped announcement alone has no actual completion authority.
  let snapshot=ironsmith::snapshot::ObjectSnapshot::from_object_id(&game,maker).unwrap();let named=ironsmith::snapshot::ObjectSnapshot::from_object_id(&game,object).unwrap();
  let fabricated=ironsmith::events::NamedActionEvent::new("sample forge",A,maker,snapshot,vec![named],1,ironsmith::events::cause::EventCause::effect());
  let event=ironsmith::triggers::TriggerEvent::new(fabricated,Default::default());game.queue_trigger_event(Default::default(),event);
  assert!(!can_cast(&game,candidate,A),"a bare event cannot counterfeit an operation receipt");
  complete(&mut game,maker,object,"sample forge",A);assert!(can_cast(&game,candidate,A),"global reduction matches the independent prospective spell");
  let object=game.move_object_by_game_rule(object,Zone::Graveyard).unwrap();game.object_mut(object).unwrap().name="Changed later".into();game.move_object_by_game_rule(maker,Zone::Graveyard).unwrap();
  assert!(can_cast(&game,candidate,A),"historical names survive participant and performer departure");
  let context=game.filter_context_for(A,Some(observer));let mut filter=ObjectFilter::default();filter.shares_name_with_named_action_objects=Some(ironsmith_core::NamedActionObjectNameCondition{label:"sample forge".into(),player:PlayerFilter::You,this_turn:true});
  let snapshot=ironsmith::snapshot::ObjectSnapshot::from_object_id(&game,candidate).unwrap();assert!(filter.matches_snapshot(&snapshot,&context,&game));
  game.turn_store.turn_history=Default::default();game.turn.turn_number+=1;
  assert!(!can_cast(&game,candidate,A),"the source's this-turn restriction expires at the turn boundary");
  assert!(!filter.matches_snapshot(&snapshot,&context,&game));filter.shares_name_with_named_action_objects.as_mut().unwrap().this_turn=false;assert!(filter.matches_snapshot(&snapshot,&context,&game),"explicit full-game history remains available");
 }
}
#[test]
fn retained_split_card_both_names_match_and_unrelated_names_do_not(){
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);
 let performer=game.create_object_from_definition(&definition("Independent performer","Type: Artifact"),A,Zone::Battlefield);
 let mut split=definition("First Name","Type: Instant\nYou gain 1 life.");split.card.linked_face_layout=ironsmith::card::LinkedFaceLayout::Split;split.card.other_face_name=Some("Second Name".into());
 let object=game.create_object_from_definition(&split,A,Zone::Hand);complete(&mut game,performer,object,"inscribe example",A);
 let candidate=game.create_object_from_definition(&definition("Second Name","Type: Instant\nYou gain 1 life."),A,Zone::Hand);
 let mut filter=ObjectFilter::default();filter.shares_name_with_named_action_objects=Some(ironsmith_core::NamedActionObjectNameCondition{label:"inscribe example".into(),player:PlayerFilter::You,this_turn:true});
 let context=game.filter_context_for(A,None);assert!(filter.matches(game.object(candidate).unwrap(),&context,&game));
 let other=game.create_object_from_definition(&definition("Unrelated Name","Type: Instant\nYou gain 1 life."),A,Zone::Hand);assert!(!filter.matches(game.object(other).unwrap(),&context,&game));
}
