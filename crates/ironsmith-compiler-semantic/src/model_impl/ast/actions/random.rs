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
        /// "... and ignore the lower roll" instead of choosing one result.
        ignore_lower: bool,
        /// "roll a die twice ... the first result ... the second result":
        /// nothing is chosen; the rolls keep their order.
        in_order: bool,
    },
    /// "Choose 1, 2, or 3 at random": the result is the chosen number.
    ChooseNumberAtRandom {
        choices: Vec<u32>,
    },
    /// "Roll X d6" / "roll two six-sided dice" whose results are read back
    /// as "the total of those results" (CR 706.2): one simultaneous roll of
    /// `count` dice; the result is their sum.
    RollDiceTotal {
        count: Value,
        sides: u32,
        surface: Option<DieSurface>,
        /// "roll a d20 for each player being attacked and ignore all but the
        /// highest roll" (Iron Mastiff): the result is the highest roll, not
        /// the total.
        highest_only: bool,
    },
}
