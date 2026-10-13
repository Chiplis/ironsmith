//! Discover keyword action implementation.
//!
//! Discover N (701.55): Exile cards from the top of your library until you exile a
//! nonland card with mana value N or less. You may cast that card without paying
//! its mana cost or put it into your hand. Put the rest on the bottom of your
//! library in a random order.

use crate::effect::{Effect, EffectOutcome, OutcomeValue};
use crate::effects::CompletedEffectOutputs;
use crate::effects::EffectExecutor;
use crate::effects::consult_helpers::{
    LibraryBottomOrder, LibraryConsultMode,
};
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::{KeywordActionEvent, KeywordActionKind};
use crate::game_state::GameState;
use crate::tag::TagKey;
use crate::zone::Zone;
pub use ironsmith_core::DiscoverEffect;


/// Effect that resolves a discover action for a player.
impl EffectExecutor for DiscoverEffect {
    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) {
        super::runtime_helpers::visit_library_cast_program(visitor, self.player.clone(), LibraryConsultMode::Exile,
            crate::target::ObjectFilter::nonland().with_mana_value(crate::filter::Comparison::LessThanOrEqualExpr(Box::new(self.count.clone()))),
            crate::effects::ConsultTopOfLibraryStopRule::FirstMatch, "__discover_all", "__discover_match", true, LibraryBottomOrder::Random);
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        ctx.with_object_tags(["__discover_all", "__discover_match"].map(TagKey::from), |ctx| {
        let mut consultation = None;
        let instruction = crate::effects::composition::execute_transaction(
            game,
            ctx,
            || CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                let player_id = resolve_player_filter(game, &self.player, ctx)?;
                let count = resolve_value(game, &self.count, ctx)?.max(0) as u32;
                let program = super::library_cast_program::FirstMatchCastProgram {
                    player: player_id, all: "__discover_all".into(), matched: "__discover_match".into(),
                    limit: count, inclusive: true, source_name: None, fallback_to_hand: true,
                };
                let Some(receipt) = program.execute(game, ctx, |_, _, _| Ok(None))?
                else { return Ok(CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0))); };
                consultation = Some(receipt.consultation);
                let selected_object = receipt.selected;
                let casted_spell = receipt.cast;
                let mut phases = receipt.fallback.into_iter().collect::<Vec<_>>();
                let cleanup = receipt.cleanup;

                let value = if let Some(id) = selected_object {
                    OutcomeValue::Objects(vec![id])
                } else {
                    OutcomeValue::Count(0)
                };

                if ctx.decision_maker.awaiting_choice() {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                // Cast observations were captured at the successful cast, before cleanup.
                if let Some(outputs) = casted_spell {
                    phases.insert(0, outputs);
                }
                phases.push(cleanup);
                phases.push(CompletedEffectOutputs::aggregate_only(
                    crate::effects::composition::complete_keyword_action(
                        game,
                        ctx,
                        KeywordActionEvent::new(
                            KeywordActionKind::Discover,
                            player_id,
                            ctx.source,
                            count,
                        ),
                    )?,
                ));
                let mut primary = EffectOutcome::resolved();
                primary.set_value(value);
                let aggregate = EffectOutcome::aggregate_with_primary_result(
                    primary,
                    phases.iter().map(|outputs| outputs.outcome.clone()),
                );
                let mut outputs = CompletedEffectOutputs::aggregate_only(aggregate);
                outputs.retain_batch_children(phases);
                Ok(outputs)
            },
        );
        if ctx.decision_maker.awaiting_choice() {
            return instruction
                .map(|_| CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)));
        }
        instruction.map(|outputs| {
            if let Some(consult) = consultation {
                let observations = consult.attach_to_outputs(EffectOutcome::resolved());
                let aggregate = EffectOutcome::aggregate_with_primary_result(
                    outputs.outcome.clone(),
                    [observations.outcome.clone()],
                );
                let mut outputs = outputs.project_aggregate(aggregate);
                outputs.retain_batch_children([observations]);
                outputs
            } else {
                outputs
            }
        })

        })
    }
}

#[cfg(test)]
mod replacement_discover_hand_owner_contract_tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::DecisionMaker;
    use crate::effect::Value;
    use crate::ids::{CardId,ObjectId,PlayerId};
    use crate::mana::{ManaCost,ManaSymbol};
    use crate::object::CounterType;
    use crate::replacement::{ReplacementAction,ReplacementEffect};
    use crate::snapshot::ObjectSnapshot;
    use crate::target::{ChooseSpec,ObjectFilter};
    use crate::types::CardType;
    struct Answers {pause:bool,pending:bool,added:usize,binding:bool}
    impl DecisionMaker for Answers {
        fn decide_boolean(&mut self,game:&GameState,context:&crate::decisions::context::BooleanContext)->bool {
            if context.player==PlayerId::from_index(0) {return false;}
            self.added+=1;
            let hand=&game.player(PlayerId::from_index(0)).unwrap().hand;assert_eq!(hand.len(),1);
            assert_eq!(game.exile.len(),1,"hand additions precede remainder cleanup");assert!(game.player(PlayerId::from_index(0)).unwrap().library.is_empty());
            if self.binding {assert_eq!(game.counter_count(hand[0],CounterType::PlusOnePlusOne),1);}
            self.pending=self.pause;!self.pending
        }
        fn awaiting_choice(&self)->bool {self.pending}
    }
    fn card(game:&mut GameState,owner:PlayerId,kind:CardType,zone:Zone)->ObjectId {
        game.create_object_from_card(&CardBuilder::new(CardId::new(),"Discover fixture").card_types(vec![kind])
            .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(1)])).build(),owner,zone)
    }
    fn check(mode:u8) {
        let mut game=crate::tests::test_helpers::setup_two_player_game();let alice=PlayerId::from_index(0);let bob=PlayerId::from_index(1);
        let parent=card(&mut game,alice,CardType::Artifact,Zone::Battlefield);let source=card(&mut game,bob,CardType::Artifact,Zone::Battlefield);
        let spell=card(&mut game,alice,CardType::Instant,Zone::Library);let land=card(&mut game,alice,CardType::Land,Zone::Library);
        let sentinel=ObjectSnapshot::from_object(game.object(parent).unwrap(),&game);
        let actions=match mode {1=>vec![Effect::gain_life(3),Effect::lose_life(Value::X)],3=>vec![Effect::new(crate::effects::PutCountersEffect::new(CounterType::PlusOnePlusOne,1,ChooseSpec::tagged("it"))),Effect::new(crate::effects::composition::MayEffect::new_for_player(vec![Effect::gain_life(0)], crate::target::PlayerFilter::You))],_=>vec![Effect::gain_life(3),Effect::new(crate::effects::composition::MayEffect::new_for_player(vec![Effect::gain_life(4)], crate::target::PlayerFilter::You))]};
        let shield=game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source,bob,
            crate::events::zones::matchers::WouldChangeZoneMatcher::new(ObjectFilter::default(),Some(Zone::Exile),Some(Zone::Hand)),ReplacementAction::Additionally(actions)));
        game.take_pending_trigger_events();let ids=game.next_object_id_counter();let objects=game.objects_in_deterministic_order().len();
        let mut dm=Answers {pause:mode==2,pending:false,added:0,binding:mode==3};let mut ctx=ExecutionContext::new(parent,alice,&mut dm);ctx.set_tagged_objects("it",vec![sentinel.clone()]);
        let result=DiscoverEffect::you(3).execute(&mut game,&mut ctx);
        if mode==1 {assert!(matches!(result,Err(ExecutionError::UnresolvableValue(_))));}
        else if mode==2 {assert!(ctx.decision_maker.awaiting_choice());assert!(result.unwrap().events.is_empty());}
        else {
            let outcome=result.unwrap();let arrived=outcome.explicit_objects().unwrap();assert_eq!(arrived.len(),1);assert_eq!(game.object(arrived[0]).unwrap().zone,Zone::Hand);
            assert_eq!(game.player(alice).unwrap().life,20);assert_eq!(game.player(bob).unwrap().life,if mode==3 {20}else{27});
            assert!(game.exile.is_empty());assert!(game.stack.is_empty());assert_eq!(game.player(alice).unwrap().library.len(),1);
            if mode==3 {assert_eq!(game.counter_count(arrived[0],CounterType::PlusOnePlusOne),1);}
            else {assert_eq!(outcome.events.iter().filter_map(|event|event.downcast::<crate::events::LifeGainEvent>()).map(|event|(event.player,event.amount)).collect::<Vec<_>>(),vec![(bob,3),(bob,4)]);}
            assert_eq!(outcome.events.iter().filter(|event|event.downcast::<KeywordActionEvent>().is_some()).count(),1);
            assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
        }
        assert_eq!(ctx.source,parent);assert_eq!(ctx.controller,alice);assert_eq!(ctx.get_tagged_all("it").unwrap()[0].object_id,sentinel.object_id);
        if mode==1||mode==2 {
            assert_eq!(game.player(alice).unwrap().library,vec![spell,land]);assert!(game.player(alice).unwrap().hand.is_empty());assert!(game.exile.is_empty());
            assert_eq!(game.next_object_id_counter(),ids);assert_eq!(game.objects_in_deterministic_order().len(),objects);assert_eq!(game.player(bob).unwrap().life,20);
            assert!(game.effect_store.replacement_effects.get_effect(shield).is_some());assert!(game.take_pending_trigger_events().is_empty());
            assert!(ctx.get_tagged_all("__discover_all").is_none());assert!(ctx.get_tagged_all("__discover_match").is_none());
        }
        drop(ctx);if mode==0||mode==3 {assert_eq!(dm.added,1);}
        if mode==2 {
            assert_eq!(dm.added,1);dm.pause=false;dm.pending=false;let mut ctx=ExecutionContext::new(parent,alice,&mut dm);
            let outcome=DiscoverEffect::you(3).execute(&mut game,&mut ctx).unwrap();assert_eq!(outcome.explicit_objects().unwrap().len(),1);assert_eq!(game.player(bob).unwrap().life,27);
            assert!(!ctx.decision_maker.awaiting_choice());drop(ctx);assert_eq!(dm.added,2);
        }
    }
    #[test] fn additions_follow_hand_move_before_cleanup() {check(0);}
    #[test] fn error_restores_entire_discover_instruction() {check(1);}
    #[test] fn pending_replays_entire_discover_instruction() {check(2);}
    #[test] fn addition_binds_actual_hand_arrival() {check(3);}
}
