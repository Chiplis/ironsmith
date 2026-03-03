//! Static ability identity enum.
//!
//! This enum provides unique identifiers for each type of static ability.
//! Used for identity checks like `ability.id() == StaticAbilityId::Flying`.

/// Unique identifier for each type of static ability.
///
/// This is used for identity checking without pattern matching on trait objects.
/// When checking if an ability is a specific type, compare against this enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StaticAbilityId {
    // === Keyword abilities ===
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
    Assist,
    SplitSecond,
    Rebound,
    Cascade,
    Unleash,
    ConditionalSpellKeyword,
    ThisSpellCastRestriction,

    // === Combat modifiers ===
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
    CantBeBlockedByMoreThan,
    CanAttackAsThoughNoDefender,
    MustAttack,
    MustBlock,
    CantAttack,
    CantAttackUnlessControllerCastCreatureSpellThisTurn,
    CantAttackUnlessControllerCastNonCreatureSpellThisTurn,
    CantAttackUnlessCondition,
    CantAttackYouUnlessControllerPaysPerAttacker,
    CantAttackYouUnlessControllerPaysPerAttackerBasicLandTypesAmongLandsYouControl,
    CantBlock,
    MayAssignDamageAsUnblocked,
    CreaturesAssignCombatDamageUsingToughness,
    CreaturesYouControlAssignCombatDamageUsingToughness,

    // === Continuous effect generators ===
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
    ControlAttachedPermanent,
    GrantObjectAbilityForFilter,
    SetColors,
    SetName,
    MakeColorless,
    AddSupertypes,
    RemoveSupertypes,

    // === Cost modifiers ===
    CostReduction,
    ActivatedAbilityCostReduction,
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

    // === Game rule restrictions ===
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

    // === Other abilities ===
    ChooseColorAsEnters,
    ChooseBasicLandTypeAsEnters,
    EnchantedLandIsChosenType,
    RedirectDamageToSource,
    PreventAllDamageDealtByThisPermanent,
    PreventAllDamageDealtToCreatures,
    PreventAllCombatDamageToSelf,
    PreventAllDamageToSelfByCreatures,
    PreventDamageToSelfRemoveCounter,
    DoesntUntap,
    MayChooseNotToUntapDuringUntapStep,
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
    EnterWithCountersForFilter,
    CanBeCommander,
    LevelAbilities,
    NoMaximumHandSize,
    ReduceMaximumHandSize,
    MaximumHandSizeSevenMinusYourGraveyardCardTypes,
    LibraryOfLengDiscardReplacement,
    DrawReplacementExileTopFaceDown,
    CreaturesEnteringDontCauseAbilitiesToTrigger,
    StartingLifeBonus,
    BuybackCostReduction,
    LegendRuleDoesntApply,
    AdditionalLandPlay,
    SpendManaAsAnyColor,
    SpendManaAsAnyColorActivationCosts,

    /// Generic static rule restriction ("can't" effects) with runtime support.
    RuleRestriction,

    /// Interactive ETB: Discard a matching card or redirect to another zone.
    /// Used by Mox Diamond.
    DiscardOrRedirectReplacement,

    /// Interactive ETB: Pay life or enter tapped.
    /// Used by shock lands (Godless Shrine, etc.).
    PayLifeOrEnterTappedReplacement,

    /// Unimplemented keyword-like marker text preserved from parser/builder.
    KeywordMarker,

    /// Unimplemented static rule text preserved from parser/builder.
    RuleTextPlaceholder,

    /// Typed fallback keyword text preserved from parser/builder.
    KeywordFallbackText,

    /// Typed fallback static rule text preserved from parser/builder.
    RuleFallbackText,

    /// Parser fallback marker for unsupported lines in allow-unsupported mode.
    UnsupportedParserLine,

    /// Unified grant ability that grants abilities or alternative casting methods
    /// to cards matching a filter in a specific zone.
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
            | CantBeBlockedByMoreThan
            | CanAttackAsThoughNoDefender
            | MustAttack
            | MustBlock
            | CantAttack
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
            | ControlAttachedPermanent
            | GrantObjectAbilityForFilter
            | SetColors
            | SetName
            | MakeColorless
            | AddSupertypes
            | RemoveSupertypes
            | CostReduction
            | ActivatedAbilityCostReduction
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
            | ChooseColorAsEnters
            | ChooseBasicLandTypeAsEnters
            | EnchantedLandIsChosenType
            | RedirectDamageToSource
            | PreventAllDamageDealtByThisPermanent
            | PreventAllDamageDealtToCreatures
            | PreventAllCombatDamageToSelf
            | PreventAllDamageToSelfByCreatures
            | PreventDamageToSelfRemoveCounter
            | DoesntUntap
            | MayChooseNotToUntapDuringUntapStep
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
            | EnterWithCountersForFilter
            | CanBeCommander
            | LevelAbilities
            | NoMaximumHandSize
            | ReduceMaximumHandSize
            | MaximumHandSizeSevenMinusYourGraveyardCardTypes
            | LibraryOfLengDiscardReplacement
            | DrawReplacementExileTopFaceDown
            | CreaturesEnteringDontCauseAbilitiesToTrigger
            | StartingLifeBonus
            | BuybackCostReduction
            | LegendRuleDoesntApply
            | AdditionalLandPlay
            | SpendManaAsAnyColor
            | SpendManaAsAnyColorActivationCosts
            | RuleRestriction
            | DiscardOrRedirectReplacement
            | PayLifeOrEnterTappedReplacement
            | KeywordMarker
            | RuleTextPlaceholder
            | KeywordFallbackText
            | RuleFallbackText
            | UnsupportedParserLine
            | Grants => {}
        }
    }

    /// Returns true if this is a keyword ability.
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
                | Assist
                | SplitSecond
                | Rebound
                | Cascade
                | Unleash
                | KeywordFallbackText
        )
    }

    /// Returns true if this ability grants evasion.
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
                | CantBeBlockedByMoreThan
                | Landwalk
                | CantBeBlockedAsLongAsDefendingPlayerControlsCardType
                | CantBeBlockedAsLongAsDefendingPlayerControlsCardTypes
        )
    }

    /// Returns true if this ability affects combat.
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
                | CantBeBlockedByMoreThan
                | CantBeBlockedAsLongAsDefendingPlayerControlsCardType
                | CantBeBlockedAsLongAsDefendingPlayerControlsCardTypes
                | CanAttackAsThoughNoDefender
                | MustAttack
                | MustBlock
                | CantAttack
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

    /// Returns true if this ability generates continuous effects.
    pub fn generates_continuous_effects(&self) -> bool {
        Self::exhaustive_classification_guard(*self);
        use StaticAbilityId::*;
        matches!(
            self,
            Anthem
                | GrantAbility
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
    fn test_keyword_identification() {
        assert!(StaticAbilityId::Flying.is_keyword());
        assert!(StaticAbilityId::Trample.is_keyword());
        assert!(StaticAbilityId::Protection.is_keyword());

        assert!(!StaticAbilityId::Anthem.is_keyword());
        assert!(!StaticAbilityId::BloodMoon.is_keyword());
    }

    #[test]
    fn test_evasion_identification() {
        assert!(StaticAbilityId::Flying.grants_evasion());
        assert!(StaticAbilityId::Shadow.grants_evasion());

        assert!(!StaticAbilityId::Trample.grants_evasion());
        assert!(!StaticAbilityId::Lifelink.grants_evasion());
    }

    #[test]
    fn test_continuous_effect_identification() {
        assert!(StaticAbilityId::Anthem.generates_continuous_effects());
        assert!(StaticAbilityId::BloodMoon.generates_continuous_effects());

        assert!(!StaticAbilityId::Flying.generates_continuous_effects());
        assert!(!StaticAbilityId::Hexproof.generates_continuous_effects());
    }
}
