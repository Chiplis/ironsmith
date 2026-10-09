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
    LibraryBottomOrder, LibraryConsultMode, LibraryConsultStopRule,
    execute_library_consult_with_outputs,
};
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::{KeywordActionEvent, KeywordActionKind};
use crate::game_state::GameState;
use crate::tag::TagKey;
use crate::target::PlayerFilter;
use crate::zone::Zone;
pub use ironsmith_core::DiscoverEffect;

use super::runtime_helpers::{
    cast_effect_driven_spell_without_paying, complete_effect_driven_cast_with_outputs,
    effect_driven_cast_options_for_card,
};

/// Effect that resolves a discover action for a player.
impl EffectExecutor for DiscoverEffect {
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
        let mut consultation = None;
        let instruction = crate::effects::composition::execute_transaction(
            game,
            ctx,
            || CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                let player_id = resolve_player_filter(game, &self.player, ctx)?;
                let count = resolve_value(game, &self.count, ctx)?.max(0) as u32;
                let all_tag = TagKey::from("__discover_all");
                let match_tag = TagKey::from("__discover_match");
                consultation = Some(execute_library_consult_with_outputs(
                    game,
                    ctx,
                    player_id,
                    LibraryConsultMode::Exile,
                    LibraryConsultStopRule::FirstMatch,
                    Some(&all_tag),
                    Some(&match_tag),
                    |card, _| {
                        if card.is_land() {
                            return false;
                        }
                        // CR 709.4: a split card's mana value is both halves' total.
                        (crate::filter::object_mana_value_for_filter(card).max(0) as u32) <= count
                    },
                )?);
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }

                let mut selected_object = None;
                let mut casted_spell = None;
                let mut phases = Vec::new();
                if let Some(candidate_snapshot) = ctx.get_tagged(match_tag.as_str()).cloned()
                    && let Some(candidate_obj) = game.object(candidate_snapshot.object_id)
                    && candidate_obj.zone == Zone::Exile
                {
                    let candidate_id = candidate_snapshot.object_id;

                    let candidate_name = candidate_obj.name.to_string();
                    let choice_ctx = crate::decisions::context::BooleanContext::new(
                        player_id,
                        Some(candidate_id),
                        format!("Cast {candidate_name} without paying its mana cost?"),
                    );
                    let should_cast = ctx.decision_maker.decide_boolean(game, &choice_ctx);
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::count(0),
                        ));
                    }

                    let mut put_in_hand = !should_cast;
                    if should_cast {
                        // CR 701.57a: any face whose resulting spell has mana value N
                        // or less may be cast (the other half of a split card, an
                        // Adventure, an MDFC back face).
                        let from_zone = candidate_obj.zone;
                        let filter = crate::target::ObjectFilter::nonland().with_mana_value(
                            crate::filter::Comparison::LessThanOrEqual(count as i32),
                        );
                        let options = effect_driven_cast_options_for_card(
                            game,
                            player_id,
                            ctx.source,
                            candidate_id,
                            from_zone,
                            &filter,
                        );
                        let option = match options.len() {
                            0 => None,
                            1 => options.into_iter().next(),
                            _ => {
                                let choices = options
                                    .into_iter()
                                    .map(|option| (option.label.clone(), option))
                                    .collect::<Vec<_>>();
                                let choice = crate::decisions::ask_choose_one(
                                    game,
                                    ctx.decision_maker,
                                    player_id,
                                    ctx.source,
                                    &choices,
                                );
                                if ctx.decision_maker.awaiting_choice() {
                                    return Ok(CompletedEffectOutputs::aggregate_only(
                                        EffectOutcome::count(0),
                                    ));
                                }
                                choice
                            }
                        };
                        let cast_result = match option {
                            Some(option) => cast_effect_driven_spell_without_paying(
                                game, ctx, player_id, &option,
                            )?,
                            None => None,
                        };
                        if let Some(result) = cast_result {
                            selected_object = Some(result.new_id);
                            casted_spell = Some(complete_effect_driven_cast_with_outputs(
                                EffectOutcome::resolved(),
                                game,
                                result,
                                player_id,
                                ctx.provenance,
                            )?);
                        } else if ctx.decision_maker.awaiting_choice() {
                            return Ok(CompletedEffectOutputs::aggregate_only(
                                EffectOutcome::count(0),
                            ));
                        } else {
                            // CR 701.57a: "If you don't cast it, put that card into
                            // your hand" — including when the cast attempt fails.
                            put_in_hand = true;
                        }
                    }
                    if put_in_hand
                        && game
                            .object(candidate_id)
                            .is_some_and(|object| object.zone == Zone::Exile)
                    {
                        let request = crate::effects::zones::PreparedZoneMove::capture(
                            game,
                            candidate_id,
                            Zone::Exile,
                            Zone::Hand,
                            ctx.cause.clone(),
                            None,
                        );
                        phases.push(crate::effects::zones::execute_zone_moves_with_outputs(
                            game,
                            ctx,
                            vec![request],
                            |game, _ctx, receipts| {
                                let arrivals = crate::effects::zones::movement_arrivals(
                                    game,
                                    candidate_id,
                                    &receipts[0].1,
                                );
                                selected_object = arrivals.into_iter().find(|id| {
                                    game.object(*id)
                                        .is_some_and(|object| object.zone == Zone::Hand)
                                });
                                Ok(selected_object
                                    .map(|id| EffectOutcome::with_objects(vec![id]))
                                    .unwrap_or_else(|| EffectOutcome::count(0)))
                            },
                        )?);
                        if ctx.decision_maker.awaiting_choice() {
                            return Ok(CompletedEffectOutputs::aggregate_only(
                                EffectOutcome::count(0),
                            ));
                        }
                    }
                }
                let keep_tagged = selected_object.as_ref().map(|_| match_tag.clone());
                let cleanup = crate::effects::execute_effect_with_outputs(
                    game,
                    &Effect::put_tagged_remainder_on_library_bottom(
                        all_tag,
                        keep_tagged,
                        LibraryBottomOrder::Random,
                        PlayerFilter::Specific(player_id),
                    ),
                    ctx,
                )?;

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
