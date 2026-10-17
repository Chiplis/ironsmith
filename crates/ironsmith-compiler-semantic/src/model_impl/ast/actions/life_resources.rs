//! The liferesources actions of `SubjectVerbActionAst`.

use super::*;
use ironsmith_compiler_ast::TagRef;

#[derive(Clone, PartialEq, TagKeyWalk)]
pub enum LifeResourceActionAst {
    Draw { count: Value },
    DrawForEachTaggedMatching { tag: TagRef, filter: ObjectFilter },
    LoseLife { amount: Value },
    PayLife { amount: Value },
    GainLife { amount: Value },
    NoteLifeTotal,
    /// "Redistribute any number of players' life totals" (Reverse the
    /// Sands): the acting player chooses the players and reassigns their
    /// life totals among them.
    RedistributeLifeTotals,
    PayEnergy { amount: Value },
    PayAnyEnergy { min_amount: u32 },
    PayAnyLife { min_amount: u32, maximum: Option<Value>, source_owned: bool },
}
