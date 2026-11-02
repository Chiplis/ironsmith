// native-test-suite: helper-only
//! Player history reads actual completion receipts, independently of object performers.
#[path="custom_final_next_wave_support.rs"] mod support;
use ironsmith::{CardDefinition,Effect,GameState,ObjectId,PlayerId,Zone};
use ironsmith::effects::{EffectContext,execute_effect};
use ironsmith::target::{ChooseSpec,PlayerFilter};
use ironsmith_core::value_model::PlayerActionWindow;
pub(crate) const A:PlayerId=PlayerId(0);pub(crate) const B:PlayerId=PlayerId(1);
pub(crate) fn definition(name:&str,text:&str)->CardDefinition {support::definitions(&serde_json::json!({"name":name,"executable_text":text})).unwrap().into_iter().next().unwrap()}
pub(crate) fn complete(game:&mut GameState,source:ObjectId,player:PlayerId,label:&str,objects:&[ObjectId]) {
 let snapshots=objects.iter().map(|id|ironsmith::snapshot::ObjectSnapshot::from_object_id(game,*id).unwrap()).collect();
 let effect=Effect::new(ironsmith::effects::NamedActionProgramEffect{label:label.into(),player:PlayerFilter::Specific(player),completion:ironsmith_core::effect::NamedActionProgramCompletion::ProgramCompleted,objects:Some(ChooseSpec::Tagged("subjects".into())),effects:Vec::new()});
 let mut context=EffectContext::new_default(source,player);context.tagged_objects.insert("subjects".into(),snapshots);
 let outcome=execute_effect(game,&effect,&mut context).unwrap();
 let receipt=outcome.events.iter().find_map(|event|event.downcast::<ironsmith::events::NamedActionEvent>()).expect("completed operation, including a known empty set, retains its event");
 assert_eq!(receipt.amount,objects.len() as u32);
 assert_eq!(receipt.player,player);
 assert_eq!(receipt.combat_phase,matches!(game.turn.phase,ironsmith::game_state::Phase::Combat).then_some(game.turn_store.combat_phases_started_this_turn));
 for event in outcome.events {game.queue_trigger_event(event.provenance(),event);}
}
pub(crate) fn count(game:&GameState,player:PlayerId,label:&str,window:PlayerActionWindow,amount:u32)->u32 {game.player_named_action_count_with_minimum_amount(player,label,window,amount).unwrap()}
