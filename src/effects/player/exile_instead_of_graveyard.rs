//! Exile-instead-of-graveyard replacement effect implementation.

use crate::Effect;
use crate::effect::EffectOutcome;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{ApplyReplacementEffect, EffectExecutor};
use crate::events::zones::matchers::WouldGoToGraveyardMatcher;
use crate::executor::{ExecutionContext, ExecutionError, execute_effect};
use crate::game_state::GameState;
use crate::replacement::{ReplacementAction, ReplacementEffect};
use crate::target::{ObjectFilter, PlayerFilter};
use crate::zone::Zone;

/// Effect that exiles cards that would go to a player's graveyard this turn.
#[derive(Debug, Clone, PartialEq)]
pub struct ExileInsteadOfGraveyardEffect {
    pub player: PlayerFilter,
}

impl ExileInsteadOfGraveyardEffect {
    /// Create a new effect.
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }

    /// Apply to you.
    pub fn you() -> Self {
        Self::new(PlayerFilter::You)
    }
}

impl EffectExecutor for ExileInsteadOfGraveyardEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;

        let replacement = ReplacementEffect::with_matcher(
            ctx.source,
            ctx.controller,
            WouldGoToGraveyardMatcher::new(
                ObjectFilter::default().owned_by(PlayerFilter::Specific(player_id)),
            ),
            ReplacementAction::ChangeDestination(Zone::Exile),
        );

        let apply = ApplyReplacementEffect::resolution(replacement);
        let _ = execute_effect(game, &Effect::new(apply), ctx)?;

        Ok(EffectOutcome::resolved())
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}
