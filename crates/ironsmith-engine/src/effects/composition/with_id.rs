//! WithId effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::{CostExecutableEffect, CostValidationError, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
pub type WithIdEffect = ironsmith_core::WithIdEffect<crate::effect::Effect>;

/// Effect that executes an inner effect and stores its result with an ID.
///
/// This allows later effects (like `If`) to check the result.
///
/// # Fields
///
/// * `id` - The ID to store the result under
/// * `effect` - The effect to execute
///
/// # Example
///
/// ```ignore
/// // Execute sacrifice and track result for "if you do" clause
/// let effect = WithIdEffect::new(
///     EffectId(0),
///     Effect::sacrifice(ObjectFilter::creature(), 1),
/// );
/// ```
///
/// Wraps an inner proposal so the committed outcome is still recorded under
/// this effect's outcome id, mirroring live execution.
#[derive(Debug)]
struct WithIdProposal {
    id: ironsmith_core::EffectId,
    inner: Box<dyn crate::effects::SimultaneousEffectProposal>,
}

struct RecordOriginalOutcome {
    id: ironsmith_core::EffectId,
    previous: Option<EffectOutcome>,
}

impl super::OriginalOutcomeAdapter for RecordOriginalOutcome {
    fn cancel(self: Box<Self>, _game: &mut GameState, ctx: &mut ExecutionContext) {
        ctx.effect_outcomes.remove(&self.id);
        if let Some(previous) = self.previous { ctx.store_outcome(self.id, previous); }
    }
    fn finish(
        self: Box<Self>,
        _game: &mut GameState,
        ctx: &mut ExecutionContext,
        result: Result<EffectOutcome, ExecutionError>,
    ) -> Result<EffectOutcome, ExecutionError> {
        finish_recording_outcome(ctx, self.id, self.previous, result)
    }
}

/// Prepublication phases temporarily hide this instruction's result. The
/// same-id descendant result becomes live only at original acknowledgement;
/// failed/pending preparation and cost queries restore the incoming slot.
fn with_unpublished_result_slot<T>(
    ctx: &mut ExecutionContext,
    id: ironsmith_core::EffectId,
    body: impl FnOnce(&mut ExecutionContext) -> T,
) -> T {
    let previous = ctx.effect_outcomes.remove(&id);
    let result = body(ctx);
    ctx.effect_outcomes.remove(&id);
    if let Some(previous) = previous {
        ctx.effect_outcomes.insert(id, previous);
    }
    result
}

impl crate::effects::SimultaneousEffectProposal for WithIdProposal {
    fn damage_action_inputs(&self) -> Option<crate::effects::damage::DamageActionInputs> {
        self.inner.damage_action_inputs()
    }

    fn bind_damage_action(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        owner: &crate::effects::CompletedEffectOutputs,
    ) -> Result<crate::effects::DamageActionBinding, ExecutionError> {
        let previous = ctx.effect_outcomes.remove(&self.id);
        let result = self.inner.bind_damage_action(game, ctx, owner);
        match result {
            Ok(binding) => binding
                .project(|outcome| finish_recording_outcome(ctx, self.id, previous, Ok(outcome))),
            Err(error) => finish_recording_outcome(ctx, self.id, previous, Err(error))
                .map(crate::effects::DamageActionBinding::from_outcome),
        }
    }
    fn declared_life_payment(&self) -> Option<(crate::ids::PlayerId, u32)> {
        self.inner.declared_life_payment()
    }

    fn has_simultaneous_originals(&self) -> bool {
        self.inner.has_simultaneous_originals()
    }

    fn nominal_payment_quantity(&self) -> Option<u64> {
        self.inner.nominal_payment_quantity()
    }

    fn declared_payment_resources(&self) -> Vec<crate::effects::PaymentResourceClaim> {
        self.inner.declared_payment_resources()
    }

    fn declared_life_payments(&self) -> Vec<(crate::ids::PlayerId, u32)> {
        self.inner.declared_life_payments()
    }

    fn prepare_selection(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        with_unpublished_result_slot(ctx, self.id, |ctx| self.inner.prepare_selection(game, ctx))
    }

    fn prepare_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        with_unpublished_result_slot(ctx, self.id, |ctx| self.inner.prepare_original(game, ctx))
    }

    fn seal_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        with_unpublished_result_slot(ctx, self.id, |ctx| self.inner.seal_original(game, ctx))
    }

    fn commit_original(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::SimultaneousEffectCommit, ExecutionError> {
        self.commit_original_with_outputs(game, ctx)
            .map(crate::effects::SimultaneousEffectCommit::into_aggregate)
    }

    fn commit_original_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        ExecutionError,
    > {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::SimultaneousEffectCommit::finished(
                crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            ));
        }
        let previous = ctx.effect_outcomes.remove(&self.id);
        match self.inner.commit_original_with_outputs(game, ctx) {
            Ok(receipt) => super::adapt_original_outcome_with_outputs(
                receipt,
                Box::new(RecordOriginalOutcome {
                    id: self.id,
                    previous,
                }),
                game,
                ctx,
            ),
            Err(error) => finish_recording_outcome(ctx, self.id, previous, Err(error))
                .map(crate::effects::CompletedEffectOutputs::aggregate_only)
                .map(crate::effects::SimultaneousEffectCommit::finished),
        }
    }

    fn commit(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        let previous = ctx.effect_outcomes.remove(&self.id);
        let result = self.inner.commit(game, ctx);
        finish_recording_outcome(ctx, self.id, previous, result)
    }
}

/// Publish a result only when the instruction completes. Pending execution
/// restores this result slot just as failure does; replay records it once.
fn finish_recording_outcome(
    ctx: &mut ExecutionContext,
    id: ironsmith_core::EffectId,
    previous: Option<EffectOutcome>,
    result: Result<EffectOutcome, ExecutionError>,
) -> Result<EffectOutcome, ExecutionError> {
    match result {
        Ok(outcome) if !ctx.decision_maker.awaiting_choice() => {
            // A same-id descendant may intentionally own the more specific
            // result. Preserve it after successful live or prepared execution.
            ctx.effect_outcomes
                .entry(id)
                .or_insert_with(|| outcome.clone());
            Ok(outcome)
        }
        unresolved => {
            ctx.effect_outcomes.remove(&id);
            if let Some(previous) = previous {
                ctx.store_outcome(id, previous);
            }
            unresolved
        }
    }
}

impl EffectExecutor for WithIdEffect {
    fn supports_replacement_draw_continuation(&self) -> bool {
        crate::effects::replacement::replacement_effect_supported(&self.effect)
    }
    fn prepare_replacement_draw_continuation_with_outputs(
        &self, game: &mut GameState, ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>, ExecutionError> {
        let cursor = self.select_prepared_action_program(game, ctx)?;
        super::object_iteration::prepare_iteration_continuation(cursor, game, ctx)
    }

    fn supports_prepared_action_program(&self) -> bool {
        self.effect.0.supports_prepared_action_program()
    }
    fn select_prepared_action_program(
        &self,
        _game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Option<Box<dyn crate::effects::ActionProgramCursor>>, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(None);
        }
        let previous = ctx.effect_outcomes.remove(&self.id);
        Ok(Some(super::action_program::adapted_child_program_cursor(
            self.effect.as_ref().clone(),
            Box::new(RecordOriginalOutcome {
                id: self.id,
                previous,
            }),
        )))
    }

    fn transparent_cost_precheck_child_effect(&self) -> Option<&crate::effect::Effect> {
        None
    }

    fn supports_damage_action_cohort(&self) -> bool {
        self.effect.0.supports_damage_action_cohort()
    }
    fn visit_prepared_selection_bindings(
        &self,
        visitor: &mut dyn FnMut(crate::effects::PreparedSelectionBinding),
    ) {
        visitor(crate::effects::PreparedSelectionBinding::Outcome(self.id));
        self.effect.0.visit_prepared_selection_bindings(visitor);
    }

    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        self.effect
            .0
            .as_cost_executable()
            .map(|_| self as &dyn CostExecutableEffect)
    }

    fn cost_description(&self) -> Option<String> { self.effect.0.cost_description() }
    fn references_cost_x(&self) -> bool { self.effect.0.references_cost_x() }
    fn max_cost_x(&self, game: &GameState, source: crate::ids::ObjectId, controller: crate::ids::PlayerId) -> Option<u32> {
        self.effect.0.max_cost_x(game, source, controller)
    }

    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&crate::effect::Effect)) {
        visitor(&self.effect);
    }

    fn transparent_child_effect(&self) -> Option<&crate::effect::Effect> {
        Some(&self.effect)
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn supports_simultaneous_player_action(&self) -> bool {
        self.effect.0.supports_simultaneous_player_action()
    }

    fn is_read_only_simultaneous_player_action(&self) -> bool {
        self.effect.0.is_read_only_simultaneous_player_action()
    }

    fn prepare_simultaneous_player_action(
        &self,
        game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        let inner = with_unpublished_result_slot(ctx, self.id, |ctx| {
            self.effect.prepare_simultaneous_player_action(game, ctx)
        })?;
        Ok(Box::new(WithIdProposal { id: self.id, inner }))
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        execute_with_id_with_outputs(
            self,
            game,
            ctx,
            crate::effects::EffectExecutionPurpose::Action,
        )
    }

    fn get_target_spec(&self) -> Option<&crate::target::ChooseSpec> {
        // Delegate to inner effect
        self.effect.0.get_target_spec()
    }

    fn decision_related_object_specs(&self) -> Vec<crate::target::ChooseSpec> {
        self.effect.0.decision_related_object_specs()
    }

    fn target_description(&self) -> &'static str {
        // Delegate to inner effect
        self.effect.0.target_description()
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        // Delegate to inner effect
        self.effect.0.get_target_count()
    }
}

fn execute_with_id_with_outputs(
    effect: &WithIdEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    purpose: crate::effects::EffectExecutionPurpose,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    if ctx.decision_maker.awaiting_choice() {
        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
            EffectOutcome::count(0),
        ));
    }
    let previous = ctx.effect_outcomes.remove(&effect.id);
    let adapter: Box<dyn super::OriginalOutcomeAdapter> = Box::new(RecordOriginalOutcome {
        id: effect.id,
        previous,
    });
    let result = purpose.execute(game, &effect.effect, ctx);
    adapter.finish_with_outputs(game, ctx, result)
}

impl CostExecutableEffect for WithIdEffect {
    fn cost_choice_candidate_is_eligible(
        &self,
        game: &GameState,
        execution: &mut ExecutionContext,
        reason: crate::costs::PaymentReason,
        tag: &crate::tag::TagKey,
        object: crate::ids::ObjectId,
    ) -> Option<bool> {
        with_unpublished_result_slot(execution, self.id, |execution| {
            self.effect
                .0
                .as_cost_executable()?
                .cost_choice_candidate_is_eligible(game, execution, reason, tag, object)
        })
    }

    fn execute_payment_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        execute_with_id_with_outputs(
            self,
            game,
            ctx,
            crate::effects::EffectExecutionPurpose::Payment,
        )
    }

    fn payment_bindings_are_owned_by_children(&self) -> bool {
        true
    }

    fn supports_prepared_payment(&self) -> bool {
        self.effect
            .0
            .as_cost_executable()
            .is_some_and(|cost| cost.supports_prepared_payment())
    }

    fn prepare_simultaneous_payment(
        &self,
        game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        let inner = with_unpublished_result_slot(ctx, self.id, |ctx| {
            self.effect.prepare_simultaneous_payment(game, ctx)
        })?;
        Ok(Box::new(WithIdProposal { id: self.id, inner }))
    }

    fn can_execute_as_cost_with_context(
        &self,
        game: &GameState,
        ctx: &mut ExecutionContext,
        reason: crate::costs::PaymentReason,
    ) -> Result<(), CostValidationError> {
        with_unpublished_result_slot(ctx, self.id, |ctx| {
            crate::costs::check_effect_cost_program(
                std::slice::from_ref(self.effect.as_ref()),
                game,
                ctx,
                reason,
            )
        })
    }

    fn canonical_cost_effect(&self) -> Option<crate::effect::Effect> {
        let child = self
            .effect
            .0
            .as_cost_executable()?
            .canonical_cost_effect()?;
        let mut replacement = self.clone();
        replacement.effect = Box::new(child);
        Some(crate::effect::Effect::new(replacement))
    }

    fn can_execute_as_cost_with_reason(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
        reason: crate::costs::PaymentReason,
    ) -> Result<(), CostValidationError> {
        let mut ctx = ExecutionContext::new_default(source, controller);
        ctx.x_value = game.object(source).and_then(|object| object.x_value);
        CostExecutableEffect::can_execute_as_cost_with_context(self, game, &mut ctx, reason)
    }

    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
    ) -> Result<(), CostValidationError> {
        CostExecutableEffect::can_execute_as_cost_with_reason(
            self,
            game,
            source,
            controller,
            crate::costs::PaymentReason::Other,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::PlayerId;
    use crate::test_prelude::*;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    #[test]
    fn test_with_id_stores_result() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = WithIdEffect::new(EffectId(0), Effect::gain_life(5));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Result should be returned
        assert_eq!(result.value, crate::effect::OutcomeValue::Count(5));

        // Result should be stored
        let stored = ctx.get_outcome(EffectId(0)).unwrap();
        assert_eq!(stored.value, crate::effect::OutcomeValue::Count(5));
    }

    #[test]
    fn test_with_id_stores_full_outcome() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let outcome = WithIdEffect::new(EffectId(0), Effect::gain_life(5))
            .execute(&mut game, &mut ctx)
            .expect("with id should execute");

        let stored = ctx.get_outcome(EffectId(0)).expect("stored outcome");
        assert_eq!(stored, &outcome);
    }

    #[test]
    fn test_with_id_multiple_effects() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        // Store first effect result
        let effect1 = WithIdEffect::new(EffectId(0), Effect::gain_life(3));
        effect1.execute(&mut game, &mut ctx).unwrap();

        // Store second effect result
        let effect2 = WithIdEffect::new(EffectId(1), Effect::gain_life(7));
        effect2.execute(&mut game, &mut ctx).unwrap();

        // Both should be stored
        assert_eq!(
            ctx.get_outcome(EffectId(0)).unwrap().value,
            crate::effect::OutcomeValue::Count(3)
        );
        assert_eq!(
            ctx.get_outcome(EffectId(1)).unwrap().value,
            crate::effect::OutcomeValue::Count(7)
        );
    }

    #[test]
    fn test_with_id_overwrites() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        // Store first result
        let effect1 = WithIdEffect::new(EffectId(0), Effect::gain_life(3));
        effect1.execute(&mut game, &mut ctx).unwrap();

        // Store second result with same ID
        let effect2 = WithIdEffect::new(EffectId(0), Effect::gain_life(7));
        effect2.execute(&mut game, &mut ctx).unwrap();

        // Should have second result
        assert_eq!(
            ctx.get_outcome(EffectId(0)).unwrap().value,
            crate::effect::OutcomeValue::Count(7)
        );
    }

    #[test]
    fn outer_same_id_wrapper_preserves_descendant_result() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let id = EffectId(7);

        // The terminal coordinated child returns Count(0), while the first
        // child records the successful instruction under the same id. The
        // outer wrapper must not erase that more-specific result.
        let inner = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
            Effect::with_id(id.0, Effect::gain_life(5)),
            Effect::conditional_only(
                crate::effect::Condition::LifeTotalOrLess(-1),
                vec![Effect::draw(1)],
            ),
        ]));
        let outer = WithIdEffect::new(id, inner);

        let outer_outcome = outer
            .execute(&mut game, &mut ctx)
            .expect("outer wrapper should resolve");
        assert_eq!(outer_outcome.value, crate::effect::OutcomeValue::Count(0));
        assert_eq!(
            ctx.get_outcome(id).map(|outcome| &outcome.value),
            Some(&crate::effect::OutcomeValue::Count(5))
        );
    }

    #[test]
    fn outer_wrapper_stores_its_result_without_same_id_descendant() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let id = EffectId(7);

        WithIdEffect::new(id, Effect::gain_life(5))
            .execute(&mut game, &mut ctx)
            .expect("wrapper should resolve");

        assert_eq!(
            ctx.get_outcome(id).map(|outcome| &outcome.value),
            Some(&crate::effect::OutcomeValue::Count(5))
        );
    }

    #[test]
    fn test_with_id_clone_box() {
        let effect = WithIdEffect::new(EffectId(0), Effect::gain_life(1));
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("WithIdEffect"));
    }
}

#[cfg(test)]
mod pending_result_publication_contract_tests {
    use super::*;
    use crate::effect::{Effect, EffectId};
    use crate::effects::PlayerCountersEffect;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::object::CounterType;
    use crate::target::PlayerFilter;
    struct Decisions {
        answer: Option<usize>,
        pending: bool,
        calls: usize,
    }
    impl crate::decision::DecisionMaker for Decisions {
        fn awaiting_choice(&self) -> bool {
            self.pending
        }
        fn decide_options(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            assert!(!self.pending);
            assert_eq!(ctx.player, PlayerId::from_index(1));
            assert_eq!(ctx.options.len(), 2);
            self.calls += 1;
            if let Some(answer) = self.answer.take() {
                vec![answer]
            } else {
                self.pending = true;
                vec![]
            }
        }
    }
    fn fixture() -> (GameState, ObjectId, PlayerId, PlayerId) {
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let card = crate::card::CardBuilder::new(CardId::new(), "Pending result owner")
            .card_types(vec![crate::types::CardType::Artifact])
            .build();
        let source = game.create_object_from_card(&card, alice, crate::zone::Zone::Battlefield);
        (game, source, alice, bob)
    }
    fn run(simultaneous: bool, already_pending: bool) {
        for previous in [false, true] {
            let (mut game, source, alice, bob) = fixture();
            let mut shields = Vec::new();
            for extra in [0, 1] {
                let effect=crate::static_abilities::StaticAbility::add_player_counters_placement_replacement(PlayerFilter::Specific(bob),Some(CounterType::Energy),extra,"Player counter result choice".into()).generate_replacement_effect(source,alice).unwrap();
                shields.push(
                    game.effect_store
                        .replacement_effects
                        .add_one_shot_effect(effect),
                );
            }
            let effect = WithIdEffect::new(
                EffectId(77),
                Effect::new(PlayerCountersEffect::new(
                    CounterType::Energy,
                    1,
                    PlayerFilter::Specific(bob),
                )),
            );
            let mut dm = Decisions {
                answer: None,
                pending: already_pending,
                calls: 0,
            };
            let mut ctx = ExecutionContext::new(source, alice, &mut dm);
            ctx.store_outcome(EffectId(31), EffectOutcome::count(42));
            if previous {
                ctx.store_outcome(EffectId(77), EffectOutcome::count(7));
            }
            let out = if simultaneous {
                effect
                    .prepare_simultaneous_player_action(&game, &mut ctx)
                    .unwrap()
                    .commit(&mut game, &mut ctx)
                    .unwrap()
            } else {
                effect.execute(&mut game, &mut ctx).unwrap()
            };
            assert!(ctx.decision_maker.awaiting_choice());
            assert_eq!(out.as_count(), Some(0));
            assert!(out.events.is_empty());
            assert_eq!(game.player(bob).unwrap().energy_counters, 0);
            assert_eq!(
                ctx.get_outcome(EffectId(77)).map(|out| out.count_or_zero()),
                if previous { Some(7) } else { None },
                "pending work must not publish or overwrite a completed instruction receipt"
            );
            assert_eq!(ctx.get_outcome(EffectId(31)).unwrap().as_count(), Some(42));
            assert!(shields.iter().all(|id| {
                game.effect_store
                    .replacement_effects
                    .get_effect(*id)
                    .is_some()
            }));
            assert!(game.take_pending_trigger_events().is_empty());
            let saved = crate::effects::ExecutionContextCheckpoint::capture(&ctx);
            drop(ctx);
            assert_eq!(dm.calls, usize::from(!already_pending));
            let mut replay = Decisions {
                answer: Some(0),
                pending: false,
                calls: 0,
            };
            let mut ctx = ExecutionContext::new(source, alice, &mut replay);
            saved.restore(&mut ctx);
            let out = if simultaneous {
                effect
                    .prepare_simultaneous_player_action(&game, &mut ctx)
                    .unwrap()
                    .commit(&mut game, &mut ctx)
                    .unwrap()
            } else {
                effect.execute(&mut game, &mut ctx).unwrap()
            };
            assert!(!ctx.decision_maker.awaiting_choice());
            assert_eq!(out.as_count(), Some(2));
            assert_eq!(ctx.get_outcome(EffectId(77)).unwrap().as_count(), Some(2));
            assert_eq!(ctx.get_outcome(EffectId(31)).unwrap().as_count(), Some(42));
            assert_eq!(game.player(bob).unwrap().energy_counters, 2);
            assert!(shields.iter().all(|id| {
                game.effect_store
                    .replacement_effects
                    .get_effect(*id)
                    .is_none()
            }));
            assert_eq!(
                out.events_of_type::<crate::events::MarkersChangedEvent>()
                    .count(),
                1
            );
            assert!(game.take_pending_trigger_events().is_empty());
        }
    }
    #[test]
    fn live_with_id_pending_preserves_absent_and_existing_receipts() {
        run(false, false);
    }
    #[test]
    fn simultaneous_with_id_pending_preserves_absent_and_existing_receipts() {
        run(true, false);
    }
    #[test]
    fn live_with_id_already_pending_does_not_publish() {
        run(false, true);
    }
    #[test]
    fn simultaneous_with_id_already_pending_does_not_publish() {
        run(true, true);
    }
    #[test]
    fn with_id_real_storage_failure_restores_previous_receipt() {
        let (mut game, source, alice, bob) = fixture();
        game.player_mut(bob).unwrap().energy_counters = u32::MAX;
        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.store_outcome(EffectId(77), EffectOutcome::count(7));
        let effect = WithIdEffect::new(
            EffectId(77),
            Effect::new(PlayerCountersEffect::new(
                CounterType::Energy,
                1,
                PlayerFilter::Specific(bob),
            )),
        );
        assert_eq!(
            effect.execute(&mut game, &mut ctx).unwrap_err(),
            ExecutionError::ResourceLimitExceeded {
                resource: "player counter placement",
                requested: u128::from(u32::MAX) + 1,
                maximum: u128::from(u32::MAX)
            }
        );
        assert_eq!(ctx.get_outcome(EffectId(77)).unwrap().as_count(), Some(7));
        assert_eq!(game.player(bob).unwrap().energy_counters, u32::MAX);
        assert!(game.take_pending_trigger_events().is_empty());
    }
}
