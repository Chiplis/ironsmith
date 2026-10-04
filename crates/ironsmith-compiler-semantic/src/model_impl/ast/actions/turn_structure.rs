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
}
