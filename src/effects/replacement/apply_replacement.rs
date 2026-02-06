//! Apply replacement effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::replacement::ReplacementEffect;

/// How to register the replacement effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplacementApplyMode {
    /// Register as a one-shot effect (consumed after one use).
    OneShot,
    /// Register as a resolution effect (persists until removed).
    Resolution,
}

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
}

impl EffectExecutor for ApplyReplacementEffect {
    fn execute(
        &self,
        game: &mut GameState,
        _ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        match self.mode {
            ReplacementApplyMode::OneShot => {
                game.replacement_effects
                    .add_one_shot_effect(self.effect.clone());
            }
            ReplacementApplyMode::Resolution => {
                game.replacement_effects
                    .add_resolution_effect(self.effect.clone());
            }
        }

        Ok(EffectOutcome::resolved())
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}
