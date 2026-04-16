//! Apply replacement effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::{EffectExecutionCategory, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::replacement::ReplacementEffect;
pub use ironsmith_core::ReplacementApplyMode;

/// Effect that registers a replacement effect with the game state.
#[derive(Debug, Clone)]
pub struct ApplyReplacementEffect {
    /// The replacement effect to register.
    pub effect: ReplacementEffect,
    /// How to register it.
    pub mode: ReplacementApplyMode,
}

impl ApplyReplacementEffect {
    pub fn one_shot(effect: ReplacementEffect) -> Self {
        Self {
            effect,
            mode: ReplacementApplyMode::OneShot,
        }
    }

    pub fn resolution(effect: ReplacementEffect) -> Self {
        Self {
            effect,
            mode: ReplacementApplyMode::Resolution,
        }
    }

    pub fn until_end_of_turn(effect: ReplacementEffect) -> Self {
        Self {
            effect,
            mode: ReplacementApplyMode::UntilEndOfTurn,
        }
    }
}

impl EffectExecutor for ApplyReplacementEffect {
    fn execute(
        &self,
        game: &mut GameState,
        _ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        match self.mode {
            ReplacementApplyMode::OneShot => {
                game.effect_store
                    .replacement_effects
                    .add_one_shot_effect(self.effect.clone());
            }
            ReplacementApplyMode::UntilEndOfTurn => {
                game.effect_store
                    .replacement_effects
                    .add_until_end_of_turn_effect(self.effect.clone());
            }
            ReplacementApplyMode::Resolution => {
                game.effect_store
                    .replacement_effects
                    .add_resolution_effect(self.effect.clone());
            }
        }

        Ok(EffectOutcome::resolved())
    }

    fn primary_execution_category(&self) -> EffectExecutionCategory {
        EffectExecutionCategory::ReplacementRegistration
    }
}
