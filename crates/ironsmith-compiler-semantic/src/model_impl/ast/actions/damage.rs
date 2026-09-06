//! The damage actions of `SubjectVerbActionAst`.

use super::*;

#[derive(Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub enum DamageActionAst {
    DealDamage {
        amount: Value,
        target: TargetAst,
        unpreventable: bool,
    },
    DealDamageEach {
        amount: Value,
        filter: ObjectFilter,
    },
    DealDamageEqualToPower {
        source: TargetAst,
        amount: Value,
        target: TargetAst,
        unpreventable: bool,
    },
    DealDistributedDamage {
        amount: Value,
        target: TargetAst,
        source: TargetAst,
        chooser: PlayerFilter,
        distribution: ironsmith_core::DamageDistributionMode,
    },
    HealDamage {
        target: TargetAst,
        amount: Option<Value>,
    },
}
