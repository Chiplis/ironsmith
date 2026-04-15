use crate::effect::{Effect, EffectId, EffectOutcome, EffectPredicate};
use crate::effects::{EffectExecutor, SequenceEffect};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;

#[derive(Debug, Clone, PartialEq)]
pub struct RepeatProcessEffect {
    pub effects: Vec<Effect>,
    pub condition: EffectId,
    pub predicate: EffectPredicate,
}

impl RepeatProcessEffect {
    pub fn new(effects: Vec<Effect>, condition: EffectId, predicate: EffectPredicate) -> Self {
        Self {
            effects,
            condition,
            predicate,
        }
    }
}

impl EffectExecutor for RepeatProcessEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let sequence = SequenceEffect::new(self.effects.clone());
        let mut all_events = Vec::new();
        let mut all_execution_facts = Vec::new();
        let (status, value) = loop {
            let outcome = sequence.execute(game, ctx)?;
            all_events.extend(outcome.events.clone());
            all_execution_facts.extend(outcome.execution_facts.clone());

            if outcome.status.is_failure() {
                break (outcome.status, outcome.value);
            }

            let should_continue = ctx
                .get_outcome(self.condition)
                .is_some_and(|outcome| self.predicate.evaluate_outcome(outcome));
            if !should_continue {
                break (outcome.status, outcome.value);
            }
        };

        Ok(EffectOutcome::with_details(
            status,
            value,
            all_events,
            all_execution_facts,
        ))
    }
}
