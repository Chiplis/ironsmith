//! Proliferate effect implementation.

use crate::decision::FallbackStrategy;
use crate::decisions::{ProliferateSpec, make_decision_with_fallback};
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_value;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::processing::{
    TraitEventResult, process_trait_event_with_execution_context,
};
use crate::events::{Event, KeywordActionEvent, KeywordActionKind};
use crate::game_state::GameState;
use crate::object::CounterType;
use crate::snapshot::ObjectSnapshot;
use crate::triggers::TriggerEvent;
pub use ironsmith_core::ProliferateEffect;

use crate::effects::composition::mechanic_actions::execute_keyword_action_replacement_effects;

/// Effect that proliferates (adds counters to permanents/players with counters).
///
/// For each permanent with counters and each player with counters, adds one
/// counter of each type they already have.
///
/// # Example
///
/// ```ignore
/// let effect = ProliferateEffect::new(1);
/// ```
impl EffectExecutor for ProliferateEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        game.clear_pending_decision_controllers();
        let checkpoint = game.clone();
        let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        let result = (|| {
            let count = resolve_value(game, &self.count, ctx)?.max(0) as usize;
            if count == 0 {
                return Ok(EffectOutcome::resolved());
            }

            let mut proliferated_total = 0;
            let mut outcome = EffectOutcome::count(0);

            for _ in 0..count {
                let would_event = Event::new_with_provenance(
                    KeywordActionEvent::new(
                        KeywordActionKind::Proliferate,
                        ctx.controller,
                        ctx.source,
                        1,
                    ).with_snapshot(game.object(ctx.source)
                        .map(|object| ObjectSnapshot::from_object_with_calculated_characteristics(object, game))
                        .or_else(|| ctx.source_snapshot.clone())),
                    ctx.provenance,
                );
                let replacement_result = process_trait_event_with_execution_context(game, would_event, ctx)?;
                let iteration_outcome = crate::effects::replacement::execute_event_expansion_with_bindings(game, ctx, replacement_result, |game, ctx, original| {
                    let mut outcome = EffectOutcome::count(0);
                    match original {
                        TraitEventResult::Replaced { effects, source, controller, context, .. } => {
                            let snapshot = context.event.inner().snapshot().cloned();
                            return execute_keyword_action_replacement_effects(game, ctx, effects, source, controller, &context, snapshot);
                        }
                        TraitEventResult::Prevented => return Ok(EffectOutcome::prevented()),
                        TraitEventResult::NeedsChoice { .. } | TraitEventResult::NeedsInteraction { .. } => {
                            if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
                            return Err(ExecutionError::InternalError("proliferate suspended without a captured decision".into()));
                        }
                        TraitEventResult::Proceed(_) | TraitEventResult::Modified(_) => {}
                        TraitEventResult::Expanded { .. } => return Err(ExecutionError::InternalError("proliferate commit received an unflattened result".into())),
                    }

                let mut proliferated_count = 0;
                let mut proliferated_permanents = Vec::new();

                let eligible_permanents: Vec<crate::ids::ObjectId> = game
                    .battlefield
                    .iter()
                    // CR 702.26b: phased-out permanents can't be chosen.
                    .filter(|&&perm_id| !game.is_phased_out(perm_id))
                    .filter_map(|&perm_id| {
                        game.object(perm_id).and_then(|obj| {
                            if obj.counters.is_empty() || game.is_phased_out(perm_id) {
                                None
                            } else {
                                Some(perm_id)
                            }
                        })
                    })
                    .collect();

                let eligible_players: Vec<crate::ids::PlayerId> = game
                    .players
                    .iter()
                    .filter_map(|p| {
                        let has_counters = !p.counter_types_with_counters().is_empty();
                        (p.is_in_game() && has_counters).then_some(p.id)
                    })
                    .collect();

                let selections = make_decision_with_fallback(
                    game,
                    &mut ctx.decision_maker,
                    ctx.controller,
                    Some(ctx.source),
                    ProliferateSpec::new(
                        ctx.source,
                        eligible_permanents.clone(),
                        eligible_players.clone(),
                    ),
                    FallbackStrategy::Maximum,
                );
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(EffectOutcome::count(0));
                }

                let mut chosen_permanents: Vec<_> = selections
                    .permanents
                    .into_iter()
                    .filter(|perm_id| eligible_permanents.contains(perm_id))
                    .collect();
                let mut chosen_players: Vec<_> = selections
                    .players
                    .into_iter()
                    .filter(|player_id| eligible_players.contains(player_id))
                    .collect();
                chosen_permanents.sort_unstable();
                chosen_permanents.dedup();
                chosen_players.sort_unstable();
                chosen_players.dedup();

                // CR 701.34b: one selected player per shared-poison team receives
                // poison. Other kinds of counters still go to every selected
                // player. Choose the recipient before committing any counters.
                let mut poison_recipients = Vec::new();
                let mut handled_team_members = Vec::new();
                for &player in &chosen_players {
                    if handled_team_members.contains(&player) {
                        continue;
                    }
                    let members = game
                        .two_headed_giant_team_members(player)
                        .unwrap_or_else(|| vec![player]);
                    handled_team_members.extend(members.iter().copied());
                    let selected: Vec<_> = chosen_players
                        .iter()
                        .copied()
                        .filter(|id| members.contains(id))
                        .filter(|id| game.player(*id).is_some_and(|p| p.poison_counters > 0))
                        .collect();
                    let recipient = if selected.len() > 1 {
                        let options: Vec<_> = selected
                            .iter()
                            .map(|id| (game.player(*id).unwrap().name.to_string(), *id))
                            .collect();
                        crate::decisions::ask_choose_one(
                            game,
                            &mut ctx.decision_maker,
                            ctx.controller,
                            ctx.source,
                            &options,
                        )
                    } else {
                        selected.first().copied()
                    };
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(EffectOutcome::count(0));
                    }
                    if let Some(recipient) = recipient {
                        poison_recipients.push(recipient);
                    }
                }

                // One proliferate is one simultaneous counter-placing event
                // (CR 603.2c).
                let mut counter_batch: Option<crate::provenance::ProvNodeId> = None;
                for perm_id in chosen_permanents {
                    let Some(counter_types): Option<Vec<CounterType>> =
                        game.object(perm_id).and_then(|obj| {
                            (!obj.counters.is_empty())
                                .then(|| obj.counters.keys().copied().collect())
                        })
                    else {
                        continue;
                    };

                    for ct in counter_types {
                        let event = Event::put_counters(perm_id, ct, 1, ctx.cause.clone())
                            .with_provenance(ctx.provenance);
                        let mut placement =
                            super::execute_object_counter_placement(game, ctx, event)?;
                        if ctx.decision_maker.awaiting_choice() {
                            return Ok(EffectOutcome::count(0));
                        }
                        if placement.count_or_zero() > 0 {
                            if let Some(objects) = placement.affected_objects() {
                                proliferated_permanents.extend_from_slice(objects);
                            }
                        }
                        for event in &mut placement.events {
                            if event.kind() == crate::events::EventKind::MarkersChanged {
                                let batch = *counter_batch.get_or_insert_with(|| {
                                    game.alloc_child_event_provenance(
                                        ctx.provenance,
                                        crate::events::EventKind::MarkersChanged,
                                    )
                                });
                                *event = event.clone().with_simultaneous_batch(batch);
                            }
                        }
                        outcome = EffectOutcome::aggregate([outcome, placement]);
                    }
                }
                proliferated_permanents.sort_unstable();
                proliferated_permanents.dedup();
                proliferated_count += proliferated_permanents.len() as i32;

                for player_id in chosen_players {
                    let Some(counters) = game
                        .player(player_id)
                        .map(crate::player::Player::counter_types_with_counters)
                    else {
                        continue;
                    };
                    if counters.is_empty() {
                        continue;
                    }

                    let mut received_counter = false;
                    for counter_type in counters {
                        if counter_type == CounterType::Poison
                            && !poison_recipients.contains(&player_id)
                        {
                            continue;
                        }
                        // Player-counter placement already runs through the
                        // replacement/prevention pipeline inside this centralized
                        // helper. Do not pre-process it here or replacements such
                        // as counter doubling would be applied twice.
                        let event = Event::put_player_counters(
                            player_id,
                            counter_type,
                            1,
                            ctx.cause.clone(),
                        )
                        .with_provenance(ctx.provenance);
                        let placement = crate::effects::counters::execute_player_counter_placement(
                            game, ctx, event,
                        )?;
                        received_counter |= placement.count_or_zero() > 0;
                        outcome = EffectOutcome::aggregate([outcome, placement]);
                        if ctx.decision_maker.awaiting_choice() {
                            return Ok(outcome);
                        }
                    }
                    if received_counter {
                        proliferated_count += 1;
                    }
                }

                proliferated_total += proliferated_count;
                outcome = outcome.with_affected_objects(proliferated_permanents);
                outcome.set_value(crate::effect::OutcomeValue::Count(proliferated_count));
                Ok(outcome.with_event(TriggerEvent::new_with_provenance(
                    KeywordActionEvent::new(KeywordActionKind::Proliferate, ctx.controller, ctx.source, 1), ctx.provenance,
                )))
                }, |_, context, _| {
                    let action = crate::events::downcast_event::<KeywordActionEvent>(context.event.inner())
                        .filter(|action| action.action == KeywordActionKind::Proliferate)
                        .ok_or_else(|| ExecutionError::InternalError("proliferate addition captured an incompatible event".into()))?;
                    let object_tags = action.snapshot.as_ref().map(|snapshot| vec![
                        ("__it__".to_owned(), vec![snapshot.clone()]), ("it".to_owned(), vec![snapshot.clone()]),
                    ]).unwrap_or_default();
                    Ok(crate::effects::replacement::ReplacementProgramBindings { targets: None, object_tags })
                })?;
                if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
                outcome = EffectOutcome::aggregate([outcome, iteration_outcome]);
            }

            // Counter-placement receipts and the keyword summary can name the
            // same permanent. Report the affected set once, retaining additions.
            let mut affected = Vec::new();
            outcome.execution_facts.retain(|fact| {
                if let crate::effect::ExecutionFact::AffectedObjects(ids) = fact {
                    for id in ids {
                        if !affected.contains(id) { affected.push(*id); }
                    }
                    false
                } else { true }
            });
            outcome = outcome.with_affected_objects(affected);
            outcome.set_value(crate::effect::OutcomeValue::Count(proliferated_total));
            Ok(outcome)
        })();
        if result.is_err() || ctx.decision_maker.awaiting_choice() {
            game.restore_execution_checkpoint(checkpoint, result.is_ok() && ctx.decision_maker.awaiting_choice());
            context_checkpoint.restore(ctx);
            if ctx.decision_maker.awaiting_choice() && result.is_ok() {
                return Ok(EffectOutcome::count(0));
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::decision::DecisionMaker;
    use crate::decisions::specs::ProliferateResponse;
    use crate::events::EventKind;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::types::CardType;
    use crate::zone::Zone;
    use std::collections::VecDeque;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn make_creature_card(card_id: u32, name: &str) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    }

    fn create_creature_with_counters(
        game: &mut GameState,
        name: &str,
        controller: PlayerId,
        counter_type: CounterType,
        count: u32,
    ) -> ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name);
        let mut obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        obj.counters.insert(counter_type, count);
        game.add_object(obj);
        id
    }

    struct ScriptedProliferateDecisionMaker {
        responses: VecDeque<ProliferateResponse>,
    }

    impl DecisionMaker for ScriptedProliferateDecisionMaker {
        fn decide_proliferate(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::ProliferateContext,
        ) -> ProliferateResponse {
            self.responses.pop_front().unwrap_or_default()
        }
    }

    #[test]
    fn test_proliferate_permanents() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature_with_counters(
            &mut game,
            "Hangarback Walker",
            alice,
            CounterType::PlusOnePlusOne,
            3,
        );
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ProliferateEffect::new(1);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1)); // 1 permanent proliferated
        let obj = game.object(creature_id).unwrap();
        assert_eq!(obj.counters.get(&CounterType::PlusOnePlusOne), Some(&4)); // 3 + 1
    }

    #[test]
    fn test_proliferate_reports_affected_permanents_for_tagging() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature_with_counters(
            &mut game,
            "Tagged Proliferate Creature",
            alice,
            CounterType::PlusOnePlusOne,
            1,
        );
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ProliferateEffect::new(1);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.affected_objects(), Some([creature_id].as_slice()));
    }

    #[test]
    fn test_proliferate_excludes_permanents_when_counter_placement_is_prevented() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature_with_counters(
            &mut game,
            "Counter-Prohibited Creature",
            alice,
            CounterType::PlusOnePlusOne,
            1,
        );
        game.effect_store
            .cant_effects
            .cant_have_counters_placed
            .insert(creature_id);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let result = ProliferateEffect::new(1)
            .execute(&mut game, &mut ctx)
            .expect("proliferate should resolve through counter prevention");

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(0));
        assert_eq!(
            game.counter_count(creature_id, CounterType::PlusOnePlusOne),
            1
        );
        assert!(
            result.affected_objects().is_none_or(<[ObjectId]>::is_empty),
            "a permanent that had no counter put on it must not be exported as affected"
        );
    }

    #[test]
    fn test_proliferate_multiple_counter_types() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, "Multi-Counter Creature");
        let mut obj = Object::from_card(id, &card, alice, Zone::Battlefield);
        obj.counters.insert(CounterType::PlusOnePlusOne, 2);
        obj.counters.insert(CounterType::MinusOneMinusOne, 1);
        game.add_object(obj);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ProliferateEffect::new(1);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1)); // 1 permanent proliferated
        let obj = game.object(id).unwrap();
        assert_eq!(obj.counters.get(&CounterType::PlusOnePlusOne), Some(&3)); // 2 + 1
        assert_eq!(obj.counters.get(&CounterType::MinusOneMinusOne), Some(&2)); // 1 + 1
    }

    #[test]
    fn test_proliferate_poison_counters() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Give Alice some poison counters
        game.players[0].poison_counters = 5;

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ProliferateEffect::new(1);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1)); // 1 player counter proliferated
        assert_eq!(game.players[0].poison_counters, 6); // 5 + 1
    }

    #[test]
    fn test_proliferate_energy_counters() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Give Alice some energy counters
        game.players[0].energy_counters = 3;

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ProliferateEffect::new(1);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1));
        assert_eq!(game.players[0].energy_counters, 4); // 3 + 1
    }

    #[test]
    fn test_proliferate_nothing() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        // No permanents with counters, no players with counters
        let effect = ProliferateEffect::new(1);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(0));
    }

    #[test]
    fn test_proliferate_multiple_permanents() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let creature1 = create_creature_with_counters(
            &mut game,
            "Creature 1",
            alice,
            CounterType::PlusOnePlusOne,
            2,
        );
        let creature2 = create_creature_with_counters(
            &mut game,
            "Creature 2",
            bob,
            CounterType::MinusOneMinusOne,
            1,
        );

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ProliferateEffect::new(1);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2)); // 2 permanents proliferated

        let obj1 = game.object(creature1).unwrap();
        assert_eq!(obj1.counters.get(&CounterType::PlusOnePlusOne), Some(&3)); // 2 + 1

        let obj2 = game.object(creature2).unwrap();
        assert_eq!(obj2.counters.get(&CounterType::MinusOneMinusOne), Some(&2)); // 1 + 1
    }

    #[test]
    fn test_proliferate_clone_box() {
        let effect = ProliferateEffect::new(1);
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("ProliferateEffect"));
    }

    #[test]
    fn test_proliferate_default() {
        let effect = ProliferateEffect::default();
        assert_eq!(effect, ProliferateEffect::new(1));
    }

    #[test]
    fn test_proliferate_twice_repeats_action() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature_with_counters(
            &mut game,
            "Hangarback Walker",
            alice,
            CounterType::PlusOnePlusOne,
            3,
        );
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ProliferateEffect::new(2);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        let obj = game.object(creature_id).unwrap();
        assert_eq!(obj.counters.get(&CounterType::PlusOnePlusOne), Some(&5));
    }

    #[test]
    fn test_proliferate_can_choose_subset_of_eligible_permanents_and_players() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let untouched = create_creature_with_counters(
            &mut game,
            "Untouched",
            alice,
            CounterType::PlusOnePlusOne,
            2,
        );
        let chosen =
            create_creature_with_counters(&mut game, "Chosen", bob, CounterType::Charge, 1);
        game.players[0].poison_counters = 2;
        game.players[0].energy_counters = 3;
        game.players[1].experience_counters = 1;

        let source = game.new_object_id();
        let mut decision_maker = ScriptedProliferateDecisionMaker {
            responses: VecDeque::from([ProliferateResponse {
                permanents: vec![chosen],
                players: vec![alice],
            }]),
        };
        let mut ctx = ExecutionContext::new(source, alice, &mut decision_maker);

        let result = ProliferateEffect::new(1)
            .execute(&mut game, &mut ctx)
            .expect("subset proliferate should resolve");

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        assert_eq!(
            game.object(untouched)
                .and_then(|obj| obj.counters.get(&CounterType::PlusOnePlusOne).copied()),
            Some(2)
        );
        assert_eq!(
            game.object(chosen)
                .and_then(|obj| obj.counters.get(&CounterType::Charge).copied()),
            Some(2)
        );
        assert_eq!(game.players[0].poison_counters, 3);
        assert_eq!(game.players[0].energy_counters, 4);
        assert_eq!(game.players[1].experience_counters, 1);
    }

    #[test]
    fn test_proliferate_can_choose_nothing_and_still_perform_keyword_action() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let creature_id = create_creature_with_counters(
            &mut game,
            "Hangarback Walker",
            alice,
            CounterType::PlusOnePlusOne,
            3,
        );
        game.players[0].poison_counters = 4;

        let source = game.new_object_id();
        let mut decision_maker = ScriptedProliferateDecisionMaker {
            responses: VecDeque::from([ProliferateResponse::default()]),
        };
        let mut ctx = ExecutionContext::new(source, alice, &mut decision_maker);

        let result = ProliferateEffect::new(1)
            .execute(&mut game, &mut ctx)
            .expect("empty proliferate choice should resolve");

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(0));
        assert_eq!(
            game.object(creature_id)
                .and_then(|obj| obj.counters.get(&CounterType::PlusOnePlusOne).copied()),
            Some(3)
        );
        assert_eq!(game.players[0].poison_counters, 4);
        assert_eq!(result.events.len(), 1);
        assert_eq!(result.events[0].kind(), EventKind::KeywordAction);
        let keyword = result.events[0]
            .inner()
            .as_any()
            .downcast_ref::<KeywordActionEvent>()
            .expect("expected keyword action event");
        assert_eq!(keyword.action, KeywordActionKind::Proliferate);
        assert_eq!(keyword.player, alice);
        assert_eq!(keyword.amount, 1);
    }

    #[test]
    fn test_proliferate_twice_rechooses_targets_each_time() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let creature_id = create_creature_with_counters(
            &mut game,
            "Hangarback Walker",
            alice,
            CounterType::PlusOnePlusOne,
            1,
        );
        game.players[1].poison_counters = 2;

        let source = game.new_object_id();
        let mut decision_maker = ScriptedProliferateDecisionMaker {
            responses: VecDeque::from([
                ProliferateResponse {
                    permanents: vec![creature_id],
                    players: Vec::new(),
                },
                ProliferateResponse {
                    permanents: Vec::new(),
                    players: vec![bob],
                },
            ]),
        };
        let mut ctx = ExecutionContext::new(source, alice, &mut decision_maker);

        let result = ProliferateEffect::new(2)
            .execute(&mut game, &mut ctx)
            .expect("proliferate twice should resolve");

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        assert_eq!(
            game.object(creature_id)
                .and_then(|obj| obj.counters.get(&CounterType::PlusOnePlusOne).copied()),
            Some(2)
        );
        assert_eq!(game.players[1].poison_counters, 3);
        assert_eq!(
            result
                .events
                .iter()
                .filter(|event| event.kind() == EventKind::KeywordAction)
                .count(),
            2
        );
    }
}
