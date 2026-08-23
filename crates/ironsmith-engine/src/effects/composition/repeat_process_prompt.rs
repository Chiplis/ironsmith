use crate::decision::FallbackStrategy;
use crate::decisions::ask_may_choice;
use crate::effect::{EffectOutcome, ExecutionFact};
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;

#[derive(Debug, Clone, PartialEq)]
pub struct RepeatProcessPromptEffect {
    pub kind: ironsmith_core::RepeatProcessPromptKind,
    pub fallback: FallbackStrategy,
}

impl RepeatProcessPromptEffect {
    pub fn new(kind: ironsmith_core::RepeatProcessPromptKind) -> Self {
        Self {
            kind,
            fallback: FallbackStrategy::Decline,
        }
    }

    pub fn description(&self) -> &'static str {
        self.kind.prompt_text()
    }
}

impl EffectExecutor for RepeatProcessPromptEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let should_continue = ask_may_choice(
            game,
            &mut ctx.decision_maker,
            ctx.iteration.iterated_player.unwrap_or(ctx.controller),
            ctx.source,
            self.description().to_string(),
            self.fallback,
        );

        if should_continue {
            return Ok(EffectOutcome::resolved().with_execution_fact(ExecutionFact::Accepted));
        }

        Ok(EffectOutcome::declined())
    }
}
