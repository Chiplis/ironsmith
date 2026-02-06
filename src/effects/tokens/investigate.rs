//! Investigate effect implementation.

use crate::cards::tokens::clue_token_definition;
use crate::effect::{EffectOutcome, EffectResult, Value};
use crate::effects::helpers::resolve_value;
use crate::effects::{CreateTokenEffect, EffectExecutor};
use crate::events::{KeywordActionEvent, KeywordActionKind};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::triggers::TriggerEvent;

/// Effect that performs the investigate keyword action.
///
/// Each investigate creates a Clue token as a separate action.
#[derive(Debug, Clone, PartialEq)]
pub struct InvestigateEffect {
    /// How many times to investigate.
    pub count: Value,
}

impl InvestigateEffect {
    /// Create a new investigate effect.
    pub fn new(count: impl Into<Value>) -> Self {
        Self {
            count: count.into(),
        }
    }
}

impl EffectExecutor for InvestigateEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let count = resolve_value(game, &self.count, ctx)?.max(0) as usize;
        if count == 0 {
            return Ok(EffectOutcome::from_result(EffectResult::Resolved));
        }

        let mut outcomes = Vec::with_capacity(count);
        let mut action_events = Vec::with_capacity(count);
        for _ in 0..count {
            let effect = CreateTokenEffect::you(clue_token_definition(), 1);
            outcomes.push(effect.execute(game, ctx)?);
            action_events.push(TriggerEvent::new(KeywordActionEvent::new(
                KeywordActionKind::Investigate,
                ctx.controller,
                ctx.source,
                1,
            )));
        }

        Ok(EffectOutcome::aggregate(outcomes).with_events(action_events))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}
