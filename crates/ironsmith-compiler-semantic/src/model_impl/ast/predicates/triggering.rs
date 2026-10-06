//! The triggering actions of `PredicateAst`.

use super::*;

#[derive(Debug, Clone, PartialEq, TagKeyWalk)]
pub enum TriggeringPredicateAst {
    /// The object in the surrounding tap event is becoming tapped for the
    /// first time this turn. This is per object, not per triggered ability.
    TriggeringObjectBecameTappedFirstTimeThisTurn,
    /// The object in the surrounding counter event is receiving counters for
    /// the first time this turn. This is per object, not per triggered
    /// ability.
    TriggeringObjectHadCountersPutFirstTimeThisTurn,
    TriggeringObjectHadToAttackThisCombat,
    /// "If you won" inside a "Whenever you clash" ability: the triggering
    /// clash was won by this ability's controller.
    YouWonTriggeringClash,
    /// The triggering ability activation was paid with at least this much
    /// mana.
    TriggeringAbilityManaSpentToActivateAtLeast(u32),
    /// The entering permanent entered transformed.
    TriggeringObjectEnteredTransformed,
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
    TriggeringSpellWasKicked,
    YouChoseAnotherRingBearer,
    TriggeringEventCausedBy { controller: PlayerFilter, effect_like_only: bool },
    CombatParticipant(ironsmith_core::CombatParticipantCondition),
}
