//! Sequence effect implementation.
//!
//! Runs a list of effects in order and aggregates their outcomes.

use crate::effect::{Effect, EffectOutcome, EffectResult};
use crate::effects::EffectExecutor;
use crate::executor::{ExecutionContext, ExecutionError, execute_effect};
use crate::game_state::GameState;
use crate::ids::ObjectId;

/// Effect that executes multiple effects in sequence.
#[derive(Debug, Clone, PartialEq)]
pub struct SequenceEffect {
    /// Effects to execute in order.
    pub effects: Vec<Effect>,
}

impl SequenceEffect {
    /// Create a new SequenceEffect.
    pub fn new(effects: Vec<Effect>) -> Self {
        Self { effects }
    }
}

impl EffectExecutor for SequenceEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if self.effects.is_empty() {
            return Ok(EffectOutcome::count(0));
        }

        let mut outcomes = Vec::with_capacity(self.effects.len());
        let mut events = Vec::new();

        for effect in &self.effects {
            let outcome = execute_effect(game, effect, ctx)?;
            events.extend(outcome.events.clone());

            if outcome.result.is_failure() {
                return Ok(EffectOutcome {
                    result: outcome.result,
                    events,
                });
            }

            outcomes.push(outcome);
        }

        let mut total_count: i32 = 0;
        let mut has_count = false;
        let mut last_non_count = EffectResult::Resolved;
        let mut last_objects: Option<Vec<ObjectId>> = None;

        for outcome in outcomes {
            match outcome.result {
                EffectResult::Count(n) => {
                    total_count += n;
                    has_count = true;
                }
                EffectResult::Objects(objs) => {
                    last_objects = Some(objs.clone());
                    last_non_count = EffectResult::Objects(objs);
                }
                other => {
                    last_non_count = other;
                }
            }
        }

        let result = if has_count {
            EffectResult::Count(total_count)
        } else if matches!(last_non_count, EffectResult::Resolved) {
            if let Some(objs) = last_objects {
                EffectResult::Objects(objs)
            } else {
                last_non_count
            }
        } else {
            last_non_count
        };

        Ok(EffectOutcome { result, events })
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}
