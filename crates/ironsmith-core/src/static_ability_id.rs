//! Static ability identity enum.
//!
//! This enum provides unique identifiers for each type of static ability.

/// Unique identifier for each type of static ability.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    MustAttack,
    GoadedBySourceController,
    MustAttackAttachedController,
    AllCreaturesAttackAttachedControllerEachCombatIfAble,
    AttachedGoadedBySourceController,
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
    ExhaustAbilitiesAsThoughUnactivatedThisTurn,
    VoteAdditionalTimeWhileVoting,
    VoteAdditionalVoteWhileVoting,
    EnchantedLandIsChosenType,
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
    PreventAllNoncombatDamageToPermanentsMatching,
    PreventAllDamageToSelfFromSourcesMatching,
    PreventAllDamageToSelfByCreatures,
    PreventDamageToYouFromSourceFilter,
    PreventDamageToSelfRemoveCounter,
    PreventDamageToSelfPutCountersInstead,
    PreventConstrainedDamageToSelfPutCountersInstead,
    ReplaceDamageWithCountersInstead,
    PreventDamageToOtherCreatureYouControlPutCountersInstead,
    PreventAllNoncombatDamageToOtherCreaturesYouControl,
    DoesntUntap,
    UntapDuringEachOtherPlayersUntapStep,
    MayChooseNotToUntapDuringUntapStep,
    ChooseCreatureTypeAsEnters,
    EntersTapped,
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
    LoseGameReplacement,
    KeywordActionReplacement,
    ExileToCounteredExileInsteadOfGraveyard,
    ExileToExileInsteadOfGraveyard,
    ExileWouldDieInstead,
    ModifyDamageAmountReplacement,
    DoubleCountersReplacement,
    AddCountersPlacementReplacement,
    PlayerCounterPerTurnLimitReplacement,
    DoubleTokenCreationReplacement,
    AddTokenCreationReplacement,
    CreaturesEnteringDontCauseAbilitiesToTrigger,
    DuplicateMatchingTriggeredAbilities,
    DungeonRoomTriggerDuplication,
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
    PregameAction,
    DeckConstructionRuleText,
    DraftRuleText,
    HiddenAgenda,
    DoubleAgenda,
    KeywordText,
    KeywordMarker,
    SourceLineKeywordGroup,
    SourceLineStaticGroup,
    KeywordFallbackText,
    RuleFallbackText,
    UnsupportedParserLine,
    Grants,
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
            | MustAttack
            | GoadedBySourceController
            | MustAttackAttachedController
            | AllCreaturesAttackAttachedControllerEachCombatIfAble
            | AttachedGoadedBySourceController
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
            | PlayersCantCycle
            | PlayersSkipUpkeep
            | PlayerSkipsDrawStep
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
            | ExhaustAbilitiesAsThoughUnactivatedThisTurn
            | VoteAdditionalTimeWhileVoting
            | VoteAdditionalVoteWhileVoting
            | EnchantedLandIsChosenType
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
            | PreventAllNoncombatDamageToPermanentsMatching
            | PreventAllDamageToSelfFromSourcesMatching
            | PreventAllDamageToSelfByCreatures
            | PreventDamageToYouFromSourceFilter
            | PreventDamageToSelfRemoveCounter
            | PreventDamageToSelfPutCountersInstead
            | PreventConstrainedDamageToSelfPutCountersInstead
            | ReplaceDamageWithCountersInstead
            | PreventDamageToOtherCreatureYouControlPutCountersInstead
            | PreventAllNoncombatDamageToOtherCreaturesYouControl
            | DoesntUntap
            | UntapDuringEachOtherPlayersUntapStep
            | MayChooseNotToUntapDuringUntapStep
            | ChooseCreatureTypeAsEnters
            | EntersTapped
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
            | LoseGameReplacement
            | KeywordActionReplacement
            | ExileToCounteredExileInsteadOfGraveyard
            | ExileToExileInsteadOfGraveyard
            | ExileWouldDieInstead
            | ModifyDamageAmountReplacement
            | DoubleCountersReplacement
            | AddCountersPlacementReplacement
            | PlayerCounterPerTurnLimitReplacement
            | DoubleTokenCreationReplacement
            | AddTokenCreationReplacement
            | CreaturesEnteringDontCauseAbilitiesToTrigger
            | DuplicateMatchingTriggeredAbilities
            | DungeonRoomTriggerDuplication
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
            | DiscardOrRedirectReplacement
            | SacrificeOrRedirectReplacement
            | PayLifeOrEnterTappedReplacement
            | PregameAction
            | DeckConstructionRuleText
            | DraftRuleText
            | HiddenAgenda
            | DoubleAgenda
            | KeywordText
            | KeywordMarker
            | SourceLineKeywordGroup
            | SourceLineStaticGroup
            | KeywordFallbackText
            | RuleFallbackText
            | UnsupportedParserLine
            | Grants => {}
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
                | CanBlockOnlyFlying
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
                | CantBlock
                | MayAssignDamageAsUnblocked
                | YouAssignCombatDamageOfCreaturesAttackingYou
                | ThisCreatureAssignsCombatDamageUsingToughness
                | CreaturesAssignCombatDamageUsingToughness
                | CreaturesYouControlAssignCombatDamageUsingToughness
                | LethalDamageToCreaturesYouControlUsesPower
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
