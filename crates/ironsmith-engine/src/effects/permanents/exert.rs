//! Exert cost implementation.
//!
//! Comprehensive Rules reference (as of February 27, 2026):
//! - 701.43a: To exert a permanent, you choose to have it not untap during your
//!   next untap step.
//! - 701.43b: A permanent can be exerted even if it's untapped or was already
//!   exerted this turn.
//! - 701.43c: A permanent that isn't on the battlefield can't be exerted.

use crate::effect::{EffectOutcome, Restriction, Until};
use crate::effects::{CostExecutableEffect, CostValidationError, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::{KeywordActionEvent, KeywordActionKind};
use crate::game_state::GameState;
use crate::zone::Zone;
pub type ExertCostEffect = ironsmith_core::ExertCostEffect;

impl EffectExecutor for ExertCostEffect {
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        crate::effects::composition::execute_compound(game, ctx, |game, ctx| {
            let Some(source) = game.object(ctx.source) else {
                return Err(ExecutionError::Impossible(
                    "Only permanents on the battlefield can be exerted".to_string(),
                ));
            };
            if source.zone != Zone::Battlefield {
                return Err(ExecutionError::Impossible(
                    "Only permanents on the battlefield can be exerted".to_string(),
                ));
            }
            let restriction = crate::effects::CantEffect::new(
                Restriction::untap(crate::target::ObjectFilter::specific(ctx.source)),
                // Exert is owned by the player paying the cost, even after
                // the permanent changes controller (CR 701.43a). Reuse the
                // fixed-player occurrence/cutoff owner, not controller tenure.
                Until::PlayersNextUntapStep {
                    player: crate::target::PlayerFilter::Specific(ctx.controller),
                },
            )
            .execute_child(game, ctx)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            crate::effects::composition::complete_keyword_action_with_result(
                game,
                ctx,
                restriction,
                KeywordActionEvent::new(KeywordActionKind::Exert, ctx.controller, ctx.source, 1),
            )
        })
    }

    fn cost_description(&self) -> Option<String> {
        Some(self.display_text.clone())
    }
}

impl CostExecutableEffect for ExertCostEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        _controller: crate::ids::PlayerId,
    ) -> Result<(), CostValidationError> {
        if game
            .object(source)
            .is_some_and(|object| object.zone == Zone::Battlefield)
        {
            Ok(())
        } else {
            Err(CostValidationError::Other(
                "Only permanents on the battlefield can be exerted".to_string(),
            ))
        }
    }
}
