//! The reveallook actions of `SubjectVerbActionAst`.

use ironsmith_compiler_ast::TagRef;
use super::*;

#[derive(Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub enum RevealLookActionAst {
    RevealHand,
    RevealTop,
    RevealTagged {
        tag: TagRef,
    },
    RevealCardsFromHand {
        count: ChoiceCount,
        count_value: Option<Value>,
        tag: TagRef,
    },
    LookAtTopCards {
        count: Value,
        tag: TagRef,
        reveal: bool,
    },
    LookAtObjects {
        filter: ObjectFilter,
    },
    LookAtTarget {
        target: TargetAst,
    },
    LookAtHand {
        target: TargetAst,
    },
}
