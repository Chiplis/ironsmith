use std::any::Any;

pub use ironsmith_core::{
    ConditionalSpellKeywordKind, ConditionalSpellKeywordSpec, GraveyardCountMetric,
    ManaSpendPermission, PregameActionKind, PregameBeginOnBattlefieldSpec, StaticAbilityId,
    ThisSpellCastTiming,
};
pub const TOPH_FIRST_METALBENDER: StaticAbilityId = StaticAbilityId::TophFirstMetalbender;
pub const PREVENT_ALL_DAMAGE_DEALT_BY_THIS_PERMANENT: StaticAbilityId =
    StaticAbilityId::PreventAllDamageDealtByThisPermanent;

#[derive(Debug, Clone, PartialEq)]
pub struct StaticAbility {
    pub id: Option<StaticAbilityId>,
    pub label: String,
    pub payload: StaticAbilityPayload,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum StaticAbilityPayload {
    #[default]
    None,
    Anthem(Anthem),
    AttachedAbilityGrant(Box<AttachedAbilityGrant>),
    Conditional {
        ability: Box<StaticAbility>,
        condition: crate::ConditionExpr,
    },
    GrantAbility(Box<GrantAbility>),
    GrantObjectAbilityForFilter(Box<GrantObjectAbilityForFilter>),
    CopyActivatedAbilities(CopyActivatedAbilities),
    CostReduction(CostReduction),
    CostReductionManaCost(CostReductionManaCost),
    CostIncrease(CostIncrease),
    CostIncreaseManaCost(CostIncreaseManaCost),
    ThisSpellCostReduction(ThisSpellCostReduction),
    ThisSpellCostReductionManaCost(ThisSpellCostReductionManaCost),
    LevelAbility(Box<crate::ability::LevelAbility>),
    Protection(crate::ability::ProtectionFrom),
    RuleRestriction {
        restriction: crate::effect::Restriction,
        display: String,
    },
    PregameAction {
        kind: PregameActionKind,
        text: String,
    },
    Ward(crate::cost::TotalCost),
    CanBlockAdditionalCreatureEachCombat(usize),
    ExertAttack {
        only_if_not_exerted_this_turn: bool,
        linked_trigger: Option<crate::ability::TriggeredAbility>,
        display: String,
    },
    EquipmentGrant(Vec<StaticAbility>),
    RemoveAllAbilities(crate::target::ObjectFilter),
    RemoveAllAbilitiesExceptMana(crate::target::ObjectFilter),
    SetBasePowerToughness {
        filter: crate::target::ObjectFilter,
        power: i32,
        toughness: i32,
    },
    AddCardTypes {
        filter: crate::target::ObjectFilter,
        card_types: Vec<crate::types::CardType>,
    },
    SetCardTypes {
        filter: crate::target::ObjectFilter,
        card_types: Vec<crate::types::CardType>,
    },
    AddSubtypes {
        filter: crate::target::ObjectFilter,
        subtypes: Vec<crate::types::Subtype>,
    },
    AddAllSubtypesOfFamily {
        filter: crate::target::ObjectFilter,
        family: crate::types::SubtypeFamily,
    },
    SetCreatureSubtypes {
        filter: crate::target::ObjectFilter,
        subtypes: Vec<crate::types::Subtype>,
    },
    MakeColorless(crate::target::ObjectFilter),
    CostIncreasePerTargetBeyondFirst(u32),
    MinimumSpellTotalMana(u32),
    ActivatedAbilityCostReduction {
        filter: crate::target::ObjectFilter,
        reduction: u32,
        condition: Option<ActivatedAbilityCostCondition>,
        per_matching_objects: Option<crate::target::ObjectFilter>,
        minimum_total_mana: Option<u32>,
    },
    ActivatedAbilityCostIncrease {
        filter: crate::target::ObjectFilter,
        increase: crate::cost::TotalCost,
    },
    ChoosePlayerAsEnters(String),
    ChooseCreatureTypeAsEnters(String),
    EnterAsCopyAsEnters {
        spec: EnterAsCopyAsEntersSpec,
        display: String,
    },
    DoubleDamageFromSourcesYouControlOfChosenType(String),
    AdditionalLandPlays(u32),
    RevealFirstCardYouDrawEachTurn {
        optional: bool,
        your_turns_only: bool,
    },
    ExileToCounteredExileInsteadOfGraveyard {
        player: crate::target::PlayerFilter,
        counter_type: crate::object::CounterType,
    },
    CharacteristicDefiningPt {
        power: crate::effect::Value,
        toughness: crate::effect::Value,
    },
    DiscardOrRedirectReplacement {
        filter: crate::target::ObjectFilter,
        redirect_zone: crate::zone::Zone,
    },
    PayLifeOrEnterTapped(u32),
    ManaSpendPermission {
        permission: ManaSpendPermission,
        display: String,
    },
    Landwalk(LandwalkKind),
    Bloodthirst(u32),
    CantAttackYouUnlessControllerPaysPerAttacker(u32),
    CantAttackYouUnlessControllerPaysPerAttackerBasicLandTypesAmongLandsYouControl,
    Grants(Box<crate::grant::GrantSpec>),
    EntersTappedUnlessCondition {
        condition: crate::ConditionExpr,
        display: String,
    },
    EntersWithCountersIfCondition {
        counter: crate::object::CounterType,
        count: crate::effect::Value,
        condition: crate::ConditionExpr,
        display: String,
    },
    EntersWithCountersValue {
        counter: crate::object::CounterType,
        count: crate::effect::Value,
    },
    EntersTappedForFilter(crate::target::ObjectFilter),
    EntersUntappedForFilter(crate::target::ObjectFilter),
    EntersWithCountersAndSubtypesForFilter {
        filter: crate::target::ObjectFilter,
        counter: crate::object::CounterType,
        count: crate::effect::Value,
        subtypes: Vec<crate::types::Subtype>,
    },
}

impl StaticAbility {
    fn identified(id: StaticAbilityId, label: impl Into<String>) -> Self {
        Self {
            id: Some(id),
            label: label.into(),
            payload: StaticAbilityPayload::None,
        }
    }

    pub fn new(label: impl std::fmt::Debug + 'static) -> Self {
        let label_any = &label as &dyn Any;
        if let Some(id) = label_any.downcast_ref::<StaticAbilityId>() {
            return Self::identified(*id, format!("{id:?}"));
        }
        if let Some(payload) = label_any.downcast_ref::<Anthem>() {
            return Self {
                id: Some(StaticAbilityId::Anthem),
                label: "anthem".to_string(),
                payload: StaticAbilityPayload::Anthem(payload.clone()),
            };
        }
        if let Some(payload) = label_any.downcast_ref::<AttachedAbilityGrant>() {
            return Self {
                id: Some(StaticAbilityId::AttachedAbilityGrant),
                label: payload.display.clone(),
                payload: StaticAbilityPayload::AttachedAbilityGrant(Box::new(payload.clone())),
            };
        }
        if let Some(payload) = label_any.downcast_ref::<GrantAbility>() {
            return Self {
                id: Some(StaticAbilityId::GrantAbility),
                label: "grant ability".to_string(),
                payload: StaticAbilityPayload::GrantAbility(Box::new(payload.clone())),
            };
        }
        if let Some(payload) = label_any.downcast_ref::<GrantObjectAbilityForFilter>() {
            return Self {
                id: Some(StaticAbilityId::GrantObjectAbilityForFilter),
                label: payload.display.clone(),
                payload: StaticAbilityPayload::GrantObjectAbilityForFilter(Box::new(
                    payload.clone(),
                )),
            };
        }
        if let Some(payload) = label_any.downcast_ref::<CopyActivatedAbilities>() {
            return Self {
                id: Some(StaticAbilityId::CopyActivatedAbilities),
                label: "copy activated abilities".to_string(),
                payload: StaticAbilityPayload::CopyActivatedAbilities(payload.clone()),
            };
        }
        if let Some(payload) = label_any.downcast_ref::<CostReduction>() {
            return Self {
                id: Some(StaticAbilityId::CostReduction),
                label: "cost reduction".to_string(),
                payload: StaticAbilityPayload::CostReduction(payload.clone()),
            };
        }
        if let Some(payload) = label_any.downcast_ref::<CostReductionManaCost>() {
            return Self {
                id: Some(StaticAbilityId::CostReductionManaCost),
                label: "cost reduction mana cost".to_string(),
                payload: StaticAbilityPayload::CostReductionManaCost(payload.clone()),
            };
        }
        if let Some(payload) = label_any.downcast_ref::<CostIncrease>() {
            return Self {
                id: Some(StaticAbilityId::CostIncrease),
                label: "cost increase".to_string(),
                payload: StaticAbilityPayload::CostIncrease(payload.clone()),
            };
        }
        if let Some(payload) = label_any.downcast_ref::<CostIncreaseManaCost>() {
            return Self {
                id: Some(StaticAbilityId::CostIncreaseManaCost),
                label: "cost increase mana cost".to_string(),
                payload: StaticAbilityPayload::CostIncreaseManaCost(payload.clone()),
            };
        }
        if let Some(payload) = label_any.downcast_ref::<ThisSpellCostReduction>() {
            return Self {
                id: Some(StaticAbilityId::ThisSpellCostReduction),
                label: "this spell cost reduction".to_string(),
                payload: StaticAbilityPayload::ThisSpellCostReduction(payload.clone()),
            };
        }
        if let Some(payload) = label_any.downcast_ref::<ThisSpellCostReductionManaCost>() {
            return Self {
                id: Some(StaticAbilityId::ThisSpellCostReductionManaCost),
                label: "this spell cost reduction mana cost".to_string(),
                payload: StaticAbilityPayload::ThisSpellCostReductionManaCost(payload.clone()),
            };
        }
        if let Some(payload) = label_any.downcast_ref::<LandwalkKind>() {
            return Self {
                id: Some(StaticAbilityId::Landwalk),
                label: payload.display(),
                payload: StaticAbilityPayload::Landwalk(*payload),
            };
        }
        Self {
            id: None,
            label: format!("{label:?}"),
            payload: StaticAbilityPayload::None,
        }
    }

    pub fn level(ability: crate::ability::LevelAbility) -> Self {
        Self {
            id: Some(StaticAbilityId::LevelAbilities),
            label: "level".to_string(),
            payload: StaticAbilityPayload::LevelAbility(Box::new(ability)),
        }
    }

    pub fn flash() -> Self {
        Self {
            id: Some(StaticAbilityId::Flash),
            label: "flash".to_string(),
            payload: StaticAbilityPayload::None,
        }
    }

    pub fn this_spell_cast_restriction(
        _kind: ThisSpellCastRestrictionKind,
        text: impl Into<String>,
    ) -> Self {
        Self {
            id: Some(StaticAbilityId::ThisSpellCastRestriction),
            label: text.into(),
            payload: StaticAbilityPayload::None,
        }
    }

    pub fn morph(_cost: crate::cost::TotalCost) -> Self {
        Self::new("morph")
    }

    pub fn megamorph(_cost: crate::cost::TotalCost) -> Self {
        Self::new("megamorph")
    }

    pub fn keyword_marker(marker: impl std::fmt::Debug) -> Self {
        let text = format!("{marker:?}").trim_matches('"').to_string();
        if let Some(ability) = Self::known_keyword_marker(&text) {
            return ability;
        }
        Self::identified(StaticAbilityId::KeywordMarker, text)
    }

    fn known_keyword_marker(text: &str) -> Option<Self> {
        let normalized = text.trim().trim_end_matches('.').to_ascii_lowercase();
        if normalized.ends_with(" can't be blocked") || normalized.ends_with(" cant be blocked") {
            return Some(Self::unblockable());
        }
        Some(match normalized.as_str() {
            "flying" => Self::flying(),
            "first strike" => Self::first_strike(),
            "double strike" => Self::double_strike(),
            "deathtouch" => Self::deathtouch(),
            "lifelink" => Self::lifelink(),
            "vigilance" => Self::vigilance(),
            "trample" => Self::trample(),
            "reach" => Self::reach(),
            "defender" => Self::defender(),
            "flash" => Self::flash(),
            "haste" => Self::haste(),
            "menace" => Self::menace(),
            "hexproof" => Self::hexproof(),
            "indestructible" => Self::indestructible(),
            "shroud" => Self::shroud(),
            "wither" => Self::wither(),
            "infect" => Self::infect(),
            "skulk" => Self::skulk(),
            "prowess" => Self::prowess(),
            "cascade" => Self::cascade(),
            "unleash" => Self::unleash(),
            "split second" => Self::split_second(),
            "rebound" => Self::rebound(),
            "fear" => Self::fear(),
            "intimidate" => Self::intimidate(),
            "shadow" => Self::shadow(),
            "horsemanship" => Self::horsemanship(),
            "flanking" => Self::flanking(),
            "umbra armor" => Self::umbra_armor(),
            "phasing" => Self::phasing(),
            "improvise" => Self::improvise(),
            "convoke" => Self::convoke(),
            "affinity for artifacts" => Self::affinity_for_artifacts(),
            "delve" => Self::delve(),
            "changeling" => Self::changeling(),
            "this creature can't be blocked" | "can't be blocked" => Self::unblockable(),
            "plainswalk" => Self::landwalk(crate::types::Subtype::Plains),
            "islandwalk" => Self::landwalk(crate::types::Subtype::Island),
            "swampwalk" => Self::landwalk(crate::types::Subtype::Swamp),
            "mountainwalk" => Self::landwalk(crate::types::Subtype::Mountain),
            "forestwalk" => Self::landwalk(crate::types::Subtype::Forest),
            "snow plainswalk" => Self::snow_landwalk(crate::types::Subtype::Plains),
            "snow islandwalk" => Self::snow_landwalk(crate::types::Subtype::Island),
            "snow swampwalk" => Self::snow_landwalk(crate::types::Subtype::Swamp),
            "snow mountainwalk" => Self::snow_landwalk(crate::types::Subtype::Mountain),
            "snow forestwalk" => Self::snow_landwalk(crate::types::Subtype::Forest),
            "landwalk" => Self::any_landwalk(),
            "nonbasic landwalk" => Self::nonbasic_landwalk(),
            "artifact landwalk" => Self::artifact_landwalk(),
            "protection from white" => Self::protection(crate::ability::ProtectionFrom::Color(
                crate::color::ColorSet::WHITE,
            )),
            "protection from blue" => Self::protection(crate::ability::ProtectionFrom::Color(
                crate::color::ColorSet::BLUE,
            )),
            "protection from black" => Self::protection(crate::ability::ProtectionFrom::Color(
                crate::color::ColorSet::BLACK,
            )),
            "protection from red" => Self::protection(crate::ability::ProtectionFrom::Color(
                crate::color::ColorSet::RED,
            )),
            "protection from green" => Self::protection(crate::ability::ProtectionFrom::Color(
                crate::color::ColorSet::GREEN,
            )),
            "protection from all colors" => {
                Self::protection(crate::ability::ProtectionFrom::AllColors)
            }
            "protection from colorless" => {
                Self::protection(crate::ability::ProtectionFrom::Colorless)
            }
            "protection from everything" => {
                Self::protection(crate::ability::ProtectionFrom::Everything)
            }
            "protection from human" | "protection from humans" => {
                Self::protection(crate::ability::ProtectionFrom::Permanents(
                    crate::target::ObjectFilter::creature()
                        .with_subtype(crate::types::Subtype::Human),
                ))
            }
            _ => return None,
        })
    }

    pub fn restriction(
        restriction: crate::effect::Restriction,
        detail: impl std::fmt::Debug,
    ) -> Self {
        let display = format!("{detail:?}").trim_matches('"').to_string();
        Self {
            id: Some(StaticAbilityId::RuleRestriction),
            label: display.clone(),
            payload: StaticAbilityPayload::RuleRestriction {
                restriction,
                display,
            },
        }
    }

    pub fn protection(filter: crate::ability::ProtectionFrom) -> Self {
        Self {
            id: Some(StaticAbilityId::Protection),
            label: "protection".to_string(),
            payload: StaticAbilityPayload::Protection(filter),
        }
    }

    pub fn must_attack() -> Self {
        Self::identified(StaticAbilityId::MustAttack, "must attack")
    }

    pub fn must_block() -> Self {
        Self::identified(StaticAbilityId::MustBlock, "must block")
    }

    pub fn unblockable() -> Self {
        Self::identified(StaticAbilityId::Unblockable, "unblockable")
    }

    pub fn make_colorless(filter: crate::target::ObjectFilter) -> Self {
        Self {
            id: Some(StaticAbilityId::MakeColorless),
            label: "make colorless".to_string(),
            payload: StaticAbilityPayload::MakeColorless(filter),
        }
    }

    pub fn cant_block() -> Self {
        Self {
            id: Some(StaticAbilityId::CantBlock),
            label: "cant block".to_string(),
            payload: StaticAbilityPayload::None,
        }
    }

    pub fn grants(spec: crate::grant::GrantSpec) -> Self {
        Self {
            id: Some(StaticAbilityId::Grants),
            label: "grants".to_string(),
            payload: StaticAbilityPayload::Grants(Box::new(spec)),
        }
    }

    pub fn set_base_power_toughness(
        filter: crate::target::ObjectFilter,
        power: i32,
        toughness: i32,
    ) -> Self {
        Self {
            id: Some(StaticAbilityId::SetBasePowerToughnessForFilter),
            label: "set base power toughness".to_string(),
            payload: StaticAbilityPayload::SetBasePowerToughness {
                filter,
                power,
                toughness,
            },
        }
    }

    pub fn can_attack_as_though_no_defender() -> Self {
        Self::new("can attack as though no defender")
    }

    pub fn set_colors(_filter: impl std::fmt::Debug, _colors: crate::color::ColorSet) -> Self {
        Self::new("set colors")
    }

    pub fn add_card_types(
        filter: crate::target::ObjectFilter,
        card_types: Vec<crate::types::CardType>,
    ) -> Self {
        Self {
            id: Some(StaticAbilityId::AddCardTypes),
            label: "add card types".to_string(),
            payload: StaticAbilityPayload::AddCardTypes { filter, card_types },
        }
    }

    pub fn add_subtypes(
        filter: crate::target::ObjectFilter,
        subtypes: Vec<crate::types::Subtype>,
    ) -> Self {
        Self {
            id: Some(StaticAbilityId::AddSubtypes),
            label: "add subtypes".to_string(),
            payload: StaticAbilityPayload::AddSubtypes { filter, subtypes },
        }
    }

    pub fn changeling() -> Self {
        Self::identified(StaticAbilityId::Changeling, "changeling")
    }

    pub fn ward(amount: impl Into<crate::cost::TotalCost>) -> Self {
        Self {
            id: Some(StaticAbilityId::Ward),
            label: "ward".to_string(),
            payload: StaticAbilityPayload::Ward(amount.into()),
        }
    }

    pub fn can_block_additional_creature_each_combat(additional: usize) -> Self {
        Self {
            id: Some(StaticAbilityId::CanBlockAdditionalCreatureEachCombat),
            label: "can block additional creature".to_string(),
            payload: StaticAbilityPayload::CanBlockAdditionalCreatureEachCombat(additional),
        }
    }

    pub fn doesnt_untap() -> Self {
        Self {
            id: Some(StaticAbilityId::DoesntUntap),
            label: "doesnt untap".to_string(),
            payload: StaticAbilityPayload::None,
        }
    }

    pub fn cant_attack() -> Self {
        Self::identified(StaticAbilityId::CantAttack, "cant attack")
    }

    pub fn enters_tapped_unless_condition(
        condition: crate::ConditionExpr,
        display: impl Into<String>,
    ) -> Self {
        let display = display.into();
        Self {
            id: Some(StaticAbilityId::EntersTappedUnlessCondition),
            label: display.clone(),
            payload: StaticAbilityPayload::EntersTappedUnlessCondition { condition, display },
        }
    }

    pub fn haste() -> Self {
        Self::identified(StaticAbilityId::Haste, "haste")
    }

    pub fn flying() -> Self {
        Self::identified(StaticAbilityId::Flying, "flying")
    }

    pub fn trample() -> Self {
        Self::identified(StaticAbilityId::Trample, "trample")
    }

    pub fn menace() -> Self {
        Self::identified(StaticAbilityId::Menace, "menace")
    }

    pub fn hexproof() -> Self {
        Self::identified(StaticAbilityId::Hexproof, "hexproof")
    }

    pub fn improvise() -> Self {
        Self::identified(StaticAbilityId::Improvise, "improvise")
    }

    pub fn convoke() -> Self {
        Self::identified(StaticAbilityId::Convoke, "convoke")
    }

    pub fn affinity_for_artifacts() -> Self {
        Self::identified(
            StaticAbilityId::AffinityForArtifacts,
            "affinity for artifacts",
        )
    }

    pub fn delve() -> Self {
        Self::identified(StaticAbilityId::Delve, "delve")
    }

    pub fn first_strike() -> Self {
        Self::identified(StaticAbilityId::FirstStrike, "first strike")
    }

    pub fn double_strike() -> Self {
        Self::identified(StaticAbilityId::DoubleStrike, "double strike")
    }

    pub fn deathtouch() -> Self {
        Self::identified(StaticAbilityId::Deathtouch, "deathtouch")
    }

    pub fn lifelink() -> Self {
        Self::identified(StaticAbilityId::Lifelink, "lifelink")
    }

    pub fn vigilance() -> Self {
        Self::identified(StaticAbilityId::Vigilance, "vigilance")
    }

    pub fn reach() -> Self {
        Self::identified(StaticAbilityId::Reach, "reach")
    }

    pub fn defender() -> Self {
        Self::identified(StaticAbilityId::Defender, "defender")
    }

    pub fn phasing() -> Self {
        Self::identified(StaticAbilityId::Phasing, "phasing")
    }

    pub fn indestructible() -> Self {
        Self::identified(StaticAbilityId::Indestructible, "indestructible")
    }

    pub fn shroud() -> Self {
        Self::identified(StaticAbilityId::Shroud, "shroud")
    }

    pub fn wither() -> Self {
        Self::identified(StaticAbilityId::Wither, "wither")
    }

    pub fn infect() -> Self {
        Self::identified(StaticAbilityId::Infect, "infect")
    }

    pub fn cascade() -> Self {
        Self::identified(StaticAbilityId::Cascade, "cascade")
    }

    pub fn skulk() -> Self {
        Self::identified(StaticAbilityId::Skulk, "skulk")
    }

    pub fn prowess() -> Self {
        Self::identified(StaticAbilityId::Prowess, "prowess")
    }

    pub fn granted_inline_ability(&self) -> Option<&crate::ability::Ability> {
        None
    }

    pub fn toxic(_amount: u32) -> Self {
        Self::new("toxic")
    }

    pub fn unleash() -> Self {
        Self::identified(StaticAbilityId::Unleash, "unleash")
    }

    pub fn any_landwalk() -> Self {
        Self::new(LandwalkKind::AnyLand)
    }

    pub fn nonbasic_landwalk() -> Self {
        Self::new(LandwalkKind::NonbasicLand)
    }

    pub fn artifact_landwalk() -> Self {
        Self::new(LandwalkKind::ArtifactLand)
    }

    pub fn landwalk(kind: crate::types::Subtype) -> Self {
        Self::new(LandwalkKind::Subtype {
            subtype: kind,
            snow: false,
        })
    }

    pub fn snow_landwalk(subtype: crate::types::Subtype) -> Self {
        Self::new(LandwalkKind::Subtype {
            subtype,
            snow: true,
        })
    }

    pub fn hexproof_from(_filter: crate::target::ObjectFilter) -> Self {
        Self::new("hexproof from")
    }

    pub fn id(&self) -> StaticAbilityId {
        self.id.unwrap_or(StaticAbilityId::RuleFallbackText)
    }

    pub fn display(&self) -> String {
        self.label.clone()
    }

    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.label = text.into();
        self
    }

    pub fn with_condition(self, condition: crate::ConditionExpr) -> Self {
        match self.payload {
            StaticAbilityPayload::Conditional { ability, .. } => StaticAbility {
                id: ability.id,
                label: ability.label.clone(),
                payload: StaticAbilityPayload::Conditional { ability, condition },
            },
            payload => {
                let ability = StaticAbility {
                    id: self.id,
                    label: self.label,
                    payload,
                };
                StaticAbility {
                    id: ability.id,
                    label: ability.label.clone(),
                    payload: StaticAbilityPayload::Conditional {
                        ability: Box::new(ability),
                        condition,
                    },
                }
            }
        }
    }
    pub fn unwrap_or(self, _fallback: Self) -> Self {
        self
    }
    pub fn unwrap_or_else<F: FnOnce() -> Self>(self, _fallback: F) -> Self {
        self
    }

    pub fn unsupported_parser_line(text: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::new(format!("{} ({})", text.into(), reason.into()))
    }

    pub fn characteristic_defining_pt(
        power: impl Into<crate::effect::Value>,
        toughness: impl Into<crate::effect::Value>,
    ) -> Self {
        Self {
            id: Some(StaticAbilityId::CharacteristicDefiningPT),
            label: "characteristic defining pt".to_string(),
            payload: StaticAbilityPayload::CharacteristicDefiningPt {
                power: power.into(),
                toughness: toughness.into(),
            },
        }
    }

    pub fn cant_be_blocked_by_more_than(_count: impl std::fmt::Debug) -> Self {
        Self::new("cant be blocked by more than")
    }
    pub fn enters_tapped_ability() -> Self {
        Self::identified(StaticAbilityId::EntersTapped, "enters tapped")
    }
    pub fn remove_all_abilities(filter: crate::target::ObjectFilter) -> Self {
        Self {
            id: Some(StaticAbilityId::RemoveAllAbilitiesForFilter),
            label: "remove all abilities".to_string(),
            payload: StaticAbilityPayload::RemoveAllAbilities(filter),
        }
    }
    pub fn can_block_only_flying() -> Self {
        Self::identified(StaticAbilityId::CanBlockOnlyFlying, "can block only flying")
    }
    pub fn partner() -> Self {
        Self::identified(StaticAbilityId::Partner, "partner")
    }
    pub fn assist() -> Self {
        Self::identified(StaticAbilityId::Assist, "assist")
    }
    pub fn split_second() -> Self {
        Self::identified(StaticAbilityId::SplitSecond, "split second")
    }
    pub fn rebound() -> Self {
        Self::identified(StaticAbilityId::Rebound, "rebound")
    }
    pub fn fear() -> Self {
        Self::identified(StaticAbilityId::Fear, "fear")
    }
    pub fn intimidate() -> Self {
        Self::identified(StaticAbilityId::Intimidate, "intimidate")
    }
    pub fn shadow() -> Self {
        Self::identified(StaticAbilityId::Shadow, "shadow")
    }
    pub fn horsemanship() -> Self {
        Self::identified(StaticAbilityId::Horsemanship, "horsemanship")
    }
    pub fn flanking() -> Self {
        Self::identified(StaticAbilityId::Flanking, "flanking")
    }
    pub fn umbra_armor() -> Self {
        Self::identified(StaticAbilityId::UmbraArmor, "umbra armor")
    }
    pub fn bloodthirst(amount: u32) -> Self {
        Self {
            id: Some(StaticAbilityId::Bloodthirst),
            label: format!("bloodthirst {amount}"),
            payload: StaticAbilityPayload::Bloodthirst(amount),
        }
    }
    pub fn krrik_black_mana_may_be_paid_with_life() -> Self {
        Self {
            id: Some(StaticAbilityId::BlackManaMayBePaidWithLife),
            label: "krrik".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn minimum_spell_total_mana(amount: u32) -> Self {
        Self {
            id: Some(StaticAbilityId::MinimumSpellTotalMana),
            label: "minimum spell total mana".into(),
            payload: StaticAbilityPayload::MinimumSpellTotalMana(amount),
        }
    }
    pub fn cant_pay_life_or_sacrifice_nonland_for_cast_or_activate() -> Self {
        Self {
            id: Some(StaticAbilityId::CantPayLifeOrSacrificeNonlandForCastOrActivate),
            label: "cant pay life or sac nonland".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn cant_attack_unless_condition(
        _spec: CantAttackUnlessConditionSpec,
        _display: impl Into<String>,
    ) -> Self {
        Self::new("cant attack unless condition")
    }
    pub fn cant_attack_its_owner() -> Self {
        Self {
            id: Some(StaticAbilityId::CantAttackItsOwner),
            label: "cant attack its owner".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn reduce_activated_ability_costs(
        filter: crate::target::ObjectFilter,
        reduction: u32,
        minimum_total_mana: Option<u32>,
    ) -> Self {
        Self {
            id: Some(StaticAbilityId::ActivatedAbilityCostReduction),
            label: "reduce activated ability costs".into(),
            payload: StaticAbilityPayload::ActivatedAbilityCostReduction {
                filter,
                reduction,
                condition: None,
                per_matching_objects: None,
                minimum_total_mana,
            },
        }
    }
    pub fn reduce_activated_ability_costs_if_targets(
        filter: crate::target::ObjectFilter,
        reduction: u32,
        condition: ActivatedAbilityCostCondition,
        minimum_total_mana: Option<u32>,
    ) -> Self {
        Self {
            id: Some(StaticAbilityId::ActivatedAbilityCostReduction),
            label: "reduce activated ability costs if targets".into(),
            payload: StaticAbilityPayload::ActivatedAbilityCostReduction {
                filter,
                reduction,
                condition: Some(condition),
                per_matching_objects: None,
                minimum_total_mana,
            },
        }
    }
    pub fn reduce_activated_ability_costs_for_each(
        filter: crate::target::ObjectFilter,
        reduction: u32,
        per_filter: crate::target::ObjectFilter,
        minimum_total_mana: Option<u32>,
    ) -> Self {
        Self {
            id: Some(StaticAbilityId::ActivatedAbilityCostReduction),
            label: "reduce activated ability costs for each".into(),
            payload: StaticAbilityPayload::ActivatedAbilityCostReduction {
                filter,
                reduction,
                condition: None,
                per_matching_objects: Some(per_filter),
                minimum_total_mana,
            },
        }
    }
    pub fn remove_all_abilities_except_mana(filter: crate::target::ObjectFilter) -> Self {
        Self {
            id: Some(StaticAbilityId::RemoveAllAbilitiesExceptManaForFilter),
            label: "remove all abilities except mana".to_string(),
            payload: StaticAbilityPayload::RemoveAllAbilitiesExceptMana(filter),
        }
    }
    pub fn set_card_types(
        filter: crate::target::ObjectFilter,
        card_types: Vec<crate::types::CardType>,
    ) -> Self {
        Self {
            id: Some(StaticAbilityId::SetCardTypes),
            label: "set card types".to_string(),
            payload: StaticAbilityPayload::SetCardTypes { filter, card_types },
        }
    }
    pub fn set_creature_subtypes(
        filter: crate::target::ObjectFilter,
        subtypes: Vec<crate::types::Subtype>,
    ) -> Self {
        Self {
            id: Some(StaticAbilityId::SetCreatureSubtypes),
            label: "set creature subtypes".to_string(),
            payload: StaticAbilityPayload::SetCreatureSubtypes { filter, subtypes },
        }
    }
    pub fn add_all_subtypes_of_family(
        filter: crate::target::ObjectFilter,
        family: crate::types::SubtypeFamily,
    ) -> Self {
        Self {
            id: Some(StaticAbilityId::AddAllSubtypesOfFamily),
            label: "add all subtypes of family".to_string(),
            payload: StaticAbilityPayload::AddAllSubtypesOfFamily { filter, family },
        }
    }
    pub fn prevent_constrained_damage_to_self_put_counters_instead(
        _counter_type: crate::object::CounterType,
        _display: impl Into<String>,
        _source_filter: Option<crate::target::ObjectFilter>,
        _combat_only: Option<bool>,
    ) -> Self {
        Self::new("prevent constrained damage to self put counters instead")
    }
    pub fn untap_during_each_other_players_untap_step(
        _filter: crate::target::ObjectFilter,
        _display: impl Into<String>,
    ) -> Self {
        Self::new("untap during each other players untap step")
    }
    pub fn pregame_action(kind: PregameActionKind, display: impl Into<String>) -> Self {
        let text = display.into();
        Self {
            id: Some(StaticAbilityId::PregameAction),
            label: text.clone(),
            payload: StaticAbilityPayload::PregameAction { kind, text },
        }
    }
    pub fn reduce_maximum_hand_size(_player: crate::target::PlayerFilter, _by: u32) -> Self {
        Self::new("reduce maximum hand size")
    }
    pub fn equipment_grant(abilities: Vec<StaticAbility>) -> Self {
        Self {
            id: Some(StaticAbilityId::EquipmentGrant),
            label: "equipment grant".to_string(),
            payload: StaticAbilityPayload::EquipmentGrant(abilities),
        }
    }
    pub fn soulbond_shared_object_ability(_ability: crate::ability::Ability) -> Self {
        Self::new("soulbond shared object ability")
    }
    pub fn grant_object_ability_for_filter(
        filter: crate::target::ObjectFilter,
        ability: crate::ability::Ability,
        display: impl Into<String>,
    ) -> Self {
        let display = display.into();
        Self::new(GrantObjectAbilityForFilter::new(filter, ability, display))
    }
    pub fn boast_twice_each_turn() -> Self {
        Self::new("boast twice each turn")
    }
    pub fn first_equip_cost_alternative(_display: impl Into<String>) -> Self {
        Self::new("first equip cost alternative")
    }
    pub fn vote_additional_time_while_voting() -> Self {
        Self::new("vote additional time while voting")
    }
    pub fn vote_additional_vote_while_voting() -> Self {
        Self::new("vote additional vote while voting")
    }
    pub fn exert_attack(
        only_if_not_exerted_this_turn: bool,
        linked_trigger: Option<crate::ability::TriggeredAbility>,
        display: impl Into<String>,
    ) -> Self {
        let display = display.into();
        Self {
            id: Some(StaticAbilityId::ExertAttack),
            label: display.clone(),
            payload: StaticAbilityPayload::ExertAttack {
                only_if_not_exerted_this_turn,
                linked_trigger,
                display,
            },
        }
    }
    pub fn cant_attack_you_unless_controller_pays_per_attacker(cost: u32) -> Self {
        Self {
            id: Some(StaticAbilityId::CantAttackYouUnlessControllerPaysPerAttacker),
            label: "cant attack you unless pays per attacker".to_string(),
            payload: StaticAbilityPayload::CantAttackYouUnlessControllerPaysPerAttacker(cost),
        }
    }
    pub fn cant_attack_you_unless_controller_pays_per_attacker_basic_land_types_among_lands_you_control()
    -> Self {
        Self {
            id: Some(
                StaticAbilityId::CantAttackYouUnlessControllerPaysPerAttackerBasicLandTypesAmongLandsYouControl,
            ),
            label: "cant attack you unless pays per attacker basic land types".to_string(),
            payload:
                StaticAbilityPayload::CantAttackYouUnlessControllerPaysPerAttackerBasicLandTypesAmongLandsYouControl,
        }
    }
    pub fn cant_be_blocked_by_power_or_less(_power: i32) -> Self {
        Self::new("cant be blocked by power or less")
    }
    pub fn cant_be_blocked_by_power_or_greater(_power: i32) -> Self {
        Self::new("cant be blocked by power or greater")
    }
    pub fn cant_attack_unless_controller_cast_creature_spell_this_turn() -> Self {
        Self::new("cant attack unless controller cast creature spell")
    }
    pub fn cant_attack_unless_controller_cast_noncreature_spell_this_turn() -> Self {
        Self::new("cant attack unless controller cast noncreature spell")
    }
    pub fn players_cant_gain_life() -> Self {
        Self::new("players cant gain life")
    }
    pub fn players_cant_search() -> Self {
        Self::new("players cant search")
    }
    pub fn damage_cant_be_prevented() -> Self {
        Self::new("damage cant be prevented")
    }
    pub fn you_cant_lose_game() -> Self {
        Self::new("you cant lose game")
    }
    pub fn opponents_cant_win_game() -> Self {
        Self::new("opponents cant win game")
    }
    pub fn your_life_total_cant_change() -> Self {
        Self::new("your life total cant change")
    }
    pub fn opponents_cant_cast_spells() -> Self {
        Self::new("opponents cant cast spells")
    }
    pub fn opponents_cant_draw_extra_cards() -> Self {
        Self::new("opponents cant draw extra cards")
    }
    pub fn cant_have_counters_placed() -> Self {
        Self::new("cant have counters placed")
    }
    pub fn cant_be_countered_ability() -> Self {
        Self::identified(
            StaticAbilityId::CantBeCountered,
            "cant be countered ability",
        )
    }
    pub fn permanents_you_control_cant_be_sacrificed() -> Self {
        Self::new("permanents you control cant be sacrificed")
    }
    pub fn cant_be_blocked_as_long_as_defending_player_controls_card_type(
        _card_type: crate::types::CardType,
    ) -> Self {
        Self::new("cant be blocked as long as defending controls card type")
    }
    pub fn cant_be_blocked_as_long_as_defending_player_controls_card_types(
        _card_types: Vec<crate::types::CardType>,
    ) -> Self {
        Self::new("cant be blocked as long as defending controls card types")
    }
    pub fn set_name(_filter: impl std::fmt::Debug, _name: impl Into<String>) -> Self {
        Self::new("set name")
    }
    pub fn soulbond_shared_power_toughness(_power: i32, _toughness: i32) -> Self {
        Self::new("soulbond shared power toughness")
    }
    pub fn soulbond_shared_ability(_ability: StaticAbility) -> Self {
        Self::new("soulbond shared ability")
    }
    pub fn add_colors(_filter: impl std::fmt::Debug, _colors: crate::color::ColorSet) -> Self {
        Self::new("add colors")
    }
    pub fn control_attached_permanent(_display: impl Into<String>) -> Self {
        Self::new("control attached permanent")
    }
    pub fn prevent_damage_to_self_remove_counter(
        _counter_type: crate::object::CounterType,
        _amount: u32,
    ) -> Self {
        Self::new("prevent damage to self remove counter")
    }
    pub fn prevent_damage_to_self_put_counters_instead(
        _counter_type: crate::object::CounterType,
        _display: impl Into<String>,
    ) -> Self {
        Self::new("prevent damage to self put counters instead")
    }
    pub fn add_supertypes(
        _filter: impl std::fmt::Debug,
        _types: Vec<crate::types::Supertype>,
    ) -> Self {
        Self::new("add supertypes")
    }
    pub fn reveal_first_card_you_draw_each_turn(optional: bool, your_turns_only: bool) -> Self {
        Self {
            id: Some(StaticAbilityId::RevealFirstCardYouDrawEachTurn),
            label: "reveal first card".into(),
            payload: StaticAbilityPayload::RevealFirstCardYouDrawEachTurn {
                optional,
                your_turns_only,
            },
        }
    }
    pub fn increase_activated_ability_costs(
        filter: crate::target::ObjectFilter,
        increase: crate::cost::TotalCost,
    ) -> Self {
        Self {
            id: Some(StaticAbilityId::ActivatedAbilityCostIncrease),
            label: "increase activated ability costs".to_string(),
            payload: StaticAbilityPayload::ActivatedAbilityCostIncrease { filter, increase },
        }
    }
    pub fn cant_be_blocked_by_lower_power_than_source() -> Self {
        Self {
            id: Some(StaticAbilityId::CantBeBlockedByLowerPowerThanSource),
            label: "cant be blocked by lower power than source".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn doctors_companion() -> Self {
        Self {
            id: Some(StaticAbilityId::DoctorsCompanion),
            label: "doctors companion".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn conditional_spell_keyword(_spec: ConditionalSpellKeywordSpec) -> Self {
        Self {
            id: Some(StaticAbilityId::ConditionalSpellKeyword),
            label: "conditional spell keyword".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn damage_not_removed_during_cleanup() -> Self {
        Self::new("damage not removed during cleanup")
    }
    pub fn choose_basic_land_type_as_enters(_display: impl Into<String>) -> Self {
        Self::new("choose basic land type as enters")
    }
    pub fn choose_land_type_as_enters(_display: impl Into<String>) -> Self {
        Self::new("choose land type as enters")
    }
    pub fn enchanted_land_is_chosen_type(_display: impl Into<String>) -> Self {
        Self {
            id: Some(StaticAbilityId::EnchantedLandIsChosenType),
            label: "enchanted land is chosen type".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn add_chosen_creature_type(
        _filter: crate::target::ObjectFilter,
        _display: impl Into<String>,
    ) -> Self {
        Self {
            id: Some(StaticAbilityId::AddChosenCreatureType),
            label: "add chosen creature type".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn set_chosen_color(
        _filter: crate::target::ObjectFilter,
        _display: impl Into<String>,
    ) -> Self {
        Self::new("set chosen color")
    }
    pub fn choose_creature_type_as_enters(display: impl Into<String>) -> Self {
        let display = display.into();
        Self {
            id: Some(StaticAbilityId::ChooseCreatureTypeAsEnters),
            label: display.clone(),
            payload: StaticAbilityPayload::ChooseCreatureTypeAsEnters(display),
        }
    }
    pub fn choose_named_option_as_enters(
        _options: Vec<String>,
        _display: impl Into<String>,
    ) -> Self {
        Self::new("choose named option as enters")
    }
    pub fn duplicate_matching_triggered_abilities(
        _source_filter: Option<crate::target::ObjectFilter>,
        _event_matcher: Option<crate::triggers::Trigger>,
        _count: u32,
        _display: impl Into<String>,
    ) -> Self {
        Self::new("duplicate matching triggered abilities")
    }
    pub fn suppress_matching_triggered_abilities(
        _source_filter: Option<crate::target::ObjectFilter>,
        _event_matcher: Option<crate::triggers::Trigger>,
        _display: impl Into<String>,
    ) -> Self {
        Self {
            id: Some(StaticAbilityId::SuppressMatchingTriggeredAbilities),
            label: "suppress matching triggered abilities".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn double_damage_from_sources_you_control_of_chosen_type(
        display: impl Into<String>,
    ) -> Self {
        let display = display.into();
        Self {
            id: Some(StaticAbilityId::DoubleDamageFromSourcesYouControlOfChosenType),
            label: display.clone(),
            payload: StaticAbilityPayload::DoubleDamageFromSourcesYouControlOfChosenType(display),
        }
    }
    pub fn with_enter_as_copy_as_enters(
        spec: EnterAsCopyAsEntersSpec,
        display: impl Into<String>,
    ) -> Self {
        let display = display.into();
        Self {
            id: Some(StaticAbilityId::EnterAsCopyAsEnters),
            label: display.clone(),
            payload: StaticAbilityPayload::EnterAsCopyAsEnters { spec, display },
        }
    }
    pub fn choose_color_as_enters(
        _excluded: Option<crate::color::Color>,
        _display: impl Into<String>,
    ) -> Self {
        Self {
            id: Some(StaticAbilityId::ChooseColorAsEnters),
            label: "choose color as enters".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn choose_player_as_enters(display: impl Into<String>) -> Self {
        let display = display.into();
        Self {
            id: Some(StaticAbilityId::ChoosePlayerAsEnters),
            label: display.clone(),
            payload: StaticAbilityPayload::ChoosePlayerAsEnters(display),
        }
    }
    pub fn redirect_damage_from_you_and_other_permanents_to_source() -> Self {
        Self::new("redirect damage from you and other permanents to source")
    }
    pub fn max_attackers_each_combat(_n: impl std::fmt::Debug) -> Self {
        Self::new("max attackers each combat")
    }
    pub fn max_blockers_each_combat(_n: impl std::fmt::Debug) -> Self {
        Self::new("max blockers each combat")
    }
    pub fn shuffle_into_library_from_graveyard() -> Self {
        Self {
            id: Some(StaticAbilityId::ShuffleIntoLibraryFromGraveyard),
            label: "shuffle into library from graveyard".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn permanents_enter_tapped() -> Self {
        Self {
            id: Some(StaticAbilityId::AllPermanentsEnterTapped),
            label: "permanents enter tapped".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn creatures_entering_dont_cause_abilities_to_trigger() -> Self {
        Self {
            id: Some(StaticAbilityId::CreaturesEnteringDontCauseAbilitiesToTrigger),
            label: "creatures entering dont cause abilities to trigger".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn creatures_assign_combat_damage_using_toughness() -> Self {
        Self::new("creatures assign combat damage using toughness")
    }
    pub fn creatures_you_control_assign_combat_damage_using_toughness() -> Self {
        Self {
            id: Some(StaticAbilityId::CreaturesYouControlAssignCombatDamageUsingToughness),
            label: "creatures you control assign damage using toughness".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn players_cant_cycle() -> Self {
        Self {
            id: Some(StaticAbilityId::PlayersCantCycle),
            label: "players cant cycle".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn starting_life_bonus(_amount: i32) -> Self {
        Self::new("starting life bonus")
    }
    pub fn buyback_cost_reduction(_amount: impl Into<crate::effect::Value>) -> Self {
        Self::new("buyback cost reduction")
    }
    pub fn cost_increase_per_target_beyond_first(cost: u32) -> Self {
        Self {
            id: Some(StaticAbilityId::CostIncreasePerAdditionalTarget),
            label: "cost increase per target beyond first".to_string(),
            payload: StaticAbilityPayload::CostIncreasePerTargetBeyondFirst(cost),
        }
    }
    pub fn players_skip_upkeep() -> Self {
        Self {
            id: Some(StaticAbilityId::PlayersSkipUpkeep),
            label: "players skip upkeep".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn legend_rule_doesnt_apply() -> Self {
        Self::new("legend rule doesnt apply")
    }
    pub fn blood_moon() -> Self {
        Self {
            id: Some(StaticAbilityId::BloodMoon),
            label: "blood moon".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn remove_supertypes(
        _filter: crate::target::ObjectFilter,
        _types: Vec<crate::types::Supertype>,
    ) -> Self {
        Self {
            id: Some(StaticAbilityId::RemoveSupertypes),
            label: "remove supertypes".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn prevent_all_damage_dealt_to_creatures() -> Self {
        Self {
            id: Some(StaticAbilityId::PreventAllDamageDealtToCreatures),
            label: "prevent all damage dealt to creatures".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn prevent_damage_to_other_creature_you_control_put_counters_instead(
        _counter_type: crate::object::CounterType,
        _display: impl Into<String>,
    ) -> Self {
        Self {
            id: Some(StaticAbilityId::PreventDamageToOtherCreatureYouControlPutCountersInstead),
            label: "prevent damage to other creature you control put counters instead".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn prevent_all_combat_damage_to_self() -> Self {
        Self {
            id: Some(StaticAbilityId::PreventAllCombatDamageToSelf),
            label: "prevent all combat damage to self".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn prevent_all_damage_to_self_by_creatures() -> Self {
        Self {
            id: Some(StaticAbilityId::PreventAllDamageToSelfByCreatures),
            label: "prevent all damage to self by creatures".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn may_choose_not_to_untap_during_untap_step(_subject: impl std::fmt::Debug) -> Self {
        Self::new("may choose not to untap during untap step")
    }
    pub fn flying_only_restriction() -> Self {
        Self {
            id: Some(StaticAbilityId::FlyingOnlyRestriction),
            label: "flying only restriction".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn flying_restriction() -> Self {
        Self {
            id: Some(StaticAbilityId::FlyingRestriction),
            label: "flying restriction".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn may_assign_damage_as_unblocked() -> Self {
        Self {
            id: Some(StaticAbilityId::MayAssignDamageAsUnblocked),
            label: "may assign damage as unblocked".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn remove_ability(_filter: crate::target::ObjectFilter, _ability: StaticAbility) -> Self {
        Self::new("remove ability")
    }
    pub fn rule_text_placeholder(_text: impl Into<String>) -> Self {
        Self {
            id: Some(StaticAbilityId::RuleFallbackText),
            label: "rule text placeholder".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn additional_land_plays(count: u32) -> Self {
        Self {
            id: Some(StaticAbilityId::RuleRestriction),
            label: "additional land plays".to_string(),
            payload: StaticAbilityPayload::AdditionalLandPlays(count),
        }
    }
    pub fn no_maximum_hand_size() -> Self {
        Self {
            id: Some(StaticAbilityId::NoMaximumHandSize),
            label: "no maximum hand size".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn can_be_commander() -> Self {
        Self {
            id: Some(StaticAbilityId::CanBeCommander),
            label: "can be commander".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn max_hand_size_seven_minus_your_graveyard_card_types(
        _player: crate::target::PlayerFilter,
        _min_card_types: u32,
    ) -> Self {
        Self::new("max hand size seven minus your graveyard card types")
    }
    pub fn library_of_leng_discard_replacement() -> Self {
        Self {
            id: Some(StaticAbilityId::LibraryOfLengDiscardReplacement),
            label: "library of leng discard replacement".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn draw_replacement_exile_top_face_down() -> Self {
        Self {
            id: Some(StaticAbilityId::DrawReplacementExileTopFaceDown),
            label: "draw replacement exile top face down".into(),
            payload: StaticAbilityPayload::None,
        }
    }
    pub fn exile_to_countered_exile_instead_of_graveyard(
        player: crate::target::PlayerFilter,
        counter_type: crate::object::CounterType,
    ) -> Self {
        Self {
            id: Some(StaticAbilityId::ExileToCounteredExileInsteadOfGraveyard),
            label: "exile to countered exile instead of graveyard".into(),
            payload: StaticAbilityPayload::ExileToCounteredExileInsteadOfGraveyard {
                player,
                counter_type,
            },
        }
    }
    pub fn discard_or_redirect_replacement(
        filter: crate::target::ObjectFilter,
        redirect_zone: crate::zone::Zone,
    ) -> Self {
        Self {
            id: Some(StaticAbilityId::DiscardOrRedirectReplacement),
            label: "discard or redirect replacement".into(),
            payload: StaticAbilityPayload::DiscardOrRedirectReplacement {
                filter,
                redirect_zone,
            },
        }
    }
    pub fn pay_life_or_enter_tapped(value: u32) -> Self {
        Self {
            id: Some(StaticAbilityId::PayLifeOrEnterTappedReplacement),
            label: "pay life or enter tapped".to_string(),
            payload: StaticAbilityPayload::PayLifeOrEnterTapped(value),
        }
    }
    pub fn copy_activated_abilities(copy: CopyActivatedAbilities) -> Self {
        Self {
            id: Some(StaticAbilityId::CopyActivatedAbilities),
            label: "copy activated abilities".to_string(),
            payload: StaticAbilityPayload::CopyActivatedAbilities(copy),
        }
    }
    pub fn mana_spend_permission(perm: ManaSpendPermission, display: impl Into<String>) -> Self {
        let display = display.into();
        Self {
            id: Some(StaticAbilityId::ManaSpendPermission),
            label: display.clone(),
            payload: StaticAbilityPayload::ManaSpendPermission {
                permission: perm,
                display,
            },
        }
    }
    pub fn enters_with_counters_if_condition(
        counter: crate::object::CounterType,
        count: crate::effect::Value,
        condition: crate::ConditionExpr,
        display: impl Into<String>,
    ) -> Self {
        let display = display.into();
        Self {
            id: Some(StaticAbilityId::EnterWithCountersIfCondition),
            label: display.clone(),
            payload: StaticAbilityPayload::EntersWithCountersIfCondition {
                counter,
                count,
                condition,
                display,
            },
        }
    }
    pub fn enters_with_counters_value(
        counter: crate::object::CounterType,
        count: crate::effect::Value,
    ) -> Self {
        Self {
            id: Some(StaticAbilityId::EnterWithCounters),
            label: "enters with counters value".to_string(),
            payload: StaticAbilityPayload::EntersWithCountersValue { counter, count },
        }
    }
    pub fn enters_tapped_for_filter(filter: crate::target::ObjectFilter) -> Self {
        Self {
            id: Some(StaticAbilityId::EnterTappedForFilter),
            label: "enters tapped for filter".to_string(),
            payload: StaticAbilityPayload::EntersTappedForFilter(filter),
        }
    }
    pub fn enters_untapped_for_filter(filter: crate::target::ObjectFilter) -> Self {
        Self {
            id: Some(StaticAbilityId::EnterUntappedForFilter),
            label: "enters untapped for filter".to_string(),
            payload: StaticAbilityPayload::EntersUntappedForFilter(filter),
        }
    }
    pub fn enters_tapped_unless_control_two_or_more_other_lands() -> Self {
        Self::identified(
            StaticAbilityId::EntersTappedUnlessControlTwoOrMoreOtherLands,
            "enters tapped unless control two or more other lands",
        )
    }
    pub fn enters_tapped_unless_control_two_or_fewer_other_lands() -> Self {
        Self::identified(
            StaticAbilityId::EntersTappedUnlessControlTwoOrFewerOtherLands,
            "enters tapped unless control two or fewer other lands",
        )
    }
    pub fn enters_tapped_unless_control_two_or_more_basic_lands() -> Self {
        Self::identified(
            StaticAbilityId::EntersTappedUnlessControlTwoOrMoreBasicLands,
            "enters tapped unless control two or more basic lands",
        )
    }
    pub fn enters_tapped_unless_a_player_has_13_or_less_life() -> Self {
        Self::identified(
            StaticAbilityId::EntersTappedUnlessAPlayerHas13OrLessLife,
            "enters tapped unless a player has 13 or less life",
        )
    }
    pub fn enters_tapped_unless_two_or_more_opponents() -> Self {
        Self::identified(
            StaticAbilityId::EntersTappedUnlessTwoOrMoreOpponents,
            "enters tapped unless two or more opponents",
        )
    }
    pub fn enters_with_counters_and_subtypes_for_filter(
        filter: crate::target::ObjectFilter,
        counter: crate::object::CounterType,
        count: crate::effect::Value,
        subtypes: Vec<crate::types::Subtype>,
    ) -> Self {
        Self {
            id: Some(StaticAbilityId::EnterWithCountersForFilter),
            label: "enters with counters and subtypes for filter".to_string(),
            payload: StaticAbilityPayload::EntersWithCountersAndSubtypesForFilter {
                filter,
                counter,
                count,
                subtypes,
            },
        }
    }
}

pub type ThisSpellCastCondition = ironsmith_core::ThisSpellCostCondition;
pub type ThisSpellCostCondition = ironsmith_core::ThisSpellCostCondition;

#[derive(Debug, Clone, PartialEq)]
pub struct ThisSpellCastRestrictionKind {
    pub label: String,
}

impl ThisSpellCastRestrictionKind {
    fn named(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
        }
    }

    pub fn during_declare_attackers_step() -> Self {
        Self::named("during declare attackers step")
    }

    pub fn during_declare_attackers_step_if_you_were_attacked_this_step() -> Self {
        Self::named("during declare attackers step if you were attacked")
    }

    pub fn during_combat() -> Self {
        Self::named("during combat")
    }

    pub fn during_combat_before_blockers_are_declared() -> Self {
        Self::named("during combat before blockers")
    }

    pub fn during_combat_after_blockers_are_declared() -> Self {
        Self::named("during combat after blockers")
    }

    pub fn during_combat_on_your_turn_before_blockers_are_declared() -> Self {
        Self::named("during combat on your turn before blockers")
    }

    pub fn during_combat_on_opponents_turn() -> Self {
        Self::named("during combat on opponents turn")
    }

    pub fn before_attackers_are_declared() -> Self {
        Self::named("before attackers are declared")
    }

    pub fn before_combat_damage_step() -> Self {
        Self::named("before combat damage step")
    }

    pub fn during_opponents_upkeep() -> Self {
        Self::named("during opponents upkeep")
    }

    pub fn during_opponents_turn_after_upkeep() -> Self {
        Self::named("during opponents turn after upkeep")
    }

    pub fn during_your_end_step() -> Self {
        Self::named("during your end step")
    }

    pub fn if_you_cast_another_spell_this_turn() -> Self {
        Self::named("if you cast another spell this turn")
    }

    pub fn if_you_cast_another_green_spell_this_turn() -> Self {
        Self::named("if you cast another green spell this turn")
    }

    pub fn if_opponent_cast_creature_spell_this_turn() -> Self {
        Self::named("if opponent cast creature spell this turn")
    }

    pub fn if_creature_is_attacking_you() -> Self {
        Self::named("if creature is attacking you")
    }

    pub fn after_combat() -> Self {
        Self::named("after combat")
    }

    pub fn if_no_permanents_named_on_battlefield(name: &'static str) -> Self {
        Self::named(format!("if no permanents named {name}"))
    }

    pub fn if_you_control_snow_land() -> Self {
        Self::named("if you control snow land")
    }

    pub fn if_you_control_fewer_creatures_than_each_opponent() -> Self {
        Self::named("if you control fewer creatures than each opponent")
    }

    pub fn if_you_control_subtype_or_more(subtype: crate::types::Subtype, count: u32) -> Self {
        Self::named(format!("if you control {count}+ {subtype}"))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LandwalkKind {
    Subtype {
        subtype: crate::types::Subtype,
        snow: bool,
    },
    AnyLand,
    NonbasicLand,
    ArtifactLand,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Anthem {
    pub filter: Option<crate::target::ObjectFilter>,
    pub power: AnthemValue,
    pub toughness: AnthemValue,
    pub condition: Option<crate::ConditionExpr>,
}

impl Anthem {
    pub fn new(filter: crate::target::ObjectFilter, power: i32, toughness: i32) -> Self {
        Self {
            filter: Some(filter),
            power: AnthemValue::Fixed(power),
            toughness: AnthemValue::Fixed(toughness),
            condition: None,
        }
    }
    pub fn for_source(power: i32, toughness: i32) -> Self {
        Self {
            filter: None,
            power: AnthemValue::Fixed(power),
            toughness: AnthemValue::Fixed(toughness),
            condition: None,
        }
    }
    pub fn with_values(mut self, power: AnthemValue, toughness: AnthemValue) -> Self {
        self.power = power;
        self.toughness = toughness;
        self
    }
    pub fn with_condition(mut self, condition: crate::ConditionExpr) -> Self {
        self.condition = Some(condition);
        self
    }
}

pub use ironsmith_core::{AnthemCountExpression, AnthemValue};

#[derive(Debug, Clone, PartialEq)]
pub struct AttachedAbilityGrant {
    pub ability: crate::ability::Ability,
    pub display: String,
    pub condition: Option<crate::ConditionExpr>,
}

impl AttachedAbilityGrant {
    pub fn new(ability: crate::ability::Ability, display: impl Into<String>) -> Self {
        Self {
            ability,
            display: display.into(),
            condition: None,
        }
    }
    pub fn with_condition(mut self, condition: crate::ConditionExpr) -> Self {
        self.condition = Some(condition);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AttachedChosenLandwalkGrant {
    pub display: String,
    pub snow: bool,
}

impl AttachedChosenLandwalkGrant {
    pub fn new(display: impl Into<String>, snow: bool) -> Self {
        Self {
            display: display.into(),
            snow,
        }
    }
    pub fn with_condition(self, _condition: crate::ConditionExpr) -> Self {
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GrantAbility {
    pub filter: crate::target::ObjectFilter,
    pub ability: crate::ability::Ability,
    pub condition: Option<crate::ConditionExpr>,
}

impl GrantAbility {
    pub fn new(filter: crate::target::ObjectFilter, ability: crate::ability::Ability) -> Self {
        Self {
            filter,
            ability,
            condition: None,
        }
    }
    pub fn source(ability: impl Into<crate::ability::Ability>) -> Self {
        Self {
            filter: crate::target::ObjectFilter::source(),
            ability: ability.into(),
            condition: None,
        }
    }
    pub fn with_condition(mut self, condition: crate::ConditionExpr) -> Self {
        self.condition = Some(condition);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GrantObjectAbilityForFilter {
    pub filter: crate::target::ObjectFilter,
    pub ability: crate::ability::Ability,
    pub display: String,
    pub condition: Option<crate::ConditionExpr>,
}

impl GrantObjectAbilityForFilter {
    pub fn new(
        filter: crate::target::ObjectFilter,
        ability: crate::ability::Ability,
        display: impl Into<String>,
    ) -> Self {
        Self {
            filter,
            ability,
            display: display.into(),
            condition: None,
        }
    }
    pub fn with_condition(mut self, condition: crate::ConditionExpr) -> Self {
        self.condition = Some(condition);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CopyActivatedAbilities {
    pub filter: crate::target::ObjectFilter,
    pub exclude_source_name: bool,
}

impl CopyActivatedAbilities {
    pub fn new(filter: crate::target::ObjectFilter) -> Self {
        Self {
            filter,
            exclude_source_name: false,
        }
    }
    pub fn with_exclude_source_name(mut self, exclude: bool) -> Self {
        self.exclude_source_name = exclude;
        self
    }
    pub fn with_exclude_source_id(self, _exclude: bool) -> Self {
        self
    }
    pub fn with_display(self, _display: impl Into<String>) -> Self {
        self
    }
    pub fn with_counter(self, _counter: crate::object::CounterType) -> Self {
        self
    }
    pub fn with_condition(self, _condition: crate::ConditionExpr) -> Self {
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CostReduction {
    pub filter: crate::target::ObjectFilter,
    pub amount: crate::effect::Value,
}

impl CostReduction {
    pub fn new(filter: crate::target::ObjectFilter, amount: crate::effect::Value) -> Self {
        Self { filter, amount }
    }
    pub fn with_condition(self, _condition: crate::ConditionExpr) -> Self {
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CostReductionManaCost {
    pub filter: crate::target::ObjectFilter,
    pub cost: crate::mana::ManaCost,
}

impl CostReductionManaCost {
    pub fn new(filter: crate::target::ObjectFilter, cost: crate::mana::ManaCost) -> Self {
        Self { filter, cost }
    }
    pub fn with_condition(self, _condition: crate::ConditionExpr) -> Self {
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CostIncrease {
    pub filter: crate::target::ObjectFilter,
    pub amount: crate::effect::Value,
}

impl CostIncrease {
    pub fn new(filter: crate::target::ObjectFilter, amount: crate::effect::Value) -> Self {
        Self { filter, amount }
    }
    pub fn with_condition(self, _condition: crate::ConditionExpr) -> Self {
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CostIncreaseManaCost {
    pub filter: crate::target::ObjectFilter,
    pub cost: crate::mana::ManaCost,
}

impl CostIncreaseManaCost {
    pub fn new(filter: crate::target::ObjectFilter, cost: crate::mana::ManaCost) -> Self {
        Self { filter, cost }
    }
    pub fn with_condition(self, _condition: crate::ConditionExpr) -> Self {
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ThisSpellCostReduction {
    pub amount: crate::effect::Value,
    pub condition: ThisSpellCostCondition,
}

impl ThisSpellCostReduction {
    pub fn new(amount: crate::effect::Value, condition: ThisSpellCostCondition) -> Self {
        Self { amount, condition }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ThisSpellCostReductionManaCost {
    pub cost: crate::mana::ManaCost,
    pub condition: ThisSpellCostCondition,
}

impl ThisSpellCostReductionManaCost {
    pub fn new(cost: crate::mana::ManaCost, condition: ThisSpellCostCondition) -> Self {
        Self { cost, condition }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SetColorsForFilter {
    pub filter: crate::target::ObjectFilter,
    pub color: crate::color::ColorSet,
}

impl SetColorsForFilter {
    pub fn new(filter: crate::target::ObjectFilter, color: crate::color::ColorSet) -> Self {
        Self { filter, color }
    }
    pub fn with_condition(self, _condition: crate::ConditionExpr) -> Self {
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RemoveCardTypesForFilter {
    pub filter: crate::target::ObjectFilter,
    pub types: Vec<crate::types::CardType>,
}

impl RemoveCardTypesForFilter {
    pub fn new(filter: crate::target::ObjectFilter, types: Vec<crate::types::CardType>) -> Self {
        Self { filter, types }
    }
    pub fn with_condition(self, _condition: crate::ConditionExpr) -> Self {
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ActivatedAbilityCostCondition {
    TargetsExactly {
        count: usize,
        filter: crate::target::ObjectFilter,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum AttackCostCondition {
    PayGenericPerSourceCounter {
        counter_type: crate::object::CounterType,
        amount_per_counter: u32,
    },
    ReturnPermanentsToOwnersHand {
        filter: crate::target::ObjectFilter,
        count: u32,
    },
    SacrificePermanents {
        filter: crate::target::ObjectFilter,
        count: u32,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum AttackingGroupAttackCondition {
    AtLeastNOtherCreaturesAttack(u32),
    BlackOrGreenCreatureAlsoAttacks,
    CreatureWithGreaterPowerAlsoAttacks,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DefendingPlayerAttackCondition {
    Controls(crate::target::ObjectFilter),
    ControlsEnchantmentOrEnchantedPermanent,
    HasCardsInGraveyardOrMore(u32),
    IsMonarch,
    IsPoisoned,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CantAttackUnlessConditionSpec {
    AttackCost(AttackCostCondition),
    AttackingGroupCondition(AttackingGroupAttackCondition),
    BattlefieldCountAtLeast {
        filter: crate::target::ObjectFilter,
        count: u32,
    },
    ControllerControlsMoreThanDefendingPlayer(crate::target::ObjectFilter),
    ControllerGraveyardHasCardsAtLeast(u32),
    DefendingPlayerCondition(DefendingPlayerAttackCondition),
    OpponentWasDealtDamageThisTurn,
    SourceCondition(crate::ConditionExpr),
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnterAsCopyAsEntersSpec {
    pub filter: crate::target::ObjectFilter,
    pub may: bool,
    pub enters_tapped_if_chosen: bool,
    pub added_card_types: Vec<crate::types::CardType>,
    pub added_subtypes: Vec<crate::types::Subtype>,
    pub added_abilities: Vec<crate::ability::Ability>,
}

impl LandwalkKind {
    pub fn display(self) -> String {
        match self {
            Self::Subtype {
                subtype,
                snow: false,
            } => format!("{subtype}walk"),
            Self::Subtype {
                subtype,
                snow: true,
            } => format!("Snow {subtype}walk"),
            Self::AnyLand => "Landwalk".to_string(),
            Self::NonbasicLand => "Nonbasic landwalk".to_string(),
            Self::ArtifactLand => "Artifact landwalk".to_string(),
        }
    }
}
