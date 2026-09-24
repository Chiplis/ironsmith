use crate::effect::{EffectOutcome, ExecutionFact};
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;

/// "Choose 1, 2, or 3 at random": picks one listed number with the game's
/// deterministic RNG. The outcome count is the chosen number, so a following
/// "that many" reads it.
#[derive(Debug, Clone, PartialEq)]
pub struct ChooseNumberAtRandomEffect {
    pub choices: Vec<u32>,
}

impl ChooseNumberAtRandomEffect {
    pub fn new(choices: Vec<u32>) -> Self {
        Self { choices }
    }
}

impl EffectExecutor for ChooseNumberAtRandomEffect {
    fn execute(
        &self,
        game: &mut GameState,
        _ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let mut choices = self.choices.clone();
        game.shuffle_slice(&mut choices);
        let Some(&chosen) = choices.first() else {
            return Ok(EffectOutcome::count(0));
        };
        Ok(EffectOutcome::count(chosen as i32).with_execution_fact(ExecutionFact::ChosenNumber(chosen)))
    }
}
