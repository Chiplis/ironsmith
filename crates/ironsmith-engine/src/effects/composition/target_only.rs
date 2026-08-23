//! TargetOnly effect implementation.
//!
//! This effect resolves a target and does nothing else. It exists for cards
//! whose rules text only establishes a target (e.g., "Target permanent.").

use crate::effect::EffectOutcome;
use crate::effects::helpers::{resolve_objects_for_effect, resolve_players_from_spec};
use crate::effects::{EffectExecutor, TargetReusePolicy};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;
pub use ironsmith_core::TargetOnlyEffect;

impl EffectExecutor for TargetOnlyEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if let Ok(objects) = resolve_objects_for_effect(game, ctx, &self.target)
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

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_chooser(&self) -> Option<&crate::target::PlayerFilter> {
        self.chooser.as_ref()
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        Some(self.target.count())
    }

    fn target_reuse_policy(&self) -> TargetReusePolicy {
        if self.explicit_declaration {
            TargetReusePolicy::AlwaysDeclareNew
        } else {
            TargetReusePolicy::SyntheticPrelude
        }
    }

    fn target_description(&self) -> &'static str {
        "target"
    }
}
