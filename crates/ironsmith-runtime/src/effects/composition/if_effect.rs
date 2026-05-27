//! If effect implementation.

use crate::effect::{EffectOutcome, EffectPredicate, EffectPredicateRuntimeExt, ExecutionFact};
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError, execute_effect};
use crate::game_state::GameState;
use crate::target::ChooseSpec;
pub type IfEffect = ironsmith_core::IfEffect<crate::effect::Effect>;

/// Effect that branches based on a prior effect's result.
///
/// Looks up the result of an effect executed with `WithId`, evaluates the predicate,
/// and executes either `then` or `else_` effects.
///
/// # Fields
///
/// * `condition` - The EffectId to check
/// * `predicate` - How to evaluate success
/// * `then` - Effects to execute if predicate is true
/// * `else_` - Effects to execute if predicate is false
///
/// # Example
///
/// ```ignore
/// // "Sacrifice a creature. If you do, draw two cards."
/// let effects = vec![
///     Effect::with_id(EffectId(0), Effect::sacrifice(ObjectFilter::creature(), 1)),
///     Effect::if_then(
///         EffectId(0),
///         EffectPredicate::Happened,
///         vec![Effect::draw(2)],
///     ),
/// ];
/// ```
impl EffectExecutor for IfEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&crate::effect::Effect)) {
        for effect in &self.then {
            visitor(effect);
        }
        for effect in &self.else_ {
            visitor(effect);
        }
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let outcome = ctx
            .get_outcome(self.condition)
            .ok_or(ExecutionError::EffectNotFound(self.condition))?;

        let match_repetitions = if let EffectPredicate::Value(cmp) = &self.predicate {
            let chosen_numbers = outcome
                .execution_facts
                .iter()
                .filter_map(|fact| match fact {
                    ExecutionFact::ChosenNumber(n) => Some(*n as i32),
                    _ => None,
                })
                .collect::<Vec<_>>();
            if chosen_numbers.is_empty() {
                None
            } else {
                let matches = chosen_numbers
                    .into_iter()
                    .filter(|value| cmp.evaluate(*value))
                    .count();
                Some(matches)
            }
        } else {
            None
        };

        let (branch, repetitions) = if let Some(matches) = match_repetitions {
            if matches > 0 {
                (&self.then, matches)
            } else {
                (&self.else_, 1)
            }
        } else if self.predicate.evaluate_outcome(outcome) {
            (&self.then, 1)
        } else {
            (&self.else_, 1)
        };

        let mut outcomes = Vec::new();
        for _ in 0..repetitions {
            for eff in branch {
                outcomes.push(execute_effect(game, eff, ctx)?);
            }
        }
        Ok(EffectOutcome::aggregate(outcomes))
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        super::target_metadata::first_target_spec(&[&self.then, &self.else_])
    }

    fn decision_related_object_specs(&self) -> Vec<ChooseSpec> {
        super::target_metadata::related_object_specs(&[&self.then, &self.else_])
    }

    fn target_description(&self) -> &'static str {
        super::target_metadata::first_target_description(&[&self.then, &self.else_], "target")
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        super::target_metadata::first_target_count(&[&self.then, &self.else_])
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
    fn test_if_then_branch_taken() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        // Simulate a prior effect that "happened"
        ctx.store_outcome(EffectId(0), EffectOutcome::count(1));

        let initial_life = game.player(alice).unwrap().life;

        let effect = IfEffect::if_then(
            EffectId(0),
            EffectPredicate::Happened,
            vec![Effect::gain_life(5)],
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Then branch should execute
        assert_eq!(result.value, crate::effect::OutcomeValue::Count(5));
        assert_eq!(game.player(alice).unwrap().life, initial_life + 5);
    }

    #[test]
    fn test_if_then_branch_not_taken() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        // Simulate a prior effect that didn't happen
        ctx.store_outcome(EffectId(0), EffectOutcome::count(0));

        let initial_life = game.player(alice).unwrap().life;

        let effect = IfEffect::if_then(
            EffectId(0),
            EffectPredicate::Happened,
            vec![Effect::gain_life(5)],
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Then branch should NOT execute (no else branch, so Resolved)
        assert_eq!(result.status, crate::effect::OutcomeStatus::Succeeded);
        assert_eq!(game.player(alice).unwrap().life, initial_life);
    }

    #[test]
    fn test_if_else_branch() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        // Simulate a prior effect that didn't happen
        ctx.store_outcome(EffectId(0), EffectOutcome::count(0));

        let initial_life = game.player(alice).unwrap().life;

        let effect = IfEffect::new(
            EffectId(0),
            EffectPredicate::Happened,
            vec![Effect::gain_life(5)],
            vec![Effect::gain_life(2)], // else branch
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Else branch should execute
        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        assert_eq!(game.player(alice).unwrap().life, initial_life + 2);
    }

    #[test]
    fn test_if_uses_full_outcome_not_only_summary_result() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        ctx.store_outcome(
            EffectId(0),
            EffectOutcome::with_details(
                crate::effect::OutcomeStatus::Succeeded,
                crate::effect::OutcomeValue::Count(0),
                vec![crate::events::RawEvent::new_with_provenance(
                    crate::events::TapEvent { permanent: source },
                    crate::provenance::ProvNodeId::default(),
                )],
                Vec::new(),
            ),
        );

        let initial_life = game.player(alice).unwrap().life;
        let effect = IfEffect::if_then(
            EffectId(0),
            EffectPredicate::Happened,
            vec![Effect::gain_life(5)],
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(5));
        assert_eq!(game.player(alice).unwrap().life, initial_life + 5);
    }

    #[test]
    fn test_if_missing_condition() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        // Don't store any result for EffectId(0)
        let effect = IfEffect::if_then(
            EffectId(0),
            EffectPredicate::Happened,
            vec![Effect::gain_life(5)],
        );
        let result = effect.execute(&mut game, &mut ctx);

        // Should error because the condition effect wasn't found
        assert!(result.is_err());
    }

    #[test]
    fn test_if_clone_box() {
        let effect = IfEffect::if_then(
            EffectId(0),
            EffectPredicate::Happened,
            vec![Effect::gain_life(1)],
        );
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("IfEffect"));
    }

    #[test]
    fn if_effect_forwards_inner_target_spec_from_then_branch() {
        let effect = IfEffect::if_then(
            EffectId(0),
            EffectPredicate::Happened,
            vec![Effect::counter(ChooseSpec::target_spell())],
        );

        assert!(effect.get_target_spec().is_some());
        assert_eq!(effect.target_description(), "spell to counter");
    }

    #[test]
    fn if_effect_forwards_inner_target_spec_from_else_branch() {
        let effect = IfEffect::new(
            EffectId(0),
            EffectPredicate::Happened,
            vec![Effect::draw(1)],
            vec![Effect::counter(ChooseSpec::target_spell())],
        );

        assert!(effect.get_target_spec().is_some());
        assert_eq!(effect.target_description(), "spell to counter");
    }
}
