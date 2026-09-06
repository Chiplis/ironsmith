//! The source actions of `PredicateAst`.

use super::*;

#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub enum SourcePredicateAst {
    SourceControllersMainPhase,
    SourceChosenOption(String),
    SourceBlockedOrBecameBlockedSinceLastUpkeep,
    SourceIsRingBearer {
        player: PlayerAst,
    },
    SourceIsTapped,
    SourceIsEquipped,
    SourceIsEnchanted,
    SourceIsSaddled,
    SourceIsRenowned,
    SourceCrewedByExactly {
        count: u32,
        filter: ObjectFilter,
    },
    SourceMatches(ObjectFilter),
    SourceHasNoCounter(CounterType),
    SourceHasCounterAtLeast {
        counter_type: CounterType,
        count: u32,
        surface: crate::SourceCounterThresholdSurface,
    },
    SourceHasCountersAtLeast(u32),
    SourceHasAttachmentsMatching {
        filter: ObjectFilter,
        comparison: crate::effect::Comparison,
        display: String,
    },
    SourcePowerAtLeast(u32),
    SourceDealtCombatDamageToPlayerThisTurn,
    SourceAttackedThisTurn,
    SourceSuspected,
    SourceCameUnderYourControlThisTurn,
    SourceAttackedOrBlockedThisTurn,
    SourceInGraveyardWithCardsAbove {
        filter: ObjectFilter,
        count: u32,
    },
    SourceIsInZone(Zone),
    SourceWasCast,
    /// "if this creature attacked a battle this turn"
    SourceAttackedBattleThisTurn,
    /// "as long as this creature is paired"
    SourceIsSoulbondPaired,
    /// "if this creature devoured two or more creatures"
    SourceDevouredCreaturesOrMore(u32),
    /// "at the beginning of its controller's end step"
    SourceControllersEndStep,
    /// "as long as this creature is attacking"
    SourceIsAttacking,
    /// "as long as this permanent is untapped"
    SourceIsUntapped,
    /// "as long as this creature is monstrous"
    SourceIsMonstrous,
}
