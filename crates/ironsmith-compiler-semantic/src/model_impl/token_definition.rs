use ironsmith_core::tag::TagKeyWalk;

use crate::color::ColorSet;
use crate::mana::ManaSymbol;
use crate::model::CompilerManaUsageRestriction as ManaUsageRestriction;
use crate::object::CounterType;
use crate::target::SourceReferenceSurface;
use crate::types::{CardType, Subtype};

/// Parser-level placeholder for an explicit "that card" stat reference in a
/// dynamic token definition. Lowering binds it to the retained card reference
/// (for example, the last exiled card) without letting an intervening token
/// creation steal the reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum BuiltinTokenShape {
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
#[derive(TagKeyWalk)]
pub enum TokenKeywordShape {
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
#[derive(TagKeyWalk)]
pub enum TokenCombatRestrictionShape {
    CantAttackOrBlockAlone,
    CantAttackOrBlock,
    Unblockable,
    CantBlock,
    MustAttack,
}

/// Specialized token rules whose authored order cannot be recovered from the
/// otherwise independent semantic fields on `CreatureTokenRulesShape`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum CreatureTokenInlineRuleKind {
    CombatRestriction,
    LeavesReturnNamedToHand,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub struct CreatureTokenInlineRulePresentation {
    pub kind: CreatureTokenInlineRuleKind,
    pub self_surface: Option<SourceReferenceSurface>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenCrewShape {
    pub amount: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub struct TokenEquipShape {
    pub amount: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenPowerAsThoughGreaterShape {
    pub amount: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub struct TokenTapManaAbilityShape {
    pub mana: Vec<ManaSymbol>,
    pub restrictions: Vec<ManaUsageRestriction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub struct TokenTapSacrificeManaLifeShape {
    pub mana_options: Vec<ManaSymbol>,
    pub life: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineNoncreatureSpellDamageShape {
    pub amount: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub struct TokenSacrificeReturnShape {
    pub card_name: String,
    pub mana_symbols: Vec<ManaSymbol>,
    pub tap_cost: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum TokenEmbeddedRuleShape {
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
#[derive(TagKeyWalk)]
pub struct TokenRulesSurfaces {
    pub embedded_rules: Vec<TokenEmbeddedRuleShape>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub struct EquipmentRulesShape {
    pub text: String,
    pub lines: Vec<EquipmentRuleLineShape>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub struct EquipmentDamageGrantShape {
    pub generic_amount: Option<u32>,
    pub tap_cost: bool,
    pub sacrifice_equipment: bool,
    pub damage_amount: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum EquipmentGrantCountShape {
    CountersAmongPermanentsYouControl(CounterType),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub struct EquipmentScaledPowerToughnessShape {
    pub power: i32,
    pub toughness: i32,
    pub count: EquipmentGrantCountShape,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum EquipmentRuleLineShape {
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
#[derive(TagKeyWalk)]
pub struct VehicleTokenShape {
    pub name: String,
    pub power_toughness: Option<(i32, i32)>,
    pub colorless: bool,
    pub flying: bool,
    pub crew_amount: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub struct ArtifactTokenShape {
    pub name: String,
    pub subtypes: Vec<Subtype>,
    pub legendary: bool,
    pub colorless: bool,
    pub colors: ColorSet,
    pub equipment_rules: Option<EquipmentRulesShape>,
    pub token_rules: TokenRulesSurfaces,
    pub leaves_damage_any_target: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub struct ShapeshifterTokenShape {
    pub changeling: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub struct AstartesWarriorTokenShape {
    pub vigilance: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub struct CreatureTokenRulesShape {
    pub token_rules: TokenRulesSurfaces,
    pub authored_inline_rules: Vec<CreatureTokenInlineRulePresentation>,
    pub cumulative_upkeep_mana_symbols: Option<Vec<ManaSymbol>>,
    pub tap_mana_ability: Option<TokenTapManaAbilityShape>,
    pub saddle_crew_power_bonus: Option<u32>,
    pub banding: bool,
    pub hexproof: bool,
    pub indestructible: bool,
    pub copies_exiled_triggered_abilities: bool,
    pub toxic_amount: Option<u32>,
    pub sacrifice_return: Option<TokenSacrificeReturnShape>,
    pub upkeep_return_name: Option<String>,
    pub upkeep_return_grants_haste: bool,
    pub dies_create_firebreathing_dragon: bool,
    pub dies_damage_any_target: Option<i32>,
    pub dies_minus_one_target_creature: bool,
    pub leaves_damage_you_and_creatures: Option<i32>,
    pub bands_with_wolves: bool,
    pub red_pump: bool,
    pub white_tap_target_creature: bool,
    pub combat_damage_poison: bool,
    pub noncreature_spell_each_opponent_damage: Option<i32>,
    pub becomes_tapped_damage_player: Option<i32>,
    pub combat_damage_gain_artifact: bool,
    pub leaves_return_named_to_hand: Option<String>,
    pub pest_dies_gain_life: bool,
    pub first_strike: bool,
    pub double_strike: bool,
    pub mercenary_pump: bool,
    pub combat_restriction: Option<TokenCombatRestrictionShape>,
    pub can_block_only_flying: bool,
    pub counter_noncreature_unless_pays: bool,
    pub changeling: bool,
    pub graveyard_anthem_card_name: Option<String>,
    pub landfall_pump: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub struct CreatureTokenShape {
    pub name: String,
    pub card_types: Vec<CardType>,
    pub subtypes: Vec<Subtype>,
    pub power_toughness: (i32, i32),
    pub legendary: bool,
    pub colors: ColorSet,
    pub use_source_chosen_color: bool,
    pub use_source_chosen_creature_type: bool,
    pub keywords: Vec<TokenKeywordShape>,
    pub rules: CreatureTokenRulesShape,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum ConstructArtifactScalingShape {
    CharacteristicDefining,
    GetsPlusOnePerArtifact,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub struct ConstructTokenShape {
    pub power_toughness: (i32, i32),
    pub artifact_scaling: Option<ConstructArtifactScalingShape>,
}

/// Parser-owned semantic token definition carried through preparation into lowering.
#[derive(Debug, Clone, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum TokenDefinitionSpec {
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
    pub fn has_intrinsic_abilities(&self) -> bool {
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
