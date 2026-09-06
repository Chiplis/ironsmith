//! The iteration actions of `EffectAst`.

use ironsmith_compiler_ast::TagRef;
use super::*;

#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub enum ForEachEffectAst {
    RepeatThisProcess,
    RepeatThisProcessMay,
    RepeatThisProcessOnce,
    RepeatEffects {
        count: Value,
        effects: Vec<EffectAst>,
    },
    ForEachOpponent {
        effects: Vec<EffectAst>,
    },
    ForEachPlayersFiltered {
        filter: PlayerFilter,
        effects: Vec<EffectAst>,
    },
    ForEachPlayer {
        effects: Vec<EffectAst>,
    },
    ForEachTargetPlayers {
        count: ChoiceCount,
        filter: PlayerFilter,
        effects: Vec<EffectAst>,
    },
    ForEachObject {
        filter: ObjectFilter,
        effects: Vec<EffectAst>,
    },
    ForEachTagged {
        tag: TagRef,
        effects: Vec<EffectAst>,
    },
    /// Iterate a tagged result while binding `IteratedPlayer` to the
    /// controller recorded by the latest block event against `blocker_tag`.
    /// The ordinary `ForEachTagged` continues to use the result snapshot's
    /// controller at the time it was tagged.
    ForEachTaggedWithControllerAtLastBlockedBy {
        tag: TagRef,
        blocker_tag: TagRef,
        effects: Vec<EffectAst>,
    },
    ForEachOpponentDoesNot {
        effects: Vec<EffectAst>,
        predicate: Option<PredicateAst>,
    },
    ForEachPlayerDoesNot {
        effects: Vec<EffectAst>,
        predicate: Option<PredicateAst>,
    },
    ForEachOpponentDid {
        effects: Vec<EffectAst>,
        predicate: Option<PredicateAst>,
        result_predicate: IfResultPredicate,
    },
    ForEachPlayerDid {
        effects: Vec<EffectAst>,
        predicate: Option<PredicateAst>,
        result_predicate: IfResultPredicate,
    },
    ForEachTaggedPlayer {
        tag: TagRef,
        effects: Vec<EffectAst>,
    },
    RepeatProcess {
        effects: Vec<EffectAst>,
        continue_effect_index: usize,
        continue_predicate: IfResultPredicate,
    },
}
