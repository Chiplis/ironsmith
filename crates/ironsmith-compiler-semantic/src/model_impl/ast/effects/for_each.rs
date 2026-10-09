//! The iteration actions of `EffectAst`.

use super::*;
use ironsmith_compiler_ast::TagRef;

#[derive(Debug, Clone, PartialEq, TagKeyWalk)]
pub enum ForEachEffectAst {
    RepeatThisProcess,
    RepeatThisProcessMay,
    RepeatThisProcessOnce,
    /// "repeat this process except that <chooser> can't choose a card already
    /// chosen for <this>": a required repeat whose later rounds exclude every
    /// object chosen in an earlier round of the same process.
    RepeatThisProcessExcludingPriorChoices,
    /// A finite number of additional executions of the preceding program.
    RepeatThisProcessAdditional { count: Value },
    RepeatEffects {
        count: Value,
        effects: Vec<EffectAst>,
    },
    ForEachOpponent {
        effects: Vec<EffectAst>,
    },
    ForEachPlayersFiltered {
        sequential: bool,
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
        require_evidence: bool,
    },
    RepeatProcess {
        effects: Vec<EffectAst>,
        continue_effect_index: usize,
        continue_predicate: IfResultPredicate,
    },
}
