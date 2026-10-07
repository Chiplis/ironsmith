//! Cascade keyword effect implementation.
//!
//! Exiles cards from the top of your library until a nonland card with lesser
//! mana value is exiled, lets you cast it without paying its mana cost, then
//! puts all other exiled cards on the bottom of your library in random order.

use crate::effects::CompletedEffectOutputs;
use crate::effect::{Effect, EffectOutcome};
use crate::effects::EffectExecutor;
use crate::effects::consult_helpers::{
    LibraryBottomOrder, LibraryConsultMode, LibraryConsultStopRule, execute_library_consult_with_outputs,
};
use crate::effects::zones::{
    BattlefieldEntryOptions, BattlefieldEntryOutcome, move_to_battlefield_with_options,
};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::mana::{ManaCost, ManaSymbol};
use crate::tag::TagKey;
use crate::target::PlayerFilter;
use crate::zone::Zone;

use super::runtime_helpers::{
    cast_effect_driven_spell_without_paying, effect_driven_cast_options_for_card,
    with_spell_cast_event,
};

/// Effect that resolves a single cascade trigger.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CascadeEffect;

impl CascadeEffect {
    /// Create a new cascade effect.
    pub fn new() -> Self {
        Self
    }
}

fn mana_value_on_stack(cost: Option<&ManaCost>, x_value: Option<u32>) -> u32 {
    let Some(cost) = cost else {
        return 0;
    };
    let x = x_value.unwrap_or(0);
    let x_pips = cost
        .pips()
        .iter()
        .filter(|pip| pip.iter().any(|symbol| matches!(symbol, ManaSymbol::X)))
        .count() as u32;
    cost.mana_value() + x_pips.saturating_mul(x)
}

fn controller_has_cascade_land_drop(game: &GameState, controller: crate::ids::PlayerId) -> bool {
    let view = crate::derived_view::DerivedGameView::from_refreshed_state(game);
    game.permanents_controlled_by(controller)
        .into_iter()
        .any(|permanent| {
            view.static_abilities_rc(permanent)
                .is_some_and(|abilities| {
                    abilities.iter().any(|ability| {
                        ability.id() == crate::static_abilities::StaticAbilityId::CascadeLandDrop
                    })
                })
        })
}

fn cascade_exiled_land_options(
    game: &GameState,
    ctx: &ExecutionContext,
    all_tag: &TagKey,
) -> Vec<(String, crate::ids::ObjectId)> {
    let Some(snapshots) = ctx.get_tagged_all(all_tag.as_str()) else {
        return Vec::new();
    };
    snapshots
        .iter()
        .filter_map(|snapshot| {
            let object_id = snapshot.object_id;
            let object = game.object(object_id)?;
            (object.zone == Zone::Exile && object.is_land())
                .then(|| (object.name.to_string(), object_id))
        })
        .collect()
}

fn maybe_put_cascade_land_onto_battlefield(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    all_tag: &TagKey,
    source_name: &str,
) -> Result<Option<CompletedEffectOutputs>, ExecutionError> {
    if !controller_has_cascade_land_drop(game, ctx.controller) {
        return Ok(None);
    }

    let options = cascade_exiled_land_options(game, ctx, all_tag);
    if options.is_empty() {
        return Ok(None);
    }

    let choice_ctx = crate::decisions::context::BooleanContext::new(
        ctx.controller,
        Some(ctx.source),
        "Put a land card exiled with cascade onto the battlefield tapped?".to_string(),
    )
    .with_source_name(source_name);
    let should_put_land = ctx.decision_maker.decide_boolean(game, &choice_ctx);
    if ctx.decision_maker.awaiting_choice() || !should_put_land {
        return Ok(None);
    }

    let chosen_id = if options.len() == 1 {
        options[0].1
    } else {
        let Some(chosen) = crate::decisions::ask_choose_one(
            game,
            ctx.decision_maker,
            ctx.controller,
            ctx.source,
            &options,
        ) else {
            return Ok(None);
        };
        chosen
    };

    let entry = move_to_battlefield_with_options(
        game,
        ctx,
        chosen_id,
        BattlefieldEntryOptions::specific(ctx.controller, true),
    )?;
    if ctx.decision_maker.awaiting_choice() {
        return Ok(None);
    }
    let entry = entry.ok_or_else(|| {
        ExecutionError::InternalError(
            "cascade land entry lost its receipt without pending input".into(),
        )
    })?;
    let original = match &entry.outcome {
        BattlefieldEntryOutcome::Moved(new_id) => EffectOutcome::with_objects(vec![*new_id]),
        BattlefieldEntryOutcome::Redirected(change) => {
            EffectOutcome::with_objects(change.new_object_ids.clone())
        }
        BattlefieldEntryOutcome::Prevented => EffectOutcome::count(0),
    };
    // Remove precisely the departed incarnation before the added program runs.
    if !matches!(&entry.outcome, BattlefieldEntryOutcome::Prevented)
        && let Some(tagged) = ctx.tagged_objects.get_mut(all_tag)
    {
        tagged.retain(|snapshot| snapshot.object_id != chosen_id);
    }
    crate::effects::zones::finish_battlefield_entry_receipts_with_outputs(
        game,
        ctx,
        original,
        vec![entry],
    )
    .map(Some)
}

impl EffectExecutor for CascadeEffect {
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
        let instruction = crate::effects::composition::execute_transaction(
            game,
            ctx,
            || CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                let (source_mana_value, source_name) =
                    if let Some(source_obj) = game.object(ctx.source) {
                        (
                            mana_value_on_stack(
                                source_obj.mana_cost.as_deref(),
                                ctx.x_value.or(source_obj.x_value),
                            ),
                            source_obj.name.to_string(),
                        )
                    } else if let Some(snapshot) = ctx.source_snapshot.as_ref() {
                        (
                            mana_value_on_stack(
                                snapshot.mana_cost.as_ref(),
                                ctx.x_value.or(snapshot.x_value),
                            ),
                            snapshot.name.to_string(),
                        )
                    } else {
                        return Ok(CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::target_invalid(),
                        ));
                    };
                let all_tag = TagKey::from("__cascade_all");
                let match_tag = TagKey::from("__cascade_match");
                let consult = execute_library_consult_with_outputs(
                    game,
                    ctx,
                    ctx.controller,
                    LibraryConsultMode::Exile,
                    LibraryConsultStopRule::FirstMatch,
                    Some(&all_tag),
                    Some(&match_tag),
                    |card, _| {
                        if card.is_land() {
                            return false;
                        }
                        // CR 709.4: a split card's mana value is both halves' total.
                        (crate::filter::object_mana_value_for_filter(card).max(0) as u32)
                            < source_mana_value
                    },
                )?;

                if ctx.decision_maker.awaiting_choice() {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                let mut observations = vec![consult.attach_to_outputs(EffectOutcome::resolved())];
                let cascade_land =
                    maybe_put_cascade_land_onto_battlefield(game, ctx, &all_tag, &source_name)?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                let cascade_land_ids = cascade_land
                    .as_ref()
                    .and_then(|outcome| outcome.outcome.explicit_objects())
                    .unwrap_or(&[])
                    .to_vec();
                if let Some(outcome) = cascade_land {
                    observations.push(outcome);
                }

                let mut casted_card = None;
                let mut cast_outcome = None;
                if let Some(candidate_snapshot) = ctx.get_tagged(match_tag.as_str()).cloned()
                    && let Some(candidate_obj) = game.object(candidate_snapshot.object_id)
                    && candidate_obj.zone == Zone::Exile
                {
                    let candidate_id = candidate_snapshot.object_id;
                    let candidate_name = candidate_obj.name.to_string();
                    let choice_ctx = crate::decisions::context::BooleanContext::new(
                        ctx.controller,
                        Some(candidate_id),
                        format!("Cast {candidate_name} without paying its mana cost?"),
                    )
                    .with_source_name(&source_name);
                    let should_cast = ctx.decision_maker.decide_boolean(game, &choice_ctx);
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::count(0),
                        ));
                    }
                    if should_cast {
                        let filter = crate::target::ObjectFilter::nonland().with_mana_value(
                            crate::filter::Comparison::LessThan(source_mana_value as i32),
                        );
                        let options = effect_driven_cast_options_for_card(
                            game,
                            ctx.controller,
                            ctx.source,
                            candidate_id,
                            Zone::Exile,
                            &filter,
                        );
                        let option = if options.len() == 1 {
                            Some(options[0].clone())
                        } else if options.len() > 1 {
                            let choices = options
                                .iter()
                                .cloned()
                                .map(|option| (option.label.clone(), option))
                                .collect::<Vec<_>>();
                            crate::decisions::ask_choose_one(
                                game,
                                ctx.decision_maker,
                                ctx.controller,
                                ctx.source,
                                &choices,
                            )
                        } else {
                            None
                        };
                        if ctx.decision_maker.awaiting_choice() {
                            return Ok(CompletedEffectOutputs::aggregate_only(
                                EffectOutcome::count(0),
                            ));
                        }
                        if let Some(option) = option
                            && let Some(result) = cast_effect_driven_spell_without_paying(
                                game,
                                ctx,
                                ctx.controller,
                                &option,
                            )?
                        {
                            casted_card = Some((candidate_id, result.new_id, result.from_zone));
                            // Capture cast observations before remainder movement can change the stack object.
                            cast_outcome = Some(with_spell_cast_event(
                                EffectOutcome::with_objects(vec![result.new_id]),
                                game,
                                result.new_id,
                                ctx.controller,
                                result.from_zone,
                                ctx.provenance,
                            )?);
                        }
                        if ctx.decision_maker.awaiting_choice() {
                            return Ok(CompletedEffectOutputs::aggregate_only(
                                EffectOutcome::count(0),
                            ));
                        }
                    }
                }
                let keep_tagged = casted_card.as_ref().map(|_| match_tag.clone());
                let cleanup = crate::effects::execute_effect_with_outputs(
                    game,
                    &Effect::put_tagged_remainder_on_library_bottom(
                        all_tag,
                        keep_tagged,
                        LibraryBottomOrder::Random,
                        PlayerFilter::You,
                    ),
                    ctx,
                )?;

                if ctx.decision_maker.awaiting_choice() {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                let primary = if let Some(outcome) = cast_outcome {
                    let primary = EffectOutcome::with_objects(
                        outcome.explicit_objects().unwrap_or(&[]).to_vec(),
                    );
                    observations.push(CompletedEffectOutputs::aggregate_only(outcome));
                    primary
                } else if !cascade_land_ids.is_empty() {
                    EffectOutcome::with_objects(cascade_land_ids)
                } else {
                    EffectOutcome::count(0)
                };
                observations.push(cleanup);
                let mut outcome = EffectOutcome::aggregate(
                    observations.iter().map(|outputs| outputs.outcome.clone()),
                );
                outcome.status = primary.status;
                outcome.value = primary.value;
                let mut outputs = CompletedEffectOutputs::aggregate_only(outcome);
                outputs.retain_batch_children(observations);
                Ok(outputs)
            },
        );
        if ctx.decision_maker.awaiting_choice() {
            return instruction
                .map(|_| CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)));
        }
        instruction
    }
}

#[cfg(test)]
mod replacement_cascade_owner_contract_tests {
    use super::*;
    use crate::ability::Ability;
    use crate::card::CardBuilder;
    use crate::decision::DecisionMaker;
    use crate::effect::Value;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::object::CounterType;
    use crate::replacement::{ReplacementAction, ReplacementEffect};
    use crate::snapshot::ObjectSnapshot;
    use crate::static_abilities::StaticAbility;
    use crate::target::{ChooseSpec, ObjectFilter};
    use crate::types::CardType;
    struct Answers { pause: bool, pending: bool, land: bool, binding: bool, added_calls: usize }
    impl DecisionMaker for Answers {
        fn decide_boolean(&mut self, game: &GameState, context: &crate::decisions::context::BooleanContext) -> bool {
            if !self.land { self.land = true; return true; }
            let lands = game.battlefield.iter().copied().filter(|id| game.object(*id).unwrap().is_land()).collect::<Vec<_>>();
            assert_eq!(lands.len(), 1);
            // The matching spell has not been cast during the land addition.
            if context.player == PlayerId::from_index(1) {
                self.added_calls += 1;
                assert!(game.stack.is_empty());
                assert_eq!(game.exile.len(), 1);
                if self.binding { assert_eq!(game.counter_count(lands[0], CounterType::PlusOnePlusOne), 1); }
                self.pending = self.pause;
                return !self.pending;
            }
            false
        }
        fn awaiting_choice(&self) -> bool { self.pending }
    }
    fn card(game: &mut GameState, owner: PlayerId, kind: CardType, zone: Zone) -> ObjectId {
        game.create_object_from_card(&CardBuilder::new(CardId::new(), "Cascade fixture")
            .card_types(vec![kind]).mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(1)])).build(), owner, zone)
    }
    fn check(mode: u8) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
        let definition = crate::CardDefinitionBuilder::new(CardId::new(), "Cascade source")
            .card_types(vec![CardType::Artifact]).mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(4)]))
            .with_ability(Ability::static_ability(StaticAbility::cascade_land_drop())).build();
        let parent = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let source = card(&mut game, bob, CardType::Artifact, Zone::Battlefield);
        let spell = card(&mut game, alice, CardType::Instant, Zone::Library);
        let land = card(&mut game, alice, CardType::Land, Zone::Library);
        game.refresh_continuous_state();
        let sentinel = ObjectSnapshot::from_object(game.object(parent).unwrap(), &game);
        let effects = match mode {
            1 => vec![Effect::gain_life(3), Effect::lose_life(Value::X)],
            3 => vec![Effect::new(crate::effects::PutCountersEffect::new(CounterType::PlusOnePlusOne, 1, ChooseSpec::tagged("it"))), Effect::new(crate::effects::composition::MayEffect::new_for_player(vec![Effect::gain_life(0)], crate::target::PlayerFilter::You))],
            _ => vec![Effect::gain_life(3), Effect::new(crate::effects::composition::MayEffect::new_for_player(vec![Effect::gain_life(4)], crate::target::PlayerFilter::You))],
        };
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source, bob,
            crate::events::zones::matchers::WouldChangeZoneMatcher::new(ObjectFilter::default().with_type(CardType::Land), Some(Zone::Exile), Some(Zone::Battlefield)), ReplacementAction::Additionally(effects)));
        game.take_pending_trigger_events(); let ids = game.next_object_id_counter(); let objects = game.objects_in_deterministic_order().len();
        let mut dm = Answers {pause: mode == 2, pending: false, land: false, binding: mode == 3, added_calls: 0};
        let mut ctx = ExecutionContext::new(parent, alice, &mut dm); ctx.set_tagged_objects("it", vec![sentinel.clone()]);
        let result = CascadeEffect::new().execute(&mut game, &mut ctx);
        if mode == 1 { assert!(matches!(result, Err(ExecutionError::UnresolvableValue(_)))); }
        else if mode == 2 { assert!(ctx.decision_maker.awaiting_choice()); assert!(result.unwrap().events.is_empty()); }
        else {
            let outcome = result.unwrap(); let arrivals = outcome.explicit_objects().unwrap(); assert_eq!(arrivals.len(), 1);
            let arrival = arrivals[0]; assert!(game.battlefield.contains(&arrival));
            assert_eq!(game.player(alice).unwrap().life, 20); assert_eq!(game.player(bob).unwrap().life, if mode == 3 {20} else {27});
            assert_eq!(game.player(alice).unwrap().lands_played_this_turn, 0);
            assert!(game.stack.is_empty()); assert!(game.exile.is_empty()); assert_eq!(game.player(alice).unwrap().library.len(), 1);
            if mode == 3 { assert_eq!(game.counter_count(arrival, CounterType::PlusOnePlusOne), 1); }
            else { assert_eq!(outcome.events.iter().filter_map(|event| event.downcast::<crate::events::LifeGainEvent>()).map(|event| (event.player,event.amount)).collect::<Vec<_>>(),vec![(bob,3),(bob,4)]); }
            assert!(outcome.affected_object_memory().unwrap().iter().any(|memory| memory.zone == Zone::Exile));
            assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
        }
        assert_eq!(ctx.source,parent); assert_eq!(ctx.controller,alice); assert_eq!(ctx.get_tagged_all("it").unwrap()[0].object_id,sentinel.object_id);
        assert_eq!(game.counter_count(parent,CounterType::PlusOnePlusOne),0);
        if mode == 1 || mode == 2 {
            assert_eq!(game.player(alice).unwrap().library,vec![spell,land]); assert_eq!(game.object(land).unwrap().zone,Zone::Library);
            assert_eq!(game.next_object_id_counter(),ids); assert_eq!(game.objects_in_deterministic_order().len(),objects);
            assert_eq!(game.player(bob).unwrap().life,20); assert!(game.exile.is_empty()); assert!(game.stack.is_empty());
            assert!(game.effect_store.replacement_effects.get_effect(shield).is_some()); assert!(game.take_pending_trigger_events().is_empty());
            assert!(ctx.get_tagged_all("__cascade_all").is_none()); assert!(ctx.get_tagged_all("__cascade_match").is_none());
        }
        drop(ctx);
        if mode == 0 || mode == 3 { assert_eq!(dm.added_calls,1); }
        if mode == 2 {
            assert_eq!(dm.added_calls,1); dm.pause=false; dm.pending=false; dm.land=false;
            let mut ctx=ExecutionContext::new(parent,alice,&mut dm);
            let outcome=CascadeEffect::new().execute(&mut game,&mut ctx).unwrap(); assert_eq!(outcome.explicit_objects().unwrap().len(),1);
            assert_eq!(game.player(bob).unwrap().life,27); assert!(!ctx.decision_maker.awaiting_choice()); drop(ctx); assert_eq!(dm.added_calls,2);
        }
    }
    #[test] fn additions_follow_land_entry_and_precede_cast_cleanup() {check(0);}
    #[test] fn error_restores_consultation_and_land_entry() {check(1);}
    #[test] fn pending_replays_consultation_and_land_entry() {check(2);}
    #[test] fn addition_binds_actual_land_arrival() {check(3);}
}
