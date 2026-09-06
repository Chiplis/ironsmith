//! The liferesources actions of `SubjectVerbActionAst`.

use ironsmith_compiler_ast::TagRef;
use super::*;

#[derive(Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub enum LifeResourceActionAst {
    Draw {
        count: Value,
    },
    DrawForEachTaggedMatching {
        tag: TagRef,
        filter: ObjectFilter,
    },
    LoseLife {
        amount: Value,
    },
    PayLife {
        amount: Value,
    },
    GainLife {
        amount: Value,
    },
    NoteLifeTotal,
    PayEnergy {
        amount: Value,
    },
    PayAnyEnergy {
        min_amount: u32,
    },
    PayAnyLife {
        min_amount: u32,
    },
}
