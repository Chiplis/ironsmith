use ironsmith_core::tag::TagKeyWalk;

use crate::color::ColorSet;
use crate::mana::ManaSymbol;
use crate::model::CompilerManaUsageRestriction as ManaUsageRestriction;
use crate::object::CounterType;
use crate::target::SourceReferenceSurface;
use crate::types::{CardType, Subtype, Supertype};

/// Parser-level placeholder for an explicit "that card" stat reference in a
/// dynamic token definition. Lowering binds it to the retained card reference
/// (for example, the last exiled card) without letting an intervening token
/// creation steal the reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, TagKeyWalk)]
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
    VirtuousRole,
    Blood,
    Powerstone,
    Heartwood,
    Vibranium,
    Gingerbrute,
    Mutavault,
    SpellgorgerWeird,
    Tarmogoyf,
}

impl BuiltinTokenShape {
    pub fn card_types(self) -> Vec<CardType> {
        match self {
            Self::Walker | Self::EldraziSpawn | Self::EldraziScion
            | Self::SpellgorgerWeird | Self::Tarmogoyf => vec![CardType::Creature],
            Self::Gingerbrute => vec![CardType::Artifact, CardType::Creature],
            Self::Mutavault => vec![CardType::Land],
            Self::Shard | Self::WickedRole | Self::YoungHeroRole | Self::MonsterRole
            | Self::SorcererRole | Self::RoyalRole | Self::CursedRole
            | Self::VirtuousRole => vec![CardType::Enchantment],
            Self::Treasure | Self::Clue | Self::Map | Self::Lander | Self::Junk
            | Self::Mutagen | Self::Gold | Self::Food | Self::Blood | Self::Powerstone
            | Self::Heartwood | Self::Vibranium => vec![CardType::Artifact],
        }
    }

    pub fn subtypes(self) -> Vec<Subtype> {
        match self {
            Self::Treasure => vec![Subtype::Treasure], Self::Clue => vec![Subtype::Clue],
            Self::Map => vec![Subtype::Map], Self::Lander => vec![Subtype::Lander],
            Self::Junk => vec![Subtype::Junk], Self::Mutagen => vec![Subtype::Mutagen],
            Self::Gold => vec![Subtype::Gold], Self::Shard => vec![Subtype::Shard],
            Self::Walker => vec![Subtype::Zombie],
            Self::EldraziSpawn => vec![Subtype::Eldrazi, Subtype::Spawn],
            Self::EldraziScion => vec![Subtype::Eldrazi, Subtype::Scion],
            Self::Food => vec![Subtype::Food], Self::Blood => vec![Subtype::Blood],
            Self::Powerstone => vec![Subtype::Powerstone], Self::Heartwood => vec![Subtype::Heartwood],
            Self::Vibranium => vec![Subtype::Vibranium],
            Self::WickedRole | Self::YoungHeroRole | Self::MonsterRole
            | Self::SorcererRole | Self::RoyalRole | Self::CursedRole
            | Self::VirtuousRole => vec![Subtype::Aura, Subtype::Role],
            Self::Gingerbrute => vec![Subtype::Food, Subtype::Golem],
            Self::Mutavault => Vec::new(), Self::SpellgorgerWeird => vec![Subtype::Weird],
            Self::Tarmogoyf => vec![Subtype::Lhurgoyf],
        }
    }
    /// CR 111.10/111.11 supply these names explicitly. The enum is the
    /// definition authority; no finished CardDefinition name is inspected.
    pub fn fixed_name(self) -> Option<&'static str> {
        match self {
            Self::Walker => Some("Walker"),
            Self::WickedRole => Some("Wicked"),
            Self::YoungHeroRole => Some("Young Hero"),
            Self::MonsterRole => Some("Monster"),
            Self::SorcererRole => Some("Sorcerer"),
            Self::RoyalRole => Some("Royal"),
            Self::CursedRole => Some("Cursed"),
            Self::VirtuousRole => Some("Virtuous"),
            Self::Gingerbrute => Some("Gingerbrute"),
            Self::Mutavault => Some("Mutavault"),
            Self::SpellgorgerWeird => Some("Spellgorger Weird"),
            Self::Tarmogoyf => Some("Tarmogoyf"),
            Self::Treasure | Self::Clue | Self::Map | Self::Lander | Self::Junk
            | Self::Mutagen | Self::Gold | Self::Shard | Self::EldraziSpawn
            | Self::EldraziScion | Self::Food | Self::Blood | Self::Powerstone
            | Self::Heartwood | Self::Vibranium => None,
        }
    }

    pub fn text_roles(self) -> TokenDescriptionTextRoles {
        use ironsmith_core::{TokenNameTextRole, TokenWordRole};
        // These two legacy abbreviations are not predefined tokens in CR
        // 111.10. Complete Spawn/Scion descriptions use the Creature owner.
        let role = if matches!(self, Self::EldraziSpawn | Self::EldraziScion) {
            TokenWordRole::Unrecorded
        } else { TokenWordRole::RulesImplied };
        TokenDescriptionTextRoles {
            name: if self.fixed_name().is_some() { TokenNameTextRole::Explicit }
                else { TokenNameTextRole::SubtypeDerived },
            colors: role, subtypes: role, abilities: role,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, TagKeyWalk)]
pub enum TokenKeywordShape {
    Flying,
    WardGeneric(u32),
    Firebending(u32),
    /// "devour N" (CR 702.82a).
    Devour(u32),
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
    Shroud,
    /// "protection from black" / "protection from red and from white"
    ProtectionFromColors(crate::color::ColorSet),
    Indestructible,
    Infect,
    Flash,
    Islandwalk,
    Mountainwalk,
    Forestwalk,
    Swampwalk,
    Plainswalk,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, TagKeyWalk)]
pub enum TokenCombatRestrictionShape {
    CantAttackOrBlockAlone,
    CantAttackOrBlock,
    Unblockable,
    CantBlock,
    MustAttack,
}

/// Specialized token rules whose authored order cannot be recovered from the
/// otherwise independent semantic fields on `CreatureTokenRulesShape`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, TagKeyWalk)]
pub enum CreatureTokenInlineRuleKind {
    CombatRestriction,
    LeavesReturnNamedToHand,
}

#[derive(Debug, Clone, PartialEq, Eq, TagKeyWalk)]
pub struct CreatureTokenInlineRulePresentation {
    pub kind: CreatureTokenInlineRuleKind,
    pub self_surface: Option<SourceReferenceSurface>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenCrewShape {
    pub amount: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, TagKeyWalk)]
pub struct TokenEquipShape {
    pub amount: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenPowerAsThoughGreaterShape {
    pub amount: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, TagKeyWalk)]
pub struct TokenTapManaAbilityShape {
    pub mana: Vec<ManaSymbol>,
    pub restrictions: Vec<ManaUsageRestriction>,
}

#[derive(Debug, Clone, PartialEq, Eq, TagKeyWalk)]
pub struct TokenTapSacrificeManaLifeShape {
    pub mana_options: Vec<ManaSymbol>,
    pub life: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineNoncreatureSpellDamageShape {
    pub amount: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, TagKeyWalk)]
pub struct TokenSacrificeReturnShape {
    pub card_name: String,
    pub mana_symbols: Vec<ManaSymbol>,
    pub tap_cost: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, TagKeyWalk)]
pub enum TokenEmbeddedRuleShape {
    MaximumBlockers {
        maximum: usize,
    },
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

#[derive(Debug, Clone, Default, PartialEq, Eq, TagKeyWalk)]
pub struct TokenRulesSurfaces {
    pub embedded_rules: Vec<TokenEmbeddedRuleShape>,
}

#[derive(Debug, Clone, PartialEq, Eq, TagKeyWalk)]
pub struct EquipmentRulesShape {
    pub text: String,
    pub lines: Vec<EquipmentRuleLineShape>,
}

#[derive(Debug, Clone, PartialEq, Eq, TagKeyWalk)]
pub struct EquipmentDamageGrantShape {
    pub generic_amount: Option<u32>,
    pub tap_cost: bool,
    pub sacrifice_equipment: bool,
    pub damage_amount: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, TagKeyWalk)]
pub enum EquipmentGrantCountShape {
    CountersAmongPermanentsYouControl(CounterType),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, TagKeyWalk)]
pub struct EquipmentScaledPowerToughnessShape {
    pub power: i32,
    pub toughness: i32,
    pub count: EquipmentGrantCountShape,
}

#[derive(Debug, Clone, PartialEq, Eq, TagKeyWalk)]
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

#[derive(Debug, Clone, PartialEq, Eq, TagKeyWalk)]
pub struct TokenDescriptionTextRoles {
    pub name: ironsmith_core::TokenNameTextRole,
    pub colors: ironsmith_core::TokenWordRole,
    pub subtypes: ironsmith_core::TokenWordRole,
    pub abilities: ironsmith_core::TokenWordRole,
}

/// Explicit modifications to a typed predefined token (CR 111.10). Its
/// inherited definition remains separate from words added by this instruction.
#[derive(Debug, Clone, PartialEq, Eq, TagKeyWalk)]
pub struct ModifiedBuiltinTokenShape {
    pub template: BuiltinTokenShape,
    pub name: Option<String>,
    pub colors: Option<ColorSet>,
    pub color_words: ironsmith_core::TokenWordRole,
    pub power_toughness: Option<(i32, i32)>,
    pub supertypes: Vec<Supertype>,
    pub additional_card_types: Vec<CardType>,
    pub additional_subtypes: Vec<Subtype>,
    pub keywords: Vec<TokenKeywordShape>,
    pub keyword_words: ironsmith_core::TokenWordRole,
    pub words_complete: bool,
}

impl ModifiedBuiltinTokenShape {
    pub fn new(template: BuiltinTokenShape) -> Self {
        Self { template, name: None, colors: None, color_words: ironsmith_core::TokenWordRole::Authored,
            power_toughness: None, supertypes: Vec::new(), additional_card_types: Vec::new(),
            additional_subtypes: Vec::new(), keywords: Vec::new(), keyword_words: ironsmith_core::TokenWordRole::Authored,
            words_complete: true }
    }
    pub fn text_roles(&self) -> TokenDescriptionTextRoles {
        let mut roles = self.template.text_roles();
        if self.name.is_some() { roles.name = ironsmith_core::TokenNameTextRole::Explicit; }
        if self.colors.is_some() { roles.colors = self.color_words; }
        // Equal inherited and added subtype values can represent different
        // word occurrences. The uniform subtype role cannot prove that mix.
        if !self.additional_subtypes.is_empty() { roles.subtypes = ironsmith_core::TokenWordRole::Unrecorded; }
        if !self.words_complete { roles.colors = ironsmith_core::TokenWordRole::Unrecorded; }
        roles
    }
}
impl TokenDescriptionTextRoles {
    pub fn authored(name: ironsmith_core::TokenNameTextRole) -> Self {
        Self { name, colors: ironsmith_core::TokenWordRole::Authored,
            subtypes: ironsmith_core::TokenWordRole::Authored, abilities: ironsmith_core::TokenWordRole::Authored }
    }
    pub fn retained(&self, ability_count: usize) -> ironsmith_core::TokenTextRoles {
        ironsmith_core::TokenTextRoles { name: self.name, colors: self.colors, subtypes: self.subtypes,
            abilities: vec![self.abilities; ability_count] }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, TagKeyWalk)]
pub struct VehicleTokenShape {
    pub name: String,
    pub text_roles: Option<TokenDescriptionTextRoles>,
    pub power_toughness: Option<(i32, i32)>,
    pub colorless: bool,
    pub colors: ColorSet,
    pub legendary: bool,
    pub flying: bool,
    pub crew_amount: Option<u32>,
}

/// A land token ("a tapped colorless land token named Everywhere that is
/// every basic land type"). Basic land types give it their intrinsic mana
/// abilities (CR 305.6).
#[derive(Debug, Clone, PartialEq, Eq, TagKeyWalk)]
pub struct LandTokenShape {
    pub name: String,
    pub subtypes: Vec<Subtype>,
    pub legendary: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, TagKeyWalk)]
pub struct EnchantmentTokenShape {
    pub name: String,
    pub text_roles: Option<TokenDescriptionTextRoles>,
    pub subtypes: Vec<Subtype>,
    pub legendary: bool,
    pub colors: ColorSet,
    pub token_rules: TokenRulesSurfaces,
}

#[derive(Debug, Clone, PartialEq, Eq, TagKeyWalk)]
pub struct ArtifactTokenShape {
    pub name: String,
    pub text_roles: Option<TokenDescriptionTextRoles>,
    pub subtypes: Vec<Subtype>,
    pub legendary: bool,
    pub colorless: bool,
    pub colors: ColorSet,
    pub equipment_rules: Option<EquipmentRulesShape>,
    pub token_rules: TokenRulesSurfaces,
    pub leaves_damage_any_target: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, TagKeyWalk)]
pub struct ShapeshifterTokenShape {
    pub changeling: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, TagKeyWalk)]
pub struct AstartesWarriorTokenShape {
    pub vigilance: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, TagKeyWalk)]
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

#[derive(Debug, Clone, PartialEq, Eq, TagKeyWalk)]
pub struct CreatureTokenShape {
    pub name: String,
    pub text_roles: Option<TokenDescriptionTextRoles>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, TagKeyWalk)]
pub enum ConstructArtifactScalingShape {
    CharacteristicDefining,
    GetsPlusOnePerArtifact,
}

#[derive(Debug, Clone, PartialEq, Eq, TagKeyWalk)]
pub struct ConstructTokenShape {
    pub power_toughness: (i32, i32),
    pub artifact_scaling: Option<ConstructArtifactScalingShape>,
}

/// A lexical reference to an authored token blueprint, not to any objects
/// created while resolving it. The antecedent may be in an unexecuted branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, TagKeyWalk)]
pub enum TokenPrototypeReference {
    PreviousDefinition,
}

/// Parser-owned semantic token definition carried through preparation into lowering.
#[derive(Debug, Clone, PartialEq, Eq, TagKeyWalk)]
pub enum TokenDefinitionSpec {
    PrototypeReference(TokenPrototypeReference),
    Builtin(BuiltinTokenShape),
    Vehicle(VehicleTokenShape),
    Artifact(ArtifactTokenShape),
    Enchantment(EnchantmentTokenShape),
    Land(LandTokenShape),
    Angel,
    Wall,
    Squirrel,
    DragonEgg,
    Elephant,
    Construct(ConstructTokenShape),
    Shapeshifter(ShapeshifterTokenShape),
    AstartesWarrior(AstartesWarriorTokenShape),
    Creature(CreatureTokenShape),
    ModifiedBuiltin(ModifiedBuiltinTokenShape),
}

impl TokenDefinitionSpec {
    pub fn text_roles(&self) -> Option<TokenDescriptionTextRoles> {
        match self {
            Self::Builtin(builtin) => Some(builtin.text_roles()),
            Self::ModifiedBuiltin(shape) => Some(shape.text_roles()),
            Self::Vehicle(shape) => shape.text_roles.clone(),
            Self::Enchantment(shape) => shape.text_roles.clone(),
            Self::Artifact(shape) => shape.text_roles.clone(),
            Self::Creature(shape) => shape.text_roles.clone(),
            _ => None,
        }
    }
    pub fn mark_unproven_ability_words(&mut self) {
        if let Self::ModifiedBuiltin(shape) = self {
            shape.keyword_words = ironsmith_core::TokenWordRole::Unrecorded;
            shape.words_complete = false;
            return;
        }
        let roles = match self {
            Self::Vehicle(shape) => &mut shape.text_roles,
            Self::Enchantment(shape) => &mut shape.text_roles,
            Self::Artifact(shape) => &mut shape.text_roles,
            Self::Creature(shape) => &mut shape.text_roles,
            _ => return,
        };
        if let Some(roles) = roles { roles.abilities = ironsmith_core::TokenWordRole::Unrecorded; }
    }
    /// Whether a post-create `It has ...` sentence must remain separate from
    /// abilities already authored in the token-definition sentence.
    pub fn has_intrinsic_abilities(&self) -> bool {
        match self {
            Self::Enchantment(enchantment) => !enchantment.token_rules.embedded_rules.is_empty(),
            Self::Vehicle(vehicle) => vehicle.flying || vehicle.crew_amount.is_some(),
            Self::Artifact(artifact) => {
                artifact.equipment_rules.is_some()
                    || !artifact.token_rules.embedded_rules.is_empty()
                    || artifact.leaves_damage_any_target.is_some()
            }
            Self::Construct(construct) => construct.artifact_scaling.is_some(),
            Self::Shapeshifter(shapeshifter) => shapeshifter.changeling,
            Self::AstartesWarrior(warrior) => warrior.vigilance,
            Self::ModifiedBuiltin(_) => true,
            Self::Creature(creature) => {
                !creature.keywords.is_empty()
                    || creature.rules != CreatureTokenRulesShape::default()
            }
            // Named and built-in token shapes may carry abilities during
            // lowering even when their compact parser shape has no fields for
            // them. Treat them conservatively as nonempty.
            Self::Land(_) => false,
            Self::PrototypeReference(_)
            | Self::Builtin(_)
            | Self::Angel
            | Self::Wall
            | Self::Squirrel
            | Self::DragonEgg
            | Self::Elephant => true,
        }
    }
}
