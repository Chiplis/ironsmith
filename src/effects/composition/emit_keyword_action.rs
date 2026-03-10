//! Keyword action event emission effect.
//!
//! Some rules text triggers on a keyword action (e.g., "when you cycle this card").
//! This effect provides a generic way to emit a KeywordActionEvent as part of an
//! effect/cost pipeline so triggers can observe it.

use crate::effect::{EffectOutcome, EffectResult};
use crate::effects::{CostExecutableEffect, EffectExecutor};
use crate::events::{KeywordActionEvent, KeywordActionKind};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::triggers::TriggerEvent;

#[derive(Debug, Clone, PartialEq)]
pub struct EmitKeywordActionEffect {
    pub action: KeywordActionKind,
    pub amount: u32,
}

impl EmitKeywordActionEffect {
    pub fn new(action: KeywordActionKind, amount: u32) -> Self {
        Self { action, amount }
    }
}

impl EffectExecutor for EmitKeywordActionEffect {
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        _game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let event = TriggerEvent::new_with_provenance(
            KeywordActionEvent::new(self.action, ctx.controller, ctx.source, self.amount),
            ctx.provenance,
        );
        Ok(EffectOutcome::from_result(EffectResult::Resolved).with_event(event))
    }

    fn cost_description(&self) -> Option<String> {
        // Internal scaffolding effect used to emit trigger-visible events from costs.
        // This should not show up as part of the printed/visible cost.
        Some(String::new())
    }
}

impl CostExecutableEffect for EmitKeywordActionEffect {
    fn can_execute_as_cost(
        &self,
        _game: &GameState,
        _source: crate::ids::ObjectId,
        _controller: crate::ids::PlayerId,
    ) -> Result<(), crate::effects::CostValidationError> {
        Ok(())
    }
}
