//! Exile-instead-of-graveyard replacement effect implementation.

use crate::Effect;
use crate::effect::EffectOutcome;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{ApplyReplacementEffect, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError, execute_effect};
use crate::events::zones::matchers::WouldGoToGraveyardMatcher;
use crate::game_state::GameState;
use crate::replacement::{ReplacementAction, ReplacementEffect};
use crate::target::{ObjectFilter, PlayerFilter};
use crate::zone::Zone;
pub use ironsmith_core::ExileInsteadOfGraveyardEffect;

/// Effect that exiles cards that would go to a player's graveyard this turn.
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

        let apply = ApplyReplacementEffect::until_end_of_turn(replacement);
        let _ = execute_effect(game, &Effect::new(apply), ctx)?;

        Ok(EffectOutcome::resolved())
    }
}
