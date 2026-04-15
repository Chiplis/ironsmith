//! Sequence effect implementation.
//!
//! Runs a list of effects in order and aggregates their outcomes.

use crate::effect::{Effect, EffectOutcome};
use crate::effects::EffectExecutor;
use crate::executor::{ExecutionContext, ExecutionError, execute_effect};
use crate::game_state::GameState;

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
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

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
        let mut execution_facts = Vec::new();

        for effect in &self.effects {
            let outcome = execute_effect(game, effect, ctx)?;
            events.extend(outcome.events.clone());
            execution_facts.extend(outcome.execution_facts.clone());

            if outcome.status.is_failure() {
                return Ok(EffectOutcome::with_details(
                    outcome.status,
                    outcome.value.clone(),
                    events,
                    execution_facts,
                ));
            }

            outcomes.push(outcome);
        }

        let aggregate = EffectOutcome::aggregate(outcomes);
        Ok(EffectOutcome::with_details(
            aggregate.status,
            aggregate.value,
            events,
            execution_facts,
        ))
    }

    fn get_target_spec(&self) -> Option<&crate::target::ChooseSpec> {
        super::target_metadata::first_target_spec(&[&self.effects])
    }

    fn target_description(&self) -> &'static str {
        super::target_metadata::first_target_description(&[&self.effects], "target")
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        super::target_metadata::first_target_count(&[&self.effects])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::target::ChooseSpec;

    #[test]
    fn sequence_forwards_inner_target_spec() {
        let effect = SequenceEffect::new(vec![
            Effect::gain_life(1),
            Effect::counter(ChooseSpec::target_spell()),
        ]);

        assert!(effect.get_target_spec().is_some());
        assert_eq!(effect.target_description(), "spell to counter");
    }

    #[test]
    fn sequence_uses_conservative_summary_for_multiple_meaningful_results() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = crate::ids::PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let result = SequenceEffect::new(vec![Effect::gain_life(1), Effect::gain_life(2)])
            .execute(&mut game, &mut ctx)
            .expect("sequence should execute");

        assert_eq!(result.status, crate::effect::OutcomeStatus::Succeeded);
    }
}
