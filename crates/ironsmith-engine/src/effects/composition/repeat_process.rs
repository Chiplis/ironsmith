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
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        crate::effects::tokens::execute_resource_transaction_with_pending_value(
            game,
            ctx,
            || {
                crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::with_objects(
                    Vec::new(),
                ))
            },
            |game, ctx| {
                let sequence = SequenceEffect::new(self.effects.clone());
                let mut children = Vec::new();
                let mut continuation_count = 0i64;
                // A fresh process has chosen nothing yet.
                for history in &self.choice_history {
                    ctx.clear_object_tag(history.previously_chosen.as_str());
                    ctx.clear_object_tag(history.chosen.as_str());
                }
                let (status, value) = loop {
                    // A failed result may itself be the authored continuation gate
                    // (for example, paying an "unless" cost records Declined and then
                    // repeats the process). Remove the prior iteration's result so an
                    // earlier gate cannot accidentally drive a later iteration that
                    // failed before reaching the condition.
                    ctx.effect_outcomes.remove(&self.condition);
                    let outputs = sequence.execute_child_with_outputs(game, ctx)?;
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::count(0),
                        ));
                    }
                    let outcome = outputs.outcome.clone();
                    children.push(outputs);
                    if ctx.resolution_stopped() {
                        break (outcome.status, outcome.value);
                    }

                    let condition = ctx.get_outcome(self.condition).ok_or_else(|| {
                        ExecutionError::IncompleteEvidence(
                            "repeated process has no completed continuation receipt".into(),
                        )
                    })?;
                    let should_continue = {
                        if self.predicate == crate::effect::EffectPredicate::Happened
                            && let Some(player_counts) = condition.player_counts()
                        {
                            // A mixed optional pass can contain both accepted and
                            // declined outcomes. Its aggregate `Declined` fact must
                            // not hide that another participant acted this round.
                            player_counts.iter().any(|(_, count)| *count > 0)
                        } else {
                            super::if_effect::predicate_matches_with_context(
                                &self.predicate,
                                condition,
                                game,
                                ctx,
                            )
                        }
                    };
                    if should_continue && !ctx.resolution_stopped() {
                        continuation_count = continuation_count.checked_add(1).ok_or(
                            ExecutionError::ResourceLimitExceeded {
                                resource: "repeated process continuation count",
                                requested: continuation_count as u128 + 1,
                                maximum: i64::MAX as u128,
                            },
                        )?;
                        crate::effects::capture_triggers_before_added_program(
                            game,
                            ctx,
                            None,
                            children
                                .iter_mut()
                                .flat_map(|outputs| outputs.outcome.events.iter_mut()),
                        )?;
                        for outputs in &mut children {
                            outputs.synchronize_observations();
                        }
                        // The round just completed becomes history; the next
                        // round's choice starts from nothing and may exclude
                        // every earlier round's choice.
                        for history in &self.choice_history {
                            if let Some(chosen) = ctx.clear_object_tag(history.chosen.as_str()) {
                                ctx.tag_objects_unique(history.previously_chosen.clone(), chosen);
                            }
                        }
                        continue;
                    }
                    break (outcome.status, outcome.value);
                };

                let primary = EffectOutcome::with_details(
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
                    Vec::new(),
                    Vec::new(),
                );
                Ok(crate::effects::CompletedEffectOutputs::from_children(
                    children,
                    |outcomes| EffectOutcome::aggregate_with_primary_result(primary, outcomes),
                ))
            },
        )
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
    #[test]
    fn live_gate_receipt_is_sampled_before_branch_changes_the_predicate() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let controller = PlayerId(0);
        let source = game.new_object_id();
        let condition = EffectId(71);
        let gate = crate::effects::ConditionalEffect::if_only(
            crate::effect::Condition::LifeTotalOrLess(20), vec![Effect::gain_life(1)],
        ).with_condition_result(true);
        let process = RepeatProcessEffect::new(vec![
            Effect::draw(1), Effect::with_id(condition.0, Effect::new(gate)),
        ], condition, EffectPredicate::Value(crate::effect::Comparison::GreaterThan(0)));
        for i in 0..3 {
            let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), &format!("Card {i}"))
                .card_types(vec![crate::types::CardType::Sorcery]).build();
            game.create_object_from_card(&card, controller, crate::zone::Zone::Library);
        }
        let mut ctx = ExecutionContext::new_default(source, controller);
        process.execute(&mut game, &mut ctx).unwrap();
        assert_eq!(game.player(controller).unwrap().life, 21);
        assert_eq!(game.player(controller).unwrap().hand.len(), 2,
            "the successful first gate repeats even after its branch raises life to 21");
        assert_eq!(ctx.get_outcome(condition).unwrap().as_count(), Some(0));
    }

}
