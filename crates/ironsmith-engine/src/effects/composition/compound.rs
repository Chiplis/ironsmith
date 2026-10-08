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
        Some(pending_value),
        |_| true,
        |_| true,
        PendingDecisionRetention::SuccessfulSuspension,
        |game, ctx, _| body(game, ctx),
    )
}

/// Preserve a caller's existing body admission when a choice is already
/// pending. Successful suspension still retains decision routing and returns
/// the caller's neutral result; pending errors remain errors.
pub(crate) fn execute_transaction_from_body<'a, T, E>(
    game: &mut GameState,
    ctx: &mut ExecutionContext<'a>,
    pending_value: impl FnOnce() -> T,
    body: impl FnOnce(&mut GameState, &mut ExecutionContext<'a>) -> Result<T, E>,
) -> Result<T, E> {
    execute_transaction_with_policy(
        game,
        ctx,
        Some(pending_value),
        |_| true,
        |_| true,
        PendingDecisionRetention::SuccessfulSuspensionFromBody,
        |game, ctx, _| body(game, ctx),
    )
}

/// Restore world/context on errors or suspension, retaining pending routing
/// only for successful suspension. Return the body's exact result and preserve
/// its admission instead of introducing a gate or a neutral pending value.
pub(crate) fn execute_result_transaction<'a, T, E>(
    game: &mut GameState,
    ctx: &mut ExecutionContext<'a>,
    body: impl FnOnce(&mut GameState, &mut ExecutionContext<'a>) -> Result<T, E>,
) -> Result<T, E> {
    execute_transaction_with_policy(
        game,
        ctx,
        None::<fn() -> T>,
        |_| true,
        |_| true,
        PendingDecisionRetention::SuccessfulSuspensionFromBody,
        |game, ctx, _| body(game, ctx),
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
        Some(|| None),
        Option::is_some,
        |_| true,
        PendingDecisionRetention::AnySuspension,
        |game, ctx, _| body(game, ctx),
    )
}

/// Replay an instruction from its complete checkpoint on any suspension.
/// The pending result takes precedence over an error raised while requesting
/// that decision. Unlike routing-preserving transactions, the restored game
/// keeps the checkpoint's controller view, including any enclosing scope.
pub(crate) fn execute_checkpoint_transaction<'a, T, E>(
    game: &mut GameState,
    ctx: &mut ExecutionContext<'a>,
    pending_value: impl FnOnce() -> T,
    body: impl FnOnce(&mut GameState, &mut ExecutionContext<'a>) -> Result<T, E>,
) -> Result<T, E> {
    execute_transaction_with_policy(
        game,
        ctx,
        Some(pending_value),
        |_| true,
        |_| true,
        PendingDecisionRetention::Checkpoint,
        |game, ctx, _| body(game, ctx),
    )
}

/// Restore the complete world/context on suspension or error and return the
/// body's exact result. Ending procedures preserve pending errors and inactive
/// admission results instead of manufacturing a neutral pending outcome.
pub(crate) fn execute_result_checkpoint_transaction<'a, T, E>(
    game: &mut GameState,
    ctx: &mut ExecutionContext<'a>,
    body: impl FnOnce(&mut GameState, &mut ExecutionContext<'a>) -> Result<T, E>,
) -> Result<T, E> {
    execute_transaction_with_policy(
        game,
        ctx,
        None::<fn() -> T>,
        |_| true,
        |_| true,
        PendingDecisionRetention::CheckpointResult,
        |game, ctx, _| body(game, ctx),
    )
}

/// A raw decision owner retains its real pending routing without inventing
/// an effect source or context. The caller supplies its native pending result.
pub(crate) fn execute_decision_transaction<'a, T, E>(
    game: &mut GameState,
    decision_maker: &mut (dyn crate::decision::DecisionMaker + 'a),
    pending_value: impl FnOnce() -> T,
    body: impl FnOnce(&mut GameState, &mut (dyn crate::decision::DecisionMaker + 'a)) -> Result<T, E>,
) -> Result<T, E> {
    execute_transaction_with_policy(
        game,
        decision_maker,
        Some(pending_value),
        |_| true,
        |_| true,
        PendingDecisionRetention::SuccessfulSuspensionFromBody,
        |game, ctx, _| body(game, ctx),
    )
}

/// Raw receipt owners preserve the body's exact result on suspension or error.
pub(crate) fn execute_result_decision_transaction<'a, T, E>(
    game: &mut GameState,
    decision_maker: &mut (dyn crate::decision::DecisionMaker + 'a),
    body: impl FnOnce(&mut GameState, &mut (dyn crate::decision::DecisionMaker + 'a)) -> Result<T, E>,
) -> Result<T, E> {
    execute_transaction_with_policy(
        game,
        decision_maker,
        None::<fn() -> T>,
        |_| true,
        |_| true,
        PendingDecisionRetention::SuccessfulSuspensionFromBody,
        |game, ctx, _| body(game, ctx),
    )
}

/// Atomic world-only actions restore their complete checkpoint on error.
/// They do not acquire a decision, context, source or new action identity.
pub(crate) fn execute_world_checkpoint_transaction<T, E>(
    game: &mut GameState,
    body: impl FnOnce(&mut GameState) -> Result<T, E>,
) -> Result<T, E> {
    execute_transaction_with_policy(
        game,
        &mut (),
        None::<fn() -> T>,
        |_| true,
        |_| true,
        PendingDecisionRetention::CheckpointResult,
        |game, _, _| body(game),
    )
}

/// Expose the owner's immutable original world for event-time eligibility.
/// This native policy returns the supplied neutral result on any suspension,
/// including errors; routing is retained only for successful suspension.
pub(crate) fn execute_original_view_transaction<'a, T, E>(
    game: &mut GameState,
    ctx: &mut ExecutionContext<'a>,
    pending_value: impl FnOnce() -> T,
    body: impl FnOnce(&mut GameState, &mut ExecutionContext<'a>, &GameState) -> Result<T, E>,
) -> Result<T, E> {
    execute_transaction_with_policy(
        game,
        ctx,
        Some(pending_value),
        |_| true,
        |_| true,
        PendingDecisionRetention::SuccessfulRoutingPendingOverride,
        body,
    )
}

/// World-only resource owners retain execution-checkpoint failure metadata.
/// Their external scopes and mutable event receipts remain independently owned.
pub(crate) fn execute_world_result_transaction<T, E>(
    game: &mut GameState,
    body: impl FnOnce(&mut GameState) -> Result<T, E>,
) -> Result<T, E> {
    execute_transaction_with_policy(
        game,
        &mut (),
        None::<fn() -> T>,
        |_| true,
        |_| true,
        PendingDecisionRetention::SuccessfulSuspensionFromBody,
        |game, _, _| body(game),
    )
}

/// Root execution owns rollback only for selected failures. Nested callers
/// retain their existing body and avoid creating another world checkpoint.
/// Pending success and unselected errors keep their native result and state.
pub(crate) fn execute_error_transaction_if<'a, T, E>(
    game: &mut GameState,
    ctx: &mut ExecutionContext<'a>,
    enabled: bool,
    should_rollback_error: impl FnOnce(&E) -> bool,
    body: impl FnOnce(&mut GameState, &mut ExecutionContext<'a>) -> Result<T, E>,
) -> Result<T, E> {
    if !enabled {
        return body(game, ctx);
    }
    execute_transaction_with_policy(
        game,
        ctx,
        None::<fn() -> T>,
        |_| true,
        should_rollback_error,
        PendingDecisionRetention::ErrorOnly,
        |game, ctx, _| body(game, ctx),
    )
}

/// Payment attempts checkpoint the world while their actual context metadata
/// stays with its native owner. None, errors and suspension do not commit the
/// world; exact Option/results survive, with routing only on successful pending.
pub(crate) fn execute_optional_world_transaction<'a, T, E>(
    game: &mut GameState,
    ctx: &mut ExecutionContext<'a>,
    body: impl FnOnce(&mut GameState, &mut ExecutionContext<'a>) -> Result<Option<T>, E>,
) -> Result<Option<T>, E> {
    let mut participant = WorldOnlyExecutionContext { context: ctx };
    execute_transaction_with_policy(
        game,
        &mut participant,
        None::<fn() -> Option<T>>,
        Option::is_some,
        |_| true,
        PendingDecisionRetention::SuccessfulSuspensionFromBody,
        |game, participant, _| body(game, participant.context),
    )
}

/// This borrowed context is real; only its world participates in rollback.
/// Choice admission and metadata ownership stay with the payment instruction.
struct WorldOnlyExecutionContext<'r, 'a> {
    context: &'r mut ExecutionContext<'a>,
}
impl TransactionParticipant for WorldOnlyExecutionContext<'_, '_> {
    type Checkpoint = ();
    fn is_pending(&self) -> bool {
        self.context.decision_maker.awaiting_choice()
    }
    fn capture_checkpoint(&self) {}
    fn restore_checkpoint(&mut self, _: ()) {}
}

/// Only the real participant's state is checkpointed. Decision makers keep
/// their external choice stream; world-only actions have no context state.
trait TransactionParticipant {
    type Checkpoint;
    fn is_pending(&self) -> bool;
    fn capture_checkpoint(&self) -> Self::Checkpoint;
    fn restore_checkpoint(&mut self, checkpoint: Self::Checkpoint);
}

impl TransactionParticipant for ExecutionContext<'_> {
    type Checkpoint = ExecutionContextCheckpoint;
    fn is_pending(&self) -> bool {
        self.decision_maker.awaiting_choice()
    }
    fn capture_checkpoint(&self) -> Self::Checkpoint {
        ExecutionContextCheckpoint::capture(self)
    }
    fn restore_checkpoint(&mut self, checkpoint: Self::Checkpoint) {
        checkpoint.restore(self);
    }
}

impl TransactionParticipant for dyn crate::decision::DecisionMaker + '_ {
    type Checkpoint = ();
    fn is_pending(&self) -> bool {
        self.awaiting_choice()
    }
    fn capture_checkpoint(&self) {}
    fn restore_checkpoint(&mut self, _: ()) {}
}

impl TransactionParticipant for () {
    type Checkpoint = ();
    fn is_pending(&self) -> bool {
        false
    }
    fn capture_checkpoint(&self) {}
    fn restore_checkpoint(&mut self, _: ()) {}
}

/// Existing callers have distinct error routing contracts. They share physical
/// rollback without silently changing which pending controller view survives.
enum PendingDecisionRetention {
    SuccessfulSuspension,
    SuccessfulSuspensionFromBody,
    SuccessfulRoutingPendingOverride,
    AnySuspension,
    Checkpoint,
    CheckpointResult,
    ErrorOnly,
}

/// One owner for game/context rollback and decision suspension. The caller's
/// pure commit predicate is evaluated only for a finished successful execution.
/// Rejected results remain available to the caller, without committed actions.
fn execute_transaction_with_policy<T, E, P: TransactionParticipant + ?Sized>(
    game: &mut GameState,
    ctx: &mut P,
    mut pending_value: Option<impl FnOnce() -> T>,
    should_commit: impl FnOnce(&T) -> bool,
    should_rollback_error: impl FnOnce(&E) -> bool,
    pending_retention: PendingDecisionRetention,
    body: impl FnOnce(&mut GameState, &mut P, &GameState) -> Result<T, E>,
) -> Result<T, E> {
    if ctx.is_pending()
        && !matches!(
            pending_retention,
            PendingDecisionRetention::Checkpoint
                | PendingDecisionRetention::CheckpointResult
                | PendingDecisionRetention::SuccessfulSuspensionFromBody
                | PendingDecisionRetention::SuccessfulRoutingPendingOverride
                | PendingDecisionRetention::ErrorOnly
        )
    {
        if let Some(pending_value) = pending_value.take() {
            return Ok(pending_value());
        }
    }
    let context = ctx.capture_checkpoint();
    let checkpoint = game.clone();
    let result = body(game, ctx, &checkpoint);
    let pending = ctx.is_pending();
    let rollback = match &result {
        Err(error) => should_rollback_error(error),
        Ok(value) => {
            !matches!(pending_retention, PendingDecisionRetention::ErrorOnly)
                && (pending || !should_commit(value))
        }
    };
    if rollback {
        let retain_pending = pending
            && (result.is_ok()
                || matches!(pending_retention, PendingDecisionRetention::AnySuspension));
        if matches!(
            pending_retention,
            PendingDecisionRetention::Checkpoint | PendingDecisionRetention::CheckpointResult
        ) {
            *game = checkpoint;
        } else {
            game.restore_execution_checkpoint(checkpoint, retain_pending);
        }
        ctx.restore_checkpoint(context);
    }
    if pending
        && (result.is_ok()
            || matches!(
                pending_retention,
                PendingDecisionRetention::Checkpoint
                    | PendingDecisionRetention::SuccessfulRoutingPendingOverride
            ))
    {
        match pending_value {
            Some(pending_value) => Ok(pending_value()),
            None => result,
        }
    } else {
        result
    }
}
