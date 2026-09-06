//! The turnstructure actions of `SubjectVerbActionAst`.

use super::*;

#[derive(Clone, PartialEq)]
#[derive(TagKeyWalk)]
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
    AdditionalPhases {
        phases: Vec<crate::effects::AdditionalPhase>,
    },
}
