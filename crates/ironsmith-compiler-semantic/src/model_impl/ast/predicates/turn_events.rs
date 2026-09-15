//! The turnevents actions of `PredicateAst`.

use super::*;

#[derive(Debug, Clone, PartialEq, TagKeyWalk)]
pub enum TurnEventPredicateAst {
    OpponentLostLifeThisTurn,
    AnyPlayerLostLifeThisTurnOrMore {
        count: u32,
    },
    OpponentWasDealtDamageThisTurn,
    /// "if an opponent was dealt 7 or more damage this turn" (Spinerock Knoll)
    OpponentWasDealtDamageThisTurnOrMore(u32),
    YouAttackedWithExactlyNOtherCreaturesThisCombat(u32),
    CreatureDiedThisTurn,
    CreatureDiedThisTurnOrMore(u32),
    CreatureDealtDamageBySourceDiedThisTurn {
        victim: ObjectFilter,
        damager: DamageBySpec,
        count: u32,
    },
    CreatureCardPutIntoYourGraveyardThisTurn,
    PermanentLeftBattlefieldThisTurn,
    NonlandPermanentLeftBattlefieldThisTurn,
    SpellWasWarpedThisTurn,
    PermanentLeftBattlefieldUnderYourControlThisTurn {
        surface: crate::PermanentLeftBattlefieldControlSurface,
    },
    ObjectEnteredBattlefieldThisTurn(ObjectFilter),
    ObjectEnteredBattlefieldLastTurn(ObjectFilter),
    ObjectPutIntoGraveyardFromBattlefieldThisTurn(ObjectFilter),
    YouAttackedThisTurn,
    YouAttackedWithNOrMoreCreaturesThisTurn(u32),
    NoSpellsWereCastLastTurn,
    /// "if you attacked this turn"
    AttackedThisTurn,
    ThisAbilityResolvedThisTurnExactly(u32),
}
