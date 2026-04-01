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
use crate::events::{KeywordActionEvent, KeywordActionKind};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::triggers::TriggerEvent;
use crate::zone::Zone;

#[derive(Debug, Clone, PartialEq)]
pub struct ExertCostEffect {
    pub display_text: String,
}

impl ExertCostEffect {
    pub fn new(display_text: impl Into<String>) -> Self {
        Self {
            display_text: display_text.into(),
        }
    }
}

impl EffectExecutor for ExertCostEffect {
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
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

        game.add_restriction_effect(
            Restriction::untap(crate::target::ObjectFilter::specific(ctx.source)),
            Until::ControllersNextUntapStep,
            ctx.source,
            ctx.controller,
        );
        game.update_cant_effects();

        Ok(EffectOutcome::resolved().with_event(TriggerEvent::new_with_provenance(
            KeywordActionEvent::new(KeywordActionKind::Exert, ctx.controller, ctx.source, 1),
            ctx.provenance,
        )))
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
        if game.object(source).is_some_and(|object| object.zone == Zone::Battlefield) {
            Ok(())
        } else {
            Err(CostValidationError::Other(
                "Only permanents on the battlefield can be exerted".to_string(),
            ))
        }
    }
}
