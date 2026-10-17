//! The turnstructure actions of `SubjectVerbActionAst`.

use super::*;

#[derive(Clone, PartialEq, TagKeyWalk)]
pub enum TurnStructureActionAst {
    AdditionalLandPlays {
        count: Value,
        duration: Until,
    },
    SkipTurn,
    SkipCombatPhases,
    SkipNextCombatPhaseThisTurn,
    SkipMainPhasesThisTurn,
    SkipCombatPhasesThisTurn,
    SkipDrawStep,
    SkipScheduled { kind: ironsmith_core::ScheduledSkipKind, count: u32 },
    AdditionalPhases {
        phases: Vec<crate::effects::AdditionalPhase>,
        after_main_phase: bool,
    },
    /// "you get an additional upkeep step after this step" / "there is an
    /// additional end step after this step" (CR 500.10). `player_gets` is the
    /// CR 500.10a "<player> gets" form bound to the subject player.
    AdditionalSteps {
        step: crate::effects::AdditionalStepKind,
        count: Value,
        anchor: crate::effects::AdditionalStepAnchor,
        player_gets: bool,
    },
}
