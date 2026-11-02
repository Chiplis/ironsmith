//! Generic source-free persistent player rules and exile-incarnation status.
use ironsmith::{GameState,PlayerId,Zone,Effect,ObjectId};
use ironsmith::effects::{EffectExecutor,EffectContext,ScheduleDelayedTriggerEffect};
use ironsmith::target::{ObjectFilter,PlayerFilter,ChooseSpec};
const A:PlayerId=PlayerId(0);const B:PlayerId=PlayerId(1);
fn object(game:&mut GameState,player:PlayerId,zone:Zone)->ObjectId {
 let definition=ironsmith_compiler_runtime::compile_to_runtime_definition("Independent creature","Type: Creature\nPower/Toughness: 2/2",false).unwrap();game.create_object_from_definition(&definition,player,zone)
}
fn filter()->ObjectFilter {
 let mut filter=ObjectFilter::default().in_zone(Zone::Exile).owned_by(PlayerFilter::You);filter.custom_statuses.push("charted".into());filter
}
fn declaration()->ScheduleDelayedTriggerEffect {
 let trigger=ironsmith::triggers::Trigger::from_delayed_trigger_spec(ironsmith_core::DelayedTriggerSpec::DealsCombatDamageToPlayer {source:ObjectFilter::creature().you_control(),player:PlayerFilter::Opponent});
 let mut effect=ScheduleDelayedTriggerEffect::new(trigger,vec![Effect::move_to_zone(ChooseSpec::Object(filter()),Zone::Hand,false)],false,Vec::new(),PlayerFilter::You);
 effect.persistent_player_rule=Some("charted card retrieval".into());effect.intervening_if=Some(ironsmith::effect::Condition::ValueComparison {left:ironsmith::effect::Value::Count(filter()),operator:ironsmith_core::ValueComparisonOperator::GreaterThanOrEqual,right:ironsmith::effect::Value::Fixed(1)});
 effect
}
fn register(game:&mut GameState,source:ObjectId,player:PlayerId,effect:&ScheduleDelayedTriggerEffect) {
 effect.execute(game,&mut EffectContext::new_default(source,player)).unwrap();
}
fn hit(source:ObjectId,player:PlayerId)->ironsmith::triggers::TriggerEvent {
 ironsmith::triggers::TriggerEvent::new_with_provenance(ironsmith::events::DamageEvent::with_cause(source,ironsmith::events::DamageTarget::Player(player),2,true,ironsmith::events::cause::EventCause::combat_damage(source)),Default::default())
}
#[test]
fn persistent_registration_is_player_scoped_and_not_multiplied_by_cards_or_sources() {
 let rule=declaration();let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=object(&mut game,A,Zone::Battlefield);let other=object(&mut game,A,Zone::Battlefield);
 for _ in 0..3 {register(&mut game,source,A,&rule);register(&mut game,other,A,&rule);}assert_eq!(game.effect_store.delayed_triggers.len(),1);
 let first=object(&mut game,A,Zone::Exile);let second=object(&mut game,A,Zone::Exile);assert!(game.apply_custom_status_in_zone(first,"charted",Zone::Exile));assert!(game.apply_custom_status_in_zone(second,"charted",Zone::Exile));
 game.move_object_by_effect(source,Zone::Graveyard).unwrap();game.move_object_by_effect(other,Zone::Graveyard).unwrap();
 let creature=object(&mut game,A,Zone::Battlefield);let another=object(&mut game,A,Zone::Battlefield);let opponent=object(&mut game,B,Zone::Battlefield);
 assert!(ironsmith::triggers::check_delayed_triggers(&mut game,&hit(opponent,A)).is_empty());
 let entries=ironsmith::triggers::check_delayed_triggers_for_simultaneous_events(&mut game,&[hit(creature,B),hit(another,B)]);assert_eq!(entries.len(),2,"one per creature, not one per marked card or registration action");
 for entry in &entries {assert_eq!(entry.controller,A);assert_eq!(entry.source_kind,ironsmith::triggers::TriggeredAbilitySourceKind::GameRule);assert!(entry.source_snapshot.is_none());assert!(entry.ability.intervening_if.is_some());}
 let mut queue=ironsmith::triggers::TriggerQueue::new();for entry in entries {queue.add(entry);}
 ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut ironsmith::decision::SelectFirstDecisionMaker).unwrap();assert_eq!(game.stack.len(),2);
 while !game.stack.is_empty(){ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut ironsmith::decision::SelectFirstDecisionMaker).unwrap();}
 assert_eq!(game.player(A).unwrap().hand.len(),2);assert_eq!(game.effect_store.delayed_triggers.len(),1);
 assert!(ironsmith::triggers::check_delayed_triggers(&mut game,&hit(creature,B)).is_empty(),"no marked card means no trigger");
}
#[test]
fn last_card_departure_invalidates_intervening_condition_and_exile_return_loses_mark() {
 let rule=declaration();let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=object(&mut game,A,Zone::Battlefield);register(&mut game,source,A,&rule);
 let card=object(&mut game,A,Zone::Exile);game.apply_custom_status_in_zone(card,"charted",Zone::Exile);
 let entries=ironsmith::triggers::check_delayed_triggers(&mut game,&hit(source,B));assert_eq!(entries.len(),1);let mut queue=ironsmith::triggers::TriggerQueue::new();for entry in entries {queue.add(entry);}
 ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut ironsmith::decision::SelectFirstDecisionMaker).unwrap();
 let grave=game.move_object_by_effect(card,Zone::Graveyard).unwrap();let returned=game.move_object_by_effect(grave,Zone::Exile).unwrap();assert!(!game.has_custom_status(returned,"charted"));assert!(!game.has_custom_status(card,"charted"));
 ironsmith::game_loop::resolve_stack_entry_with(&mut game,&mut ironsmith::decision::SelectFirstDecisionMaker).unwrap();assert!(game.player(A).unwrap().hand.is_empty());
 assert!(!game.apply_custom_status_in_zone(source,"charted",Zone::Exile),"exile-only designation cannot be applied to battlefield source");
}

#[test]
fn persistent_rule_controller_uses_damage_time_creature_control_after_control_change_or_departure() {
 for departed in [false,true] {
  let rule=declaration();let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=object(&mut game,A,Zone::Battlefield);register(&mut game,source,A,&rule);register(&mut game,source,B,&rule);
  for player in [A,B] {let marked=object(&mut game,player,Zone::Exile);game.apply_custom_status_in_zone(marked,"charted",Zone::Exile);}
  let snapshot=ironsmith::snapshot::ObjectSnapshot::from_object(game.object(source).unwrap(),&game);
  let event=hit(source,B).with_source_snapshot(snapshot);
  game.set_current_controller(source,B).unwrap();if departed {game.move_object_by_effect(source,Zone::Graveyard).unwrap();}
  let entries=ironsmith::triggers::check_delayed_triggers(&mut game,&event);assert_eq!(entries.len(),1);assert_eq!(entries[0].controller,A,"the creature's damage-time controller owns the source-free rule instance");
 }
}
#[test]
fn same_named_native_rule_requires_compatible_typed_trigger_and_proven_effect_identity() {
 let mut game=GameState::new(vec!["A".into(),"B".into()],20);let source=object(&mut game,A,Zone::Battlefield);let rule=declaration();register(&mut game,source,A,&rule);
 let mut other_trigger=rule.clone();other_trigger.trigger=ironsmith::triggers::Trigger::from_delayed_trigger_spec(ironsmith_core::DelayedTriggerSpec::DealsCombatDamageToPlayer {source:ObjectFilter::creature().you_control(),player:PlayerFilter::Any});
 assert!(matches!(other_trigger.execute(&mut game,&mut EffectContext::new_default(source,A)),Err(ironsmith::effects::ExecutionError::InternalError(_))));
 // An independently built native callback lacks a canonical model. Similar
 // Debug output cannot establish it as the same immutable instruction.
 assert!(matches!(declaration().execute(&mut game,&mut EffectContext::new_default(source,A)),Err(ironsmith::effects::ExecutionError::InternalError(_))));assert_eq!(game.effect_store.delayed_triggers.len(),1);
}
