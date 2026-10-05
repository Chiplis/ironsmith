//! The random actions of `SubjectVerbActionAst`.

use super::*;

#[derive(Clone, PartialEq, TagKeyWalk)]
pub enum RandomActionAst {
    FlipCoin,
    /// Flip without a call when only the physical heads/tails face matters.
    FlipCoinFaceOnly,
    FlipCoins {
        count: u32,
        kind: ironsmith_core::CoinFlipKind,
        repeat_until_loss: bool,
        stop_condition: Option<ironsmith_core::CoinFlipStopCondition>,
        loss_action: Option<ironsmith_core::CoinFlipLossAction>,
        opponent_results: Option<(ironsmith_compiler_ast::TagRef, ironsmith_compiler_ast::TagRef)>,
        count_value: Option<Value>,
    },
    RollDie {
        sides: u32,
        surface: Option<DieSurface>,
        result_modifier: Option<ironsmith_core::effect::DieResultModifier>,
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
