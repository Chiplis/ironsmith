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
    Flanking,
    Shadow,
    Horsemanship,
    Phasing,
    Wither,
    Infect,
    Changeling,

    // === Combat modifiers ===
    Unblockable,
    FlyingRestriction,
    CanBlockFlying,
    CanAttackAsThoughNoDefender,
    MustAttack,
    MustBlock,
    CantAttack,
    CantBlock,
    MayAssignDamageAsUnblocked,

    // === Continuous effect generators ===
    Anthem,
    GrantAbility,
    RemoveAbilityForFilter,
    RemoveAllAbilitiesForFilter,
    SetBasePowerToughnessForFilter,
    EquipmentGrant,
    BloodMoon,
    Humility,
    BelloBardOfTheBrambles,
    CharacteristicDefiningPT,
    AddCardTypes,
    CopyActivatedAbilities,
    ManascapeRefractor,
    SquirrelNest,
    MycosynthLattice,
    TophFirstMetalbender,
    MarvinMurderousMimic,
    AttachedAbilityGrant,
    SetColors,
    MakeColorless,
    RemoveSupertypes,

    // === Cost modifiers ===
    CostReduction,
    CostIncrease,
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

    // === Other abilities ===
    DoesntUntap,
    EntersTapped,
    ShuffleIntoLibraryFromGraveyard,
    AllPermanentsEnterTapped,
    EnterTappedForFilter,
    EnterWithCountersForFilter,
    CanBeCommander,
    LevelAbilities,
    NoMaximumHandSize,
    LibraryOfLengDiscardReplacement,
    StartingLifeBonus,
    BuybackCostReduction,
    LegendRuleDoesntApply,
    AdditionalLandPlay,
    SpendManaAsAnyColor,
    SpendManaAsAnyColorActivationCosts,

    /// Interactive ETB: Discard a matching card or redirect to another zone.
    /// Used by Mox Diamond.
    DiscardOrRedirectReplacement,

    /// Interactive ETB: Pay life or enter tapped.
    /// Used by shock lands (Godless Shrine, etc.).
    PayLifeOrEnterTappedReplacement,

    /// Custom ability with a unique string ID.
    Custom,

    /// Unified grant ability that grants abilities or alternative casting methods
    /// to cards matching a filter in a specific zone.
    Grants,
}

impl StaticAbilityId {
    /// Returns true if this is a keyword ability.
    pub fn is_keyword(&self) -> bool {
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
                | Flanking
                | Shadow
                | Horsemanship
                | Phasing
                | Wither
                | Infect
                | Changeling
        )
    }

    /// Returns true if this ability grants evasion.
    pub fn grants_evasion(&self) -> bool {
        use StaticAbilityId::*;
        matches!(
            self,
            Flying | Shadow | Horsemanship | Fear | Intimidate | FlyingRestriction
        )
    }

    /// Returns true if this ability affects combat.
    pub fn affects_combat(&self) -> bool {
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
                | Flanking
                | Shadow
                | Horsemanship
                | Unblockable
                | FlyingRestriction
                | CanBlockFlying
                | CanAttackAsThoughNoDefender
                | MustAttack
                | MustBlock
                | CantAttack
                | CantBlock
                | MayAssignDamageAsUnblocked
        )
    }

    /// Returns true if this ability generates continuous effects.
    pub fn generates_continuous_effects(&self) -> bool {
        use StaticAbilityId::*;
        matches!(
            self,
            Anthem
                | GrantAbility
                | RemoveAllAbilitiesForFilter
                | SetBasePowerToughnessForFilter
                | EquipmentGrant
                | BloodMoon
                | Humility
                | BelloBardOfTheBrambles
                | CharacteristicDefiningPT
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
