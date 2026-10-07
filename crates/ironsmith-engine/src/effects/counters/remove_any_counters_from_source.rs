//! Remove any number of counters from the source permanent.

use crate::decision::FallbackStrategy;
use crate::decisions::{CounterRemovalSpec, NumberSpec, make_decision_with_fallback};
use crate::effect::EffectOutcome;
use crate::effects::{
    CompletedEffectOutputs, CostExecutableEffect, CostValidationError, EffectExecutor,
};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::object::CounterType;

// Keep the compiler, artifact decoder, and runtime on the same typed payload.
pub use ironsmith_core::RemoveAnyCountersFromSourceEffect;

fn max_removable(
    effect: &RemoveAnyCountersFromSourceEffect,
    game: &GameState,
    source: crate::ids::ObjectId,
) -> Result<u32, String> {
    let obj = game
        .object(source)
        .ok_or_else(|| "source not found".to_string())?;
    if obj.zone != crate::zone::Zone::Battlefield {
        return Err("source must be on the battlefield".to_string());
    }

    Ok(if let Some(counter_type) = effect.counter_type {
        obj.counters.get(&counter_type).copied().unwrap_or(0)
    } else {
        obj.counters
            .values()
            .copied()
            .try_fold(0u32, |total, count| {
                total.checked_add(count).ok_or_else(|| {
                    "counter total exceeds the supported count range".to_string()
                })
            })?
    })
}

impl EffectExecutor for RemoveAnyCountersFromSourceEffect {
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn references_cost_x(&self) -> bool {
        self.display_x
    }

    fn max_cost_x(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        _controller: crate::ids::PlayerId,
    ) -> Option<u32> {
        if !self.references_cost_x() {
            return None;
        }
        max_removable(self, game, source).ok()
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
        if ctx.decision_maker.awaiting_choice() {
            return Ok(CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        game.clear_pending_decision_controllers();
        let result = crate::effects::composition::execute_transaction(
            game,
            ctx,
            || CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| execute_source_counter_removal(self, game, ctx),
        );
        // The shared transaction restores the action; keep this adapter's
        // existing neutral suspension policy even if the child failed.
        if ctx.decision_maker.awaiting_choice() {
            return Ok(CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        result
    }

    fn cost_description(&self) -> Option<String> {
        Some(self.cost_display())
    }
}

enum SourceCounterSelection {
    Finished(EffectOutcome),
    Chosen {
        to_remove: u32,
        selections: Vec<(CounterType, u32)>,
    },
}

/// Select quantity and types once against the pre-action world. Ordinary
/// execution preserves its sequential removals; payment captures these inputs.
fn select_source_counter_removal(
    effect: &RemoveAnyCountersFromSourceEffect,
    game: &GameState,
    ctx: &mut ExecutionContext,
) -> Result<SourceCounterSelection, ExecutionError> {
    let max_removable = max_removable(effect, game, ctx.source)
        .map_err(ExecutionError::Impossible)?;

    let description = if effect.remove_all {
        "Remove all matching counters"
    } else if effect.display_x {
        "Choose X counters to remove"
    } else {
        "Choose counters to remove"
    };
    let to_remove = if effect.remove_all {
        max_removable
    } else if effect.display_x
        && let Some(x_value) = ctx.x_value
    {
        if x_value > max_removable {
            return Err(ExecutionError::Impossible(format!(
                "cannot remove X counters: X is {x_value}, but only {max_removable} counter(s) are available"
            )));
        }
        x_value
    } else {
        let chosen = make_decision_with_fallback(
            game,
            &mut ctx.decision_maker,
            ctx.controller,
            Some(ctx.source),
            NumberSpec::up_to(ctx.source, max_removable, description),
            FallbackStrategy::Maximum,
        );
        if ctx.decision_maker.awaiting_choice() {
            return Ok(SourceCounterSelection::Finished(EffectOutcome::count(0)));
        }
        chosen.min(max_removable)
    };

    if to_remove > 0 && game.is_phased_out(ctx.source) {
        return Ok(SourceCounterSelection::Finished(EffectOutcome::impossible()));
    }
    let selections = if let Some(counter_type) = effect.counter_type {
        vec![(counter_type, to_remove)]
    } else {
        let available_counters: Vec<(CounterType, u32)> = game
            .object(ctx.source)
            .map(|object| {
                object
                    .counters
                    .iter()
                    .filter(|(_, count)| **count > 0)
                    .map(|(counter_type, count)| (*counter_type, *count))
                    .collect()
            })
            .unwrap_or_default();
        make_decision_with_fallback(
            game,
            &mut ctx.decision_maker,
            ctx.controller,
            Some(ctx.source),
            CounterRemovalSpec::new(ctx.source, ctx.source, to_remove, available_counters),
            FallbackStrategy::Maximum,
        )
    };
    if ctx.decision_maker.awaiting_choice() {
        return Ok(SourceCounterSelection::Finished(EffectOutcome::count(0)));
    }
    Ok(SourceCounterSelection::Chosen {
        to_remove,
        selections,
    })
}

fn execute_source_counter_removal(
    effect: &RemoveAnyCountersFromSourceEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Result<CompletedEffectOutputs, ExecutionError> {
    let (to_remove, selections) = match select_source_counter_removal(effect, game, ctx)? {
        SourceCounterSelection::Finished(outcome) => {
            return Ok(CompletedEffectOutputs::aggregate_only(outcome));
        }
        SourceCounterSelection::Chosen {
            to_remove,
            selections,
        } => (to_remove, selections),
    };
    let mut selected_total = 0u32;
    let mut removed_total = 0u64;
    let mut outcomes = Vec::new();
    for (counter_type, requested) in selections {
        if selected_total >= to_remove {
            break;
        }
        let amount = requested.min(to_remove - selected_total);
        if amount == 0 {
            continue;
        }
        let event = crate::events::Event::remove_counters(ctx.source, counter_type, amount)
            .with_provenance(ctx.provenance);
        let outcome = super::execute_counter_removal_with_outputs(game, ctx, event)?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        let removed = u32::try_from(outcome.outcome.count_or_zero()).map_err(|_| {
            ExecutionError::InternalError("counter-removal outcome has an invalid count".into())
        })?;
        removed_total = removed_total
            .checked_add(u64::from(removed))
            .ok_or_else(|| {
                ExecutionError::InternalError(
                    "counter-removal total exceeds the supported count range".into(),
                )
            })?;
        // Replacements can change the physical amount. The chosen budget
        // counts authored actions, while the returned result counts removals.
        selected_total += amount;
        outcomes.push(outcome);
    }
    if selected_total != to_remove {
        return Err(ExecutionError::Impossible(
            "counter-removal selection did not fulfill the chosen amount".into(),
        ));
    }
    let count = i64::try_from(removed_total).map_err(|_| {
        ExecutionError::InternalError(
            "counter-removal total exceeds the supported outcome range".into(),
        )
    })?;
    let mut outcome = EffectOutcome::aggregate(outcomes.iter().map(|child| child.outcome.clone()));
    outcome.set_value(crate::effect::OutcomeValue::Count(count));
    let mut outputs =
        CompletedEffectOutputs::aggregate_only(outcome.with_requested_amount(to_remove));
    for child in outcomes {
        outputs.retain_owned_child(child);
    }
    Ok(outputs)
}

impl CostExecutableEffect for RemoveAnyCountersFromSourceEffect {
    fn supports_prepared_payment(&self) -> bool {
        true
    }

    fn prepare_simultaneous_payment(
        &self,
        game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        let (to_remove, selections) = match select_source_counter_removal(self, game, ctx)? {
            SourceCounterSelection::Finished(outcome) => {
                if !ctx.decision_maker.awaiting_choice() {
                    return Err(ExecutionError::Impossible(
                        "source counter payment cannot be prepared".into(),
                    ));
                }
                return Ok(Box::new(super::placement::PreparedCounterCost::Finished(
                    outcome,
                )));
            }
            SourceCounterSelection::Chosen {
                to_remove,
                selections,
            } => (to_remove, selections),
        };
        let mut selected_total = 0u32;
        let mut events = Vec::new();
        for (counter_type, requested) in selections {
            if selected_total >= to_remove {
                break;
            }
            let amount = requested.min(to_remove - selected_total);
            if amount == 0 {
                continue;
            }
            events.push(
                crate::events::Event::remove_counters(ctx.source, counter_type, amount)
                    .with_provenance(ctx.provenance),
            );
            selected_total += amount;
        }
        if selected_total != to_remove {
            return Err(ExecutionError::Impossible(
                "counter-removal selection did not fulfill the chosen amount".into(),
            ));
        }
        super::capture_counter_payment_with_quantity(game, ctx, events)
    }

    fn accepts_prepared_payment(
        &self,
        proposal: &dyn crate::effects::SimultaneousEffectProposal,
    ) -> bool {
        super::prepared_payment::accepts_counter_quantity_payment(proposal, self.counter_type)
    }

    fn payment_x_from_prepared_payment(
        &self,
        proposal: &dyn crate::effects::SimultaneousEffectProposal,
        _execution: &ExecutionContext,
    ) -> Result<Option<u32>, crate::effects::CostValidationError> {
        super::prepared_payment::counter_quantity_payment_x(proposal, self.counter_type)
    }

    fn validate_payment_outcome(
        &self,
        outcome: &EffectOutcome,
    ) -> Result<(), crate::effects::CostValidationError> {
        super::prepared_payment::validate_counter_quantity_payment(outcome)
    }

    fn payment_x_from_outcome(
        &self,
        outcome: &EffectOutcome,
        execution: &ExecutionContext,
    ) -> Result<Option<u32>, CostValidationError> {
        super::counter_cost_x_from_outcome(outcome, execution)
    }

    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        _controller: crate::ids::PlayerId,
    ) -> Result<(), CostValidationError> {
        max_removable(self, game, source)
            .map(|_| ())
            .map_err(CostValidationError::Other)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::costs::{Cost, CostContext, CostPaymentResult};
    use crate::ids::{CardId, PlayerId};
    use crate::types::CardType;
    use crate::{card::CardBuilder, game_state::GameState, zone::Zone};

    fn create_test_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn simple_card(name: &str, id: u32) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(id), name)
            .card_types(vec![CardType::Artifact])
            .build()
    }

    #[test]
    fn dynamic_x_counter_cost_keeps_plural_counter_noun() {
        assert_eq!(
            RemoveAnyCountersFromSourceEffect::x(Some(CounterType::Ki)).cost_display(),
            "Remove X ki counters from this source"
        );
    }

    #[test]
    fn display_text() {
        assert_eq!(
            RemoveAnyCountersFromSourceEffect::any_number(Some(CounterType::Charge)).cost_display(),
            "Remove any number of charge counters from this source"
        );
        assert_eq!(
            RemoveAnyCountersFromSourceEffect::x(Some(CounterType::Storage)).cost_display(),
            "Remove X storage counters from this source"
        );
        assert_eq!(
            RemoveAnyCountersFromSourceEffect::all(Some(CounterType::Charge)).cost_display(),
            "Remove all charge counters from this source"
        );
    }

    #[test]
    fn pay_sets_x() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let card = simple_card("Battery", 1);
        let card_id = game.create_object_from_card(&card, alice, Zone::Battlefield);
        if let Some(obj) = game.object_mut(card_id) {
            obj.counters.insert(CounterType::Charge, 3);
        }

        let cost = Cost::effect(RemoveAnyCountersFromSourceEffect::any_number(Some(
            CounterType::Charge,
        )));
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx = CostContext::new(card_id, alice, &mut dm);

        let result = cost.pay(&mut game, &mut ctx);
        assert_eq!(result, Ok(CostPaymentResult::Paid));
        assert_eq!(ctx.x_value, Some(3));
        assert_eq!(game.counter_count(card_id, CounterType::Charge), 0);
    }

    #[test]
    fn pay_all_sets_x_to_all_removed_counters() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let card = simple_card("Relic", 1);
        let card_id = game.create_object_from_card(&card, alice, Zone::Battlefield);
        if let Some(obj) = game.object_mut(card_id) {
            obj.counters.insert(CounterType::Charge, 4);
        }

        let cost = Cost::effect(RemoveAnyCountersFromSourceEffect::all(Some(
            CounterType::Charge,
        )));
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx = CostContext::new(card_id, alice, &mut dm);

        let result = cost.pay(&mut game, &mut ctx);
        assert_eq!(result, Ok(CostPaymentResult::Paid));
        assert_eq!(ctx.x_value, Some(4));
        assert_eq!(game.counter_count(card_id, CounterType::Charge), 0);
    }

    #[test]
    fn pay_x_uses_existing_cost_x() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);

        let card = simple_card("Marath Stand-In", 1);
        let card_id = game.create_object_from_card(&card, alice, Zone::Battlefield);
        if let Some(obj) = game.object_mut(card_id) {
            obj.counters.insert(CounterType::PlusOnePlusOne, 4);
        }

        let cost = Cost::effect(RemoveAnyCountersFromSourceEffect::x(Some(
            CounterType::PlusOnePlusOne,
        )));
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx = CostContext::new(card_id, alice, &mut dm).with_x(3);

        let result = cost.pay(&mut game, &mut ctx);
        assert_eq!(result, Ok(CostPaymentResult::Paid));
        assert_eq!(ctx.x_value, Some(3));
        assert_eq!(game.counter_count(card_id, CounterType::PlusOnePlusOne), 1);
    }
}

#[cfg(test)]
mod mixed_source_removal_replacement_tests {
    use super::*;
    fn check_mixed(mode: u8) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = crate::ids::PlayerId::from_index(0);
        let definition = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Mixed removal source")
            .card_types(vec![crate::types::CardType::Artifact]).build();
        let source = game.create_object_from_definition(&definition, alice, crate::zone::Zone::Battlefield);
        let charge = CounterType::Charge;
        let other = CounterType::PlusOnePlusOne;
        game.object_mut(source).unwrap().counters.insert(charge, 3);
        game.object_mut(source).unwrap().counters.insert(other, 2);
        let kinds = if mode == 3 { vec![charge] } else { vec![charge, other] };
        let mut shields = Vec::new();
        for kind in kinds {
            let action = match mode {
                0 | 3 => crate::replacement::ReplacementAction::Prevent,
                1 => crate::replacement::ReplacementAction::Instead(vec![crate::effect::Effect::gain_life(2)]),
                _ => crate::replacement::ReplacementAction::Modify(crate::replacement::EventModification::Subtract(1)),
            };
            shields.push(game.effect_store.replacement_effects.add_one_shot_effect(
                crate::replacement::ReplacementEffect::with_matcher(source, alice,
                    crate::events::counters::matchers::WouldRemoveCountersMatcher::new(crate::target::ObjectFilter::permanent(), Some(kind)), action)));
        }
        let effect = crate::effect::Effect::new(RemoveAnyCountersFromSourceEffect::all(None));
        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
        let expected_charge = if mode == 2 { 1 } else { 3 };
        let expected_other = if mode == 2 { 1 } else if mode == 3 { 0 } else { 2 };
        assert_eq!(game.counter_count(source, charge), expected_charge);
        assert_eq!(game.counter_count(source, other), expected_other);
        assert_eq!(outcome.count_or_zero(), (5 - expected_charge - expected_other) as i64);
        assert_eq!(outcome.events_of_type::<crate::events::MarkersChangedEvent>().count(), if mode == 2 { 2 } else { usize::from(mode == 3) });
        assert_eq!(outcome.events_of_type::<crate::events::LifeGainEvent>().count(), if mode == 1 { 2 } else { 0 });
        assert_eq!(game.player(alice).unwrap().life, if mode == 1 { 24 } else { 20 });
        assert!(shields.iter().all(|shield| game.effect_store.replacement_effects.get_effect(*shield).is_none()));
        let next = crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
        assert_eq!(next.count_or_zero(), (expected_charge + expected_other) as i64);
        assert_eq!(game.counter_count(source, charge), 0);
        assert_eq!(game.counter_count(source, other), 0);
        assert_eq!(next.events_of_type::<crate::events::MarkersChangedEvent>().count(), if mode == 3 { 1 } else { 2 });
        assert_eq!(next.events_of_type::<crate::events::LifeGainEvent>().count(), 0);
        assert_eq!(game.player(alice).unwrap().life, if mode == 1 { 24 } else { 20 });
    }
    #[test]
    fn mixed_source_removal_preserves_both_prevented_groups() { check_mixed(0); }
    #[test]
    fn mixed_source_removal_executes_each_independent_instead_program() { check_mixed(1); }
    #[test]
    fn mixed_source_removal_keeps_chosen_budget_separate_from_modified_amounts() { check_mixed(2); }
    #[test]
    fn mixed_source_removal_retains_unaffected_group_after_prevention() { check_mixed(3); }
}

#[cfg(test)]
mod mixed_removal_transaction_tests {
    use super::*;
    #[derive(Debug, Clone)]
    struct FailRemovalProgram;
    impl EffectExecutor for FailRemovalProgram {
        fn execute(&self, _game: &mut GameState, _ctx: &mut ExecutionContext) -> Result<EffectOutcome, ExecutionError> {
            Err(ExecutionError::InternalError("injected removal program failure".into()))
        }
    }
    struct OrderedRemovalChoices { pause: bool, pending: bool, choices: usize }
    impl crate::decision::DecisionMaker for OrderedRemovalChoices {
        fn awaiting_choice(&self) -> bool { self.pending }
        fn decide_counters(&mut self, _game: &GameState, _ctx: &crate::decisions::context::CountersContext) -> Vec<(CounterType,u32)> {
            vec![(CounterType::Charge,3),(CounterType::PlusOnePlusOne,2)]
        }
        fn decide_options(&mut self, game: &GameState, ctx: &crate::decisions::context::SelectOptionsContext) -> Vec<usize> {
            self.choices += 1;
            assert_eq!(game.player(crate::ids::PlayerId::from_index(0)).unwrap().life,21,
                "earlier replacement program must execute before the later group choice");
            if self.pause { self.pending=true; Vec::new() }
            else { ctx.options.iter().filter(|option| option.legal).take(ctx.min).map(|option| option.index).collect() }
        }
    }
    fn check_transaction(pause: bool, owner: u8) {
        let mut game=crate::tests::test_helpers::setup_two_player_game();
        let alice=crate::ids::PlayerId::from_index(0);
        let definition=crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(),"Removal transaction source")
            .card_types(vec![crate::types::CardType::Artifact]).build();
        let source=game.create_object_from_definition(&definition,alice,crate::zone::Zone::Battlefield);
        game.object_mut(source).unwrap().counters.insert(CounterType::Charge,3);
        game.object_mut(source).unwrap().counters.insert(CounterType::PlusOnePlusOne,2);
        let actions=if pause { vec![
            (CounterType::Charge,crate::replacement::ReplacementAction::Instead(vec![crate::effect::Effect::gain_life(1)])),
            (CounterType::PlusOnePlusOne,crate::replacement::ReplacementAction::Modify(crate::replacement::EventModification::Add(0))),
            (CounterType::PlusOnePlusOne,crate::replacement::ReplacementAction::Modify(crate::replacement::EventModification::Subtract(1))),
        ] } else { vec![
            (CounterType::Charge,crate::replacement::ReplacementAction::Instead(vec![crate::effect::Effect::gain_life(1)])),
            (CounterType::PlusOnePlusOne,crate::replacement::ReplacementAction::Instead(vec![crate::effect::Effect::new(FailRemovalProgram)])),
        ] };
        let shields:Vec<_>=actions.into_iter().map(|(kind,action)|game.effect_store.replacement_effects.add_one_shot_effect(
            crate::replacement::ReplacementEffect::with_matcher(source,alice,
                crate::events::counters::matchers::WouldRemoveCountersMatcher::new(crate::target::ObjectFilter::permanent(),Some(kind)),action))).collect();
        let mut filter = crate::target::ObjectFilter::permanent(); filter.source = true;
        let effect = match owner {
            0 => crate::effect::Effect::new(RemoveAnyCountersFromSourceEffect::all(None)),
            1 => crate::effect::Effect::new(crate::effects::RemoveUpToAnyCountersEffect::exact(5, crate::target::ChooseSpec::Source)),
            _ => crate::effect::Effect::new(crate::effects::RemoveAnyCountersAmongEffect::new(5, filter)),
        };
        let mut decisions=OrderedRemovalChoices{pause,pending:false,choices:0};
        let result={let mut ctx=ExecutionContext::new(source,alice,&mut decisions);crate::effects::execute_effect(&mut game,&effect,&mut ctx)};
        if pause {
            assert!(decisions.pending,"later replacement choice must suspend the whole removal instruction");
            let outcome=result.unwrap();assert_eq!(outcome.count_or_zero(),0);assert!(outcome.events.is_empty());
        } else {
            assert!(matches!(result,Err(ExecutionError::InternalError(ref message)) if message=="injected removal program failure"),
                "later replacement program failure must propagate");
        }
        assert_eq!(game.player(alice).unwrap().life,20);
        assert_eq!(game.counter_count(source,CounterType::Charge),3);
        assert_eq!(game.counter_count(source,CounterType::PlusOnePlusOne),2);
        assert!(shields.iter().all(|id|game.effect_store.replacement_effects.get_effect(*id).is_some()));
        if pause {
            decisions.pending=false;decisions.pause=false;
            let outcome={let mut ctx=ExecutionContext::new(source,alice,&mut decisions);crate::effects::execute_effect(&mut game,&effect,&mut ctx).unwrap()};
            assert_eq!(outcome.count_or_zero(),1);assert_eq!(game.player(alice).unwrap().life,21);
            assert_eq!(game.counter_count(source,CounterType::Charge),3);
            assert_eq!(game.counter_count(source,CounterType::PlusOnePlusOne),1);
            assert_eq!(outcome.events_of_type::<crate::events::LifeGainEvent>().count(),1);
            assert_eq!(outcome.events_of_type::<crate::events::MarkersChangedEvent>().count(),1);
            assert_eq!(decisions.choices,2);
            assert!(shields.iter().all(|id|game.effect_store.replacement_effects.get_effect(*id).is_none()));
        }
    }
    #[test]
    fn mixed_removal_pending_later_group_restores_earlier_program_and_shields() {check_transaction(true,0);}
    #[test]
    fn mixed_removal_failed_later_group_restores_earlier_program_and_shields() {check_transaction(false,0);}
    #[test]
    fn up_to_any_removal_pending_later_group_restores_instruction() {check_transaction(true,1);}
    #[test]
    fn up_to_any_removal_failed_later_group_restores_instruction() {check_transaction(false,1);}
    #[test]
    fn distributed_removal_pending_later_group_restores_instruction() {check_transaction(true,2);}
    #[test]
    fn distributed_removal_failed_later_group_restores_instruction() {check_transaction(false,2);}
}

#[cfg(test)]
mod distributed_removal_owner_tests {
    use super::*;
    fn check_owner(owner:u8,instead:bool) {
        let mut game=crate::tests::test_helpers::setup_two_player_game();
        let alice=crate::ids::PlayerId::from_index(0);
        let definition=crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(),"Distributed removal source")
            .card_types(vec![crate::types::CardType::Artifact]).build();
        let source=game.create_object_from_definition(&definition,alice,crate::zone::Zone::Battlefield);
        game.object_mut(source).unwrap().counters.insert(CounterType::Charge,3);
        game.object_mut(source).unwrap().counters.insert(CounterType::PlusOnePlusOne,2);
        let action=if instead{crate::replacement::ReplacementAction::Instead(vec![crate::effect::Effect::gain_life(2)])}
            else{crate::replacement::ReplacementAction::Prevent};
        let shield=game.effect_store.replacement_effects.add_one_shot_effect(crate::replacement::ReplacementEffect::with_matcher(source,alice,
            crate::events::counters::matchers::WouldRemoveCountersMatcher::new(crate::target::ObjectFilter::permanent(),Some(CounterType::Charge)),action));
        let mut filter=crate::target::ObjectFilter::permanent();filter.source=true;
        let effect=match owner {
            0=>crate::effect::Effect::new(crate::effects::RemoveUpToAnyCountersEffect::new(3,crate::target::ChooseSpec::Source)),
            1=>crate::effect::Effect::new(crate::effects::RemoveAnyCountersAmongEffect::new(3,filter).with_counter_type(Some(CounterType::Charge))),
            _=>crate::effect::Effect::new(crate::effects::RemoveAnyCountersAmongEffect::new(5,filter)),
        };
        struct Choices;
        impl crate::decision::DecisionMaker for Choices {
            fn decide_counters(&mut self,_game:&GameState,ctx:&crate::decisions::context::CountersContext)->Vec<(CounterType,u32)> {
                let mut remaining=ctx.max_total;let mut chosen=Vec::new();
                for kind in [CounterType::Charge,CounterType::PlusOnePlusOne] {
                    let available=ctx.available_counters.iter().find(|(k,_)|*k==kind).map(|(_,n)|*n).unwrap_or(0);
                    let count=available.min(u32::try_from(remaining).unwrap_or(u32::MAX));if count>0{chosen.push((kind,count));remaining-=u64::from(count);}
                }chosen
            }
        }
        let mut dm=Choices;let mut ctx=ExecutionContext::new(source,alice,&mut dm);
        let outcome=crate::effects::execute_effect(&mut game,&effect,&mut ctx).unwrap();
        assert_eq!(game.counter_count(source,CounterType::Charge),3,"actual owner must apply the charge-counter replacement");
        assert_eq!(game.counter_count(source,CounterType::PlusOnePlusOne),if owner==2{0}else{2});
        assert_eq!(outcome.count_or_zero(),if owner==2{2}else{0});
        assert_eq!(outcome.events_of_type::<crate::events::MarkersChangedEvent>().count(),usize::from(owner==2));
        assert_eq!(outcome.events_of_type::<crate::events::LifeGainEvent>().count(),usize::from(instead));
        assert_eq!(game.player(alice).unwrap().life,if instead{22}else{20});
        assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
        let next=crate::effects::execute_effect(&mut game,&effect,&mut ctx).unwrap();
        assert_eq!(next.count_or_zero(),if owner==0{3}else if owner==1{3}else{0});
        // The distributed exact-five instruction is no longer affordable after its unaffected group was removed.
        assert_eq!(game.counter_count(source,CounterType::Charge),if owner==2{3}else{0});
        assert_eq!(game.player(alice).unwrap().life,if instead{22}else{20});
    }
    #[test] fn up_to_any_counter_removal_applies_prevention(){check_owner(0,false);}
    #[test] fn up_to_any_counter_removal_executes_instead(){check_owner(0,true);}
    #[test] fn distributed_typed_removal_applies_prevention(){check_owner(1,false);}
    #[test] fn distributed_typed_removal_executes_instead(){check_owner(1,true);}
    #[test] fn distributed_mixed_removal_preserves_unaffected_group(){check_owner(2,false);}
    #[test] fn distributed_mixed_removal_executes_instead_and_unaffected_group(){check_owner(2,true);}
}

#[cfg(test)]
mod removal_observation_identity_tests {
    use super::*;
    fn check(owner:u8,rooted:bool,history_first:bool) {
        let mut game=crate::tests::test_helpers::setup_two_player_game();
        let alice=crate::ids::PlayerId::from_index(0);
        let definition=crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(),"Removal observation source")
            .card_types(vec![crate::types::CardType::Artifact]).build();
        let source=game.create_object_from_definition(&definition,alice,crate::zone::Zone::Battlefield);
        game.object_mut(source).unwrap().counters.insert(CounterType::Charge,3);
        game.object_mut(source).unwrap().counters.insert(CounterType::PlusOnePlusOne,2);
        let parent=if rooted {game.provenance_graph_mut().alloc_root(crate::provenance::ProvenanceNodeKind::EffectExecution{source,controller:alice})}
            else {crate::provenance::ProvNodeId::default()};
        assert_eq!(parent!=crate::provenance::ProvNodeId::default(),rooted);
        let mut ctx=ExecutionContext::new_default(source,alice).with_provenance(parent);
        let mut filter=crate::target::ObjectFilter::permanent();filter.source=true;
        let events=if owner==3 {
            let effect=crate::effect::Effect::new(crate::effects::RemoveCountersEffect::new(CounterType::Charge,1,crate::target::ChooseSpec::Source));
            let first=crate::effects::execute_effect(&mut game,&effect,&mut ctx).unwrap();
            let second=crate::effects::execute_effect(&mut game,&effect,&mut ctx).unwrap();
            assert_eq!(first.count_or_zero()+second.count_or_zero(),2);
            assert_eq!(game.counter_count(source,CounterType::Charge),1);
            assert_eq!(game.counter_count(source,CounterType::PlusOnePlusOne),2);
            first.events.into_iter().chain(second.events).collect::<Vec<_>>()
        } else {
            let effect=match owner {
                0=>crate::effect::Effect::new(RemoveAnyCountersFromSourceEffect::all(None)),
                1=>crate::effect::Effect::new(crate::effects::RemoveUpToAnyCountersEffect::exact(5,crate::target::ChooseSpec::Source)),
                _=>crate::effect::Effect::new(crate::effects::RemoveAnyCountersAmongEffect::new(5,filter)),
            };
            let outcome=crate::effects::execute_effect(&mut game,&effect,&mut ctx).unwrap();
            assert_eq!(outcome.count_or_zero(),5);
            assert_eq!(game.counter_count(source,CounterType::Charge),0);
            assert_eq!(game.counter_count(source,CounterType::PlusOnePlusOne),0);
            outcome.events
        };
        assert_eq!(events.iter().filter(|event|event.downcast::<crate::events::MarkersChangedEvent>().is_some()).count(),2);
        let kind=crate::events::EventKind::MarkersChanged;
        let identities=events.iter().map(|event|event.provenance()).collect::<std::collections::HashSet<_>>();
        if history_first {assert_eq!(game.turn_store.turn_history.event_kind_count(kind),2,"both committed removal observations must remain visible before notification queuing");}
        assert_eq!(identities.len(),2,"independent committed removals must not reuse their instruction parent as event identity");
        assert_eq!(game.turn_store.turn_history.event_kind_count(kind),2);
        for event in &events {
            let observation = game.provenance_graph().node(event.provenance()).unwrap();
            assert_eq!(observation.kind, crate::provenance::ProvenanceNodeKind::DerivedEvent { kind });
            let proposal = game.provenance_graph().node(observation.parent.unwrap()).unwrap();
            assert!(matches!(proposal.kind,
                crate::provenance::ProvenanceNodeKind::RootEvent { kind: crate::events::EventKind::RemoveCounters }
                | crate::provenance::ProvenanceNodeKind::DerivedEvent { kind: crate::events::EventKind::RemoveCounters }));
            assert_eq!(proposal.parent, rooted.then_some(parent));
            if rooted { assert!(game.provenance_graph().is_descendant_of(event.provenance(), parent)); }
        }
        for event in events {game.queue_trigger_event(parent,event);}
        let queued=game.take_pending_trigger_events();assert_eq!(queued.len(),2);
        assert_eq!(game.turn_store.turn_history.event_kind_count(kind),2);
        for event in &queued {game.record_turn_history_event(event);}
        assert_eq!(game.turn_store.turn_history.event_kind_count(kind),2);
    }
    #[test]fn source_any_removal_allocates_distinct_observation_ids(){check(0,true,false);}
    #[test]fn source_any_removal_preserves_each_staged_observation(){check(0,true,true);}
    #[test]fn up_to_any_removal_allocates_distinct_observation_ids(){check(1,true,false);}
    #[test]fn up_to_any_removal_preserves_each_staged_observation(){check(1,true,true);}
    #[test]fn distributed_removal_allocates_distinct_observation_ids(){check(2,true,false);}
    #[test]fn distributed_removal_preserves_each_staged_observation(){check(2,true,true);}
    #[test]fn successive_direct_removals_allocate_distinct_observation_ids(){check(3,true,false);}
    #[test]fn successive_direct_removals_preserve_each_staged_observation(){check(3,true,true);}
    #[test]fn anonymous_removal_instruction_observation_controls(){for owner in 0..4{check(owner,false,false);}}
}
