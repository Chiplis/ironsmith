//! Static ability identity enum.
//!
//! This enum provides unique identifiers for each type of static ability.

/// Unique identifier for each type of static ability.
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
    Protection,
    Reach,
    Shroud,
    Trample,
    Vigilance,
    Ward,
    Fear,
    Skulk,
    Flanking,
    UmbraArmor,
    Landwalk,
    CantBeBlockedAsLongAsDefendingPlayerControlsCardType,
    CantBeBlockedAsLongAsDefendingPlayerControlsCardTypes,
    Bloodthirst,
    Morph,
    Megamorph,
    Shadow,
    Horsemanship,
    Phasing,
    Wither,
    Infect,
    Changeling,
    Partner,
    DoctorsCompanion,
    Assist,
    SplitSecond,
    Rebound,
    Cascade,
    Unleash,
    ConditionalSpellKeyword,
    ThisSpellCastRestriction,
    Unblockable,
    FlyingRestriction,
    FlyingOnlyRestriction,
    CanBlockFlying,
    CanBlockOnlyFlying,
    CanBlockAdditionalCreatureEachCombat,
    MaxCreaturesCanAttackEachCombat,
    MaxCreaturesCanBlockEachCombat,
    CantBeBlockedByPowerOrLess,
    CantBeBlockedByPowerOrGreater,
    CantBeBlockedByLowerPowerThanSource,
    CantBeBlockedByMoreThan,
    CanAttackAsThoughNoDefender,
    MustAttack,
    ExertAttack,
    MustBlock,
    CantAttack,
    CantAttackItsOwner,
    CantAttackUnlessControllerCastCreatureSpellThisTurn,
    CantAttackUnlessControllerCastNonCreatureSpellThisTurn,
    CantAttackUnlessCondition,
    CantAttackYouUnlessControllerPaysPerAttacker,
    CantAttackYouUnlessControllerPaysPerAttackerBasicLandTypesAmongLandsYouControl,
    CantBlock,
    MayAssignDamageAsUnblocked,
    CreaturesAssignCombatDamageUsingToughness,
    CreaturesYouControlAssignCombatDamageUsingToughness,
    Anthem,
    GrantAbility,
    RemoveAbilityForFilter,
    RemoveAllAbilitiesForFilter,
    RemoveAllAbilitiesExceptManaForFilter,
    SetBasePowerToughnessForFilter,
    EquipmentGrant,
    BloodMoon,
    Humility,
    BelloBardOfTheBrambles,
    CharacteristicDefiningPT,
    AddCardTypes,
    RemoveCardTypes,
    SetCardTypes,
    AddSubtypes,
    AddAllSubtypesOfFamily,
    SetCreatureSubtypes,
    AddColors,
    CopyActivatedAbilities,
    ManascapeRefractor,
    SquirrelNest,
    MycosynthLattice,
    TophFirstMetalbender,
    MarvinMurderousMimic,
    SoulbondSharedBonus,
    AttachedAbilityGrant,
    AttachedChosenLandwalkGrant,
    ControlAttachedPermanent,
    GrantObjectAbilityForFilter,
    SetColors,
    SetName,
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
    AffinityForArtifacts,
    Delve,
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
    CantBeCountered,
    PlayersCantCycle,
    PlayersSkipUpkeep,
    DamageNotRemovedDuringCleanup,
    BlackManaMayBePaidWithLife,
    MinimumSpellTotalMana,
    CantPayLifeOrSacrificeNonlandForCastOrActivate,
    ChooseColorAsEnters,
    ChoosePlayerAsEnters,
    ChooseBasicLandTypeAsEnters,
    ChooseLandTypeAsEnters,
    ChooseNamedOptionAsEnters,
    BoastTwiceEachTurn,
    FirstEquipCostAlternative,
    VoteAdditionalTimeWhileVoting,
    VoteAdditionalVoteWhileVoting,
    EnchantedLandIsChosenType,
    AddChosenCreatureType,
    SetChosenColor,
    RedirectDamageToSource,
    PreventAllDamageDealtByThisPermanent,
    PreventAllDamageDealtToCreatures,
    PreventAllCombatDamageToSelf,
    PreventAllDamageToSelfByCreatures,
    PreventDamageToSelfRemoveCounter,
    PreventDamageToSelfPutCountersInstead,
    PreventConstrainedDamageToSelfPutCountersInstead,
    PreventDamageToOtherCreatureYouControlPutCountersInstead,
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
    EnterWithCountersForFilter,
    CanBeCommander,
    LevelAbilities,
    NoMaximumHandSize,
    ReduceMaximumHandSize,
    MaximumHandSizeSevenMinusYourGraveyardCardTypes,
    RevealFirstCardYouDrawEachTurn,
    LibraryOfLengDiscardReplacement,
    DrawReplacementExileTopFaceDown,
    ExileToCounteredExileInsteadOfGraveyard,
    CreaturesEnteringDontCauseAbilitiesToTrigger,
    DuplicateMatchingTriggeredAbilities,
    SuppressMatchingTriggeredAbilities,
    DoubleDamageFromSourcesYouControlOfChosenType,
    StartingLifeBonus,
    BuybackCostReduction,
    LegendRuleDoesntApply,
    ManaSpendPermission,
    SpendManaAsAnyColor,
    SpendManaAsAnyColorActivationCosts,
    RuleRestriction,
    DiscardOrRedirectReplacement,
    PayLifeOrEnterTappedReplacement,
    PregameAction,
    KeywordText,
    KeywordMarker,
    RuleTextPlaceholder,
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
            | Protection
            | Reach
            | Shroud
            | Trample
            | Vigilance
            | Ward
            | Fear
            | Skulk
            | Flanking
            | UmbraArmor
            | Landwalk
            | CantBeBlockedAsLongAsDefendingPlayerControlsCardType
            | CantBeBlockedAsLongAsDefendingPlayerControlsCardTypes
            | Bloodthirst
            | Morph
            | Megamorph
            | Shadow
            | Horsemanship
            | Phasing
            | Wither
            | Infect
            | Changeling
            | Partner
            | DoctorsCompanion
            | Assist
            | SplitSecond
            | Rebound
            | Cascade
            | Unleash
            | ConditionalSpellKeyword
            | ThisSpellCastRestriction
            | Unblockable
            | FlyingRestriction
            | FlyingOnlyRestriction
            | CanBlockFlying
            | CanBlockOnlyFlying
            | CanBlockAdditionalCreatureEachCombat
            | MaxCreaturesCanAttackEachCombat
            | MaxCreaturesCanBlockEachCombat
            | CantBeBlockedByPowerOrLess
            | CantBeBlockedByPowerOrGreater
            | CantBeBlockedByLowerPowerThanSource
            | CantBeBlockedByMoreThan
            | CanAttackAsThoughNoDefender
            | MustAttack
            | ExertAttack
            | MustBlock
            | CantAttack
            | CantAttackItsOwner
            | CantAttackUnlessControllerCastCreatureSpellThisTurn
            | CantAttackUnlessControllerCastNonCreatureSpellThisTurn
            | CantAttackUnlessCondition
            | CantAttackYouUnlessControllerPaysPerAttacker
            | CantAttackYouUnlessControllerPaysPerAttackerBasicLandTypesAmongLandsYouControl
            | CantBlock
            | MayAssignDamageAsUnblocked
            | CreaturesAssignCombatDamageUsingToughness
            | CreaturesYouControlAssignCombatDamageUsingToughness
            | Anthem
            | GrantAbility
            | RemoveAbilityForFilter
            | RemoveAllAbilitiesForFilter
            | RemoveAllAbilitiesExceptManaForFilter
            | SetBasePowerToughnessForFilter
            | EquipmentGrant
            | BloodMoon
            | Humility
            | BelloBardOfTheBrambles
            | CharacteristicDefiningPT
            | AddCardTypes
            | RemoveCardTypes
            | SetCardTypes
            | AddSubtypes
            | AddAllSubtypesOfFamily
            | SetCreatureSubtypes
            | AddColors
            | CopyActivatedAbilities
            | ManascapeRefractor
            | SquirrelNest
            | MycosynthLattice
            | TophFirstMetalbender
            | MarvinMurderousMimic
            | SoulbondSharedBonus
            | AttachedAbilityGrant
            | AttachedChosenLandwalkGrant
            | ControlAttachedPermanent
            | GrantObjectAbilityForFilter
            | SetColors
            | SetName
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
            | AffinityForArtifacts
            | Delve
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
            | CantBeCountered
            | PlayersCantCycle
            | PlayersSkipUpkeep
            | DamageNotRemovedDuringCleanup
            | BlackManaMayBePaidWithLife
            | MinimumSpellTotalMana
            | CantPayLifeOrSacrificeNonlandForCastOrActivate
            | ChooseColorAsEnters
            | ChoosePlayerAsEnters
            | ChooseBasicLandTypeAsEnters
            | ChooseLandTypeAsEnters
            | ChooseNamedOptionAsEnters
            | BoastTwiceEachTurn
            | FirstEquipCostAlternative
            | VoteAdditionalTimeWhileVoting
            | VoteAdditionalVoteWhileVoting
            | EnchantedLandIsChosenType
            | AddChosenCreatureType
            | SetChosenColor
            | RedirectDamageToSource
            | PreventAllDamageDealtByThisPermanent
            | PreventAllDamageDealtToCreatures
            | PreventAllCombatDamageToSelf
            | PreventAllDamageToSelfByCreatures
            | PreventDamageToSelfRemoveCounter
            | PreventDamageToSelfPutCountersInstead
            | PreventConstrainedDamageToSelfPutCountersInstead
            | PreventDamageToOtherCreatureYouControlPutCountersInstead
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
            | EnterWithCountersForFilter
            | CanBeCommander
            | LevelAbilities
            | NoMaximumHandSize
            | ReduceMaximumHandSize
            | MaximumHandSizeSevenMinusYourGraveyardCardTypes
            | RevealFirstCardYouDrawEachTurn
            | LibraryOfLengDiscardReplacement
            | DrawReplacementExileTopFaceDown
            | ExileToCounteredExileInsteadOfGraveyard
            | CreaturesEnteringDontCauseAbilitiesToTrigger
            | DuplicateMatchingTriggeredAbilities
            | SuppressMatchingTriggeredAbilities
            | DoubleDamageFromSourcesYouControlOfChosenType
            | StartingLifeBonus
            | BuybackCostReduction
            | LegendRuleDoesntApply
            | ManaSpendPermission
            | SpendManaAsAnyColor
            | SpendManaAsAnyColorActivationCosts
            | RuleRestriction
            | DiscardOrRedirectReplacement
            | PayLifeOrEnterTappedReplacement
            | PregameAction
            | KeywordText
            | KeywordMarker
            | RuleTextPlaceholder
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
                | Protection
                | Reach
                | Shroud
                | Trample
                | Vigilance
                | Ward
                | Fear
                | Skulk
                | Flanking
                | Landwalk
                | Bloodthirst
                | Morph
                | Megamorph
                | Shadow
                | Horsemanship
                | Phasing
                | Wither
                | Infect
                | Changeling
                | Partner
                | DoctorsCompanion
                | Assist
                | SplitSecond
                | Rebound
                | Cascade
                | Unleash
                | KeywordText
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
                | Landwalk
                | CantBeBlockedAsLongAsDefendingPlayerControlsCardType
                | CantBeBlockedAsLongAsDefendingPlayerControlsCardTypes
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
                | CanBlockOnlyFlying
                | MaxCreaturesCanAttackEachCombat
                | MaxCreaturesCanBlockEachCombat
                | CantBeBlockedByPowerOrLess
                | CantBeBlockedByPowerOrGreater
                | CantBeBlockedByLowerPowerThanSource
                | CantBeBlockedByMoreThan
                | CantBeBlockedAsLongAsDefendingPlayerControlsCardType
                | CantBeBlockedAsLongAsDefendingPlayerControlsCardTypes
                | CanAttackAsThoughNoDefender
                | MustAttack
                | MustBlock
                | CantAttack
                | CantAttackItsOwner
                | CantAttackUnlessControllerCastCreatureSpellThisTurn
                | CantAttackUnlessControllerCastNonCreatureSpellThisTurn
                | CantAttackUnlessCondition
                | CantAttackYouUnlessControllerPaysPerAttacker
                | CantAttackYouUnlessControllerPaysPerAttackerBasicLandTypesAmongLandsYouControl
                | CantBlock
                | MayAssignDamageAsUnblocked
                | CreaturesAssignCombatDamageUsingToughness
                | CreaturesYouControlAssignCombatDamageUsingToughness
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
                | BloodMoon
                | Humility
                | BelloBardOfTheBrambles
                | CharacteristicDefiningPT
                | AddCardTypes
                | RemoveCardTypes
                | SetCardTypes
                | AddSubtypes
                | AddColors
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
        assert!(!StaticAbilityId::BloodMoon.is_keyword());
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
        assert!(StaticAbilityId::BloodMoon.generates_continuous_effects());
        assert!(!StaticAbilityId::Flying.generates_continuous_effects());
        assert!(!StaticAbilityId::Hexproof.generates_continuous_effects());
    }
}
