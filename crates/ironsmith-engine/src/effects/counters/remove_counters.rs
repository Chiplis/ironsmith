//! Remove counters effect implementation.

use crate::effect::{EffectOutcome, Value};
use crate::effects::helpers::{resolve_single_object_for_effect, resolve_bounded_nonnegative_u32};
use crate::effects::{CostExecutableEffect, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;
pub use ironsmith_core::RemoveCountersEffect;

/// Effect that removes counters from a target permanent.
///
/// # Fields
///
/// * `counter_type` - The type of counter to remove
/// * `count` - How many counters to remove
/// * `target` - Which permanent to target
///
/// # Example
///
/// ```ignore
/// // Remove two +1/+1 counters from target creature
/// let effect = RemoveCountersEffect::new(
///     CounterType::PlusOnePlusOne,
///     2,
///     ChooseSpec::creature(),
/// );
/// ```
impl EffectExecutor for RemoveCountersEffect {
    fn supports_simultaneous_player_action(&self) -> bool {
        true
    }

    fn prepare_simultaneous_player_action(
        &self,
        _game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        // Defer removal and its replacement choices to batch commit, where
        // the enclosing each-player checkpoint owns the whole action.
        Ok(Box::new(crate::effects::DeferredPlayerActionProposal {
            effect: crate::effect::Effect::new(self.clone()),
            iterated_player: ctx.iteration.iterated_player,
        }))
    }

    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
        game.clear_pending_decision_controllers();
        let checkpoint = game.clone();
        let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        let result = (|| {
            let target_id = resolve_single_object_for_effect(game, ctx, &self.target)?;
            let requested = resolve_bounded_nonnegative_u32(game, &self.count, ctx, game.counter_count(target_id, self.counter_type))?;
            if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
            let event = crate::events::Event::remove_counters(target_id, self.counter_type, requested)
                .with_provenance(ctx.provenance);
            execute_counter_removal_event(game, ctx, event)
        })();
        if result.is_err() || ctx.decision_maker.awaiting_choice() {
            game.restore_execution_checkpoint(checkpoint, result.is_ok() && ctx.decision_maker.awaiting_choice());
            context_checkpoint.restore(ctx);
            if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
        }
        result
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "target to remove counters from"
    }

    fn cost_description(&self) -> Option<String> {
        if matches!(self.target.base(), ChooseSpec::Source)
            && let Some(count) = self.count.constant_integer()
        {
            let label = self.counter_type.description();
            return Some(if count == 1 {
                format!("Remove a {label} counter from this source")
            } else {
                format!("Remove {} {label} counters from this source", count)
            });
        }
        None
    }
}


pub(crate) fn execute_counter_removal_event(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    event: crate::events::Event,
) -> Result<EffectOutcome, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
        game.clear_pending_decision_controllers();
        let checkpoint = game.clone();
        let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        let result = (|| {
            let removal = crate::events::downcast_event::<crate::events::RemoveCountersEvent>(event.inner())
                .ok_or_else(|| ExecutionError::InternalError("counter-removal owner requires a removal event".into()))?;
            if game.object(removal.target).is_none() { return Ok(EffectOutcome::target_invalid()); }
            if game.is_phased_out(removal.target) { return Ok(EffectOutcome::count(0)); }
            let count = removal.count.min(game.counter_count(removal.target, removal.counter_type));
            if count == 0 { return Ok(EffectOutcome::count(0)); }
            // The instruction's provenance is a causal parent, not this proposal's identity.
            // Several counter groups can be removed by one instruction.
            let parent = event.provenance();
            let proposal = if game.provenance_graph().node(parent).is_some() {
                game.alloc_child_event_provenance(parent, crate::events::EventKind::RemoveCounters)
            } else {
                game.provenance_graph_mut().alloc_root_event(crate::events::EventKind::RemoveCounters)
            };
            let event = event.rewrap(removal.with_count(count)).with_provenance(proposal);
            let processed = crate::events::processing::process_trait_event_with_execution_context(game, event, ctx)?;
            if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
            commit_counter_removal(game, ctx, processed)
        })();
        if result.is_err() || ctx.decision_maker.awaiting_choice() {
            game.restore_execution_checkpoint(checkpoint, result.is_ok() && ctx.decision_maker.awaiting_choice());
            context_checkpoint.restore(ctx);
            if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
        }
        result
}

fn commit_counter_removal(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    processed: crate::events::processing::TraitEventResult,
) -> Result<EffectOutcome, ExecutionError> {
    use crate::events::{RemoveCountersEvent, downcast_event};
    use crate::events::processing::TraitEventResult;
    match processed {
        expanded @ TraitEventResult::Expanded { .. } =>
            crate::effects::replacement::execute_event_expansion_with_targets(
                game, ctx, expanded, commit_counter_removal,
                |_game, context, _outcome| {
                    let removal = downcast_event::<RemoveCountersEvent>(context.event.inner())
                        .ok_or_else(|| ExecutionError::InternalError("added counter-removal program lost its event".into()))?;
                    Ok(Some(vec![crate::effects::ResolvedTarget::Object(removal.target)]))
                },
            ),
        TraitEventResult::Proceed(event) | TraitEventResult::Modified(event) => {
            let removal = downcast_event::<RemoveCountersEvent>(event.inner()).ok_or_else(||
                ExecutionError::InternalError("counter-removal replacement returned an incompatible event".into()))?;
            if game.object(removal.target).is_none() { return Ok(EffectOutcome::target_invalid()); }
            let actual = removal.count.min(game.counter_count(removal.target, removal.counter_type));
            let count = i64::from(actual);
            match game.remove_counters(removal.target, removal.counter_type, removal.count,
                Some(ctx.source), Some(ctx.controller)) {
                Some((removed, mut notification)) => {
                    if removed != actual { return Err(ExecutionError::InternalError(
                        "counter-removal commit disagrees with its resolved amount".into())); }
                    // The committed observation is distinct from the replaceable proposal.
                    let observation = game.alloc_child_event_provenance(
                        event.provenance(), crate::events::EventKind::MarkersChanged);
                    notification = notification.with_provenance(observation);
                    if game.object(ctx.source).is_none() && let Some(snapshot) = &ctx.source_snapshot {
                        notification = notification.with_source_snapshot(snapshot.clone());
                    }
                    Ok(EffectOutcome::count(count).with_event(notification)
                        .with_affected_objects_from_game(game, vec![removal.target]))
                }
                None => Ok(EffectOutcome::count(0)),
            }
        }
        TraitEventResult::Replaced { effects, source, controller, context, .. } => {
            let removal = downcast_event::<RemoveCountersEvent>(context.event.inner()).ok_or_else(||
                ExecutionError::InternalError("counter-removal replacement lost its event".into()))?;
            let payload = crate::effects::replacement::execute_replacement_payload(
                game, ctx, &effects, source, controller, &context,
                Some(vec![crate::effects::ResolvedTarget::Object(removal.target)]),
            )?;
            let mut original = EffectOutcome::replaced();
            original.set_value(crate::effect::OutcomeValue::Count(0));
            Ok(EffectOutcome::aggregate_replacement_outcomes(original, [payload]))
        }
        TraitEventResult::Prevented => {
            let mut outcome = EffectOutcome::prevented();
            outcome.set_value(crate::effect::OutcomeValue::Count(0));
            Ok(outcome)
        }
        TraitEventResult::NeedsChoice { .. } | TraitEventResult::NeedsInteraction { .. } =>
            Err(ExecutionError::InternalError("counter removal suspended without a captured decision".into())),
    }
}

impl CostExecutableEffect for RemoveCountersEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        _controller: crate::ids::PlayerId,
    ) -> Result<(), crate::effects::CostValidationError> {
        if !matches!(self.target.base(), ChooseSpec::Source) {
            return Err(crate::effects::CostValidationError::Other(
                "remove-counters cost supports only source".to_string(),
            ));
        }
        let quantity = self.count.constant_integer().ok_or_else(||
            crate::effects::CostValidationError::Other(
                "remove-counters cost requires representable constant integer arithmetic".to_string()))?;
        let count = u32::try_from(quantity.max(0)).map_err(|_| crate::effects::CostValidationError::Other(
            "remove-counters cost exceeds the unsigned counter range".to_string()))?;
        if game.counter_count(source, self.counter_type) < count {
            return Err(crate::effects::CostValidationError::Other(
                "not enough counters".to_string(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effects::ResolvedTarget;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::test_prelude::*;
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

    #[test]
    fn test_remove_counters() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature_with_counters(
            &mut game,
            "Hangarback Walker",
            alice,
            CounterType::PlusOnePlusOne,
            5,
        );
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = RemoveCountersEffect::plus_one_counters(2, ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        let obj = game.object(creature_id).unwrap();
        assert_eq!(obj.counters.get(&CounterType::PlusOnePlusOne), Some(&3)); // 5 - 2
    }

    #[test]
    fn remove_counters_records_affected_result_memory() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature_with_counters(
            &mut game,
            "Hangarback Walker",
            alice,
            CounterType::PlusOnePlusOne,
            5,
        );
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let result = RemoveCountersEffect::plus_one_counters(2, ChooseSpec::creature())
            .execute(&mut game, &mut ctx)
            .expect("remove counters should resolve");

        assert_eq!(result.affected_objects(), Some([creature_id].as_slice()));
        let memory = result
            .affected_object_memory()
            .expect("counter target memory should be recorded");
        assert_eq!(memory.len(), 1);
        assert_eq!(memory[0].object_id, creature_id);
    }

    #[test]
    fn test_remove_more_than_available() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature_with_counters(
            &mut game,
            "Hangarback Walker",
            alice,
            CounterType::PlusOnePlusOne,
            2,
        );
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = RemoveCountersEffect::plus_one_counters(5, ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Only 2 counters were available to remove
        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        // When all counters are removed, the entry is removed from the HashMap
        assert_eq!(
            game.counter_count(creature_id, CounterType::PlusOnePlusOne),
            0
        );
    }

    #[test]
    fn test_remove_from_no_counters() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature_with_counters(
            &mut game,
            "Grizzly Bears",
            alice,
            CounterType::PlusOnePlusOne,
            0,
        );
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = RemoveCountersEffect::plus_one_counters(1, ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // No counters to remove
        assert_eq!(result.value, crate::effect::OutcomeValue::Count(0));
    }

    #[test]
    fn test_remove_counters_no_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = RemoveCountersEffect::plus_one_counters(1, ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx);

        assert!(result.is_err());
    }

    #[test]
    fn test_remove_counters_from_source_spec() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature_with_counters(
            &mut game,
            "Echo Host",
            alice,
            CounterType::PlusOnePlusOne,
            2,
        );
        let mut ctx = ExecutionContext::new_default(creature_id, alice);

        let effect = RemoveCountersEffect::plus_one_counters(1, ChooseSpec::Source);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1));
        assert_eq!(
            game.counter_count(creature_id, CounterType::PlusOnePlusOne),
            1
        );
    }

    #[test]
    fn test_remove_counters_clone_box() {
        let effect = RemoveCountersEffect::plus_one_counters(1, ChooseSpec::creature());
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("RemoveCountersEffect"));
    }

    #[test]
    fn test_remove_counters_get_target_spec() {
        let effect = RemoveCountersEffect::plus_one_counters(1, ChooseSpec::creature());
        assert!(effect.get_target_spec().is_some());
    }
}

#[cfg(test)]
mod counter_removal_replacement_owner_tests {
    use super::*;

    fn check_replaced_removal(instead: bool) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = crate::ids::PlayerId::from_index(0);
        let definition = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Counter removal source")
            .card_types(vec![crate::types::CardType::Artifact]).build();
        let source = game.create_object_from_definition(&definition, alice, crate::zone::Zone::Battlefield);
        let counter_type = crate::object::CounterType::Charge;
        game.object_mut(source).unwrap().counters.insert(counter_type, 3);
        let action = if instead {
            crate::replacement::ReplacementAction::Instead(vec![crate::effect::Effect::gain_life(2)])
        } else { crate::replacement::ReplacementAction::Prevent };
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(
            crate::replacement::ReplacementEffect::with_matcher(source, alice,
                crate::events::counters::matchers::WouldRemoveCountersMatcher::any(), action));
        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = crate::effect::Effect::new(RemoveCountersEffect::new(counter_type, 2, ChooseSpec::Source));
        let outcome = crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
        assert_eq!(game.counter_count(source, counter_type), 3,
            "counter removal must honor the declared replacement proposal before mutating counters");
        assert_eq!(outcome.count_or_zero(), 0);
        assert_eq!(game.player(alice).unwrap().life, if instead { 22 } else { 20 });
        assert_eq!(outcome.events_of_type::<crate::events::MarkersChangedEvent>().count(), 0);
        assert_eq!(outcome.events_of_type::<crate::events::LifeGainEvent>().count(), usize::from(instead));
        assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
        let next = crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
        assert_eq!(next.count_or_zero(), 2);
        assert_eq!(game.counter_count(source, counter_type), 1);
        assert_eq!(next.events_of_type::<crate::events::MarkersChangedEvent>().count(), 1);
        assert_eq!(game.player(alice).unwrap().life, if instead { 22 } else { 20 });
    }

    #[test]
    fn counter_removal_prevention_reaches_actual_effect_owner() { check_replaced_removal(false); }
    #[test]
    fn counter_removal_instead_program_reaches_actual_effect_owner() { check_replaced_removal(true); }
}

#[cfg(test)]
mod removed_counter_removal_selected_api_tests {
    use super::*;
    #[test]
    fn selected_zero_counter_removal_preserves_one_shot_until_positive_proposal() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = crate::ids::PlayerId::from_index(0);
        let definition = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Selected counter removal source")
            .card_types(vec![crate::types::CardType::Artifact]).build();
        let source = game.create_object_from_definition(&definition, alice, crate::zone::Zone::Battlefield);
        let counter_type = crate::object::CounterType::Charge;
        game.object_mut(source).unwrap().counters.insert(counter_type, 3);
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(
            crate::replacement::ReplacementEffect::with_matcher(source, alice,
                crate::events::counters::matchers::WouldRemoveCountersMatcher::any(),
                crate::replacement::ReplacementAction::Prevent));
        for count in [0, 1] {
            let result = crate::events::processing::process_event_with_chosen_replacement_trait(
                &mut game, crate::events::Event::remove_counters(source, counter_type, count), shield).unwrap();
            if count == 0 {
                let event = result.resolved_event().expect("zero removal retains its proposal without applying prevention");
                let removal = crate::events::downcast_event::<crate::events::RemoveCountersEvent>(event.inner()).unwrap();
                assert_eq!(removal.count, 0);
                assert_eq!(removal.target, source);
                assert_eq!(removal.counter_type, counter_type);
            } else {
                assert!(matches!(result, crate::events::processing::TraitEventResult::Prevented));
            }
            assert_eq!(game.effect_store.replacement_effects.get_effect(shield).is_some(), count == 0);
            assert_eq!(game.counter_count(source, counter_type), 3, "proposal APIs do not commit removal");
            assert!(game.take_pending_trigger_events().is_empty());
        }
    }
}

#[cfg(test)]
mod removed_counter_removal_quantity_tests {
    use super::*;
    fn check_removed_operation(modification: crate::replacement::EventModification) {
        struct PreferSource(crate::ids::ObjectId);
        impl crate::decision::DecisionMaker for PreferSource {
            fn decide_options(&mut self, _game: &GameState, ctx: &crate::decisions::context::SelectOptionsContext) -> Vec<usize> {
                let option = ctx.options.iter().find(|option| option.legal && option.object_id == Some(self.0))
                    .or_else(|| ctx.options.iter().find(|option| option.legal)).unwrap();
                vec![option.index]
            }
        }
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = crate::ids::PlayerId::from_index(0);
        let definition = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Counter removal quantity source")
            .card_types(vec![crate::types::CardType::Artifact]).build();
        let source = game.create_object_from_definition(&definition, alice, crate::zone::Zone::Battlefield);
        let adder = game.create_object_from_definition(&definition, alice, crate::zone::Zone::Battlefield);
        let counter_type = crate::object::CounterType::Charge;
        game.object_mut(source).unwrap().counters.insert(counter_type, 3);
        let register = |source, modification| crate::replacement::ReplacementEffect::with_matcher(source, alice,
            crate::events::counters::matchers::WouldRemoveCountersMatcher::any(),
            crate::replacement::ReplacementAction::Modify(modification));
        let removed = game.effect_store.replacement_effects.add_one_shot_effect(register(source, modification));
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(register(adder, crate::replacement::EventModification::Add(1)));
        let effect = crate::effect::Effect::new(RemoveCountersEffect::new(counter_type, 1, ChooseSpec::Source));
        let mut chooser = PreferSource(source);
        for positive in [false, true] {
            game.take_pending_trigger_events();
            let mut ctx = ExecutionContext::new(source, alice, &mut chooser);
            let outcome = crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
            assert_eq!(outcome.count_or_zero(), if positive { 2 } else { 0 });
            assert_eq!(game.counter_count(source, counter_type), if positive { 1 } else { 3 });
            assert_eq!(outcome.events_of_type::<crate::events::MarkersChangedEvent>().count(), usize::from(positive));
            assert!(game.effect_store.replacement_effects.get_effect(removed).is_none());
            assert_eq!(game.effect_store.replacement_effects.get_effect(shield).is_some(), !positive);
            if !positive { assert!(game.take_pending_trigger_events().is_empty()); }
        }
    }
    #[test]
    fn counter_removal_subtraction_to_zero_preserves_later_increase() {
        check_removed_operation(crate::replacement::EventModification::Subtract(1));
    }
    #[test]
    fn counter_removal_set_to_zero_preserves_later_increase() {
        check_removed_operation(crate::replacement::EventModification::SetTo(0));
    }
}

#[cfg(test)]
mod unsigned_counter_quantity_contract_tests {
    use super::*;
    use crate::card::{CardBuilder,PowerToughness};
    use crate::effect::{Effect,EffectId};
    use crate::effects::{execute_effect,PutCountersEffect,DealDamageEffect};
    use crate::ids::{CardId,PlayerId};
    use crate::object::CounterType;
    use crate::types::CardType;
    use crate::zone::Zone;
    fn fixture()->(GameState,crate::ids::ObjectId,PlayerId) {
        let mut game=crate::tests::test_helpers::setup_two_player_game();let alice=PlayerId::from_index(0);
        let source=game.create_object_from_card(&CardBuilder::new(CardId::new(),"Unsigned counter source").card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(1,1)).build(),alice,Zone::Battlefield);
        (game,source,alice)
    }
    #[test]
    fn unsigned_literal_constructor_preserves_full_counter_quantity() {
        for amount in [i32::MAX as u32,i32::MAX as u32+1,u32::MAX] {
            let (mut game,source,alice)=fixture();let mut ctx=ExecutionContext::new_default(source,alice);
            let outcome=PutCountersEffect::new(CounterType::Charge,amount,ChooseSpec::SpecificObject(source)).execute(&mut game,&mut ctx).unwrap();
            assert_eq!(game.counter_count(source,CounterType::Charge),amount,"unsigned literal must preserve its value");
            assert_eq!(outcome.as_count(),Some(i64::from(amount)));
        }
    }
    #[test]
    fn unsigned_prior_counter_receipt_drives_full_removal() {
        for amount in [i32::MAX as u32,i32::MAX as u32+1,u32::MAX] {
            let (mut game,source,alice)=fixture();let mut ctx=ExecutionContext::new_default(source,alice);ctx.x_value=Some(amount);
            let placed=execute_effect(&mut game,&Effect::with_id(21,Effect::new(PutCountersEffect::new(CounterType::Charge,Value::X,ChooseSpec::SpecificObject(source)))),&mut ctx).unwrap();
            assert_eq!(placed.as_count(),Some(i64::from(amount)));
            let removed=RemoveCountersEffect::new(CounterType::Charge,Value::EffectValue(EffectId(21)),ChooseSpec::SpecificObject(source)).execute(&mut game,&mut ctx).expect("representable unsigned removal must resolve");
            assert_eq!(game.counter_count(source,CounterType::Charge),0);
            assert_eq!(removed.as_count(),Some(i64::from(amount)));
        }
    }
    #[test]
    fn source_counter_prevention_consumes_full_unsigned_follow_up_quantity() {
        for amount in [i32::MAX as u32,i32::MAX as u32+1,u32::MAX] {
            let (mut game,shield,alice)=fixture();
            let attacker=game.create_object_from_card(&CardBuilder::new(CardId::new(),"Damage source").card_types(vec![CardType::Artifact]).build(),alice,Zone::Battlefield);
            game.object_mut(shield).unwrap().counters.insert(CounterType::Charge,amount);
            game.effect_store.replacement_effects.add_resolution_effect(crate::replacement::ReplacementEffect::with_matcher(shield,alice,crate::events::DamageToSelfMatcher::new(),
                crate::replacement::ReplacementAction::PreventDamageByRemovingSourceCounters {counter_type:CounterType::Charge}));
            let mut ctx=ExecutionContext::new_default(attacker,alice);ctx.x_value=Some(amount);
            let outcome=DealDamageEffect::new(Value::X,ChooseSpec::SpecificObject(shield)).execute(&mut game,&mut ctx).unwrap();
            assert_eq!(game.damage_on(shield),0,"shield prevents the unsigned amount");
            assert_eq!(game.counter_count(shield,CounterType::Charge),0,"prevention consumes exactly its unsigned counter budget");
            assert_eq!(outcome.as_count(),Some(0),"prevented original damage remains zero");
            let next=DealDamageEffect::new(1,ChooseSpec::SpecificObject(shield)).execute(&mut game,&mut ctx).unwrap();
            assert_eq!(game.damage_on(shield),1,"spent shield cannot prevent the next damage");
            assert_eq!(next.as_count(),Some(1));
        }
    }
}

#[cfg(test)]
mod wide_bounded_counter_request_tests {
    use super::*;
    use crate::effect::{Effect,EffectId};
    use crate::effects::{execute_effect,MoveAllCountersEffect,MoveCountersEffect};
    use crate::object::CounterType;
    use crate::ids::{CardId,PlayerId};
    use crate::types::CardType;
    use crate::zone::Zone;
    fn object(game:&mut GameState,alice:PlayerId)->crate::ids::ObjectId {
        let card=crate::card::CardBuilder::new(CardId::new(),"Bounded quantity recipient").card_types(vec![CardType::Artifact]).build();
        game.create_object_from_card(&card,alice,Zone::Battlefield)
    }
    fn check(movement:bool) {
        let mut game=crate::tests::test_helpers::setup_two_player_game();let alice=PlayerId::from_index(0);
        let source=object(&mut game,alice);let collected=object(&mut game,alice);let following=object(&mut game,alice);
        for kind in [CounterType::Charge,CounterType::PlusOnePlusOne] {game.object_mut(source).unwrap().counters.insert(kind,u32::MAX);}
        let mut ctx=ExecutionContext::new_default(source,alice);
        let out=execute_effect(&mut game,&Effect::with_id(27,Effect::new(MoveAllCountersEffect::new(ChooseSpec::SpecificObject(source),ChooseSpec::SpecificObject(collected)))),&mut ctx).unwrap();
        assert_eq!(out.as_count(),Some(2*i64::from(u32::MAX)),"real mixed movement produces a wider-than-event receipt");
        let amount=Value::EffectValue(EffectId(27));
        // The targeted transfer consumes its announced pair, as real casting does.
        if movement {ctx.targets=vec![crate::effects::ResolvedTarget::Object(collected),crate::effects::ResolvedTarget::Object(following)];}
        let out=if movement {
            MoveCountersEffect::new(CounterType::Charge,amount,ChooseSpec::SpecificObject(collected),ChooseSpec::SpecificObject(following)).execute(&mut game,&mut ctx)
        } else {
            RemoveCountersEffect::new(CounterType::Charge,amount,ChooseSpec::SpecificObject(collected)).execute(&mut game,&mut ctx)
        }.expect("available counters bound a realizable operation even when requested total exceeds one event range");
        assert_eq!(out.as_count(),Some(i64::from(u32::MAX)));
        assert_eq!(game.counter_count(collected,CounterType::Charge),0);
        assert_eq!(game.counter_count(collected,CounterType::PlusOnePlusOne),u32::MAX,"unrequested kind remains untouched");
        assert_eq!(game.counter_count(following,CounterType::Charge),if movement {u32::MAX} else {0});
        for kind in [CounterType::Charge,CounterType::PlusOnePlusOne] {assert_eq!(game.counter_count(source,kind),0);}
    }
    #[test] fn wide_actual_prior_total_removes_all_available_counters() {check(false);}
    #[test] fn wide_actual_prior_total_moves_all_available_counters() {check(true);}
}
