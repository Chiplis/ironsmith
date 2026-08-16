//! WithId effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::{CostExecutableEffect, CostValidationError, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError, execute_effect};
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

impl crate::effects::SimultaneousEffectProposal for WithIdProposal {
    fn commit(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let previous = ctx.effect_outcomes.remove(&self.id);
        match self.inner.commit(game, ctx) {
            Ok(outcome) => {
                // A nested effect may intentionally share this result id so a
                // later branch can inspect that specific instruction rather
                // than a transparent outer wrapper's terminal outcome. Keep
                // the nested result when it was produced during this commit.
                ctx.effect_outcomes
                    .entry(self.id)
                    .or_insert_with(|| outcome.clone());
                Ok(outcome)
            }
            Err(error) => {
                ctx.effect_outcomes.remove(&self.id);
                if let Some(previous) = previous {
                    ctx.store_outcome(self.id, previous);
                }
                Err(error)
            }
        }
    }
}

impl EffectExecutor for WithIdEffect {
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        self.effect
            .0
            .as_cost_executable()
            .map(|_| self as &dyn CostExecutableEffect)
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
        let inner = self
            .effect
            .0
            .prepare_simultaneous_player_action(game, ctx)?;
        Ok(Box::new(WithIdProposal { id: self.id, inner }))
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let previous = ctx.effect_outcomes.remove(&self.id);
        match execute_effect(game, &self.effect, ctx) {
            Ok(outcome) => {
                // Preserve a same-id descendant outcome produced while the
                // child ran. This is required when an outer conditional is
                // result-tagged for presentation but its successful action is
                // independently tagged for an exact "if you don't" branch.
                ctx.effect_outcomes
                    .entry(self.id)
                    .or_insert_with(|| outcome.clone());
                Ok(outcome)
            }
            Err(error) => {
                ctx.effect_outcomes.remove(&self.id);
                if let Some(previous) = previous {
                    ctx.store_outcome(self.id, previous);
                }
                Err(error)
            }
        }
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

impl CostExecutableEffect for WithIdEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
    ) -> Result<(), CostValidationError> {
        self.effect.0.can_execute_as_cost(game, source, controller)
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
