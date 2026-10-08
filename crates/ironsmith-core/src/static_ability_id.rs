//! Static ability identity enum.
//!
//! This enum provides unique identifiers for each type of static ability.

/// Unique identifier for each type of static ability.
use crate::tag::TagKeyWalk;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TagKeyWalk)]
pub enum StaticAbilityId {
    Flying,
    FirstStrike,
    DoubleStrike,
    Deathtouch,
    Defender,
    Flash,
    Haste,
    Hexproof,
    HexproofFrom,
    Indestructible,
    Intimidate,
    Lifelink,
    Menace,
    Banding,
    BandsWithOther,
    Protection,
    Reach,
    Shroud,
    Trample,
    Vigilance,
    Ward,
    Fear,
    Skulk,
    Prowess,
    Flanking,
    UmbraArmor,
    Landwalk,
    CantBeBlockedAsLongAsDefendingPlayerControlsCardType,
    CantBeBlockedAsLongAsDefendingPlayerControlsCardTypes,
    CantBeBlockedWhileDefendingPlayerControlsMostCreatures,
    Bloodthirst,
    Tribute,
    Daybound,
    Nightbound,
    DayNightStartsDayAsEnters,
    Morph,
    Disguise,
    Megamorph,
    Shadow,
    Horsemanship,
    Phasing,
    Wither,
    Infect,
    Changeling,
    LivingMetal,
    Companion,
    Partner,
    PartnerWith,
    StartYourEngines,
    SpaceSculptor,
    DoctorsCompanion,
    Assist,
    Ascend,
    /// "Storied": if you control three or more artifacts, legendaries, and/or
    /// Sagas, you have an enduring story for the rest of the game.
    Storied,
    SplitSecond,
    Rebound,
    Cascade,
    CascadeLandDrop,
    Splice,
    Escalate,
    ReadAhead,
    Unleash,
    ConditionalSpellKeyword,
    ThisSpellCastRestriction,
    ThisSpellXMaximum,
    ThisSpellXMinimum,
    Unblockable,
    FlyingRestriction,
    FlyingOnlyRestriction,
    CanBlockFlying,
    CanBlockAsThoughNoShadow,
    CanBlockOnlyFlying,
    CanBlockAdditionalCreatureEachCombat,
    MaxCreaturesCanAttackEachCombat,
    MaxCreaturesCanAttackYouEachCombat,
    MaxCreaturesCanBlockEachCombat,
    CantBeBlockedByPowerOrLess,
    CantBeBlockedByPowerOrGreater,
    CantBeBlockedByLowerPowerThanSource,
    CantBeBlockedByMoreThan,
    CantBeBlockedExceptByNOrMore,
    CanAttackAsThoughNoDefender,
    CanAttackAsThoughHaste,
    /// "You may activate abilities of <objects> as though those creatures had haste."
    ActivateAbilitiesAsThoughHaste,
    MustAttack,
    GoadedBySourceController,
    MustAttackAttachedController,
    AllCreaturesAttackAttachedControllerEachCombatIfAble,
    AttachedGoadedBySourceController,
    GoadMatching,
    AttachedControllerMaySacrificePermanentToIgnoreSourceEffectUntilEndOfTurn,
    AnyPlayerMayPayManaToIgnoreSourceEffectUntilEndOfTurn,
    ExertAttack,
    EnlistAttack,
    MustBlock,
    CantAttack,
    CantAttackItsOwner,
    CantAttackUnlessControllerCastCreatureSpellThisTurn,
    CantAttackUnlessControllerCastNonCreatureSpellThisTurn,
    CantAttackUnlessCondition,
    CantAttackYouUnlessControllerPaysPerAttacker,
    CantAttackYouOrPlaneswalkersUnlessControllerPaysPerAttacker,
    CantAttackYouUnlessControllerPaysPerAttackerBasicLandTypesAmongLandsYouControl,
    BlockCost,
    AttackCost,
    CantBlock,
    MayAssignDamageAsUnblocked,
    YouAssignCombatDamageOfCreaturesAttackingYou,
    ThisCreatureAssignsCombatDamageUsingToughness,
    CreaturesAssignCombatDamageUsingToughness,
    CreaturesYouControlAssignCombatDamageUsingToughness,
    LethalDamageToCreaturesYouControlUsesPower,
    Anthem,
    GrantAbility,
    RemoveAbilityForFilter,
    RemoveAllAbilitiesForFilter,
    RemoveAllAbilitiesExceptManaForFilter,
    SetBasePowerToughnessForFilter,
    SourceCharacteristicsOfLastExiledCreatureCard,
    EquipmentGrant,
    CharacteristicDefiningPT,
    AddCardTypes,
    RemoveCardTypes,
    SetCardTypes,
    AddSubtypes,
    AddAllSubtypesOfFamily,
    SetLandSubtypes,
    SetCreatureSubtypes,
    AddColors,
    CopyActivatedAbilities,
    CopyStaticAbilityVariants,
    CopyTriggeredAbilities,
    SoulbondSharedBonus,
    AttachedAbilityGrant,
    AttachedChosenLandwalkGrant,
    ControlAttachedPermanent,
    GrantObjectAbilityForFilter,
    SetColors,
    SetName,
    CountAsCardNamedForSpellEffect,
    MakeColorless,
    AddSupertypes,
    RemoveSupertypes,
    CostReduction,
    ActivatedAbilityCostReduction,
    ActivatedAbilityCostIncrease,
    ThisSpellCostReduction,
    ThisSpellCostReductionManaCost,
    CostIncrease,
    CostIncreaseLife,
    CostReductionManaCost,
    CostIncreaseManaCost,
    CostIncreasePerAdditionalTarget,
    CostIncreaseManaCostPerAdditionalTarget,
    CommanderTaxLifeSubstitution,
    Affinity,
    AffinityForArtifacts,
    Delve,
    Dredge,
    Convoke,
    Improvise,
    PlayersCantGainLife,
    PlayersCantSearch,
    DamageCantBePrevented,
    YouCantLoseGame,
    OpponentsCantWinGame,
    YourLifeTotalCantChange,
    PermanentsCantBeSacrificed,
    OpponentsCantCastSpells,
    OpponentsCantDrawExtraCards,
    CantHaveCountersPlaced,
    CounterLimit,
    CountersRemainAcrossZoneChanges,
    CantBeCountered,
    PlayersCantCycle,
    PlayersSkipUpkeep,
    PlayerSkipsDrawStep,
    PlayersSkipUntapStep,
    PlayersSkipExtraTurns,
    DamageNotRemovedDuringCleanup,
    BlackManaMayBePaidWithLife,
    DieRollResultAdjustment,
    MinimumSpellTotalMana,
    CantPayLifeOrSacrificeNonlandForCastOrActivate,
    ChooseColorAsEnters,
    ChooseColorAsBecomesAttached,
    ChoosePlayerAsEnters,
    NoteLifeTotalAsEnters,
    DiscardHandAsEnters,
    RevealFromHandAsEnters,
    ChooseCardNameAsEnters,
    ChooseBasicLandTypeAsEnters,
    ChooseLandTypeAsEnters,
    ChooseNamedOptionAsEnters,
    ChoosePowerToughnessAsEntersOrTurnsFaceUp,
    BoastTwiceEachTurn,
    FirstEquipCostAlternative,
    EquipAbilitiesAnyTime,
    LoyaltyAbilitiesAnyTime,
    ExhaustAbilitiesAsThoughUnactivatedThisTurn,
    VoteAdditionalTimeWhileVoting,
    VoteAdditionalVoteWhileVoting,
    Enchant,
    EnchantedLandIsChosenType,
    SourceLandIsChosenType,
    AddChosenCreatureType,
    AddChosenBasicLandType,
    AddChosenColor,
    SetChosenColor,
    RedirectDamageToSource,
    RedirectDamageToSourceController,
    PreventAllDamageDealtToAndByThisPermanent,
    PreventAllDamageDealtByThisPermanent,
    PreventAllCombatDamageDealtByThisPermanent,
    PreventAllDamageDealtToCreatures,
    PreventAllDamageToSelf,
    PreventAllCombatDamageToSelf,
    PreventAllCombatDamageToPermanentsMatching,
    PreventAllCombatDamageToAndByPermanentsMatching,
    PreventAllNoncombatDamageToPermanentsMatching,
    PreventAllDamageToPermanentsMatching,
    PreventAllDamageToSelfFromSourcesMatching,
    PreventAllDamageToSelfByCreatures,
    PreventDamageToYouFromSourceFilter,
    PreventDamageToSelfRemoveCounter,
    PreventDamageToSelfPutCountersInstead,
    PreventConstrainedDamageToSelfPutCountersInstead,
    DamagePreventionWithFollowUp,
    ReplaceDamageWithCountersInstead,
    PreventDamageToOtherCreatureYouControlPutCountersInstead,
    PreventAllNoncombatDamageToOtherCreaturesYouControl,
    DoesntUntap,
    UntapDuringEachOtherPlayersUntapStep,
    UntapStepLimit,
    SearchLimitedToTopCards,
    PreventAllDamageToYou,
    PlayerProtectionFrom,
    OpponentsMustTargetFlagbearers,
    MayChooseNotToUntapDuringUntapStep,
    ChooseCreatureTypeAsEnters,
    EntersTapped,
    EntersPrepared,
    EntersTappedUnlessControlTwoOrMoreOtherLands,
    EntersTappedUnlessControlTwoOrFewerOtherLands,
    EntersTappedUnlessControlTwoOrMoreBasicLands,
    EntersTappedUnlessAPlayerHas13OrLessLife,
    EntersTappedUnlessTwoOrMoreOpponents,
    EntersTappedUnlessCondition,
    EnterWithCounters,
    EnterWithCountersIfCondition,
    ShuffleIntoLibraryFromGraveyard,
    AllPermanentsEnterTapped,
    EnterTappedForFilter,
    EnterUntappedForFilter,
    EnterAsCopyAsEnters,
    AsEntersEffectProgram,
    EnterWithCountersForFilter,
    EnterWithCharacteristicsForFilter,
    CanBeCommander,
    LevelAbilities,
    NoMaximumHandSize,
    SetMaximumHandSize,
    ReduceMaximumHandSize,
    IncreaseMaximumHandSize,
    MaximumHandSizeSevenMinusYourGraveyardCardTypes,
    RevealFirstCardYouDrawEachTurn,
    LookAtTopCardOfLibrary,
    LookAtFaceDownCreaturesYouDontControl,
    AllPlayersLookAtTopCardsOfLibraries,
    AllPlayersLookAtYourTopLibraryCard,
    OpponentsPlayWithHandsRevealed,
    ControlOpponentsWhileSearchingLibraries,
    OpponentSearchExileFoundCards,
    CastThisCardFromLibraryWhileSearching,
    EffectDiscardToLibraryReplacement,
    OpponentEffectDiscardThisToBattlefieldReplacement,
    DrawReplacementExileTopFaceDown,
    DrawReplacementExileTopAndPlay,
    DrawReplacementRevealTopMatchingToHandRestBottom,
    DrawReplacementDouble,
    DrawReplacementSkipEmptyLibrary,
    ConditionalDrawReplacement,
    DrawExtraCardsReplacement,
    LoseGameReplacement,
    KeywordActionReplacement,
    ExileToCounteredExileInsteadOfGraveyard,
    ExileToExileInsteadOfGraveyard,
    ExileWouldDieInstead,
    RedirectZoneChange,
    ModifyDamageAmountReplacement,
    PreventHalfDamageReplacement,
    DoubleCountersReplacement,
    AddCountersPlacementReplacement,
    PlayerCounterPerTurnLimitReplacement,
    DoubleTokenCreationReplacement,
    MultiplyTokenCreationReplacement,
    RedirectDrawReplacement,
    DrawReplacementWithEffects,
    CreateOneOfEachTokenReplacement,
    AddTokenCreationReplacement,
    CreaturesEnteringDontCauseAbilitiesToTrigger,
    DuplicateMatchingTriggeredAbilities,
    DungeonRoomTriggerDuplication,
    /// "This effect doesn't remove this Aura." on an Aura that grants
    /// protection (CR 702.16n): this Aura isn't put into its owner's
    /// graveyard for being attached to a permanent with that protection.
    ProtectionDoesntRemoveThisAura,
    /// "This effect doesn't remove Auras." (CR 702.16n): no Aura is removed
    /// from the enchanted permanent by this Aura's protection grant.
    ProtectionDoesntRemoveAuras,
    SuppressMatchingTriggeredAbilities,
    DoubleDamageFromSourcesYouControlOfChosenType,
    StartingLifeBonus,
    BuybackCostReduction,
    LegendRuleDoesntApply,
    LegendRuleDoesntApplyToController,
    LegendRuleDoesntApplyToControllerTokens,
    ManaSpendPermission,
    SpendManaAsAnyColor,
    SpendManaAsAnyColorActivationCosts,
    RuleRestriction,
    TargetingAsThoughNoAbility,
    DiscardOrRedirectReplacement,
    SacrificeOrRedirectReplacement,
    PayLifeOrEnterTappedReplacement,
    RevealCardOrEnterTappedReplacement,
    RedirectWouldEnterReplacement,
    ManaProductionReplacement,
    ManaProductionMultiplierReplacement,
    DoubleLifeChangeReplacement,
    PregameAction,
    DeckConstructionRuleText,
    DungeonEntryRestriction,
    DraftRuleText,
    HiddenAgenda,
    DoubleAgenda,
    KeywordText,
    KeywordMarker,
    /// "If this enchantment leaves the battlefield, this effect continues
    /// until end of turn" (Titania's Song): when the permanent leaves the
    /// battlefield, the continuous effects of its other static abilities keep
    /// applying until end of turn.
    StaticEffectsContinueUntilEndOfTurnAfterLeaving,
    SourceLineKeywordGroup,
    SourceLineStaticGroup,
    KeywordFallbackText,
    RuleFallbackText,
    UnsupportedParserLine,
    Grants,
    NativeAlternativeCastFromZone,
    EntersUnderChosenControl,
    /// Toxic N (CR 702.164). The amount is carried in the ability's label.
    Toxic,
    /// Trample over planeswalkers (CR 702.19c). A variant of trample, not an
    /// instance of it: "has trample" checks don't match it.
    TrampleOverPlaneswalkers,
    /// A blocking-only permission that preserves the attacker's landwalk abilities.
    BlockingAsThoughNoLandwalk,
    /// Unbounded blocker capacity; appended to preserve serialized variant ordinals.
    CanBlockAnyNumber,
    /// Additional capacity scaled by a live typed permanent filter.
    CanBlockAdditionalForEach,
    /// Filtered, amount-based damage prevention. Appended for wire compatibility.
    PreventMatchingDamage,
    /// A controller-scoped exception to CR 704.5i.
    PlaneswalkersYouControlDontDieAtZeroLoyalty,
    /// Add to one life-gain event; appended for serialized ID compatibility.
    AddLifeGainReplacement,
    TokenCreationTemplates,
    ControllerPlaysWithHandRevealed,
    PlayersPlayWithHandsRevealed,
    /// A spell-only copy prohibition; appended for wire compatibility.
    CantBeCopied,
    RedirectMatchingDamage,
    SpellManaSpendingRestriction,
    /// Typed mana output rewrite; appended to preserve existing ordinals.
    ManaProductionRewrite,
    /// Extra numerical dice with ignored low rolls, appended for wire stability.
    ExtraDieIgnoreLowest,
    /// Conversion of mana that would be lost, preserving the existing units.
    ConvertUnspentMana,
    /// Appended to preserve existing published enum discriminants.
    ForetellSpecialActionModifier,
    ExtraCoinIgnoreOne,
    FirstCoinBatchHeadsWin,
    /// Standalone paired private inspection; appended for wire compatibility.
    LookAtSourceExiledCards,
    /// Generic "<event> instead" replacement; appended for wire compatibility.
    EventReplacementWithEffects,
    /// "You may pay <mana> rather than pay the echo cost for permanents you
    /// control"; appended for wire compatibility.
    EchoCostAlternative,
}

impl StaticAbilityId {
    fn exhaustive_classification_guard(id: StaticAbilityId) {
        use StaticAbilityId::*;
        match id {
            Flying
            | FirstStrike
            | DoubleStrike
            | Deathtouch
            | Defender
            | Flash
            | Haste
            | Hexproof
            | HexproofFrom
            | Indestructible
            | Intimidate
            | Lifelink
            | Menace
            | Banding
            | BandsWithOther
            | Protection
            | Reach
            | Shroud
            | Trample
            | Vigilance
            | Ward
            | Fear
            | Skulk
            | Prowess
            | Flanking
            | UmbraArmor
            | Landwalk
            | CantBeBlockedAsLongAsDefendingPlayerControlsCardType
            | CantBeBlockedAsLongAsDefendingPlayerControlsCardTypes
            | CantBeBlockedWhileDefendingPlayerControlsMostCreatures
            | Bloodthirst
            | Tribute
            | Daybound
            | Nightbound
            | DayNightStartsDayAsEnters
            | Morph
            | Disguise
            | Megamorph
            | Shadow
            | Horsemanship
            | Phasing
            | Wither
            | Infect
            | Changeling
            | LivingMetal
            | Companion
            | Partner
            | PartnerWith
            | StartYourEngines
            | SpaceSculptor
            | DoctorsCompanion
            | Assist
            | Ascend
            | Storied
            | SplitSecond
            | Rebound
            | Cascade
            | CascadeLandDrop
            | Splice
            | Escalate
            | ReadAhead
            | Unleash
            | ConditionalSpellKeyword
            | ThisSpellCastRestriction
            | ThisSpellXMaximum
            | ThisSpellXMinimum
            | Unblockable
            | FlyingRestriction
            | FlyingOnlyRestriction
            | CanBlockFlying
            | CanBlockAsThoughNoShadow
            | CanBlockOnlyFlying
            | CanBlockAdditionalCreatureEachCombat
            | CanBlockAnyNumber
            | CanBlockAdditionalForEach
            | MaxCreaturesCanAttackEachCombat
            | MaxCreaturesCanAttackYouEachCombat
            | MaxCreaturesCanBlockEachCombat
            | CantBeBlockedByPowerOrLess
            | CantBeBlockedByPowerOrGreater
            | CantBeBlockedByLowerPowerThanSource
            | CantBeBlockedByMoreThan
            | CantBeBlockedExceptByNOrMore
            | CanAttackAsThoughNoDefender
            | CanAttackAsThoughHaste
            | ActivateAbilitiesAsThoughHaste
            | MustAttack
            | GoadedBySourceController
            | MustAttackAttachedController
            | AllCreaturesAttackAttachedControllerEachCombatIfAble
            | AttachedGoadedBySourceController
            | GoadMatching
            | AttachedControllerMaySacrificePermanentToIgnoreSourceEffectUntilEndOfTurn
            | AnyPlayerMayPayManaToIgnoreSourceEffectUntilEndOfTurn
            | ExertAttack
            | EnlistAttack
            | MustBlock
            | CantAttack
            | CantAttackItsOwner
            | CantAttackUnlessControllerCastCreatureSpellThisTurn
            | CantAttackUnlessControllerCastNonCreatureSpellThisTurn
            | CantAttackUnlessCondition
            | CantAttackYouUnlessControllerPaysPerAttacker
            | CantAttackYouOrPlaneswalkersUnlessControllerPaysPerAttacker
            | CantAttackYouUnlessControllerPaysPerAttackerBasicLandTypesAmongLandsYouControl
            | BlockCost
            | AttackCost
            | CantBlock
            | MayAssignDamageAsUnblocked
            | YouAssignCombatDamageOfCreaturesAttackingYou
            | ThisCreatureAssignsCombatDamageUsingToughness
            | CreaturesAssignCombatDamageUsingToughness
            | CreaturesYouControlAssignCombatDamageUsingToughness
            | LethalDamageToCreaturesYouControlUsesPower
            | Anthem
            | GrantAbility
            | RemoveAbilityForFilter
            | RemoveAllAbilitiesForFilter
            | RemoveAllAbilitiesExceptManaForFilter
            | SetBasePowerToughnessForFilter
            | SourceCharacteristicsOfLastExiledCreatureCard
            | EquipmentGrant
            | CharacteristicDefiningPT
            | AddCardTypes
            | RemoveCardTypes
            | SetCardTypes
            | AddSubtypes
            | AddAllSubtypesOfFamily
            | SetLandSubtypes
            | SetCreatureSubtypes
            | AddColors
            | CopyActivatedAbilities
            | CopyStaticAbilityVariants
            | CopyTriggeredAbilities
            | SoulbondSharedBonus
            | AttachedAbilityGrant
            | AttachedChosenLandwalkGrant
            | ControlAttachedPermanent
            | GrantObjectAbilityForFilter
            | SetColors
            | SetName
            | CountAsCardNamedForSpellEffect
            | MakeColorless
            | AddSupertypes
            | RemoveSupertypes
            | CostReduction
            | ActivatedAbilityCostReduction
            | ActivatedAbilityCostIncrease
            | ThisSpellCostReduction
            | ThisSpellCostReductionManaCost
            | CostIncrease
            | CostIncreaseLife
            | CostReductionManaCost
            | CostIncreaseManaCost
            | CostIncreasePerAdditionalTarget
            | CostIncreaseManaCostPerAdditionalTarget
            | CommanderTaxLifeSubstitution
            | Affinity
            | AffinityForArtifacts
            | Delve
            | Dredge
            | Convoke
            | Improvise
            | PlayersCantGainLife
            | PlayersCantSearch
            | DamageCantBePrevented
            | YouCantLoseGame
            | OpponentsCantWinGame
            | YourLifeTotalCantChange
            | PermanentsCantBeSacrificed
            | OpponentsCantCastSpells
            | OpponentsCantDrawExtraCards
            | CantHaveCountersPlaced
            | CounterLimit
            | CountersRemainAcrossZoneChanges
            | CantBeCountered
            | CantBeCopied
            | PlayersCantCycle
            | PlayersSkipUpkeep
            | PlayerSkipsDrawStep
            | PlayersSkipUntapStep
            | PlayersSkipExtraTurns
            | DamageNotRemovedDuringCleanup
            | BlackManaMayBePaidWithLife
            | DieRollResultAdjustment
            | MinimumSpellTotalMana
            | CantPayLifeOrSacrificeNonlandForCastOrActivate
            | ChooseColorAsEnters
            | ChooseColorAsBecomesAttached
            | ChoosePlayerAsEnters
            | NoteLifeTotalAsEnters
            | DiscardHandAsEnters
            | RevealFromHandAsEnters
            | ChooseCardNameAsEnters
            | ChooseBasicLandTypeAsEnters
            | ChooseLandTypeAsEnters
            | ChooseNamedOptionAsEnters
            | ChoosePowerToughnessAsEntersOrTurnsFaceUp
            | BoastTwiceEachTurn
            | FirstEquipCostAlternative
            | EquipAbilitiesAnyTime
            | LoyaltyAbilitiesAnyTime
            | PlaneswalkersYouControlDontDieAtZeroLoyalty
            | ExhaustAbilitiesAsThoughUnactivatedThisTurn
            | VoteAdditionalTimeWhileVoting
            | VoteAdditionalVoteWhileVoting
            | Enchant
            | EnchantedLandIsChosenType
            | SourceLandIsChosenType
            | AddChosenCreatureType
            | AddChosenBasicLandType
            | AddChosenColor
            | SetChosenColor
            | RedirectDamageToSource
            | RedirectDamageToSourceController
            | PreventAllDamageDealtToAndByThisPermanent
            | PreventAllDamageDealtByThisPermanent
            | PreventAllCombatDamageDealtByThisPermanent
            | PreventAllDamageDealtToCreatures
            | PreventAllDamageToSelf
            | PreventAllCombatDamageToSelf
            | PreventAllCombatDamageToPermanentsMatching
            | PreventAllCombatDamageToAndByPermanentsMatching
            | PreventAllNoncombatDamageToPermanentsMatching
            | PreventAllDamageToPermanentsMatching
            | PreventAllDamageToSelfFromSourcesMatching
            | PreventAllDamageToSelfByCreatures
            | PreventDamageToYouFromSourceFilter
            | PreventDamageToSelfRemoveCounter
            | PreventDamageToSelfPutCountersInstead
            | PreventConstrainedDamageToSelfPutCountersInstead
            | DamagePreventionWithFollowUp
            | ReplaceDamageWithCountersInstead
            | PreventDamageToOtherCreatureYouControlPutCountersInstead
            | PreventAllNoncombatDamageToOtherCreaturesYouControl
            | DoesntUntap
            | UntapDuringEachOtherPlayersUntapStep
            | UntapStepLimit
            | SearchLimitedToTopCards
            | PreventAllDamageToYou
            | PlayerProtectionFrom
            | OpponentsMustTargetFlagbearers
            | MayChooseNotToUntapDuringUntapStep
            | ChooseCreatureTypeAsEnters
            | EntersTapped
            | EntersPrepared
            | EntersTappedUnlessControlTwoOrMoreOtherLands
            | EntersTappedUnlessControlTwoOrFewerOtherLands
            | EntersTappedUnlessControlTwoOrMoreBasicLands
            | EntersTappedUnlessAPlayerHas13OrLessLife
            | EntersTappedUnlessTwoOrMoreOpponents
            | EntersTappedUnlessCondition
            | EnterWithCounters
            | EnterWithCountersIfCondition
            | ShuffleIntoLibraryFromGraveyard
            | AllPermanentsEnterTapped
            | EnterTappedForFilter
            | EnterUntappedForFilter
            | EnterAsCopyAsEnters
            | AsEntersEffectProgram
            | EnterWithCountersForFilter
            | EnterWithCharacteristicsForFilter
            | CanBeCommander
            | LevelAbilities
            | NoMaximumHandSize
            | SetMaximumHandSize
            | ReduceMaximumHandSize
            | IncreaseMaximumHandSize
            | MaximumHandSizeSevenMinusYourGraveyardCardTypes
            | RevealFirstCardYouDrawEachTurn
            | LookAtTopCardOfLibrary
            | LookAtFaceDownCreaturesYouDontControl
            | AllPlayersLookAtTopCardsOfLibraries
            | AllPlayersLookAtYourTopLibraryCard
            | OpponentsPlayWithHandsRevealed
            | ControllerPlaysWithHandRevealed
            | PlayersPlayWithHandsRevealed
            | ControlOpponentsWhileSearchingLibraries
            | OpponentSearchExileFoundCards
            | CastThisCardFromLibraryWhileSearching
            | EffectDiscardToLibraryReplacement
            | OpponentEffectDiscardThisToBattlefieldReplacement
            | DrawReplacementExileTopFaceDown
            | DrawReplacementExileTopAndPlay
            | DrawReplacementRevealTopMatchingToHandRestBottom
            | DrawReplacementDouble
            | DrawReplacementSkipEmptyLibrary
            | ConditionalDrawReplacement
            | DrawExtraCardsReplacement
            | LoseGameReplacement
            | KeywordActionReplacement
            | ExileToCounteredExileInsteadOfGraveyard
            | ExileToExileInsteadOfGraveyard
            | ExileWouldDieInstead
            | RedirectZoneChange
            | ModifyDamageAmountReplacement
            | PreventHalfDamageReplacement
            | PreventMatchingDamage
            | SpellManaSpendingRestriction
            | ExtraCoinIgnoreOne
            | FirstCoinBatchHeadsWin
            | LookAtSourceExiledCards
            | ExtraDieIgnoreLowest
            | RedirectMatchingDamage
            | AddLifeGainReplacement
            | TokenCreationTemplates
            | DoubleCountersReplacement
            | AddCountersPlacementReplacement
            | PlayerCounterPerTurnLimitReplacement
            | DoubleTokenCreationReplacement
            | MultiplyTokenCreationReplacement
            | RedirectDrawReplacement
            | DrawReplacementWithEffects
            | EventReplacementWithEffects
            | EchoCostAlternative
            | CreateOneOfEachTokenReplacement
            | AddTokenCreationReplacement
            | CreaturesEnteringDontCauseAbilitiesToTrigger
            | DuplicateMatchingTriggeredAbilities
            | DungeonRoomTriggerDuplication
            | ProtectionDoesntRemoveThisAura
            | ProtectionDoesntRemoveAuras
            | SuppressMatchingTriggeredAbilities
            | DoubleDamageFromSourcesYouControlOfChosenType
            | StartingLifeBonus
            | BuybackCostReduction
            | LegendRuleDoesntApply
            | LegendRuleDoesntApplyToController
            | LegendRuleDoesntApplyToControllerTokens
            | ManaSpendPermission
            | SpendManaAsAnyColor
            | SpendManaAsAnyColorActivationCosts
            | RuleRestriction
            | TargetingAsThoughNoAbility
            | BlockingAsThoughNoLandwalk
            | DiscardOrRedirectReplacement
            | SacrificeOrRedirectReplacement
            | PayLifeOrEnterTappedReplacement
            | RevealCardOrEnterTappedReplacement
            | RedirectWouldEnterReplacement
            | ManaProductionReplacement
            | ConvertUnspentMana
            | ManaProductionRewrite
            | ManaProductionMultiplierReplacement
            | DoubleLifeChangeReplacement
            | PregameAction
            | DeckConstructionRuleText
            | DungeonEntryRestriction
            | DraftRuleText
            | HiddenAgenda
            | DoubleAgenda
            | KeywordText
            | KeywordMarker
            | StaticEffectsContinueUntilEndOfTurnAfterLeaving
            | SourceLineKeywordGroup
            | SourceLineStaticGroup
            | KeywordFallbackText
            | RuleFallbackText
            | UnsupportedParserLine
            | Grants
            | EntersUnderChosenControl
            | Toxic
            | TrampleOverPlaneswalkers
            | ForetellSpecialActionModifier
            | NativeAlternativeCastFromZone => {}
        }
    }

    pub fn is_keyword(&self) -> bool {
        Self::exhaustive_classification_guard(*self);
        use StaticAbilityId::*;
        matches!(
            self,
            Flying
                | FirstStrike
                | DoubleStrike
                | Deathtouch
                | Defender
                | Flash
                | Haste
                | Hexproof
                | HexproofFrom
                | Indestructible
                | Intimidate
                | Lifelink
                | Menace
                | Banding
                | BandsWithOther
                | Protection
                | Reach
                | Shroud
                | Trample
                | Vigilance
                | Ward
                | Fear
                | Skulk
                | Prowess
                | Flanking
                | UmbraArmor
                | Landwalk
                | Bloodthirst
                | Tribute
                | Morph
                | Disguise
                | Megamorph
                | Shadow
                | Horsemanship
                | Phasing
                | Wither
                | Infect
                | Changeling
                | LivingMetal
                | Companion
                | Partner
                | PartnerWith
                | StartYourEngines
                | SpaceSculptor
                | DoctorsCompanion
                | Assist
                | SplitSecond
                | Rebound
                | Cascade
                | Splice
                | Escalate
                | Dredge
                | EnlistAttack
                | ReadAhead
                | Unleash
                | Toxic
                | TrampleOverPlaneswalkers
                | KeywordText
                | KeywordMarker
                | KeywordFallbackText
        )
    }

    pub fn grants_evasion(&self) -> bool {
        Self::exhaustive_classification_guard(*self);
        use StaticAbilityId::*;
        matches!(
            self,
            Flying
                | Shadow
                | Horsemanship
                | Fear
                | Intimidate
                | Skulk
                | FlyingRestriction
                | FlyingOnlyRestriction
                | CantBeBlockedByPowerOrLess
                | CantBeBlockedByPowerOrGreater
                | CantBeBlockedByLowerPowerThanSource
                | CantBeBlockedByMoreThan
                | CantBeBlockedExceptByNOrMore
                | Landwalk
                | CantBeBlockedAsLongAsDefendingPlayerControlsCardType
                | CantBeBlockedAsLongAsDefendingPlayerControlsCardTypes
                | CantBeBlockedWhileDefendingPlayerControlsMostCreatures
        )
    }

    pub fn affects_combat(&self) -> bool {
        Self::exhaustive_classification_guard(*self);
        use StaticAbilityId::*;
        matches!(
            self,
            Flying
                | FirstStrike
                | DoubleStrike
                | Deathtouch
                | Defender
                | Lifelink
                | Menace
                | Banding
                | BandsWithOther
                | Reach
                | Trample
                | Vigilance
                | Fear
                | Skulk
                | Flanking
                | Landwalk
                | Shadow
                | Horsemanship
                | Unblockable
                | FlyingRestriction
                | FlyingOnlyRestriction
                | CanBlockFlying
                | CanBlockAsThoughNoShadow
                | BlockingAsThoughNoLandwalk
                | CanBlockOnlyFlying
                | CanBlockAnyNumber
                | CanBlockAdditionalForEach
                | MaxCreaturesCanAttackEachCombat
                | MaxCreaturesCanBlockEachCombat
                | CantBeBlockedByPowerOrLess
                | CantBeBlockedByPowerOrGreater
                | CantBeBlockedByLowerPowerThanSource
                | CantBeBlockedByMoreThan
                | CantBeBlockedExceptByNOrMore
                | CantBeBlockedAsLongAsDefendingPlayerControlsCardType
                | CantBeBlockedAsLongAsDefendingPlayerControlsCardTypes
                | CantBeBlockedWhileDefendingPlayerControlsMostCreatures
                | CanAttackAsThoughNoDefender
                | CanAttackAsThoughHaste
                | ActivateAbilitiesAsThoughHaste
                | MustAttack
                | MustBlock
                | CantAttack
                | CantAttackItsOwner
                | CantAttackUnlessControllerCastCreatureSpellThisTurn
                | CantAttackUnlessControllerCastNonCreatureSpellThisTurn
                | CantAttackUnlessCondition
                | CantAttackYouUnlessControllerPaysPerAttacker
                | CantAttackYouOrPlaneswalkersUnlessControllerPaysPerAttacker
                | CantAttackYouUnlessControllerPaysPerAttackerBasicLandTypesAmongLandsYouControl
                | BlockCost
                | AttackCost
                | CantBlock
                | MayAssignDamageAsUnblocked
                | YouAssignCombatDamageOfCreaturesAttackingYou
                | ThisCreatureAssignsCombatDamageUsingToughness
                | CreaturesAssignCombatDamageUsingToughness
                | CreaturesYouControlAssignCombatDamageUsingToughness
                | LethalDamageToCreaturesYouControlUsesPower
                | Toxic
                | TrampleOverPlaneswalkers
        )
    }

    pub fn generates_continuous_effects(&self) -> bool {
        Self::exhaustive_classification_guard(*self);
        use StaticAbilityId::*;
        matches!(
            self,
            Anthem
                | GrantAbility
                | AttachedAbilityGrant
                | AttachedChosenLandwalkGrant
                | RemoveAllAbilitiesForFilter
                | RemoveAllAbilitiesExceptManaForFilter
                | SetBasePowerToughnessForFilter
                | EquipmentGrant
                | GrantObjectAbilityForFilter
                | ControlAttachedPermanent
                | CharacteristicDefiningPT
                | LivingMetal
                | AddCardTypes
                | RemoveCardTypes
                | SetCardTypes
                | AddSubtypes
                | SetLandSubtypes
                | AddColors
                | AddChosenColor
                | SetColors
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyword_identification_is_stable() {
        assert!(StaticAbilityId::Flying.is_keyword());
        assert!(StaticAbilityId::Trample.is_keyword());
        assert!(StaticAbilityId::Protection.is_keyword());
        assert!(!StaticAbilityId::Anthem.is_keyword());
        assert!(!StaticAbilityId::SetLandSubtypes.is_keyword());
    }

    #[test]
    fn evasion_identification_is_stable() {
        assert!(StaticAbilityId::Flying.grants_evasion());
        assert!(StaticAbilityId::Shadow.grants_evasion());
        assert!(!StaticAbilityId::Trample.grants_evasion());
        assert!(!StaticAbilityId::Lifelink.grants_evasion());
    }

    #[test]
    fn continuous_effect_identification_is_stable() {
        assert!(StaticAbilityId::Anthem.generates_continuous_effects());
        assert!(StaticAbilityId::SetLandSubtypes.generates_continuous_effects());
        assert!(!StaticAbilityId::Flying.generates_continuous_effects());
        assert!(!StaticAbilityId::Hexproof.generates_continuous_effects());
    }
}
