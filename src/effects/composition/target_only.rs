//! TargetOnly effect implementation.
//!
//! This effect resolves a target and does nothing else. It exists for cards
//! whose rules text only establishes a target (e.g., "Target permanent.").

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_objects_from_spec, resolve_players_from_spec};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;

#[derive(Debug, Clone, PartialEq)]
pub struct TargetOnlyEffect {
    pub target: ChooseSpec,
}

impl TargetOnlyEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }
}

impl EffectExecutor for TargetOnlyEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if let Ok(objects) = resolve_objects_from_spec(game, &self.target, ctx)
            && !objects.is_empty()
        {
            return Ok(EffectOutcome::count(objects.len() as i32));
        }

        if let Ok(players) = resolve_players_from_spec(game, &self.target, ctx)
            && !players.is_empty()
        {
            return Ok(EffectOutcome::count(players.len() as i32));
        }

        Err(ExecutionError::InvalidTarget)
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}
