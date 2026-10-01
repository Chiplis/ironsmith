//! Heal marked damage from a permanent (CR 701.69a).

use crate::effect::{Effect, EffectOutcome};
use crate::effects::helpers::{resolve_single_object_for_effect, resolve_value};
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::events::processing::{
    TraitEventResult, process_trait_event_with_execution_context,
};
use crate::events::{Event, KeywordActionEvent, KeywordActionKind};
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
use crate::target::ChooseSpec;
use crate::triggers::TriggerEvent;

pub use ironsmith_core::HealDamageEffect;

use crate::effects::composition::mechanic_actions::execute_keyword_action_replacement_effects;

impl EffectExecutor for HealDamageEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let checkpoint = game.clone();
        let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        let result = (|| -> Result<EffectOutcome, ExecutionError> {
        let target_id = resolve_single_object_for_effect(game, ctx, &self.target)?;
        let Some(target) = game.object(target_id) else {
            return Ok(EffectOutcome::target_invalid());
        };
        let snapshot = ObjectSnapshot::from_object_with_calculated_characteristics(target, game);
        let controller = snapshot.controller;
        let marked = game.damage_on(target_id);
        let requested = match &self.amount {
            Some(amount) => resolve_value(game, amount, ctx)?.max(0) as u32,
            None => marked,
        };
        let healed = marked.min(requested);
        if healed == 0 {
            return Ok(EffectOutcome::count(0));
        }

        let would_event = Event::new_with_provenance(
            KeywordActionEvent::new(KeywordActionKind::Heal, controller, target_id, healed)
                .with_snapshot(Some(snapshot.clone())),
            ctx.provenance,
        );
        let replacement_result = process_trait_event_with_execution_context(game, would_event, ctx)?;
        crate::effects::replacement::execute_event_expansion_with_bindings(game, ctx, replacement_result, |game, ctx, original| {
        match original {
            TraitEventResult::Replaced {
                effects, source, controller, context, ..
            } => {
                return execute_keyword_action_replacement_effects(
                    game, ctx, effects, source, controller, &context, Some(snapshot),
                );
            }
            TraitEventResult::Prevented => return Ok(EffectOutcome::prevented()),
            TraitEventResult::NeedsChoice { .. } | TraitEventResult::NeedsInteraction { .. } => {
                return Ok(EffectOutcome::count(0));
            }
            TraitEventResult::Proceed(_) | TraitEventResult::Modified(_) => {}
            TraitEventResult::Expanded { .. } => return Err(ExecutionError::InternalError("keyword commit received an unflattened result".into())),
        }

        game.set_damage_marked(target_id, marked - healed);
        let event = TriggerEvent::new_with_provenance(
            KeywordActionEvent::new(KeywordActionKind::Heal, controller, target_id, healed)
                .with_snapshot(Some(snapshot)),
            ctx.provenance,
        );
        Ok(EffectOutcome::count(healed as i32)
            .with_affected_objects_from_game(game, vec![target_id])
            .with_event(event))
        }, |_, context, _| {
            let action = crate::events::downcast_event::<KeywordActionEvent>(context.event.inner())
                .filter(|action| action.action == KeywordActionKind::Heal)
                .ok_or_else(|| ExecutionError::InternalError("heal addition captured an incompatible event".into()))?;
            let object_tags = action.snapshot.as_ref().map(|snapshot| vec![
                ("__it__".to_owned(), vec![snapshot.clone()]),
                ("it".to_owned(), vec![snapshot.clone()]),
            ]).unwrap_or_default();
            Ok(crate::effects::replacement::ReplacementProgramBindings { targets: None, object_tags })
        })
        })();
        let pending = ctx.decision_maker.awaiting_choice();
        if pending || result.is_err() {
            *game = checkpoint;
            context_checkpoint.restore(ctx);
        }
        if pending { return Ok(EffectOutcome::count(0)); }
        result
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "permanent with marked damage to heal"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::ids::{CardId, PlayerId};
    use crate::object::Object;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_damaged_permanent(
        marked: u32,
    ) -> (GameState, crate::ids::ObjectId, PlayerId, PlayerId) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::new(), "Heal Target")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(5, 5))
            .build();
        game.add_object(Object::from_card(id, &card, bob, Zone::Battlefield));
        game.mark_damage(id, marked);
        (game, id, alice, bob)
    }

    #[test]
    fn exact_heal_removes_only_the_requested_marked_damage() {
        let (mut game, target, alice, bob) = setup_damaged_permanent(5);
        let mut ctx = ExecutionContext::new_default(target, alice);
        let outcome = HealDamageEffect::exact(ChooseSpec::SpecificObject(target), 2)
            .execute(&mut game, &mut ctx)
            .expect("heal should resolve");

        assert_eq!(game.damage_on(target), 3);
        assert_eq!(outcome.count_or_zero(), 2);
        let event = outcome.events[0]
            .downcast::<KeywordActionEvent>()
            .expect("heal should emit a typed keyword-action event");
        assert_eq!(event.action, KeywordActionKind::Heal);
        assert_eq!(event.player, bob, "the permanent's controller heals it");
        assert_eq!(event.source, target);
        assert_eq!(event.amount, 2);
    }

    #[test]
    fn exact_heal_saturates_at_the_damage_that_is_actually_marked() {
        let (mut game, target, alice, _) = setup_damaged_permanent(2);
        let mut ctx = ExecutionContext::new_default(target, alice);
        let outcome = HealDamageEffect::exact(ChooseSpec::SpecificObject(target), 7)
            .execute(&mut game, &mut ctx)
            .expect("heal should resolve");

        assert_eq!(game.damage_on(target), 0);
        assert_eq!(outcome.count_or_zero(), 2);
        assert_eq!(
            outcome.events[0]
                .downcast::<KeywordActionEvent>()
                .expect("heal event")
                .amount,
            2
        );
    }

    #[test]
    fn is_healed_surface_removes_all_marked_damage() {
        let (mut game, target, alice, _) = setup_damaged_permanent(4);
        let mut ctx = ExecutionContext::new_default(target, alice);
        let outcome = HealDamageEffect::all(ChooseSpec::SpecificObject(target))
            .execute(&mut game, &mut ctx)
            .expect("heal should resolve");

        assert_eq!(game.damage_on(target), 0);
        assert_eq!(outcome.count_or_zero(), 4);
    }

    #[test]
    fn healing_when_no_damage_is_marked_is_a_no_op_without_an_action_event() {
        let (mut game, target, alice, _) = setup_damaged_permanent(0);
        let mut ctx = ExecutionContext::new_default(target, alice);
        let outcome = HealDamageEffect::all(ChooseSpec::SpecificObject(target))
            .execute(&mut game, &mut ctx)
            .expect("heal should resolve");

        assert_eq!(outcome.count_or_zero(), 0);
        assert!(outcome.events.is_empty());
    }
}


#[cfg(test)]
mod additional_contract_tests {
    use super::*;
    use crate::effect::{Effect,Value};
    use crate::ids::{CardId,PlayerId};
    use crate::object::CounterType;
    use crate::replacement::{ReplacementAction,ReplacementEffect};
    struct PauseAdded { pause:bool,pending:bool,questions:usize }
    impl crate::decision::DecisionMaker for PauseAdded {
        fn decide_boolean(&mut self,_game:&GameState,_ctx:&crate::decisions::context::BooleanContext)->bool {
            self.questions+=1;if self.pause { self.pending=true;false }else{true}
        }
        fn awaiting_choice(&self)->bool {self.pending}
    }
    fn check_additional_heal(mode:u8) {
        let mut game=crate::tests::test_helpers::setup_two_player_game();
        let alice=PlayerId::from_index(0);let bob=PlayerId::from_index(1);
        let card=crate::card::CardBuilder::new(CardId::new(),"Heal addition probe")
            .card_types(vec![crate::types::CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(4,4)).build();
        let target=game.create_object_from_card(&card,alice,crate::zone::Zone::Battlefield);
        let replacement_source=game.create_object_from_card(&card,bob,crate::zone::Zone::Battlefield);
        game.mark_damage(target,3);
        let effects=if mode==3 { vec![Effect::new(crate::effects::PutCountersEffect::new(CounterType::PlusOnePlusOne,1,ChooseSpec::tagged("it")))] }
        else {
            let mut effects=vec![Effect::gain_life(2)];
            if mode==1 {effects.push(Effect::lose_life(Value::X));}
            if mode==2 {effects.push(Effect::may(vec![Effect::gain_life(4)]));}
            effects.push(Effect::gain_life(8));effects
        };
        let shield=game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(replacement_source,bob,
            crate::events::WouldKeywordActionMatcher::new(KeywordActionKind::Heal,crate::target::ObjectFilter::default()),
            ReplacementAction::Additionally(effects)));
        game.take_pending_trigger_events();let before_id=game.next_object_id_counter();
        let parent_tag=ObjectSnapshot::from_object(game.object(replacement_source).unwrap(),&game);
        let effect=HealDamageEffect::exact(ChooseSpec::SpecificObject(target),2);
        let mut dm=PauseAdded {pause:mode==2,pending:false,questions:0};
        let mut ctx=ExecutionContext::new_default(target,alice).with_decision_maker(&mut dm);
        ctx.set_tagged_objects("it",vec![parent_tag.clone()]);
        let result=effect.execute(&mut game,&mut ctx);
        if mode==1 {assert!(matches!(result,Err(ExecutionError::UnresolvableValue(_))));}
        else {
            let outcome=result.unwrap();
            if mode==2 {assert!(ctx.decision_maker.awaiting_choice());assert!(outcome.events.is_empty());}
            else {
                assert_eq!(outcome.count_or_zero(),2);assert_eq!(game.damage_on(target),1);
                if mode==3 {
                    assert_eq!(game.object(target).unwrap().counters.get(&CounterType::PlusOnePlusOne),Some(&1));
                    assert!(!game.object(replacement_source).unwrap().counters.contains_key(&CounterType::PlusOnePlusOne));
                }else{assert_eq!(game.player(bob).unwrap().life,30);}
                let mut events=game.take_pending_trigger_events();events.extend(outcome.events);
                let heals=events.iter().filter_map(|event|event.downcast::<KeywordActionEvent>()).filter(|action|action.action==KeywordActionKind::Heal).collect::<Vec<_>>();
                assert_eq!(heals.len(),1);assert_eq!(heals[0].source,target);assert_eq!(heals[0].amount,2);
            }
        }
        assert_eq!(ctx.source,target);assert_eq!(ctx.controller,alice);
        assert_eq!(ctx.get_tagged_all("it").unwrap()[0].object_id,parent_tag.object_id);
        assert!(ctx.replacement.suppressed_replacement_effects.is_empty());drop(ctx);
        assert_eq!(game.player(alice).unwrap().life,20);assert_eq!(game.next_object_id_counter(),before_id);
        assert_eq!(game.effect_store.replacement_effects.get_effect(shield).is_some(),mode==1||mode==2);
        if mode==1||mode==2 {assert_eq!(game.damage_on(target),3);assert_eq!(game.player(bob).unwrap().life,20);assert!(game.take_pending_trigger_events().is_empty());}
        if mode==2 {
            assert_eq!(dm.questions,1);let mut replay=PauseAdded {pause:false,pending:false,questions:0};
            let mut ctx=ExecutionContext::new_default(target,alice).with_decision_maker(&mut replay);
            ctx.set_tagged_objects("it",vec![parent_tag]);let outcome=effect.execute(&mut game,&mut ctx).unwrap();drop(ctx);
            assert_eq!(replay.questions,1);assert_eq!(outcome.count_or_zero(),2);assert_eq!(game.damage_on(target),1);
            assert_eq!(game.player(bob).unwrap().life,34);assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
            let mut events=game.take_pending_trigger_events();events.extend(outcome.events);
            assert_eq!(events.iter().filter_map(|event|event.downcast::<crate::events::LifeGainEvent>()).map(|gain|gain.amount).collect::<Vec<_>>(),vec![2,4,8]);
            assert_eq!(events.iter().filter_map(|event|event.downcast::<KeywordActionEvent>()).filter(|action|action.action==KeywordActionKind::Heal).count(),1);
        }
    }
    #[test] fn additional_heal_preserves_primary_count_and_payload() {check_additional_heal(0);}
    #[test] fn additional_heal_error_restores_original_and_prefix() {check_additional_heal(1);}
    #[test] fn additional_heal_pending_restores_then_replays_once() {check_additional_heal(2);}
    #[test] fn additional_heal_binds_action_object_without_changing_parent_tag() {check_additional_heal(3);}
}
