//! Selected movement instructions retain the shared zone owner's lifecycle.

use super::{AppliedZoneChange, PreparedZoneMove};
use crate::effect::EffectOutcome;
use crate::effects::{
    CompletedEffectOutputs, ExecutionContext, ExecutionError, SimultaneousEffectCommit,
    SimultaneousEffectProposal,
};
use crate::events::processing::{PreparedEventOutcome, PreparedZoneChange};
use crate::game_state::GameState;
use crate::ids::ObjectId;

type MovementReceipt = (ObjectId, PreparedEventOutcome<AppliedZoneChange>);
type MovementProposal = (ObjectId, PreparedEventOutcome<PreparedZoneChange>);
type MovementProjection = dyn FnOnce(
        &mut GameState,
        &mut ExecutionContext<'_>,
        &[MovementReceipt],
        usize,
    ) -> Result<EffectOutcome, ExecutionError>
    + Send;

/// Selection fixes subjects, snapshots and authored arrival metadata. It must
/// not commit a move or execute replacement-added programs.
pub(super) trait ZoneMovementInstruction: std::fmt::Debug + Send {
    fn select(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<SelectedZoneMovement, ExecutionError>;
}

pub(super) enum SelectedZoneMovement {
    Finished(EffectOutcome),
    Moves {
        requests: Vec<PreparedZoneMove>,
        projection: Box<MovementProjection>,
    },
}

impl SelectedZoneMovement {
    pub(super) fn moves(
        requests: Vec<PreparedZoneMove>,
        projection: impl FnOnce(
            &mut GameState,
            &mut ExecutionContext<'_>,
            &[MovementReceipt],
            usize,
        ) -> Result<EffectOutcome, ExecutionError>
        + Send
        + 'static,
    ) -> Self {
        Self::Moves {
            requests,
            projection: Box::new(projection),
        }
    }
}

enum MovementInstructionState {
    Selection(Box<dyn ZoneMovementInstruction>),
    Prepared {
        proposals: Vec<MovementProposal>,
        projection: Box<MovementProjection>,
    },
    Finished(EffectOutcome),
    Preparing,
}

struct PreparedMovementInstruction {
    iterated_player: Option<crate::ids::PlayerId>,
    state: MovementInstructionState,
}

impl std::fmt::Debug for PreparedMovementInstruction {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PreparedMovementInstruction")
            .field(
                "selected",
                &!matches!(self.state, MovementInstructionState::Selection(_)),
            )
            .finish_non_exhaustive()
    }
}

pub(super) fn prepare_movement_instruction(
    instruction: impl ZoneMovementInstruction + 'static,
    ctx: &ExecutionContext,
) -> Box<dyn SimultaneousEffectProposal> {
    Box::new(PreparedMovementInstruction {
        iterated_player: ctx.iteration.iterated_player,
        state: MovementInstructionState::Selection(Box::new(instruction)),
    })
}

/// Ordinary and enclosing simultaneous execution consume the same retained
/// proposal; the enclosing coordinator owns transactions and batch scopes.
pub(super) fn execute_movement_instruction(
    instruction: impl ZoneMovementInstruction + 'static,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Result<CompletedEffectOutputs, ExecutionError> {
    crate::effects::composition::execute_transaction(
        game,
        ctx,
        || CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
        |game, ctx| {
            crate::effects::composition::complete_prepared_original_with_outputs(
                prepare_movement_instruction(instruction, ctx),
                game,
                ctx,
                false,
            )
        },
    )
}

impl SimultaneousEffectProposal for PreparedMovementInstruction {
    fn has_simultaneous_originals(&self) -> bool {
        matches!(&self.state, MovementInstructionState::Prepared { proposals, .. } if proposals.len() > 1)
    }

    fn prepare_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        let iterated_player = self.iterated_player;
        ctx.with_temp_iterated_player(iterated_player, |ctx| {
            if !matches!(self.state, MovementInstructionState::Selection(_)) {
                return Ok(());
            }
            let MovementInstructionState::Selection(instruction) =
                std::mem::replace(&mut self.state, MovementInstructionState::Preparing)
            else {
                unreachable!();
            };
            let selected = instruction.select(game, ctx)?;
            if ctx.decision_maker.awaiting_choice() {
                self.state = MovementInstructionState::Finished(EffectOutcome::count(0));
                return Ok(());
            }
            self.state = match selected {
                SelectedZoneMovement::Finished(outcome) => {
                    MovementInstructionState::Finished(outcome)
                }
                SelectedZoneMovement::Moves {
                    requests,
                    projection,
                } => {
                    let proposals = super::prepare_zone_moves(game, ctx, requests)?;
                    if ctx.decision_maker.awaiting_choice() {
                        MovementInstructionState::Finished(EffectOutcome::count(0))
                    } else {
                        MovementInstructionState::Prepared {
                            proposals,
                            projection,
                        }
                    }
                }
            };
            Ok(())
        })
    }

    fn commit_original_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
        let iterated_player = self.iterated_player;
        ctx.with_temp_iterated_player(iterated_player, |ctx| match self.state {
            MovementInstructionState::Finished(outcome) => Ok(SimultaneousEffectCommit::finished(
                CompletedEffectOutputs::aggregate_only(outcome),
            )),
            MovementInstructionState::Prepared {
                proposals,
                projection,
            } => {
                let pending_start = game.effect_store.pending_trigger_events.len();
                let receipts =
                    super::prepared_move::commit_prepared_zone_moves(game, ctx, proposals)?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(SimultaneousEffectCommit::finished(
                        CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
                    ));
                }
                let original = projection(game, ctx, &receipts, pending_start)?;
                super::complete_movement_batch(game, ctx, original, receipts, true)
                    .map(SimultaneousEffectCommit::into_retained)
            }
            MovementInstructionState::Selection(_) | MovementInstructionState::Preparing => {
                Err(ExecutionError::InternalError(
                    "movement instruction committed before selection and replacement preparation"
                        .into(),
                ))
            }
        })
    }

    fn commit_original(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<SimultaneousEffectCommit, ExecutionError> {
        self.commit_original_with_outputs(game, ctx)
            .map(SimultaneousEffectCommit::into_aggregate)
    }

    fn commit(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let iterated_player = self.iterated_player;
        ctx.with_temp_iterated_player(iterated_player, |ctx| {
            let simultaneous = self.has_simultaneous_originals();
            crate::effects::composition::complete_prepared_original_with_outputs(
                self,
                game,
                ctx,
                simultaneous,
            )
            .map(CompletedEffectOutputs::into_outcome)
        })
    }
}
