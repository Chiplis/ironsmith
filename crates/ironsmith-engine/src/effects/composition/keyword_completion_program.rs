//! An immediate program followed by one keyword completion notification.
//!
//! Replacement of a whole keyword action belongs to its existing outer owner.
//! This program owns only the body and its completion, so adapters can expose
//! the same executable body to structural queries without synthesizing events.
use crate::effect::{Effect, EffectOutcome, Value};
use crate::effects::{CompletedEffectOutputs, EffectExecutor, ExecutionContext, ExecutionError};
use crate::events::{KeywordActionEvent, KeywordActionKind};
use crate::game_state::GameState;
use crate::target::PlayerFilter;

#[derive(Debug, Clone)]
pub(crate) struct KeywordCompletionProgram {
    pub action: KeywordActionKind,
    pub amount: Value,
    pub player: PlayerFilter,
    pub body: Effect,
}

impl EffectExecutor for KeywordCompletionProgram {
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
            |game, ctx| {
                let player =
                    crate::effects::helpers::resolve_player_filter(game, &self.player, ctx)?;
                let amount =
                    crate::effects::helpers::resolve_nonnegative_u32(game, &self.amount, ctx)?;
                let source = ctx.source;
                let body = self.body.0.execute_child_with_outputs(game, ctx)?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                super::complete_keyword_action_with_outputs(
                    game,
                    ctx,
                    body,
                    KeywordActionEvent::new(self.action, player, source, amount),
                )
            },
        )
    }
}
