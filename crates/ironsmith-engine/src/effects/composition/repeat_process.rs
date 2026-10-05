use crate::effect::{Effect, EffectOutcome};
use crate::effects::{EffectExecutor, SequenceEffect};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;

pub type RepeatProcessEffect = ironsmith_core::RepeatProcessEffect<Effect>;

impl EffectExecutor for RepeatProcessEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) {
        for effect in &self.effects {
            visitor(effect);
        }
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        crate::effects::tokens::execute_resource_transaction_atomically(game, ctx, |game, ctx| {
        let sequence = SequenceEffect::new(self.effects.clone());
        let mut all_events = Vec::new();
        let mut all_execution_facts = Vec::new();
        let mut continuation_count = 0i64;
        let mut reported_cursor = 0usize;
        let (status, value) = loop {
            // A failed result may itself be the authored continuation gate
            // (for example, paying an "unless" cost records Declined and then
            // repeats the process). Remove the prior iteration's result so an
            // earlier gate cannot accidentally drive a later iteration that
            // failed before reaching the condition.
            ctx.effect_outcomes.remove(&self.condition);
            let mut outcome = sequence.execute(game, ctx)?;
            if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::resolved()); }
            all_events.try_reserve(outcome.events.len()).map_err(|_| ExecutionError::ResourceAllocationFailed {
                resource: "repeated process events", requested: outcome.events.len(),
            })?;
            all_execution_facts.try_reserve(outcome.execution_facts.len()).map_err(|_| ExecutionError::ResourceAllocationFailed {
                resource: "repeated process receipts", requested: outcome.execution_facts.len(),
            })?;
            all_events.append(&mut outcome.events);
            all_execution_facts.append(&mut outcome.execution_facts);
            if ctx.resolution_stopped() { break (outcome.status, outcome.value); }

            let condition = ctx.get_outcome(self.condition).ok_or_else(|| ExecutionError::IncompleteEvidence(
                "repeated process has no completed continuation receipt".into(),
            ))?;
            let should_continue = {
                if self.predicate == crate::effect::EffectPredicate::Happened
                    && let Some(player_counts) = condition.player_counts()
                {
                    // A mixed optional pass can contain both accepted and
                    // declined outcomes. Its aggregate `Declined` fact must
                    // not hide that another participant acted this round.
                    player_counts.iter().any(|(_, count)| *count > 0)
                } else { super::if_effect::predicate_matches_with_context(
                    &self.predicate,
                    condition,
                    game,
                    ctx,
                ) }
            };
            if should_continue {
                continuation_count = continuation_count.checked_add(1).ok_or(ExecutionError::ResourceLimitExceeded {
                    resource: "repeated process continuation count",
                    requested: continuation_count as u128 + 1, maximum: i64::MAX as u128,
                })?;
                if crate::effects::match_triggers_at_instruction_boundary(game, ctx, None, all_events[reported_cursor..].iter()) {
                    reported_cursor = all_events.len();
                }
                continue;
            }
            break (outcome.status, outcome.value);
        };

        Ok(EffectOutcome::with_details(
            if continuation_count > 0 {
                crate::effect::OutcomeStatus::Succeeded
            } else {
                status
            },
            if continuation_count > 0 || value.as_count().is_none() {
                crate::effect::OutcomeValue::Count(continuation_count)
            } else {
                value
            },
            all_events,
            EffectOutcome::merge_execution_facts(all_execution_facts),
        ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::DecisionMaker;
    use crate::decisions::context::BooleanContext;
    use crate::effect::{EffectId, EffectPredicate, OutcomeStatus};
    use crate::effects::MayEffect;
    use crate::ids::PlayerId;

    struct BooleanScript {
        responses: std::vec::IntoIter<bool>,
    }

    impl DecisionMaker for BooleanScript {
        fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
            self.responses.next().unwrap_or(false)
        }
    }

    #[test]
    fn accepted_iterations_are_exposed_as_the_repeat_outcome_count() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let controller = PlayerId::from_index(0);
        let source = game.new_object_id();
        let initial_life = game
            .player(controller)
            .expect("controller should exist")
            .life;
        let mut decisions = BooleanScript {
            responses: vec![true, true, false].into_iter(),
        };
        let mut ctx =
            ExecutionContext::new_default(source, controller).with_decision_maker(&mut decisions);
        let condition = EffectId(7);
        let repeated_may = Effect::with_id(
            condition.0,
            Effect::new(MayEffect::new(vec![Effect::gain_life(1)])),
        );
        let repeat =
            RepeatProcessEffect::new(vec![repeated_may], condition, EffectPredicate::Happened);

        let outcome = repeat
            .execute(&mut game, &mut ctx)
            .expect("repeat process should execute");

        assert_eq!(outcome.status, OutcomeStatus::Succeeded);
        assert_eq!(outcome.as_count(), Some(2));
        assert_eq!(
            game.player(controller)
                .expect("controller should exist")
                .life,
            initial_life + 2
        );
    }
}
