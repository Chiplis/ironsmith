use crate::effect::{Effect, EffectOutcome, OutcomeStatus, OutcomeValue, Value};
use crate::effects::{EffectExecutor, SequenceEffect};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::resolve_value;

#[derive(Debug, Clone, PartialEq)]
pub struct RepeatEffectsEffect {
    pub count: Value,
    pub effects: Vec<Effect>,
}

impl RepeatEffectsEffect {
    pub fn new(count: Value, effects: Vec<Effect>) -> Self {
        Self { count, effects }
    }
}

impl EffectExecutor for RepeatEffectsEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let count = resolve_value(game, &self.count, ctx)?.max(0) as usize;
        let sequence = SequenceEffect::new(self.effects.clone());
        let mut all_events = Vec::new();
        let mut all_execution_facts = Vec::new();

        for _ in 0..count {
            let outcome = sequence.execute(game, ctx)?;
            all_events.extend(outcome.events.clone());
            all_execution_facts.extend(outcome.execution_facts.clone());
            if outcome.status.is_failure() {
                return Ok(EffectOutcome::with_details(
                    outcome.status,
                    outcome.value,
                    all_events,
                    all_execution_facts,
                ));
            }
        }

        Ok(EffectOutcome::with_details(
            OutcomeStatus::Succeeded,
            OutcomeValue::None,
            all_events,
            all_execution_facts,
        ))
    }
}
