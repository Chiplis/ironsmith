//! Acquired program payloads and their actual execution frame move together.

use crate::effects::{ExecutionContext, ExecutionContextCheckpoint, ExecutionError};
use crate::game_state::GameState;

/// The payload owns its scheduling/lifecycle policy. This frame neither selects
/// an action nor recaptures source evidence while transferring an acquired root.
pub(crate) struct CapturedProgramFrame<T> {
    pub(crate) program: T,
    pub(crate) context: ExecutionContextCheckpoint,
}

impl<T: std::fmt::Debug> std::fmt::Debug for CapturedProgramFrame<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CapturedProgramFrame")
            .field("program", &self.program)
            .finish_non_exhaustive()
    }
}

impl<T> CapturedProgramFrame<T> {
    pub(crate) fn capture(program: T, ctx: &ExecutionContext) -> Self {
        Self {
            program,
            context: ExecutionContextCheckpoint::capture(ctx),
        }
    }

    /// Map actual owned payload metadata without selecting a new source/frame
    /// or evaluating future authored operands. The existing checkpoint moves.
    pub(crate) fn map<U>(self, map: impl FnOnce(T) -> U) -> CapturedProgramFrame<U> {
        let Self { program, context } = self;
        CapturedProgramFrame {
            program: map(program),
            context,
        }
    }

    /// Run in the acquired child frame, borrowing only the caller's decision
    /// maker. Caller restoration/export remains its domain owner's contract.
    pub(crate) fn run<R>(
        self,
        game: &mut GameState,
        parent: &mut ExecutionContext,
        run: impl FnOnce(&mut GameState, &mut ExecutionContext, T) -> Result<R, ExecutionError>,
    ) -> Result<R, ExecutionError> {
        let Self { program, context } = self;
        let mut child = context.reborrow(&mut *parent.decision_maker);
        run(game, &mut child, program)
    }
}
