use crate::effect::EffectOutcome;
use crate::effects::{ApplyReplacementEffect, EffectExecutor, ExecutionContext, ExecutionError};
use crate::events::ReplacementPriority;
use crate::events::zones::matchers::WouldEnterBattlefieldMatcher;
use crate::game_state::GameState;
use crate::replacement::{ReplacementAction, ReplacementEffect};

pub type RegisterEnterUnderControlReplacementEffect =
    ironsmith_core::RegisterEnterUnderControlReplacementEffect;

impl EffectExecutor for RegisterEnterUnderControlReplacementEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let replacement = ReplacementEffect::with_matcher(
            ctx.source,
            ctx.controller,
            WouldEnterBattlefieldMatcher::new(self.filter.clone()),
            ReplacementAction::EnterUnderControl(ctx.controller),
        )
        .with_priority_override(ReplacementPriority::ControlChanging);

        ApplyReplacementEffect {
            effect: replacement,
            mode: self.mode,
        }
        .execute(game, ctx)
    }
}
