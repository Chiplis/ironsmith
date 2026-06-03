use crate::effect::EffectOutcome;
use crate::effects::{ApplyReplacementEffect, EffectExecutor, ExecutionContext, ExecutionError};
use crate::events::cards::matchers::WouldDrawCardMatcher;
use crate::game_state::GameState;
use crate::replacement::{ReplacementAction, ReplacementEffect};

pub type RegisterDrawReplacementEffect =
    ironsmith_core::RegisterDrawReplacementEffect<crate::effect::Effect>;

impl EffectExecutor for RegisterDrawReplacementEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player = if self.player == crate::target::PlayerFilter::IteratedPlayer {
            ctx.iteration
                .iterated_player
                .map(crate::target::PlayerFilter::Specific)
                .unwrap_or(crate::target::PlayerFilter::IteratedPlayer)
        } else {
            self.player.clone()
        };
        let replacement = ReplacementEffect::with_matcher(
            ctx.source,
            ctx.controller,
            WouldDrawCardMatcher::new(player),
            ReplacementAction::Instead(self.replacement_effects.clone()),
        );

        ApplyReplacementEffect {
            effect: replacement,
            mode: self.mode,
        }
        .execute(game, ctx)
    }
}
