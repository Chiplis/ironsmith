//! The damage actions of `SubjectVerbActionAst`.

use super::*;

#[derive(Clone, PartialEq, TagKeyWalk)]
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
    /// "[If ...,] excess damage is dealt to that creature's controller
    /// instead": modifies the damage instruction immediately before it.
    ExcessDamageToController {
        condition: Option<PredicateAst>,
    },
    /// One source deals one shared amount to a union of referenced recipients
    /// and quantified groups. The complete set is sampled before any damage.
    DealDamageToRecipients {
        amount: Value,
        recipients: Vec<TargetAst>,
        object_groups: Vec<ObjectFilter>,
        player_groups: Vec<PlayerFilter>,
    },
    /// A source set deals each member's independently evaluated amount to one
    /// recipient in one simultaneous damage occurrence.
    DealDamageBySources {
        sources: Vec<TargetAst>,
        source_binding: ironsmith_core::DamageSourceSetBinding,
        amount: Value,
        target: TargetAst,
    },
}
