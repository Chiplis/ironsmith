//! Apply continuous effect implementation.

use crate::continuous::{ContinuousEffect, EffectSourceType, EffectTarget, Modification};
use crate::effect::{EffectOutcome, Until};
use crate::effects::EffectExecutor;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;

/// Effect that registers a continuous effect with the game state.
///
/// This is a low-level primitive used by other effects to compose
/// continuous effects without duplicating registration logic.
#[derive(Debug, Clone, PartialEq)]
pub struct ApplyContinuousEffect {
    /// Which objects the continuous effect applies to.
    pub target: EffectTarget,
    /// The modification to apply.
    pub modification: Modification,
    /// How long the effect lasts.
    pub until: Until,
    /// Optional source type (e.g., resolution lock).
    pub source_type: Option<EffectSourceType>,
}

impl ApplyContinuousEffect {
    /// Create a new apply continuous effect.
    pub fn new(target: EffectTarget, modification: Modification, until: Until) -> Self {
        Self {
            target,
            modification,
            until,
            source_type: None,
        }
    }

    /// Set the source type for the continuous effect.
    pub fn with_source_type(mut self, source_type: EffectSourceType) -> Self {
        self.source_type = Some(source_type);
        self
    }
}

impl EffectExecutor for ApplyContinuousEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let mut effect = ContinuousEffect::new(
            ctx.source,
            ctx.controller,
            self.target.clone(),
            self.modification.clone(),
        )
        .until(self.until.clone());

        if let Some(source_type) = &self.source_type {
            effect = effect.with_source_type(source_type.clone());
        }

        game.continuous_effects.add_effect(effect);

        Ok(EffectOutcome::resolved())
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}
