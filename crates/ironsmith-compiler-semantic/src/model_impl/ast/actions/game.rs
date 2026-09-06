//! The game actions of `SubjectVerbActionAst`.

use super::*;

#[derive(Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub enum GameActionAst {
    ExtraTurnAfterTurn {
        anchor: ExtraTurnAnchorAst,
    },
    ReverseTurnOrder,
    EndTurn,
    EndCombatPhase,
    LoseGame,
    WinGame,
}
