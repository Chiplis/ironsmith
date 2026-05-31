use crate::effect::EffectOutcome;
use crate::effects::{
    ApplyReplacementEffect, EffectExecutionCategory, EffectExecutor, ExecutionContext,
    ExecutionError,
};
use crate::events::zones::matchers::WouldEnterBattlefieldMatcher;
use crate::game_state::GameState;
use crate::replacement::{ReplacementAction, ReplacementEffect};

pub type RegisterEnterWithCountersReplacementEffect =
    ironsmith_core::RegisterEnterWithCountersReplacementEffect;

impl EffectExecutor for RegisterEnterWithCountersReplacementEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let replacement = ReplacementEffect::with_matcher(
            ctx.source,
            ctx.controller,
            WouldEnterBattlefieldMatcher::new(self.filter.clone()),
            ReplacementAction::EnterWithCounters {
                counter_type: self.counter_type,
                count: self.count.clone(),
                added_subtypes: Vec::new(),
                added_abilities: Vec::new(),
            },
        );

        ApplyReplacementEffect {
            effect: replacement,
            mode: self.mode,
        }
        .execute(game, ctx)
    }

    fn primary_execution_category(&self) -> EffectExecutionCategory {
        EffectExecutionCategory::ReplacementRegistration
    }
}
