use crate::ability::ManaUsageRestriction;
use crate::color::ColorSet;
use crate::mana::ManaSymbol;
use crate::object::CounterType;
use crate::target::SourceReferenceSurface;
use crate::types::{CardType, Subtype};

/// Parser-level placeholder for an explicit "that card" stat reference in a
/// dynamic token definition. Lowering binds it to the retained card reference
/// (for example, the last exiled card) without letting an intervening token
/// creation steal the reference.
pub(crate) const TOKEN_DYNAMIC_THAT_CARD_TAG: &str = "__token_dynamic_that_card";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BuiltinTokenShape {
    Treasure,
    Clue,
    Map,
    Lander,
    Junk,
    Mutagen,
    Gold,
    Shard,
    Walker,
    EldraziSpawn,
    EldraziScion,
    Food,
    WickedRole,
    YoungHeroRole,
    MonsterRole,
    SorcererRole,
    RoyalRole,
    CursedRole,
    Blood,
    Powerstone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TokenKeywordShape {
    Flying,
    WardGeneric(u32),
    Firebending(u32),
    Defender,
    Prowess,
    Vigilance,
    Trample,
    Lifelink,
    Deathtouch,
    Haste,
    Menace,
    Reach,
    FirstStrike,
    DoubleStrike,
    Hexproof,
    Indestructible,
    Infect,
    Flash,
    Islandwalk,
    Mountainwalk,
    Forestwalk,
    Swampwalk,
    Plainswalk,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TokenCombatRestrictionShape {
    CantAttackOrBlockAlone,
    CantAttackOrBlock,
    Unblockable,
    CantBlock,
    MustAttack,
}

/// Specialized token rules whose authored order cannot be recovered from the
/// otherwise independent semantic fields on `CreatureTokenRulesShape`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CreatureTokenInlineRuleKind {
    CombatRestriction,
    LeavesReturnNamedToHand,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CreatureTokenInlineRulePresentation {
    pub(crate) kind: CreatureTokenInlineRuleKind,
    pub(crate) self_surface: Option<SourceReferenceSurface>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TokenCrewShape {
    pub(crate) amount: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TokenEquipShape {
    pub(crate) amount: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TokenPowerAsThoughGreaterShape {
    pub(crate) amount: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TokenTapManaAbilityShape {
    pub(crate) mana: Vec<ManaSymbol>,
    pub(crate) restrictions: Vec<ManaUsageRestriction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TokenTapSacrificeManaLifeShape {
    pub(crate) mana_options: Vec<ManaSymbol>,
    pub(crate) life: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InlineNoncreatureSpellDamageShape {
    pub(crate) amount: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TokenSacrificeReturnShape {
    pub(crate) card_name: String,
    pub(crate) mana_symbols: Vec<ManaSymbol>,
    pub(crate) tap_cost: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TokenEmbeddedRuleShape {
    CantBlockOrBeBlockedByNonSubtypeCreatures {
        subtype: Subtype,
    },
    OpponentCastsCreatureRemoveCreatureTypeUntilEndOfTurn,
    PowerToughnessEqualCreaturesYouControl,
    LandEntersPutCountersOnSelf {
        counter_type: CounterType,
        count: u32,
    },
    DiesCreateBuiltinToken {
        token: BuiltinTokenShape,
        count: u32,
    },
    DealsDamageToPlayerPutCounters {
        combat_only: bool,
        counter_type: CounterType,
        count: u32,
    },
    DealsDamageToPlayerLoseGame {
        combat_only: bool,
    },
    DealsDamageToPlaneswalkerDestroy {
        combat_only: bool,
    },
    BeginningOfYourUpkeepSacrificeAnotherCreatureOrSourceDamagesYou {
        damage: i32,
    },
    TapSacrificeAddManaOfAnyColor,
    TapSacrificeAddManaOrGainLife(TokenTapSacrificeManaLifeShape),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct TokenRulesSurfaces {
    pub(crate) embedded_rules: Vec<TokenEmbeddedRuleShape>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EquipmentRulesShape {
    pub(crate) text: String,
    pub(crate) lines: Vec<EquipmentRuleLineShape>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EquipmentDamageGrantShape {
    pub(crate) generic_amount: Option<u32>,
    pub(crate) tap_cost: bool,
    pub(crate) sacrifice_equipment: bool,
    pub(crate) damage_amount: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EquipmentGrantCountShape {
    CountersAmongPermanentsYouControl(CounterType),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EquipmentScaledPowerToughnessShape {
    pub(crate) power: i32,
    pub(crate) toughness: i32,
    pub(crate) count: EquipmentGrantCountShape,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum EquipmentRuleLineShape {
    GrantedDamage {
        display_text: String,
        grant: EquipmentDamageGrantShape,
    },
    StaticGrant {
        display_text: String,
        power_toughness: Option<(i32, i32)>,
        scaled_power_toughness: Option<EquipmentScaledPowerToughnessShape>,
        keywords: Vec<TokenKeywordShape>,
    },
    Equip(TokenEquipShape),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VehicleTokenShape {
    pub(crate) name: String,
    pub(crate) power_toughness: Option<(i32, i32)>,
    pub(crate) colorless: bool,
    pub(crate) flying: bool,
    pub(crate) crew_amount: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ArtifactTokenShape {
    pub(crate) name: String,
    pub(crate) subtypes: Vec<Subtype>,
    pub(crate) legendary: bool,
    pub(crate) colorless: bool,
    pub(crate) colors: ColorSet,
    pub(crate) equipment_rules: Option<EquipmentRulesShape>,
    pub(crate) token_rules: TokenRulesSurfaces,
    pub(crate) leaves_damage_any_target: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShapeshifterTokenShape {
    pub(crate) changeling: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AstartesWarriorTokenShape {
    pub(crate) vigilance: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct CreatureTokenRulesShape {
    pub(crate) token_rules: TokenRulesSurfaces,
    pub(crate) authored_inline_rules: Vec<CreatureTokenInlineRulePresentation>,
    pub(crate) cumulative_upkeep_mana_symbols: Option<Vec<ManaSymbol>>,
    pub(crate) tap_mana_ability: Option<TokenTapManaAbilityShape>,
    pub(crate) saddle_crew_power_bonus: Option<u32>,
    pub(crate) banding: bool,
    pub(crate) hexproof: bool,
    pub(crate) indestructible: bool,
    pub(crate) copies_exiled_triggered_abilities: bool,
    pub(crate) toxic_amount: Option<u32>,
    pub(crate) sacrifice_return: Option<TokenSacrificeReturnShape>,
    pub(crate) upkeep_return_name: Option<String>,
    pub(crate) upkeep_return_grants_haste: bool,
    pub(crate) dies_create_firebreathing_dragon: bool,
    pub(crate) dies_damage_any_target: Option<i32>,
    pub(crate) dies_minus_one_target_creature: bool,
    pub(crate) leaves_damage_you_and_creatures: Option<i32>,
    pub(crate) bands_with_wolves: bool,
    pub(crate) red_pump: bool,
    pub(crate) white_tap_target_creature: bool,
    pub(crate) combat_damage_poison: bool,
    pub(crate) noncreature_spell_each_opponent_damage: Option<i32>,
    pub(crate) becomes_tapped_damage_player: Option<i32>,
    pub(crate) combat_damage_gain_artifact: bool,
    pub(crate) leaves_return_named_to_hand: Option<String>,
    pub(crate) pest_dies_gain_life: bool,
    pub(crate) first_strike: bool,
    pub(crate) double_strike: bool,
    pub(crate) mercenary_pump: bool,
    pub(crate) combat_restriction: Option<TokenCombatRestrictionShape>,
    pub(crate) can_block_only_flying: bool,
    pub(crate) counter_noncreature_unless_pays: bool,
    pub(crate) changeling: bool,
    pub(crate) graveyard_anthem_card_name: Option<String>,
    pub(crate) landfall_pump: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CreatureTokenShape {
    pub(crate) name: String,
    pub(crate) card_types: Vec<CardType>,
    pub(crate) subtypes: Vec<Subtype>,
    pub(crate) power_toughness: (i32, i32),
    pub(crate) legendary: bool,
    pub(crate) colors: ColorSet,
    pub(crate) use_source_chosen_color: bool,
    pub(crate) use_source_chosen_creature_type: bool,
    pub(crate) keywords: Vec<TokenKeywordShape>,
    pub(crate) rules: CreatureTokenRulesShape,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConstructArtifactScalingShape {
    CharacteristicDefining,
    GetsPlusOnePerArtifact,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConstructTokenShape {
    pub(crate) power_toughness: (i32, i32),
    pub(crate) artifact_scaling: Option<ConstructArtifactScalingShape>,
}

/// Parser-owned semantic token definition carried through preparation into lowering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TokenDefinitionSpec {
    PriorCreated,
    Builtin(BuiltinTokenShape),
    Vehicle(VehicleTokenShape),
    Artifact(ArtifactTokenShape),
    Angel,
    Wall,
    Squirrel,
    DragonEgg,
    Elephant,
    Construct(ConstructTokenShape),
    Shapeshifter(ShapeshifterTokenShape),
    AstartesWarrior(AstartesWarriorTokenShape),
    Creature(CreatureTokenShape),
}

impl TokenDefinitionSpec {
    /// Whether a post-create `It has ...` sentence must remain separate from
    /// abilities already authored in the token-definition sentence.
    pub(crate) fn has_intrinsic_abilities(&self) -> bool {
        match self {
            Self::Vehicle(vehicle) => vehicle.flying || vehicle.crew_amount.is_some(),
            Self::Artifact(artifact) => {
                artifact.equipment_rules.is_some()
                    || !artifact.token_rules.embedded_rules.is_empty()
                    || artifact.leaves_damage_any_target.is_some()
            }
            Self::Construct(construct) => construct.artifact_scaling.is_some(),
            Self::Shapeshifter(shapeshifter) => shapeshifter.changeling,
            Self::AstartesWarrior(warrior) => warrior.vigilance,
            Self::Creature(creature) => {
                !creature.keywords.is_empty()
                    || creature.rules != CreatureTokenRulesShape::default()
            }
            // Named and built-in token shapes may carry abilities during
            // lowering even when their compact parser shape has no fields for
            // them. Treat them conservatively as nonempty.
            Self::PriorCreated
            | Self::Builtin(_)
            | Self::Angel
            | Self::Wall
            | Self::Squirrel
            | Self::DragonEgg
            | Self::Elephant => true,
        }
    }
}
