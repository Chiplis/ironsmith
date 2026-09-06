//! The random actions of `SubjectVerbActionAst`.

use super::*;

#[derive(Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub enum RandomActionAst {
    FlipCoin,
    /// Flip without a call when only the physical heads/tails face matters.
    FlipCoinFaceOnly,
    RollDie {
        sides: u32,
        surface: Option<DieSurface>,
    },
    RollDiceChooseResult {
        count: u32,
        sides: u32,
        surface: Option<DieSurface>,
    },
}
