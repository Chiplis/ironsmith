//! A lexical result binding: nested programs cannot overwrite their caller's
//! result slot, including when a choice suspends or a child returns an error.
use crate::effect::{Effect, EffectId, EffectOutcome};
use crate::effects::{CompletedEffectOutputs, EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;

#[derive(Debug, Clone)]
pub(crate) struct ScopedOutcomeProgram {
    pub id: EffectId,
    pub producer: Effect,
    pub continuation: Vec<Effect>,
    pub preserve_producer_result: bool,
}
impl EffectExecutor for ScopedOutcomeProgram {
    fn library_movement_policy(
        &self,
    ) -> ironsmith_core::effect_capabilities::LibraryMovementPolicy {
        ironsmith_core::effect_capabilities::LibraryMovementPolicy::Inspect
    }
    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) {
        visitor(&self.producer);
        for child in &self.continuation {
            visitor(child);
        }
    }
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(CompletedEffectOutputs::into_outcome)
    }
    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        let previous = ctx.effect_outcomes.remove(&self.id);
        let result = super::execute_transaction(
            game,
            ctx,
            || CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                let producer = self.producer.0.execute_child_with_outputs(game, ctx)?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                ctx.store_outcome(self.id, producer.outcome.clone());
                let primary = producer.outcome.summary_projection();
                let mut children = vec![producer];
                for effect in &self.continuation {
                    if ctx.resolution_stopped() { break; }
                    children.push(effect.0.execute_child_with_outputs(game, ctx)?);
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::count(0),
                        ));
                    }
                    if ctx.resolution_stopped() {
                        break;
                    }
                }
                Ok(CompletedEffectOutputs::from_children(
                    children,
                    |children| {
                        if self.preserve_producer_result {
                            EffectOutcome::aggregate_with_primary_result(primary, children)
                        } else {
                            EffectOutcome::aggregate(children)
                        }
                    },
                ))
            },
        );
        ctx.effect_outcomes.remove(&self.id);
        if let Some(previous) = previous {
            ctx.effect_outcomes.insert(self.id, previous);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nested_result_bindings_restore_the_caller_slot() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let player = crate::ids::PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, player);
        let id = EffectId(777);
        ctx.store_outcome(id, EffectOutcome::count(7));
        let read = Effect::gain_life(crate::effect::Value::EffectValue(id));
        ScopedOutcomeProgram {
            id,
            preserve_producer_result: false,
            producer: Effect::gain_life(2),
            continuation: vec![
                Effect::new(ScopedOutcomeProgram {
                    id,
                    preserve_producer_result: false,
                    producer: Effect::gain_life(3),
                    continuation: vec![read.clone()],
                }),
                read,
            ],
        }
        .execute(&mut game, &mut ctx)
        .unwrap();
        assert_eq!(game.player(player).unwrap().life, 30);
        assert_eq!(ctx.get_outcome(id).unwrap().count_or_zero(), 7);
    }

    #[derive(Debug, Clone)]
    struct Fail;
    impl EffectExecutor for Fail {
        fn execute(
            &self,
            _: &mut GameState,
            _: &mut ExecutionContext,
        ) -> Result<EffectOutcome, ExecutionError> {
            Err(ExecutionError::Impossible(
                "test continuation failed".into(),
            ))
        }
    }
    #[test]
    fn failed_continuation_restores_the_binding_and_the_producer() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let player = crate::ids::PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, player);
        let id = EffectId(777);
        ctx.store_outcome(id, EffectOutcome::count(7));
        assert!(
            ScopedOutcomeProgram {
                id,
                preserve_producer_result: false,
                producer: Effect::gain_life(2),
                continuation: vec![Effect::new(Fail)],
            }
            .execute(&mut game, &mut ctx)
            .is_err()
        );
        assert_eq!(game.player(player).unwrap().life, 20);
        assert_eq!(ctx.get_outcome(id).unwrap().count_or_zero(), 7);
    }
}
