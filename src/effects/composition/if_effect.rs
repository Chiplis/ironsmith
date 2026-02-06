//! If effect implementation.

use crate::effect::{Effect, EffectId, EffectOutcome, EffectPredicate};
use crate::effects::EffectExecutor;
use crate::executor::{ExecutionContext, ExecutionError, execute_effect};
use crate::game_state::GameState;

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
#[derive(Debug, Clone, PartialEq)]
pub struct IfEffect {
    /// The effect ID to check (must have been executed with WithId earlier).
    pub condition: EffectId,
    /// How to evaluate success.
    pub predicate: EffectPredicate,
    /// Effects to execute if predicate is true.
    pub then: Vec<Effect>,
    /// Effects to execute if predicate is false.
    pub else_: Vec<Effect>,
}

impl IfEffect {
    /// Create a new If effect.
    pub fn new(
        condition: EffectId,
        predicate: EffectPredicate,
        then: Vec<Effect>,
        else_: Vec<Effect>,
    ) -> Self {
        Self {
            condition,
            predicate,
            then,
            else_,
        }
    }

    /// Create an "if ... then" effect with no else clause.
    pub fn if_then(condition: EffectId, predicate: EffectPredicate, then: Vec<Effect>) -> Self {
        Self::new(condition, predicate, then, vec![])
    }
}

impl EffectExecutor for IfEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let result = ctx
            .get_result(self.condition)
            .ok_or(ExecutionError::EffectNotFound(self.condition))?;

        let branch = if self.predicate.evaluate(result) {
            &self.then
        } else {
            &self.else_
        };

        let mut outcomes = Vec::new();
        for eff in branch {
            outcomes.push(execute_effect(game, eff, ctx)?);
        }
        Ok(EffectOutcome::aggregate(outcomes))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::EffectResult;
    use crate::ids::PlayerId;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    #[test]
    fn test_if_then_branch_taken() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        // Simulate a prior effect that "happened"
        ctx.store_result(EffectId(0), EffectResult::Count(1));

        let initial_life = game.player(alice).unwrap().life;

        let effect = IfEffect::if_then(
            EffectId(0),
            EffectPredicate::Happened,
            vec![Effect::gain_life(5)],
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Then branch should execute
        assert_eq!(result.result, EffectResult::Count(5));
        assert_eq!(game.player(alice).unwrap().life, initial_life + 5);
    }

    #[test]
    fn test_if_then_branch_not_taken() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        // Simulate a prior effect that didn't happen
        ctx.store_result(EffectId(0), EffectResult::Count(0));

        let initial_life = game.player(alice).unwrap().life;

        let effect = IfEffect::if_then(
            EffectId(0),
            EffectPredicate::Happened,
            vec![Effect::gain_life(5)],
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Then branch should NOT execute (no else branch, so Resolved)
        assert_eq!(result.result, EffectResult::Resolved);
        assert_eq!(game.player(alice).unwrap().life, initial_life);
    }

    #[test]
    fn test_if_else_branch() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        // Simulate a prior effect that didn't happen
        ctx.store_result(EffectId(0), EffectResult::Count(0));

        let initial_life = game.player(alice).unwrap().life;

        let effect = IfEffect::new(
            EffectId(0),
            EffectPredicate::Happened,
            vec![Effect::gain_life(5)],
            vec![Effect::gain_life(2)], // else branch
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Else branch should execute
        assert_eq!(result.result, EffectResult::Count(2));
        assert_eq!(game.player(alice).unwrap().life, initial_life + 2);
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
}
