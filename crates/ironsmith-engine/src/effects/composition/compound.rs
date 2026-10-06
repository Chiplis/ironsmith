//! Replay boundary for a compound instruction with multiple child actions.

use crate::effect::EffectOutcome;
use crate::effects::{ExecutionContext, ExecutionContextCheckpoint, ExecutionError};
use crate::game_state::GameState;

/// Child owners keep their own action boundaries. If a later child suspends,
/// replay the whole compound from its original world, retaining the pending
/// decision metadata needed to resume it. Callers must stop after each pending
/// child; this boundary does not change their sequencing or simultaneity.
pub(crate) fn execute_compound<'a>(
    game: &mut GameState,
    ctx: &mut ExecutionContext<'a>,
    body: impl FnOnce(
        &mut GameState,
        &mut ExecutionContext<'a>,
    ) -> Result<EffectOutcome, ExecutionError>,
) -> Result<EffectOutcome, ExecutionError> {
    execute_transaction(game, ctx, || EffectOutcome::count(0), body)
}

/// Prepared children and ordinary compounds share the same atomic boundary.
/// A prepared result may retain a continuation rather than a completed effect
/// outcome; the rollback contract is independent of that result's shape.
pub(crate) fn execute_transaction<'a, T, E>(
    game: &mut GameState,
    ctx: &mut ExecutionContext<'a>,
    pending_value: impl FnOnce() -> T,
    body: impl FnOnce(&mut GameState, &mut ExecutionContext<'a>) -> Result<T, E>,
) -> Result<T, E> {
    execute_transaction_with_policy(
        game,
        ctx,
        pending_value,
        |_| true,
        PendingDecisionRetention::SuccessfulSuspension,
        body,
    )
}

/// An optional program owns its acceptance contract: Some commits its actual
/// result; None rolls back the complete program before an alternative runs.
/// Preserve the payment boundary's pending routing even for a suspended error.
pub(crate) fn execute_optional_transaction<'a, T, E>(
    game: &mut GameState,
    ctx: &mut ExecutionContext<'a>,
    body: impl FnOnce(&mut GameState, &mut ExecutionContext<'a>) -> Result<Option<T>, E>,
) -> Result<Option<T>, E> {
    execute_transaction_with_policy(
        game,
        ctx,
        || None,
        Option::is_some,
        PendingDecisionRetention::AnySuspension,
        body,
    )
}

/// Existing callers have distinct error routing contracts. They share physical
/// rollback without silently changing which pending controller view survives.
enum PendingDecisionRetention {
    SuccessfulSuspension,
    AnySuspension,
}

/// One owner for game/context rollback and decision suspension. The caller's
/// pure commit predicate is evaluated only for a finished successful execution.
/// Rejected results remain available to the caller, without committed actions.
fn execute_transaction_with_policy<'a, T, E>(
    game: &mut GameState,
    ctx: &mut ExecutionContext<'a>,
    pending_value: impl FnOnce() -> T,
    should_commit: impl FnOnce(&T) -> bool,
    pending_retention: PendingDecisionRetention,
    body: impl FnOnce(&mut GameState, &mut ExecutionContext<'a>) -> Result<T, E>,
) -> Result<T, E> {
    if ctx.decision_maker.awaiting_choice() {
        return Ok(pending_value());
    }
    let context = ExecutionContextCheckpoint::capture(ctx);
    let checkpoint = game.clone();
    let result = body(game, ctx);
    let pending = ctx.decision_maker.awaiting_choice();
    let rollback = match &result {
        Err(_) => true,
        Ok(value) => pending || !should_commit(value),
    };
    if rollback {
        let retain_pending = pending
            && (result.is_ok()
                || matches!(pending_retention, PendingDecisionRetention::AnySuspension));
        game.restore_execution_checkpoint(checkpoint, retain_pending);
        context.restore(ctx);
    }
    if result.is_ok() && pending {
        Ok(pending_value())
    } else {
        result
    }
}
