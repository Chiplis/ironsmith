//! The conditionals actions of `EffectAst`.

use super::*;

#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub enum ConditionalEffectAst {
    UnlessPays {
        effects: Vec<EffectAst>,
        player: PlayerAst,
        cost: ironsmith_core::TotalCost<crate::model::CompilerCost>,
        /// The Oracle clause says the cost may be paid before the delayed
        /// step, rather than when the delayed consequence resolves.
        before_delayed_step: bool,
    },
    UnlessAction {
        effects: Vec<EffectAst>,
        alternative: Vec<EffectAst>,
        player: PlayerAst,
    },
    Conditional {
        predicate: PredicateAst,
        if_true: Vec<EffectAst>,
        if_false: Vec<EffectAst>,
    },
    /// A resolution-time gate authored after the effect as
    /// "<effect> if <predicate>". Keeping this distinct from an ordinary
    /// conditional preserves word order and prevents trigger preparation from
    /// treating it as an intervening-if condition.
    TrailingIf {
        predicate: PredicateAst,
        effects: Vec<EffectAst>,
    },
    /// A resolution-time gate printed after the effect as
    /// "<effect> unless <positive predicate>". Keeping this distinct from a
    /// sole ordinary conditional prevents triggered-ability preparation from
    /// promoting it to an intervening-if condition.
    TrailingUnless {
        predicate: PredicateAst,
        effects: Vec<EffectAst>,
    },
    /// Lower `effect` (which must lower to a single runtime effect) under a
    /// fresh internal effect id, then emit an `if_then(id, DidNotHappen,
    /// otherwise)`. The effect id stays internal to lowering and is never
    /// exposed in the AST. Lowers to `Effect::with_id` + `Effect::if_then`.
    IfEffectDidNotHappen {
        effect: Box<EffectAst>,
        otherwise: Vec<EffectAst>,
    },
    /// Lower one producer under a fresh internal effect id, then gate
    /// `if_true` on a typed predicate over that exact producer's outcome.
    /// The internal id is compiler bookkeeping and never appears in parsed
    /// card text.
    IfEffectResult {
        effect: Box<EffectAst>,
        predicate: crate::effect::EffectPredicate,
        if_true: Vec<EffectAst>,
    },
    ResolvedIfResult {
        condition: EffectId,
        predicate: IfResultPredicate,
        effects: Vec<EffectAst>,
    },
    ResolvedWhenResult {
        condition: EffectId,
        predicate: IfResultPredicate,
        effects: Vec<EffectAst>,
    },
    IfResult {
        predicate: IfResultPredicate,
        effects: Vec<EffectAst>,
    },
    WhenResult {
        predicate: IfResultPredicate,
        effects: Vec<EffectAst>,
    },
}
