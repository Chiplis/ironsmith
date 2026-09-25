//! The random actions of `SubjectVerbActionAst`.

use super::*;

#[derive(Clone, PartialEq, TagKeyWalk)]
pub enum RandomActionAst {
    FlipCoin,
    /// Flip without a call when only the physical heads/tails face matters.
    FlipCoinFaceOnly,
    FlipCoins {
        count: u32,
    },
    RollDie {
        sides: u32,
        surface: Option<DieSurface>,
    },
    RollDiceChooseResult {
        count: u32,
        sides: u32,
        surface: Option<DieSurface>,
    },
    /// "Choose 1, 2, or 3 at random": the result is the chosen number.
    ChooseNumberAtRandom {
        choices: Vec<u32>,
    },
}
