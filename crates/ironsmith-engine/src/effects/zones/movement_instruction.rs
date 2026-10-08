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
pub(crate) trait ZoneMovementInstruction: std::fmt::Debug + Send {
    fn select(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<SelectedZoneMovement, ExecutionError>;
}

pub(crate) enum SelectedZoneMovement {
    Finished(EffectOutcome),
    Moves {
        requests: Vec<PreparedZoneMove>,
        projection: Box<MovementProjection>,
    },
}

impl SelectedZoneMovement {
    pub(crate) fn moves(
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
    Selected {
        requests: Vec<PreparedZoneMove>,
        projection: Box<MovementProjection>,
    },
    Prepared {
        proposals: Vec<MovementProposal>,
        projection: Box<MovementProjection>,
        draws: super::ZoneInstructionDraws,
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

pub(crate) fn prepare_movement_instruction(
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
pub(crate) fn execute_movement_instruction(
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

    fn prepare_selection(
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
                } => MovementInstructionState::Selected {
                    requests,
                    projection,
                },
            };
            Ok(())
        })
    }

    fn prepare_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        self.prepare_selection(game, ctx)?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(());
        }
        let state = std::mem::replace(&mut self.state, MovementInstructionState::Preparing);
        self.state = match state {
            MovementInstructionState::Selected {
                requests,
                projection,
            } => {
                let (proposals, draws) = ctx
                    .with_temp_iterated_player(self.iterated_player, |ctx| {
                        super::prepare_zone_moves(game, ctx, requests)
                    })?;
                MovementInstructionState::Prepared {
                    proposals,
                    projection,
                    draws,
                }
            }
            other => other,
        };
        Ok(())
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
                mut draws,
            } => {
                let pending_start = game.effect_store.pending_trigger_events.len();
                let receipts = super::prepared_move::commit_prepared_zone_moves(
                    game, ctx, proposals, &mut draws,
                )?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(SimultaneousEffectCommit::finished(
                        CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
                    ));
                }
                let original = projection(game, ctx, &receipts, pending_start)?;
                Ok(draws.finish(original, receipts, ctx).into_retained())
            }
            MovementInstructionState::Selection(_)
            | MovementInstructionState::Selected { .. }
            | MovementInstructionState::Preparing => Err(ExecutionError::InternalError(
                "movement instruction committed before selection and replacement preparation"
                    .into(),
            )),
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

/// A replacement-created movement keeps its non-draw originals now and returns
/// the same prepared completion at the first actual draw boundary.
pub(crate) fn prepare_movement_draw_continuation(
    instruction: impl ZoneMovementInstruction + 'static,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
    let mut proposal = prepare_movement_instruction(instruction, ctx);
    proposal.prepare_selection(game, ctx)?;
    if ctx.decision_maker.awaiting_choice() {
        return Ok(SimultaneousEffectCommit::finished(
            CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
        ));
    }
    proposal.prepare_original(game, ctx)?;
    if ctx.decision_maker.awaiting_choice() {
        return Ok(SimultaneousEffectCommit::finished(
            CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
        ));
    }
    proposal.seal_original(game, ctx)?;
    let committed = proposal.commit_original_with_outputs(game, ctx)?;
    if let Some(mut completion) = committed.completion {
        let mut original = committed.outcome;
        completion.freeze(game)?;
        completion.observe_original(game, ctx, &mut original.outcome)?;
        original.synchronize_observations();
        let prepared = completion.prepare_draw_boundary_from_outputs(game, ctx, original)?;
        Ok(prepared)
    } else {
        Ok(committed)
    }
}
