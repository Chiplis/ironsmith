//! Transient programs for exact identities and already-resolved decisions.
use crate::effect::{Effect, EffectOutcome, ExecutionFact};
use crate::effects::{CompletedEffectOutputs, EffectExecutor, ExecutionContext, ExecutionError};
use crate::{
    game_state::GameState,
    ids::{ObjectId, PlayerId},
    zone::Zone,
};

/// A shared optional-decision boundary. Pending and declined are distinct;
/// callers own payment claims and lexical player bindings.
pub(crate) fn optional_decision(
    game: &GameState,
    ctx: &mut ExecutionContext,
    player: PlayerId,
    spec: crate::decisions::MaySpec,
    fallback: crate::decision::FallbackStrategy,
) -> Option<bool> {
    let accepted = crate::decisions::make_decision_with_fallback(
        game,
        &mut ctx.decision_maker,
        player,
        Some(ctx.source),
        spec,
        fallback,
    );
    (!ctx.decision_maker.awaiting_choice()).then_some(accepted)
}

#[derive(Debug, Clone)]
pub(crate) struct ResolvedOptionalProgram {
    pub player: PlayerId,
    pub spec: crate::decisions::MaySpec,
    pub body: Effect,
}
impl EffectExecutor for ResolvedOptionalProgram {
    fn library_movement_policy(
        &self,
    ) -> ironsmith_core::effect_capabilities::LibraryMovementPolicy {
        ironsmith_core::effect_capabilities::LibraryMovementPolicy::Inspect
    }
    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) {
        visitor(&self.body);
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
        super::execute_transaction(
            game,
            ctx,
            || CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| match optional_decision(
                game,
                ctx,
                self.player,
                self.spec.clone(),
                crate::decision::FallbackStrategy::Decline,
            ) {
                None => Ok(CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::count(0),
                )),
                Some(false) => Ok(CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::count(0).with_execution_fact(ExecutionFact::Declined),
                )),
                Some(true) => self.body.0.execute_child_with_outputs(game, ctx),
            },
        )
    }
}

/// Check the captured incarnation after the preceding action. Never follows a
/// stable identity through a zone change or substitutes last-known information.
#[derive(Debug, Clone)]
pub(crate) struct IfPresentProgram {
    pub object: ObjectId,
    pub zone: Zone,
    pub effects: Vec<Effect>,
}
impl EffectExecutor for IfPresentProgram {
    fn library_movement_policy(
        &self,
    ) -> ironsmith_core::effect_capabilities::LibraryMovementPolicy {
        ironsmith_core::effect_capabilities::LibraryMovementPolicy::Inspect
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
            .map(CompletedEffectOutputs::into_outcome)
    }
    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        if game
            .object(self.object)
            .is_none_or(|object| object.zone != self.zone)
            || game.is_phased_out(self.object)
        {
            return Ok(CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        let children =
            super::sequence::execute_ordered_children_with_outputs(game, ctx, &self.effects)?;
        Ok(CompletedEffectOutputs::from_children(
            children,
            EffectOutcome::aggregate,
        ))
    }
}
