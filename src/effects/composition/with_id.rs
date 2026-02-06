//! WithId effect implementation.

use crate::effect::{Effect, EffectId, EffectOutcome};
use crate::effects::EffectExecutor;
use crate::executor::{ExecutionContext, ExecutionError, execute_effect};
use crate::game_state::GameState;

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
#[derive(Debug, Clone, PartialEq)]
pub struct WithIdEffect {
    /// The ID to store the result under.
    pub id: EffectId,
    /// The effect to execute.
    pub effect: Box<Effect>,
}

impl WithIdEffect {
    /// Create a new WithId effect.
    pub fn new(id: EffectId, effect: Effect) -> Self {
        Self {
            id,
            effect: Box::new(effect),
        }
    }
}

impl EffectExecutor for WithIdEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let outcome = execute_effect(game, &self.effect, ctx)?;
        ctx.store_result(self.id, outcome.result.clone());
        Ok(outcome)
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn get_target_spec(&self) -> Option<&crate::target::ChooseSpec> {
        // Delegate to inner effect
        self.effect.0.get_target_spec()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::EffectResult;
    use crate::ids::PlayerId;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
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
        assert_eq!(result.result, EffectResult::Count(5));

        // Result should be stored
        let stored = ctx.get_result(EffectId(0)).unwrap();
        assert_eq!(*stored, EffectResult::Count(5));
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
            *ctx.get_result(EffectId(0)).unwrap(),
            EffectResult::Count(3)
        );
        assert_eq!(
            *ctx.get_result(EffectId(1)).unwrap(),
            EffectResult::Count(7)
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
            *ctx.get_result(EffectId(0)).unwrap(),
            EffectResult::Count(7)
        );
    }

    #[test]
    fn test_with_id_clone_box() {
        let effect = WithIdEffect::new(EffectId(0), Effect::gain_life(1));
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("WithIdEffect"));
    }
}
