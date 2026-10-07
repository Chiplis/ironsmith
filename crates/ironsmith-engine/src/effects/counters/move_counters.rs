//! Move counters effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::{CompletedEffectOutputs, EffectExecutor};
use crate::effects::helpers::{resolve_bounded_nonnegative_u32, resolve_objects_for_effect};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;
pub use ironsmith_core::MoveCountersEffect;

impl EffectExecutor for MoveCountersEffect {
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
        if ctx.decision_maker.awaiting_choice() {
            return Ok(CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        game.clear_pending_decision_controllers();
        crate::effects::composition::execute_transaction(
            game,
            ctx,
            || CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                let finished = |outcome| Ok(CompletedEffectOutputs::aggregate_only(outcome));
                // Explicit target roles remain distinct even when their filters
                // compare equal. Untargeted references and plural donor sets are
                // resolved independently, never truncated to their first member.
                let legacy_pair = matches!(self.from.base(), ChooseSpec::Object(_))
                    && matches!(self.to.base(), ChooseSpec::Object(_))
                    && ctx.targets.len() >= 2;
                let (donors, recipients) = if (self.from.is_target() && self.to.is_target()) || legacy_pair {
                    let Some((from, to)) = super::assigned_counter_transfer_pair(ctx) else {
                        return finished(EffectOutcome::target_invalid());
                    };
                    (vec![from], vec![to])
                } else {
                    let resolve = |game: &mut GameState, ctx: &mut ExecutionContext, spec: &ChooseSpec| {
                        if matches!(spec.base(), ChooseSpec::Source) { Ok(vec![ctx.source]) }
                        else { resolve_objects_for_effect(game, ctx, spec) }
                    };
                    let donors = resolve(game, ctx, &self.from)?;
                    if ctx.decision_maker.awaiting_choice() { return finished(EffectOutcome::count(0)); }
                    let recipients = resolve(game, ctx, &self.to)?;
                    (donors, recipients)
                };
                if ctx.decision_maker.awaiting_choice() { return finished(EffectOutcome::count(0)); }
                let [to_id] = recipients.as_slice() else {
                    if recipients.is_empty() { return finished(EffectOutcome::target_invalid()); }
                    return Err(ExecutionError::InternalError("counter transfer requires one bound destination".into()));
                };
                let to_id = *to_id;
                if !super::move_destination_can_receive_counters(game, to_id, self.counter_type) {
                    return finished(EffectOutcome::count(0));
                }

                // Complete every amount choice before applying any removal or
                // replacement program. A zero allocation contributes no event.
                let mut allocations = Vec::new();
                let mut seen = std::collections::HashSet::new();
                let mut total = 0u32;
                for from_id in donors {
                    if !seen.insert(from_id) || from_id == to_id || game.is_phased_out(from_id)
                        || game.object(from_id).is_none() {
                        continue;
                    }
                    let available = game.counter_count(from_id, self.counter_type);
                    if available == 0 { continue; }
                    let to_move = match &self.count {
                        ironsmith_core::effect::CounterMoveAmount::Exact(value) =>
                            resolve_bounded_nonnegative_u32(game, value, ctx, available)?,
                        ironsmith_core::effect::CounterMoveAmount::All => available,
                        ironsmith_core::effect::CounterMoveAmount::AnyNumber => {
                            let name = game.object(from_id).map(|object| object.name.as_str()).unwrap_or("the permanent");
                            let spec = crate::decisions::NumberSpec::up_to(ctx.source, available,
                                format!("Choose how many {} counters to move from {name}", self.counter_type.description()));
                            let chosen = crate::decisions::make_decision_with_fallback(game, &mut ctx.decision_maker,
                                ctx.controller, Some(ctx.source), spec, crate::decision::FallbackStrategy::Maximum);
                            if ctx.decision_maker.awaiting_choice() { return finished(EffectOutcome::count(0)); }
                            chosen
                        }
                    }.min(available);
                    if to_move == 0 { continue; }
                    total = total.checked_add(to_move).ok_or_else(|| ExecutionError::InternalError(
                        "counter transfer destination exceeds the supported counter count range".into()))?;
                    allocations.push((from_id, to_move));
                }
                if total == 0 { return finished(EffectOutcome::count(0)); }

                // This instruction is one simultaneous counter operation. Every
                // removal and the combined placement are proposed against the
                // pre-mutation state; appended programs cannot change a later
                // donor's counters or the destination before its original commits.
                let mut removals = Vec::new();
                for (from_id, amount) in allocations {
                    let event = crate::events::Event::remove_counters(from_id, self.counter_type, amount)
                        .with_provenance(ctx.provenance);
                    removals.push(super::prepare_counter_removal(game, ctx, event)?);
                    if ctx.decision_maker.awaiting_choice() { return finished(EffectOutcome::count(0)); }
                }
                let placement = super::prepare_counter_placement(game, ctx,
                    crate::events::Event::put_counters(to_id, self.counter_type, total, ctx.cause.clone())
                        .with_provenance(ctx.provenance))?;
                if ctx.decision_maker.awaiting_choice() { return finished(EffectOutcome::count(0)); }
                // All donors and the one placement commit their originals before
                // the shared completion owner freezes and runs any additions.
                let mut removed = 0i64;
                let outcomes = crate::effects::composition::execute_simultaneous_originals_with_default_outputs(
                    game,
                    ctx,
                    true,
                    |game, ctx| {
                        let mut committed = Vec::new();
                        for removal in removals {
                            let receipt = super::commit_prepared_counter_removal_original_with_outputs(
                                game, ctx, removal,
                            )?;
                            if ctx.decision_maker.awaiting_choice() { return Ok(Vec::new()); }
                            removed = removed.checked_add(receipt.outcome.outcome.instruction_result().count_or_zero())
                                .ok_or_else(|| ExecutionError::InternalError(
                                    "counter transfer receipt exceeds the supported count range".into(),
                                ))?;
                            committed.push(receipt);
                        }
                        committed.push(super::commit_prepared_counter_original_with_outputs(
                            game, ctx, placement,
                        )?);
                        Ok(committed)
                    },
                )?;
                if ctx.decision_maker.awaiting_choice() { return finished(EffectOutcome::count(0)); }
                // The instruction result counts original removals. Multiplied
                // placement and appended programs retain their own receipts.
                let outcome = EffectOutcome::aggregate_with_primary_result(
                    EffectOutcome::count(removed),
                    outcomes.iter().map(|outputs| outputs.outcome.clone()),
                );
                let mut outputs = CompletedEffectOutputs::aggregate_only(outcome);
                outputs.retain_batch_children(outcomes);
                Ok(outputs)
            },
        )
    }

    fn get_target_spec(&self) -> Option<&crate::target::ChooseSpec> {
        Some(&self.from)
    }

    fn target_description(&self) -> &'static str {
        "creature to move counters from"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effects::ResolvedTarget;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::{CounterType, Object};
    use crate::types::CardType;
    use crate::zone::Zone;

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

    fn create_creature(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name);
        let obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    #[test]
    fn test_move_counters() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let from_id = create_creature_with_counters(
            &mut game,
            "Source Creature",
            alice,
            CounterType::PlusOnePlusOne,
            5,
        );
        let to_id = create_creature(&mut game, "Target Creature", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice).with_targets(vec![
            ResolvedTarget::Object(from_id),
            ResolvedTarget::Object(to_id),
        ]);

        let effect = MoveCountersEffect::plus_one_counters(3);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(3));

        let from_obj = game.object(from_id).unwrap();
        assert_eq!(
            from_obj.counters.get(&CounterType::PlusOnePlusOne),
            Some(&2)
        ); // 5 - 3

        let to_obj = game.object(to_id).unwrap();
        assert_eq!(to_obj.counters.get(&CounterType::PlusOnePlusOne), Some(&3));
    }

    #[test]
    fn test_move_counters_limited_by_available() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let from_id = create_creature_with_counters(
            &mut game,
            "Source Creature",
            alice,
            CounterType::PlusOnePlusOne,
            2,
        );
        let to_id = create_creature(&mut game, "Target Creature", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice).with_targets(vec![
            ResolvedTarget::Object(from_id),
            ResolvedTarget::Object(to_id),
        ]);

        // Request 5 but only 2 available
        let effect = MoveCountersEffect::plus_one_counters(5);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2)); // Limited by available

        // When all counters are removed, the entry is removed from the HashMap
        assert_eq!(game.counter_count(from_id, CounterType::PlusOnePlusOne), 0);
        assert_eq!(game.counter_count(to_id, CounterType::PlusOnePlusOne), 2);
    }

    #[test]
    fn test_move_counters_no_counters() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let from_id = create_creature(&mut game, "Source Creature", alice);
        let to_id = create_creature(&mut game, "Target Creature", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice).with_targets(vec![
            ResolvedTarget::Object(from_id),
            ResolvedTarget::Object(to_id),
        ]);

        let effect = MoveCountersEffect::plus_one_counters(3);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(0));
    }

    #[test]
    fn test_move_counters_insufficient_targets() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let from_id = create_creature_with_counters(
            &mut game,
            "Source Creature",
            alice,
            CounterType::PlusOnePlusOne,
            5,
        );
        let source = game.new_object_id();

        // Only one target provided
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(from_id)]);

        let effect = MoveCountersEffect::plus_one_counters(3);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.status, crate::effect::OutcomeStatus::TargetInvalid);
    }

    #[test]
    fn test_move_counters_clone_box() {
        let effect = MoveCountersEffect::plus_one_counters(1);
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("MoveCountersEffect"));
    }
}


#[cfg(test)]
mod move_removal_replacement_owner_tests {
    use super::*;
    fn check_owner(owner:u8,instead:bool) {
        let mut game=crate::tests::test_helpers::setup_two_player_game();
        let alice=crate::ids::PlayerId::from_index(0);
        let definition=crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(),"Move removal source")
            .card_types(vec![crate::types::CardType::Artifact]).build();
        let source=game.create_object_from_definition(&definition,alice,crate::zone::Zone::Battlefield);
        let destination=game.create_object_from_definition(&definition,alice,crate::zone::Zone::Battlefield);
        let kind=crate::object::CounterType::Charge;
        game.object_mut(source).unwrap().counters.insert(kind,3);
        let action=if instead {crate::replacement::ReplacementAction::Instead(vec![crate::effect::Effect::gain_life(2)])}
            else {crate::replacement::ReplacementAction::Prevent};
        let shield=game.effect_store.replacement_effects.add_one_shot_effect(crate::replacement::ReplacementEffect::with_matcher(source,alice,
            crate::events::counters::matchers::WouldRemoveCountersMatcher::new(crate::target::ObjectFilter::permanent(),Some(kind)),action));
        let effect=match owner {
            0=>crate::effect::Effect::new(MoveCountersEffect::new(kind,2,ChooseSpec::Source,ChooseSpec::SpecificObject(destination))),
            1=>crate::effect::Effect::new(crate::effects::MoveAllCountersEffect::new(ChooseSpec::Source,ChooseSpec::SpecificObject(destination))),
            _=>crate::effect::Effect::new(crate::effects::MoveOneCounterEffect::new(ChooseSpec::Source,ChooseSpec::SpecificObject(destination))),
        };
        let mut ctx=ExecutionContext::new_default(source,alice);
        let outcome=crate::effects::execute_effect(&mut game,&effect,&mut ctx).unwrap();
        assert_eq!(game.counter_count(source,kind),3,"the move's removal component must honor its own replacement");
        assert_eq!(game.player(alice).unwrap().life,if instead{22}else{20});
        assert_eq!(outcome.events_of_type::<crate::events::LifeGainEvent>().count(),usize::from(instead));
        assert_eq!(outcome.events_of_type::<crate::events::MarkersChangedEvent>().filter(|event|event.is_removed()).count(),0);
        assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
        // This reproduces removal bypass and payload loss. First placement
        // quantity/movement receipt coupling remains a separate rules-backed audit.
        let previous_destination_count=game.counter_count(destination,kind);
        let moved=match owner{0=>2,1=>3,_=>1};
        let next=crate::effects::execute_effect(&mut game,&effect,&mut ctx).unwrap();
        assert_eq!(next.count_or_zero(),moved as i64);
        assert_eq!(game.counter_count(source,kind),3-moved);
        assert_eq!(game.counter_count(destination,kind),previous_destination_count+moved);
        assert_eq!(next.events_of_type::<crate::events::MarkersChangedEvent>().count(),2);
        assert_eq!(next.events_of_type::<crate::events::LifeGainEvent>().count(),0);
        assert_eq!(game.player(alice).unwrap().life,if instead{22}else{20});
    }
    #[test]fn move_counters_honors_removal_prevention(){check_owner(0,false);}
    #[test]fn move_counters_executes_removal_instead_payload(){check_owner(0,true);}
    #[test]fn move_all_counters_honors_removal_prevention(){check_owner(1,false);}
    #[test]fn move_all_counters_executes_removal_instead_payload(){check_owner(1,true);}
    #[test]fn move_one_counter_honors_removal_prevention(){check_owner(2,false);}
    #[test]fn move_one_counter_executes_removal_instead_payload(){check_owner(2,true);}
}

#[cfg(test)]
mod self_move_counter_prohibition_tests {
    use super::*;
    fn check(owner: u8, history_first: bool) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = crate::ids::PlayerId::from_index(0);
        let definition = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Self move source")
            .card_types(vec![crate::types::CardType::Artifact]).build();
        let source = game.create_object_from_definition(&definition, alice, crate::zone::Zone::Battlefield);
        game.object_mut(source).unwrap().counters.insert(crate::object::CounterType::Charge, 3);
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(
            crate::replacement::ReplacementEffect::with_matcher(source, alice,
                crate::events::counters::matchers::WouldPutCountersMatcher::any(),
                crate::replacement::ReplacementAction::Modify(crate::replacement::EventModification::Multiply(2))));
        let effect = match owner {
            0 => crate::effect::Effect::new(crate::effects::MoveOneCounterEffect::new(ChooseSpec::Source, ChooseSpec::Source)),
            _ => crate::effect::Effect::new(crate::effects::MoveAllCountersEffect::new(ChooseSpec::Source, ChooseSpec::Source)),
        };
        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
        if history_first { assert!(outcome.events.is_empty(), "an impossible self move cannot publish removal or placement observations"); }
        assert_eq!(game.counter_count(source, crate::object::CounterType::Charge), 3,
            "a same-object move cannot remove and re-add counters through a multiplier");
        assert_eq!(outcome.count_or_zero(), 0);
        assert!(outcome.events.is_empty());
        assert_eq!(game.turn_store.turn_history.event_kind_count(crate::events::EventKind::MarkersChanged), 0);
        assert!(game.effect_store.replacement_effects.get_effect(shield).is_some(), "no counter event means no one-shot consumption");
    }
    #[test] fn move_one_to_itself_cannot_duplicate_counters() { check(0, false); }
    #[test] fn move_one_to_itself_has_no_observations() { check(0, true); }
    #[test] fn move_all_to_itself_cannot_duplicate_counters() { check(1, false); }
    #[test] fn move_all_to_itself_has_no_observations() { check(1, true); }
}

#[cfg(test)]
mod phased_source_move_counter_tests {
    use super::*;
    fn check(history_first: bool) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = crate::ids::PlayerId::from_index(0);
        let definition = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Phased movement source")
            .card_types(vec![crate::types::CardType::Artifact]).build();
        let source = game.create_object_from_definition(&definition, alice, crate::zone::Zone::Battlefield);
        let destination = game.create_object_from_definition(&definition, alice, crate::zone::Zone::Battlefield);
        let kind = crate::object::CounterType::Charge;
        game.object_mut(source).unwrap().counters.insert(kind, 3);
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(
            crate::replacement::ReplacementEffect::with_matcher(destination, alice,
                crate::events::counters::matchers::WouldPutCountersMatcher::any(),
                crate::replacement::ReplacementAction::Modify(crate::replacement::EventModification::Multiply(2))));
        game.phase_out(source);
        assert!(game.is_phased_out(source));
        let effect = crate::effect::Effect::new(MoveCountersEffect::new(kind, 2,
            ChooseSpec::Source, ChooseSpec::SpecificObject(destination)));
        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
        if history_first { assert!(outcome.events.is_empty(), "a move with an absent phased source cannot publish placement events"); }
        assert_eq!(game.counter_count(destination, kind), 0,
            "ordinary effects cannot create moved counters from a phased-out source");
        assert_eq!(game.counter_count(source, kind), 3);
        assert_eq!(outcome.count_or_zero(), 0);
        assert!(outcome.events.is_empty());
        assert!(game.effect_store.replacement_effects.get_effect(shield).is_some());
        game.phase_in(source);
        let next = crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
        assert_eq!(next.count_or_zero(), 2);
        assert_eq!(game.counter_count(source, kind), 1);
        assert_eq!(game.counter_count(destination, kind), 4);
        assert_eq!(next.events_of_type::<crate::events::MarkersChangedEvent>().count(), 2);
        assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
    }
    #[test] fn phased_source_move_cannot_create_destination_counters() { check(false); }
    #[test] fn phased_source_move_cannot_publish_or_consume_placement() { check(true); }
}

#[cfg(test)]
mod phased_move_owner_parity_tests {
    use super::*;
    fn check(owner: u8, destination_phased: bool) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = crate::ids::PlayerId::from_index(0);
        let definition = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Phased move parity")
            .card_types(vec![crate::types::CardType::Artifact]).build();
        let source = game.create_object_from_definition(&definition, alice, crate::zone::Zone::Battlefield);
        let destination = game.create_object_from_definition(&definition, alice, crate::zone::Zone::Battlefield);
        let sponsor = game.create_object_from_definition(&definition, alice, crate::zone::Zone::Battlefield);
        let kind = crate::object::CounterType::Charge;
        game.object_mut(source).unwrap().counters.insert(kind, 3);
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(
            crate::replacement::ReplacementEffect::with_matcher(sponsor, alice,
                crate::events::counters::matchers::WouldPutCountersMatcher::any(),
                crate::replacement::ReplacementAction::Modify(crate::replacement::EventModification::Multiply(2))));
        let phased = if destination_phased { destination } else { source };
        game.phase_out(phased);
        let effect = match owner {
            0 => crate::effect::Effect::new(MoveCountersEffect::new(kind, 2, ChooseSpec::Source, ChooseSpec::SpecificObject(destination))),
            1 => crate::effect::Effect::new(crate::effects::MoveOneCounterEffect::new(ChooseSpec::Source, ChooseSpec::SpecificObject(destination))),
            _ => crate::effect::Effect::new(crate::effects::MoveAllCountersEffect::new(ChooseSpec::Source, ChooseSpec::SpecificObject(destination))),
        };
        struct Choices(usize);
        impl crate::decision::DecisionMaker for Choices {
            fn decide_counters(&mut self, _game: &GameState, ctx: &crate::decisions::context::CountersContext) -> Vec<(crate::object::CounterType, u32)> {
                self.0 += 1;
                vec![(crate::object::CounterType::Charge, u32::try_from(ctx.max_total.min(1)).unwrap())]
            }
        }
        let mut choices = Choices(0);
        let outcome = {
            let mut ctx = ExecutionContext::new(source, alice, &mut choices);
            crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap()
        };
        assert_eq!(game.counter_count(source, kind), 3, "an absent movement endpoint cannot remove source counters");
        assert_eq!(game.counter_count(destination, kind), 0);
        assert_eq!(outcome.count_or_zero(), 0);
        assert!(outcome.events.is_empty());
        assert!(game.effect_store.replacement_effects.get_effect(shield).is_some());
        assert_eq!(choices.0, 0, "an absent endpoint cannot require a counter choice");
        game.phase_in(phased);
        let next = {
            let mut ctx = ExecutionContext::new(source, alice, &mut choices);
            crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap()
        };
        let moved = match owner { 0 => 2, 1 => 1, _ => 3 };
        assert_eq!(next.count_or_zero(), moved);
        assert_eq!(game.counter_count(source, kind), 3 - moved as u32);
        assert_eq!(game.counter_count(destination, kind), 2 * moved as u32);
        assert_eq!(next.events_of_type::<crate::events::MarkersChangedEvent>().count(), 2);
        assert_eq!(choices.0, usize::from(owner == 1));
        assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
    }
    #[test] fn fixed_move_ignores_phased_destination() { check(0, true); }
    #[test] fn one_move_ignores_phased_destination() { check(1, true); }
    #[test] fn all_move_ignores_phased_destination() { check(2, true); }
    #[test] fn one_move_does_not_choose_from_phased_source() { check(1, false); }
    #[test] fn all_move_phased_source_control() { check(2, false); }
}

#[cfg(test)]
mod move_component_application_tests {
    use super::*;
    use crate::ids::{CardId, PlayerId};
    use crate::object::CounterType;
    use crate::replacement::{EventModification, ReplacementAction, ReplacementEffect};
    fn setup(owner: u8, counters: u32) -> (GameState, crate::ids::ObjectId, crate::ids::ObjectId, crate::effect::Effect, u32) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let definition = crate::cards::CardDefinitionBuilder::new(CardId::new(), "Move component fixture")
            .card_types(vec![crate::types::CardType::Artifact]).build();
        let source = game.create_object_from_definition(&definition, alice, crate::zone::Zone::Battlefield);
        let destination = game.create_object_from_definition(&definition, alice, crate::zone::Zone::Battlefield);
        game.object_mut(source).unwrap().counters.insert(CounterType::Charge, counters);
        let (effect, budget) = match owner {
            0 => (crate::effect::Effect::new(MoveCountersEffect::new(CounterType::Charge, 2, ChooseSpec::Source, ChooseSpec::SpecificObject(destination))), 2),
            1 => (crate::effect::Effect::new(crate::effects::MoveAllCountersEffect::new(ChooseSpec::Source, ChooseSpec::SpecificObject(destination))), counters),
            _ => (crate::effect::Effect::new(crate::effects::MoveOneCounterEffect::new(ChooseSpec::Source, ChooseSpec::SpecificObject(destination))), 1),
        };
        (game, source, destination, effect, budget)
    }
    fn replacement(game: &mut GameState, source: crate::ids::ObjectId, action: ReplacementAction) -> crate::replacement::ReplacementEffectId {
        game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source, PlayerId::from_index(0),
            crate::events::counters::matchers::WouldRemoveCountersMatcher::new(crate::target::ObjectFilter::permanent(), Some(CounterType::Charge)), action))
    }
    fn quantity(owner: u8) {
        let (mut game, source, destination, effect, budget) = setup(owner, 5);
        let removed = if owner == 2 { 0 } else { 1 };
        let shield = replacement(&mut game, source, ReplacementAction::Modify(EventModification::SetTo(removed)));
        let put_shield = game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source, PlayerId::from_index(0),
            crate::events::counters::matchers::WouldPutCountersMatcher::any(), ReplacementAction::Modify(EventModification::Multiply(2))));
        let mut ctx = ExecutionContext::new_default(source, PlayerId::from_index(0));
        let out = crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
        assert_eq!(game.counter_count(source, CounterType::Charge), 5 - removed, "resolved removal quantity must be committed");
        // CR 122.5 decomposes the instruction into removal and placement.
        // Replacing one component does not rewrite the other's authored quantity.
        assert_eq!(game.counter_count(destination, CounterType::Charge), 2 * budget);
        assert_eq!(out.count_or_zero(), if owner == 0 { removed as i64 } else { budget as i64 });
        assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
        assert!(game.effect_store.replacement_effects.get_effect(put_shield).is_none());
        let markers = out.events_of_type::<crate::events::MarkersChangedEvent>().collect::<Vec<_>>();
        assert_eq!(markers.iter().filter(|e| e.is_removed()).map(|e| e.amount).sum::<u32>(), removed);
        assert_eq!(markers.iter().filter(|e| e.is_added()).map(|e| e.amount).sum::<u32>(), 2 * budget);
        assert_eq!(markers.len(), if removed == 0 { 1 } else { 2 });
    }
    struct Answers { pause: bool, pending: bool, calls: usize }
    impl crate::decision::DecisionMaker for Answers {
        fn decide_counters(&mut self, _game: &GameState, ctx: &crate::decisions::context::CountersContext) -> Vec<(CounterType, u32)> {
            assert_eq!(ctx.min_total, 1);
            assert_eq!(ctx.max_total, 1);
            vec![(CounterType::Charge, 1)]
        }
        fn decide_boolean(&mut self, _game: &GameState, _ctx: &crate::decisions::context::BooleanContext) -> bool {
            self.calls += 1; self.pending = self.pause; !self.pending
        }
        fn awaiting_choice(&self) -> bool { self.pending }
    }
    fn pending(owner: u8) {
        let (mut game, source, destination, effect, budget) = setup(owner, 3);
        let shield = replacement(&mut game, source, ReplacementAction::Instead(vec![crate::effect::Effect::may(vec![crate::effect::Effect::gain_life(2)])]));
        let mut answers = Answers { pause: true, pending: false, calls: 0 };
        let out = { let mut ctx = ExecutionContext::new(source, PlayerId::from_index(0), &mut answers); crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap() };
        assert!(answers.pending, "removal replacement choice must suspend the move");
        assert_eq!(out.count_or_zero(), 0); assert!(out.events.is_empty());
        assert_eq!(game.counter_count(source, CounterType::Charge), 3);
        assert_eq!(game.counter_count(destination, CounterType::Charge), 0);
        assert_eq!(game.player(PlayerId::from_index(0)).unwrap().life, 20);
        assert!(game.effect_store.replacement_effects.get_effect(shield).is_some());
        answers.pause = false; answers.pending = false;
        let out = { let mut ctx = ExecutionContext::new(source, PlayerId::from_index(0), &mut answers); crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap() };
        assert_eq!(answers.calls, 2);
        assert_eq!(game.counter_count(source, CounterType::Charge), 3);
        assert_eq!(game.counter_count(destination, CounterType::Charge), budget);
        assert_eq!(game.player(PlayerId::from_index(0)).unwrap().life, 22);
        assert_eq!(out.count_or_zero(), if owner == 0 { 0 } else { budget as i64 });
        assert_eq!(out.events_of_type::<crate::events::LifeGainEvent>().count(), 1);
        assert_eq!(out.events_of_type::<crate::events::MarkersChangedEvent>().count(), 1);
        assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
    }
    #[derive(Debug, Clone)] struct Fail;
    impl crate::effects::EffectExecutor for Fail {
        fn execute(&self, _game: &mut GameState, _ctx: &mut ExecutionContext) -> Result<EffectOutcome, ExecutionError> {
            Err(ExecutionError::InternalError("injected move removal failure".into()))
        }
    }
    fn error(owner: u8) {
        let (mut game, source, destination, effect, _) = setup(owner, 3);
        let shield = replacement(&mut game, source, ReplacementAction::Instead(vec![crate::effect::Effect::gain_life(2), crate::effect::Effect::new(Fail)]));
        let mut ctx = ExecutionContext::new_default(source, PlayerId::from_index(0));
        let result = crate::effects::execute_effect(&mut game, &effect, &mut ctx);
        assert!(matches!(result, Err(ExecutionError::InternalError(ref s)) if s == "injected move removal failure"), "replacement payload failure must propagate");
        assert_eq!(game.player(PlayerId::from_index(0)).unwrap().life, 20);
        assert_eq!(game.counter_count(source, CounterType::Charge), 3);
        assert_eq!(game.counter_count(destination, CounterType::Charge), 0);
        assert!(game.effect_store.replacement_effects.get_effect(shield).is_some());
    }
    #[test]
    fn single_counter_move_requires_one_counter_in_its_prompt() {
        let (mut game, source, destination, effect, _) = setup(2, 3);
        struct PickOne { calls: usize }
        impl crate::decision::DecisionMaker for PickOne {
            fn decide_counters(&mut self, _game: &GameState, ctx: &crate::decisions::context::CountersContext) -> Vec<(CounterType, u32)> {
                self.calls += 1;
                assert_eq!(ctx.min_total, 1, "a mandatory one-counter move cannot offer a zero-counter choice");
                assert_eq!(ctx.max_total, 1);
                vec![(CounterType::Charge, 1)]
            }
        }
        let mut answers = PickOne { calls: 0 };
        let out = { let mut ctx = ExecutionContext::new(source, PlayerId::from_index(0), &mut answers); crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap() };
        assert_eq!(answers.calls, 1);
        assert_eq!(out.count_or_zero(), 1);
        assert_eq!(game.counter_count(source, CounterType::Charge), 2);
        assert_eq!(game.counter_count(destination, CounterType::Charge), 1);
        assert_eq!(out.events_of_type::<crate::events::MarkersChangedEvent>().count(), 2);
    }
    #[derive(Debug, Clone)]
    struct PauseThenFail;
    impl crate::effects::EffectExecutor for PauseThenFail {
        fn execute(&self, game: &mut GameState, ctx: &mut ExecutionContext) -> Result<EffectOutcome, ExecutionError> {
            crate::decisions::ask_may_choice(game, &mut ctx.decision_maker, ctx.controller, ctx.source,
                "Pause before injected failure", crate::decision::FallbackStrategy::Decline);
            Err(ExecutionError::ResourceLimitExceeded {
                resource: "injected counter-transfer failure", requested: 2, maximum: 1,
            })
        }
    }
    #[test]
    fn all_and_one_counter_moves_preserve_typed_failure_even_after_child_suspends() {
        for owner in [1, 2] {
            let (mut game, source, destination, effect, _) = setup(owner, 3);
            let shield = replacement(&mut game, source,
                ReplacementAction::Additionally(vec![crate::effect::Effect::new(PauseThenFail)]));
            let mut answers = Answers { pause: true, pending: false, calls: 0 };
            let result = {
                let mut ctx = ExecutionContext::new(source, PlayerId::from_index(0), &mut answers);
                crate::effects::execute_effect(&mut game, &effect, &mut ctx)
            };
            assert!(answers.pending);
            assert!(matches!(result, Err(ExecutionError::ResourceLimitExceeded {
                resource: "injected counter-transfer failure", requested: 2, maximum: 1,
            })));
            assert_eq!(game.counter_count(source, CounterType::Charge), 3);
            assert_eq!(game.counter_count(destination, CounterType::Charge), 0);
            assert!(game.effect_store.replacement_effects.get_effect(shield).is_some());
        }
    }
    #[test] fn fixed_move_respects_independent_component_quantities() { quantity(0); }
    #[test] fn all_move_respects_independent_component_quantities() { quantity(1); }
    #[test] fn one_move_respects_independent_component_quantities() { quantity(2); }
    #[test] fn fixed_move_suspends_and_retries_removal_replacement() { pending(0); }
    #[test] fn all_move_suspends_and_retries_removal_replacement() { pending(1); }
    #[test] fn one_move_suspends_and_retries_removal_replacement() { pending(2); }
    #[test] fn fixed_move_propagates_removal_payload_failure() { error(0); }
    #[test] fn all_move_propagates_removal_payload_failure() { error(1); }
    #[test] fn one_move_propagates_removal_payload_failure() { error(2); }
}

#[cfg(test)]
mod chosen_counter_move_amount_tests {
    use super::*;
    use crate::object::CounterType;
    struct Answers { chosen: u32, pause: bool, pending: bool, calls: usize, bounds: Vec<(u32,u32)> }
    impl crate::decision::DecisionMaker for Answers {
        fn decide_number(&mut self, _game: &GameState, ctx: &crate::decisions::context::NumberContext) -> u32 {
            self.calls += 1; self.bounds.push((ctx.min,ctx.max)); self.pending=self.pause; self.chosen
        }
        fn awaiting_choice(&self) -> bool {self.pending}
    }
    fn setup(available:u32) -> (GameState,crate::ids::ObjectId,crate::ids::ObjectId,crate::effect::Effect) {
        let mut game=crate::tests::test_helpers::setup_two_player_game();
        let alice=crate::ids::PlayerId::from_index(0);
        let definition=crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(),"Chosen counter transfer")
            .card_types(vec![crate::types::CardType::Artifact]).build();
        let from=game.create_object_from_definition(&definition,alice,crate::zone::Zone::Battlefield);
        let to=game.create_object_from_definition(&definition,alice,crate::zone::Zone::Battlefield);
        game.object_mut(from).unwrap().counters.insert(CounterType::Charge,available);
        let effect=crate::effect::Effect::new(MoveCountersEffect::any_number(CounterType::Charge,ChooseSpec::Source,ChooseSpec::SpecificObject(to)));
        (game,from,to,effect)
    }
    fn run(game:&mut GameState,source:crate::ids::ObjectId,effect:&crate::effect::Effect,answers:&mut Answers)->EffectOutcome {
        let mut ctx=ExecutionContext::new(source,crate::ids::PlayerId::from_index(0),answers);
        crate::effects::execute_effect(game,effect,&mut ctx).unwrap()
    }
    #[test]
    fn any_number_move_chooses_zero_subset_or_all() {
        for chosen in [0,1,4] {
            let (mut game,from,to,effect)=setup(4);
            let mut answers=Answers{chosen,pause:false,pending:false,calls:0,bounds:vec![]};
            let out=run(&mut game,from,&effect,&mut answers);
            assert_eq!(answers.bounds,vec![(0,4)]);
            assert_eq!(game.counter_count(from,CounterType::Charge),4-chosen);
            assert_eq!(game.counter_count(to,CounterType::Charge),chosen);
            assert_eq!(out.count_or_zero(),chosen as i64);
            assert_eq!(out.events_of_type::<crate::events::MarkersChangedEvent>().count(),if chosen==0{0}else{2});
        }
    }
    #[test]
    fn any_number_move_pending_choice_keeps_both_endpoints_and_shield() {
        let (mut game,from,to,effect)=setup(4);
        let shield=game.effect_store.replacement_effects.add_one_shot_effect(crate::replacement::ReplacementEffect::with_matcher(from,crate::ids::PlayerId::from_index(0),
            crate::events::counters::matchers::WouldPutCountersMatcher::any(),crate::replacement::ReplacementAction::Modify(crate::replacement::EventModification::Multiply(2))));
        let mut answers=Answers{chosen:1,pause:true,pending:false,calls:0,bounds:vec![]};
        let out=run(&mut game,from,&effect,&mut answers);
        assert!(answers.pending);assert!(out.events.is_empty());
        assert_eq!(game.counter_count(from,CounterType::Charge),4);assert_eq!(game.counter_count(to,CounterType::Charge),0);
        assert!(game.effect_store.replacement_effects.get_effect(shield).is_some());
        answers.pause=false;answers.pending=false;
        let out=run(&mut game,from,&effect,&mut answers);
        assert_eq!(answers.calls,2);assert_eq!(out.count_or_zero(),1);
        assert_eq!(game.counter_count(from,CounterType::Charge),3);assert_eq!(game.counter_count(to,CounterType::Charge),2);
        assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
        assert_eq!(out.events_of_type::<crate::events::MarkersChangedEvent>().count(),2);
    }
    #[test]
    fn any_number_choice_bound_preserves_unsigned_available_counter_count() {
        let (mut game,from,to,effect)=setup(u32::MAX);
        let mut answers=Answers{chosen:1,pause:false,pending:false,calls:0,bounds:vec![]};
        run(&mut game,from,&effect,&mut answers);
        assert_eq!(answers.bounds,vec![(0,u32::MAX)]);
        assert_eq!(game.counter_count(from,CounterType::Charge),u32::MAX-1);assert_eq!(game.counter_count(to,CounterType::Charge),1);
    }
    #[test]
    fn any_number_move_absent_endpoint_does_not_ask_or_consume() {
        for destination in [false,true] {
            let (mut game,from,to,effect)=setup(4);game.phase_out(if destination{to}else{from});
            let mut answers=Answers{chosen:1,pause:false,pending:false,calls:0,bounds:vec![]};
            let out=run(&mut game,from,&effect,&mut answers);
            assert_eq!(answers.calls,0);assert!(out.events.is_empty());
            assert_eq!(game.counter_count(from,CounterType::Charge),4);assert_eq!(game.counter_count(to,CounterType::Charge),0);
        }
    }

    #[test]
    fn any_number_all_preserves_unsigned_quantity_in_movement_receipt() {
        for available in [i32::MAX as u32, i32::MAX as u32 + 1, u32::MAX] {
            let (mut game,from,to,effect)=setup(available);
            let mut answers=Answers{chosen:available,pause:false,pending:false,calls:0,bounds:vec![]};
            let out=run(&mut game,from,&effect,&mut answers);
            assert_eq!(answers.bounds,vec![(0,available)]);
            assert_eq!(game.counter_count(from,CounterType::Charge),0);
            assert_eq!(game.counter_count(to,CounterType::Charge),available);
            assert_eq!(out.events_of_type::<crate::events::MarkersChangedEvent>().count(),2);
            assert_eq!(i64::from(out.count_or_zero()),i64::from(available),
                "movement receipt must preserve the unsigned quantity actually chosen and moved");
        }
    }
}

#[cfg(test)]
#[path = "counted_transfer_tests.rs"]
mod counted_transfer_tests;
