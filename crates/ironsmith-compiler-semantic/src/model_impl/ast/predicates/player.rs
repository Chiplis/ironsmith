//! The player actions of `PredicateAst`.

use ironsmith_compiler_ast::TagRef;
use super::*;

#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub enum PlayerPredicateAst {
    PlayerTaggedObjectMatches {
        player: PlayerAst,
        tag: TagRef,
        filter: ObjectFilter,
        mode: ironsmith_core::TaggedObjectMatchMode,
    },
    PlayerTaggedObjectEnteredBattlefieldThisTurn {
        player: PlayerAst,
        tag: TagRef,
    },
    PlayerControls {
        player: PlayerAst,
        filter: ObjectFilter,
    },
    PlayerHasAtLeast {
        player: PlayerAst,
        filter: ObjectFilter,
        count: u32,
    },
    PlayerControlsExactly {
        player: PlayerAst,
        filter: ObjectFilter,
        count: u32,
    },
    PlayerHasAtLeastWithDifferentPowers {
        player: PlayerAst,
        filter: ObjectFilter,
        count: u32,
    },
    PlayerControlsOrHasCardInGraveyard {
        player: PlayerAst,
        control_filter: ObjectFilter,
        graveyard_filter: ObjectFilter,
    },
    PlayerOwnsCardNamedInZones {
        player: PlayerAst,
        name: String,
        zones: Vec<Zone>,
    },
    PlayerControlsNo {
        player: PlayerAst,
        filter: ObjectFilter,
    },
    PlayerControlsMost {
        player: PlayerAst,
        filter: ObjectFilter,
    },
    PlayerControlsMoreThanEachOtherPlayer {
        player: PlayerAst,
        filter: ObjectFilter,
    },
    PlayerControlsMoreThanYou {
        player: PlayerAst,
        filter: ObjectFilter,
    },
    PlayerLifeAtMostHalfStartingLifeTotal {
        player: PlayerAst,
    },
    PlayerLifeLessThanHalfStartingLifeTotal {
        player: PlayerAst,
    },
    PlayerHasLessLifeThanYou {
        player: PlayerAst,
    },
    PlayerHasMoreLifeThanYou {
        player: PlayerAst,
    },
    PlayerHasNoOpponentWithMoreLifeThan {
        player: PlayerAst,
    },
    PlayerHasMoreLifeThanEachOtherPlayer {
        player: PlayerAst,
    },
    PlayerIsMonarch {
        player: PlayerAst,
    },
    PlayerHasInitiative {
        player: PlayerAst,
    },
    PlayerHasCitysBlessing {
        player: PlayerAst,
    },
    PlayerRingTemptedThisGameOrMore {
        player: PlayerAst,
        count: u32,
    },
    PlayerCompletedDungeon {
        player: PlayerAst,
        dungeon_name: Option<String>,
    },
    PlayerTappedLandForManaThisTurn {
        player: PlayerAst,
    },
    PlayerGainedLifeThisTurnOrMore {
        player: PlayerAst,
        count: u32,
    },
    PlayerHadLandEnterBattlefieldThisTurn {
        player: PlayerAst,
    },
    PlayerDescendedThisTurn {
        player: PlayerAst,
    },
    PlayerControlsBasicLandTypesAmongLandsOrMore {
        player: PlayerAst,
        count: u32,
    },
    PlayerHasCardTypesInGraveyardOrMore {
        player: PlayerAst,
        count: u32,
    },
    PlayerCardsInHandOrMore {
        player: PlayerAst,
        count: u32,
    },
    PlayerCardsInHandOrFewer {
        player: PlayerAst,
        count: u32,
    },
    PlayerCardsInHandAtTurnStartOrMore {
        player: PlayerAst,
        count: u32,
    },
    PlayerCardsInHandAtTurnStartOrFewer {
        player: PlayerAst,
        count: u32,
    },
    PlayerHasMoreCardsInHandThanYou {
        player: PlayerAst,
    },
    PlayerHasMoreCardsInHandThanEachOtherPlayer {
        player: PlayerAst,
    },
    PlayerHasPoisonCountersOrMore {
        player: PlayerAst,
        count: u32,
    },
    PlayerCastSpellsThisTurnOrMore {
        player: PlayerAst,
        count: u32,
    },
    PlayerWouldDrawCard {
        player: PlayerAst,
    },
    PlayerWouldProliferate {
        player: PlayerAst,
    },
    PlayerWouldBeginExtraTurn {
        player: PlayerAst,
    },
    PlayerWasDealtCombatDamageByCreatureSubtypeThisTurn {
        player: PlayerAst,
        subtype: Subtype,
    },
    /// "if you rolled a 6 this turn"
    PlayerRolledResultThisTurn {
        player: PlayerAst,
        result: u32,
    },
    /// "if you committed a crime this turn"
    PlayerCommittedCrimeThisTurn {
        player: PlayerAst,
    },
    /// "if you removed a card matching this from the draft"
    PlayerRemovedDraftCardMatching {
        player: PlayerAst,
        filter: ObjectFilter,
        with_cards_named: String,
    },
}
