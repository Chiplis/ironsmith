//! Haunt exile effect: exiles the source card and schedules a delayed trigger
//! to fire the haunt card's effects when the targeted (haunted) creature dies.

use crate::effects::zones::apply_zone_change_with_context_and_additional_effects;
use crate::effect::{Effect, EffectOutcome};
use crate::effects::EffectExecutor;
use crate::effects::delayed::trigger_queue::{DelayedTriggerConfig, queue_delayed_trigger};
use crate::effects::{ExecutionContext, ExecutionError, ResolvedTarget};
use crate::game_state::GameState;
use crate::triggers::Trigger;
use crate::zone::Zone;
pub type HauntExileEffect = ironsmith_core::HauntExileEffect<Effect>;

impl EffectExecutor for HauntExileEffect {
    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) {
        for effect in &self.haunt_effects {
            visitor(effect);
        }
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::resolved()); }
        let checkpoint = game.clone();
        let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        let instruction = (|| -> Result<EffectOutcome, ExecutionError> {
        // Get the target creature (the one being haunted) from resolved targets.
        let haunted_creature_id = ctx
            .targets
            .iter()
            .find_map(|t| {
                if let ResolvedTarget::Object(id) = t {
                    Some(*id)
                } else {
                    None
                }
            })
            .ok_or(ExecutionError::InvalidTarget)?;

        // Verify the haunted creature is still on the battlefield.
        if game
            .object(haunted_creature_id)
            .is_none_or(|obj| obj.zone != Zone::Battlefield)
        {
            return Ok(EffectOutcome::resolved());
        }

        // CR 702.55a: exile the haunt card from the graveyard. The dies (or
        // "put into a graveyard during its resolution") trigger's source is
        // the pre-move object; follow the zone change to the graveyard card.
        let Some(graveyard_card) = crate::effects::helpers::resolve_source_object_id(game, ctx)
            .filter(|&id| {
                game.object(id)
                    .is_some_and(|obj| obj.zone == Zone::Graveyard)
            })
        else {
            return Ok(EffectOutcome::resolved());
        };
        let additional = ctx.additional_replacement_effects_snapshot();
        let receipt = apply_zone_change_with_context_and_additional_effects(
            game, graveyard_card, Zone::Graveyard, Zone::Exile, ctx.cause.clone(), ctx, &additional,
        )?;
        if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::resolved()); }
        let arrivals = match &receipt.original {
            crate::events::processing::EventOutcome::Proceed(change) => change.new_object_ids.clone(),
            crate::events::processing::EventOutcome::Replaced => {
                let ids = game.take_zone_change_results(graveyard_card);
                if !ids.is_empty() { game.record_zone_change_results(graveyard_card, ids.clone()); }
                ids
            }
            _ => Vec::new(),
        };
        let arrivals = arrivals.into_iter().filter(|id| game.object(*id).is_some_and(|object| object.zone == Zone::Exile)).collect::<Vec<_>>();
        let memories = arrivals.iter().filter_map(|id| crate::effect::OutcomeObjectMemory::from_object_id(game, *id)).collect::<Vec<_>>();
        let original = EffectOutcome::resolved().with_affected_objects(arrivals.clone()).with_affected_object_memory(memories);
        if let Some(exiled_id) = arrivals.first().copied()
            && let Some(exiled_snapshot) = game.object(exiled_id).map(|object| crate::snapshot::ObjectSnapshot::from_object(object, game))
        {
        let haunting_tag = crate::tag::TagKey::from("__haunting_card");

        // Schedule a one-shot delayed trigger: when the haunted creature dies,
        // execute the haunt card's effects. It only functions while the card
        // is still in exile haunting that creature (CR 702.55c).
        let mut config = DelayedTriggerConfig::new(
            Trigger::this_dies(),
            self.haunt_effects.clone(),
            true, // one-shot
            vec![haunted_creature_id],
            ctx.controller,
        )
        .with_ability_source(Some(exiled_id))
        .with_choices(self.haunt_choices.clone())
        .with_tagged_objects(std::collections::HashMap::from([(
            haunting_tag.clone(),
            vec![exiled_snapshot],
        )]));
        config.while_any_tagged_object_in_zone = Some((haunting_tag, Zone::Exile));
        queue_delayed_trigger(game, config);
        }
        // Authored haunting registration precedes any additional replacement program.
        crate::effects::zones::finish_zone_change_receipts(game, ctx, original, vec![(graveyard_card, receipt)])
        })();
        let pending = ctx.decision_maker.awaiting_choice();
        if pending || instruction.is_err() { *game = checkpoint; context_checkpoint.restore(ctx); }
        if pending { return instruction.map(|_| EffectOutcome::resolved()); }
        instruction
    }
}

#[cfg(test)]
mod replacement_haunt_owner_contract_tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::DecisionMaker;
    use crate::effect::Value;
    use crate::ids::{CardId,ObjectId,PlayerId};
    use crate::object::CounterType;
    use crate::replacement::{ReplacementAction,ReplacementEffect};
    use crate::snapshot::ObjectSnapshot;
    use crate::target::{ChooseSpec,ObjectFilter};
    use crate::types::CardType;
    struct Answers {pause:bool,pending:bool,calls:usize,binding:bool,target:ObjectId}
    impl DecisionMaker for Answers {
        fn decide_boolean(&mut self,game:&GameState,_:&crate::decisions::context::BooleanContext)->bool {
            self.calls+=1;assert_eq!(game.effect_store.delayed_triggers.len(),1,"authored haunting precedes additions");
            let delayed=&game.effect_store.delayed_triggers[0];assert_eq!(delayed.controller,PlayerId::from_index(0));assert_eq!(delayed.target_objects,vec![self.target]);
            let id=delayed.ability_source.unwrap();assert_eq!(game.object(id).unwrap().zone,Zone::Exile);
            if self.binding {assert_eq!(game.counter_count(id,CounterType::PlusOnePlusOne),1);}
            self.pending=self.pause;!self.pending
        }
        fn awaiting_choice(&self)->bool {self.pending}
    }
    fn card(game:&mut GameState,owner:PlayerId,zone:Zone)->ObjectId {
        game.create_object_from_card(&CardBuilder::new(CardId::new(),"Haunt fixture").card_types(vec![CardType::Creature]).build(),owner,zone)
    }
    fn check(mode:u8) {
        let mut game=crate::tests::test_helpers::setup_two_player_game();let alice=PlayerId::from_index(0);let bob=PlayerId::from_index(1);
        let parent=card(&mut game,alice,Zone::Graveyard);let target=card(&mut game,alice,Zone::Battlefield);let source=card(&mut game,bob,Zone::Battlefield);
        let sentinel=ObjectSnapshot::from_object(game.object(target).unwrap(),&game);
        let actions=match mode {1=>vec![Effect::gain_life(3),Effect::lose_life(Value::X)],3=>vec![Effect::new(crate::effects::PutCountersEffect::new(CounterType::PlusOnePlusOne,1,ChooseSpec::tagged("it"))),Effect::may(vec![Effect::gain_life(0)])],_=>vec![Effect::gain_life(3),Effect::may(vec![Effect::gain_life(4)])]};
        let shield=game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source,bob,
            crate::events::zones::matchers::WouldChangeZoneMatcher::new(ObjectFilter::default(),Some(Zone::Graveyard),Some(Zone::Exile)),ReplacementAction::Additionally(actions)));
        game.take_pending_trigger_events();let ids=game.next_object_id_counter();let objects=game.objects_in_deterministic_order().len();
        let mut dm=Answers {pause:mode==2,pending:false,calls:0,binding:mode==3,target};let mut ctx=ExecutionContext::new(parent,alice,&mut dm).with_targets(vec![ResolvedTarget::Object(target)]);ctx.set_tagged_objects("it",vec![sentinel.clone()]);
        let effect=HauntExileEffect::new(vec![Effect::gain_life(1)],vec![]);let result=effect.execute(&mut game,&mut ctx);
        if mode==1 {assert!(matches!(result,Err(ExecutionError::UnresolvableValue(_))));}
        else if mode==2 {assert!(ctx.decision_maker.awaiting_choice());assert!(result.unwrap().events.is_empty());}
        else {
            let outcome=result.unwrap();assert_eq!(game.effect_store.delayed_triggers.len(),1);let arrival=game.effect_store.delayed_triggers[0].ability_source.unwrap();assert_eq!(game.object(arrival).unwrap().zone,Zone::Exile);
            assert_eq!(game.player(alice).unwrap().life,20);assert_eq!(game.player(bob).unwrap().life,if mode==3 {20}else{27});
            if mode==3 {assert_eq!(game.counter_count(arrival,CounterType::PlusOnePlusOne),1);}
            else {assert_eq!(outcome.events.iter().filter_map(|event|event.downcast::<crate::events::LifeGainEvent>()).map(|event|(event.player,event.amount)).collect::<Vec<_>>(),vec![(bob,3),(bob,4)]);}
            assert!(outcome.affected_object_memory().unwrap().iter().any(|memory|memory.object_id==arrival&&memory.zone==Zone::Exile));
            assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
        }
        assert_eq!(ctx.source,parent);assert_eq!(ctx.controller,alice);assert_eq!(ctx.targets,vec![ResolvedTarget::Object(target)]);assert_eq!(ctx.get_tagged_all("it").unwrap()[0].object_id,sentinel.object_id);
        assert_eq!(game.counter_count(target,CounterType::PlusOnePlusOne),0);
        if mode==1||mode==2 {
            assert_eq!(game.object(parent).unwrap().zone,Zone::Graveyard);assert!(game.exile.is_empty());assert!(game.effect_store.delayed_triggers.is_empty());
            assert_eq!(game.next_object_id_counter(),ids);assert_eq!(game.objects_in_deterministic_order().len(),objects);assert_eq!(game.player(bob).unwrap().life,20);
            assert!(game.effect_store.replacement_effects.get_effect(shield).is_some());assert!(game.take_pending_trigger_events().is_empty());
        }
        drop(ctx);if mode==0||mode==3 {assert_eq!(dm.calls,1);}
        if mode==2 {
            assert_eq!(dm.calls,1);dm.pause=false;dm.pending=false;let mut ctx=ExecutionContext::new(parent,alice,&mut dm).with_targets(vec![ResolvedTarget::Object(target)]);
            effect.execute(&mut game,&mut ctx).unwrap();assert_eq!(game.player(bob).unwrap().life,27);assert_eq!(game.effect_store.delayed_triggers.len(),1);
            assert!(!ctx.decision_maker.awaiting_choice());drop(ctx);assert_eq!(dm.calls,2);
        }
    }
    #[test] fn additions_follow_authored_haunting_registration() {check(0);}
    #[test] fn error_restores_exile_and_delayed_registration() {check(1);}
    #[test] fn pending_replays_exile_and_delayed_registration() {check(2);}
    #[test] fn addition_binds_actual_haunting_arrival() {check(3);}
}
