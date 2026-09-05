use crate::tag::TagKeyWalk;

use crate::{CardType, Condition, ObjectFilter, PlayerFilter, Subtype};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub enum ThisSpellCostCondition {
    Always,
    YourTurn,
    NotYourTurn,
    YouLifeTotalOrLess(i32),
    OpponentHasNoCardsInHand,
    OpponentControlsLandsOrMore(u32),
    OpponentControlsAtLeastNMoreCreaturesThanYou(u32),
    TotalCreatureCardsInAllGraveyardsOrMore(u32),
    OpponentCastSpellsThisTurnOrMore(u32),
    OpponentDrewCardsThisTurnOrMore(u32),
    YouWereDealtDamageByCreaturesThisTurnOrMore(u32),
    ConditionExpr {
        condition: Condition,
        display: String,
    },
    /// A continuously checked condition authored with "as long as" rather
    /// than the ordinary trailing "if" surface.
    AsLongAsConditionExpr {
        condition: Condition,
        display: String,
    },
    TargetsPlayer(PlayerFilter),
    TargetsObject(ObjectFilter),
    TargetsObjectWhoseControllerHasCardsInGraveyardOrMore {
        filter: ObjectFilter,
        count: u32,
    },
    YouCastSpellsThisTurnOrMore {
        count: u32,
        card_types: Vec<CardType>,
    },
    YouGainedLifeThisTurnOrMore(u32),
    OpponentHasPoisonCountersOrMore(u32),
    OpponentHasCardsInGraveyardOrMore(u32),
    DistinctCardTypesInYourGraveyardOrMore(u32),
    LifeTotalLessThanStarting,
    IsNight,
    YouSacrificedArtifactThisTurn,
    YouCommittedCrimeThisTurn,
    CreatureLeftBattlefieldUnderYourControlThisTurn,
    YouHaveCardsInYourGraveyardOrMore(u32),
    YouHaveCardsOfTypesInYourGraveyardOrMore {
        count: u32,
        card_types: Vec<CardType>,
    },
    OnlyCreatureCardsInHandNamed(String),
    NoCardsInHandMatching {
        filter: ObjectFilter,
        display: String,
    },
    CardInYourGraveyardMatching {
        filter: ObjectFilter,
        display: String,
    },
    NotStartingPlayer,
    CreatureCardPutIntoYourGraveyardThisTurn,
    CreatureIsAttackingYou,
    YouDealtCombatDamageToPlayerWithSubtypeThisTurn(Subtype),
    YouDealtCombatDamageToPlayerWithSubtypeOrCommanderThisTurn(Subtype),
}
