//! Experience counters effect implementation.

use crate::effect::{EffectOutcome, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::PlayerFilter;

/// Effect that gives a player experience counters.
///
/// # Fields
///
/// * `count` - How many experience counters to add (can be fixed or variable)
/// * `player` - Which player receives the experience counters
///
/// # Example
///
/// ```ignore
/// // Get 1 experience counter
/// let effect = ExperienceCountersEffect::you(1);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ExperienceCountersEffect {
    /// How many experience counters to add.
    pub count: Value,
    /// Which player receives the counters.
    pub player: PlayerFilter,
}

impl ExperienceCountersEffect {
    /// Create a new experience counters effect.
    pub fn new(count: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            count: count.into(),
            player,
        }
    }

    /// Create an effect where you get experience counters.
    pub fn you(count: impl Into<Value>) -> Self {
        Self::new(count, PlayerFilter::You)
    }
}

impl EffectExecutor for ExperienceCountersEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let count = resolve_value(game, &self.count, ctx)?.max(0) as u32;

        if let Some(p) = game.player_mut(player_id) {
            p.experience_counters += count;
        }

        Ok(EffectOutcome::count(count as i32))
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
    fn test_experience_counters() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ExperienceCountersEffect::you(1);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(1));
        assert_eq!(game.player(alice).unwrap().experience_counters, 1);
    }

    #[test]
    fn test_experience_counters_zero() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ExperienceCountersEffect::you(0);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(0));
        assert_eq!(game.player(alice).unwrap().experience_counters, 0);
    }

    #[test]
    fn test_experience_counters_accumulate() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ExperienceCountersEffect::you(1);
        effect.execute(&mut game, &mut ctx).unwrap();
        effect.execute(&mut game, &mut ctx).unwrap();
        effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(game.player(alice).unwrap().experience_counters, 3);
    }

    #[test]
    fn test_experience_counters_clone_box() {
        let effect = ExperienceCountersEffect::you(1);
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("ExperienceCountersEffect"));
    }
}
