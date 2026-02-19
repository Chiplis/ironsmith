//! Schedule source sacrifice when a tagged object leaves the battlefield.

use crate::effect::{Effect, EffectOutcome};
use crate::effects::{
    EffectExecutor, ScheduleEffectsWhenTaggedLeavesEffect,
};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::tag::TagKey;
use crate::target::PlayerFilter;

/// Backward-compatible wrapper for the common Stangg pattern:
/// "Sacrifice this source when that token leaves the battlefield."
#[derive(Debug, Clone, PartialEq)]
pub struct SacrificeSourceWhenTaggedLeavesEffect {
    pub tag: TagKey,
    pub controller: PlayerFilter,
}

impl SacrificeSourceWhenTaggedLeavesEffect {
    pub fn new(tag: impl Into<TagKey>, controller: PlayerFilter) -> Self {
        Self {
            tag: tag.into(),
            controller,
        }
    }
}

impl EffectExecutor for SacrificeSourceWhenTaggedLeavesEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        ScheduleEffectsWhenTaggedLeavesEffect::new(
            self.tag.clone(),
            vec![Effect::sacrifice_source()],
            self.controller.clone(),
        )
        .with_current_source_as_ability_source()
        .execute(game, ctx)
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}

