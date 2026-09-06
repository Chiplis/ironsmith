//! The triggering actions of `PredicateAst`.

use super::*;

#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub enum TriggeringPredicateAst {
    /// The object in the surrounding tap event is becoming tapped for the
    /// first time this turn. This is per object, not per triggered ability.
    TriggeringObjectBecameTappedFirstTimeThisTurn,
    /// The object in the surrounding counter event is receiving counters for
    /// the first time this turn. This is per object, not per triggered
    /// ability.
    TriggeringObjectHadCountersPutFirstTimeThisTurn,
    TriggeringObjectHadToAttackThisCombat,
    TriggeringObjectHadNoCounter(CounterType),
    TriggeringObjectHadCounterAtLeast {
        counter_type: CounterType,
        count: u32,
    },
    TriggeringSpellManaSpentToCastAtLeast {
        amount: u32,
        symbol: Option<ManaSymbol>,
    },
    TriggeringSpellColoredManaSpentToCastAtLeast(u32),
}
