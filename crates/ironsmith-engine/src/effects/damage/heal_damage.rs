//! Heal marked damage by composing removal with the keyword-action envelope.

#[cfg(test)]
use crate::effect::Effect;
use crate::effect::EffectOutcome;
use crate::effects::helpers::{resolve_bounded_nonnegative_u32, resolve_single_object_for_effect};
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::events::{Event, KeywordActionEvent, KeywordActionKind};
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
use crate::target::ChooseSpec;

pub use ironsmith_core::HealDamageEffect;

impl EffectExecutor for HealDamageEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        crate::effects::composition::execute_compound(game, ctx, |game, ctx| {
            let target = resolve_single_object_for_effect(game, ctx, &self.target)?;
            let Some(object) = game.object(target) else {
                return Ok(EffectOutcome::target_invalid());
            };
            let snapshot =
                ObjectSnapshot::from_object_with_calculated_characteristics(object, game);
            let marked = game.damage_on(target);
            let requested = match &self.amount {
                Some(amount) => resolve_bounded_nonnegative_u32(game, amount, ctx, marked)?,
                None => marked,
            };
            if ctx.decision_maker.awaiting_choice() || requested == 0 {
                return Ok(EffectOutcome::count(0));
            }
            // The healed permanent performs Heal; it is also its subject.
            let action = KeywordActionEvent::new(
                KeywordActionKind::Heal,
                snapshot.controller,
                target,
                requested,
            )
            .with_snapshot(Some(snapshot));
            crate::effects::composition::execute_keyword_action(
                game,
                ctx,
                Event::new_with_provenance(action, ctx.provenance),
                crate::effects::composition::KeywordActionOutput::Body,
                crate::effects::composition::KeywordActionAmount::BodyMagnitude,
                |game, ctx, action| {
                    let removed =
                        super::remove_marked_damage(game, ctx, action.source, Some(action.amount))?;
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(EffectOutcome::count(0));
                    }
                    let healed = removed.instruction_result().count_or_zero().max(0) as u32;
                    if healed == 0 {
                        return Ok(removed);
                    }
                    let mut completed = action.clone().with_amount(healed);
                    completed.snapshot = removed
                        .instruction_result()
                        .affected_object_memory()
                        .and_then(|objects| {
                            objects
                                .iter()
                                .find(|object| object.object_id == action.source)
                        })
                        .cloned()
                        .or_else(|| action.snapshot.clone());
                    let notification =
                        crate::effects::composition::complete_keyword_action(game, ctx, completed)?;
                    Ok(EffectOutcome::aggregate_with_primary_result(
                        removed.summary_projection(),
                        [removed, notification],
                    ))
                },
            )
        })
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
