use super::{StaticAbility, StaticAbilityId, StaticAbilityKind, ThisSpellCostCondition};
use crate::continuous::ContinuousEffect;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::replacement::ReplacementEffect;

pub type CompiledStaticAbility = ironsmith_core::StaticAbility<
    crate::triggers::Trigger,
    crate::effect::Effect,
    crate::costs::Cost,
    ThisSpellCostCondition,
>;

type CompiledAbilityModel = ironsmith_core::Ability<
    CompiledStaticAbility,
    crate::triggers::Trigger,
    crate::effect::Effect,
    crate::costs::Cost,
>;

type CompiledGrantSpec = ironsmith_core::GrantSpec<
    CompiledStaticAbility,
    crate::effect::Effect,
    crate::costs::Cost,
    ThisSpellCostCondition,
>;

#[derive(Debug, Clone)]
pub struct StaticAbilityModelInterpreter {
    model: CompiledStaticAbility,
    granted_inline_ability: Option<crate::ability::Ability>,
    enter_as_copy_spec: Option<super::EnterAsCopyAsEntersSpec>,
    level_abilities: Option<Vec<crate::ability::LevelAbility>>,
    equipment_grant_abilities: Option<Vec<StaticAbility>>,
    grant_spec: Option<crate::grant::GrantSpec>,
    cost_reduction: Option<super::CostReduction>,
    activated_ability_cost_reduction: Option<super::ActivatedAbilityCostReduction>,
    activated_ability_cost_increase: Option<super::ActivatedAbilityCostIncrease>,
    cost_increase: Option<super::CostIncrease>,
    cost_reduction_mana_cost: Option<super::CostReductionManaCost>,
    cost_increase_mana_cost: Option<super::CostIncreaseManaCost>,
    this_spell_cost_reduction: Option<super::ThisSpellCostReduction>,
    this_spell_cost_reduction_mana_cost: Option<super::ThisSpellCostReductionManaCost>,
}

impl StaticAbilityModelInterpreter {
    pub fn new(model: CompiledStaticAbility) -> Self {
        let granted_inline_ability = Self::cached_granted_inline_ability(&model);
        let enter_as_copy_spec = Self::cached_enter_as_copy_spec(&model);
        let level_abilities = Self::cached_level_abilities(&model);
        let equipment_grant_abilities = Self::cached_equipment_grant_abilities(&model);
        let grant_spec = Self::cached_grant_spec(&model);
        let cost_reduction = Self::cached_cost_reduction(&model);
        let activated_ability_cost_reduction =
            Self::cached_activated_ability_cost_reduction(&model);
        let activated_ability_cost_increase = Self::cached_activated_ability_cost_increase(&model);
        let cost_increase = Self::cached_cost_increase(&model);
        let cost_reduction_mana_cost = Self::cached_cost_reduction_mana_cost(&model);
        let cost_increase_mana_cost = Self::cached_cost_increase_mana_cost(&model);
        let this_spell_cost_reduction = Self::cached_this_spell_cost_reduction(&model);
        let this_spell_cost_reduction_mana_cost =
            Self::cached_this_spell_cost_reduction_mana_cost(&model);
        Self {
            model,
            granted_inline_ability,
            enter_as_copy_spec,
            level_abilities,
            equipment_grant_abilities,
            grant_spec,
            cost_reduction,
            activated_ability_cost_reduction,
            activated_ability_cost_increase,
            cost_increase,
            cost_reduction_mana_cost,
            cost_increase_mana_cost,
            this_spell_cost_reduction,
            this_spell_cost_reduction_mana_cost,
        }
    }

    fn payload(
        &self,
    ) -> &ironsmith_core::StaticAbilityPayload<
        crate::triggers::Trigger,
        crate::effect::Effect,
        crate::costs::Cost,
        ThisSpellCostCondition,
    > {
        &self.model.payload
    }

    fn is_simple_keyword_id(id: StaticAbilityId) -> bool {
        matches!(
            id,
            StaticAbilityId::Flying
                | StaticAbilityId::FirstStrike
                | StaticAbilityId::DoubleStrike
                | StaticAbilityId::Deathtouch
                | StaticAbilityId::Defender
                | StaticAbilityId::Flash
                | StaticAbilityId::Haste
                | StaticAbilityId::Hexproof
                | StaticAbilityId::Indestructible
                | StaticAbilityId::Intimidate
                | StaticAbilityId::Lifelink
                | StaticAbilityId::Menace
                | StaticAbilityId::Reach
                | StaticAbilityId::Shroud
                | StaticAbilityId::Trample
                | StaticAbilityId::Vigilance
                | StaticAbilityId::Fear
                | StaticAbilityId::Skulk
                | StaticAbilityId::Prowess
                | StaticAbilityId::Flanking
                | StaticAbilityId::UmbraArmor
                | StaticAbilityId::Phasing
                | StaticAbilityId::Wither
                | StaticAbilityId::Infect
                | StaticAbilityId::Changeling
                | StaticAbilityId::Partner
                | StaticAbilityId::DoctorsCompanion
                | StaticAbilityId::Assist
                | StaticAbilityId::SplitSecond
                | StaticAbilityId::Rebound
                | StaticAbilityId::Cascade
                | StaticAbilityId::Unleash
                | StaticAbilityId::Bloodthirst
                | StaticAbilityId::Protection
                | StaticAbilityId::Ward
                | StaticAbilityId::Landwalk
        )
    }

    fn core_landwalk_to_runtime(
        kind: ironsmith_core::LandwalkKind,
    ) -> crate::static_abilities::LandwalkKind {
        match kind {
            ironsmith_core::LandwalkKind::Subtype { subtype, snow } => {
                crate::static_abilities::LandwalkKind::Subtype { subtype, snow }
            }
            ironsmith_core::LandwalkKind::AnyLand => crate::static_abilities::LandwalkKind::AnyLand,
            ironsmith_core::LandwalkKind::NonbasicLand => {
                crate::static_abilities::LandwalkKind::NonbasicLand
            }
            ironsmith_core::LandwalkKind::ArtifactLand => {
                crate::static_abilities::LandwalkKind::ArtifactLand
            }
        }
    }

    fn ability_from_model(ability: &CompiledAbilityModel) -> crate::ability::Ability {
        let kind = match &ability.kind {
            ironsmith_core::AbilityKind::Static(static_ability) => {
                crate::ability::AbilityKind::Static(StaticAbility::from_model(
                    static_ability.clone(),
                ))
            }
            ironsmith_core::AbilityKind::Triggered(triggered) => {
                crate::ability::AbilityKind::Triggered(triggered.clone())
            }
            ironsmith_core::AbilityKind::Activated(activated) => {
                crate::ability::AbilityKind::Activated(activated.clone())
            }
        };
        crate::ability::Ability {
            kind,
            functional_zones: ability.functional_zones.clone(),
            text: ability.text.clone(),
        }
    }

    fn static_ability_from_ability_model(ability: &CompiledAbilityModel) -> Option<StaticAbility> {
        match &ability.kind {
            ironsmith_core::AbilityKind::Static(static_ability) => {
                Some(StaticAbility::from_model(static_ability.clone()))
            }
            _ => None,
        }
    }

    fn grant_spec_from_model(spec: &CompiledGrantSpec) -> crate::grant::GrantSpec {
        let grantable = match &spec.grantable {
            ironsmith_core::Grantable::Ability(static_ability) => {
                crate::grant::Grantable::Ability(StaticAbility::from_model(static_ability.clone()))
            }
            ironsmith_core::Grantable::AlternativeCast(method) => {
                crate::grant::Grantable::AlternativeCast(method.clone())
            }
            ironsmith_core::Grantable::DerivedAlternativeCast(spec) => {
                crate::grant::Grantable::DerivedAlternativeCast(spec.clone())
            }
            ironsmith_core::Grantable::PlayFrom => crate::grant::Grantable::PlayFrom,
        };
        crate::grant::GrantSpec {
            grantable,
            filter: spec.filter.clone(),
            zone: spec.zone,
            beneficiary: spec.beneficiary.clone(),
        }
    }

    fn cached_granted_inline_ability(
        model: &CompiledStaticAbility,
    ) -> Option<crate::ability::Ability> {
        match &model.payload {
            ironsmith_core::StaticAbilityPayload::AttachedAbilityGrant(grant) => {
                Some(Self::ability_from_model(&grant.ability))
            }
            ironsmith_core::StaticAbilityPayload::Conditional { ability, .. } => {
                Self::cached_granted_inline_ability(ability)
            }
            _ => None,
        }
    }

    fn cached_enter_as_copy_spec(
        model: &CompiledStaticAbility,
    ) -> Option<super::EnterAsCopyAsEntersSpec> {
        match &model.payload {
            ironsmith_core::StaticAbilityPayload::EnterAsCopyAsEnters { spec, .. } => {
                Some(super::EnterAsCopyAsEntersSpec {
                    filter: spec.filter.clone(),
                    may: spec.may,
                    enters_tapped_if_chosen: spec.enters_tapped_if_chosen,
                    added_card_types: spec.added_card_types.clone(),
                    added_subtypes: spec.added_subtypes.clone(),
                    added_abilities: spec
                        .added_abilities
                        .iter()
                        .map(Self::ability_from_model)
                        .collect(),
                })
            }
            ironsmith_core::StaticAbilityPayload::Conditional { ability, .. } => {
                Self::cached_enter_as_copy_spec(ability)
            }
            _ => None,
        }
    }

    fn cached_level_abilities(
        model: &CompiledStaticAbility,
    ) -> Option<Vec<crate::ability::LevelAbility>> {
        match &model.payload {
            ironsmith_core::StaticAbilityPayload::LevelAbility(level) => {
                Some(vec![crate::ability::LevelAbility {
                    min_level: level.min_level,
                    max_level: level.max_level,
                    power_toughness: level.power_toughness,
                    abilities: level
                        .abilities
                        .iter()
                        .cloned()
                        .map(StaticAbility::from_model)
                        .collect(),
                }])
            }
            ironsmith_core::StaticAbilityPayload::Conditional { ability, .. } => {
                Self::cached_level_abilities(ability)
            }
            _ => None,
        }
    }

    fn cached_equipment_grant_abilities(
        model: &CompiledStaticAbility,
    ) -> Option<Vec<StaticAbility>> {
        match &model.payload {
            ironsmith_core::StaticAbilityPayload::EquipmentGrant(abilities) => Some(
                abilities
                    .iter()
                    .cloned()
                    .map(StaticAbility::from_model)
                    .collect(),
            ),
            ironsmith_core::StaticAbilityPayload::Conditional { ability, .. } => {
                Self::cached_equipment_grant_abilities(ability)
            }
            _ => None,
        }
    }

    fn cached_grant_spec(model: &CompiledStaticAbility) -> Option<crate::grant::GrantSpec> {
        match &model.payload {
            ironsmith_core::StaticAbilityPayload::Grants(spec) => {
                Some(Self::grant_spec_from_model(spec))
            }
            ironsmith_core::StaticAbilityPayload::Conditional { ability, .. } => {
                Self::cached_grant_spec(ability)
            }
            _ => None,
        }
    }

    fn cached_cost_reduction(model: &CompiledStaticAbility) -> Option<super::CostReduction> {
        match &model.payload {
            ironsmith_core::StaticAbilityPayload::CostReduction(reduction) => Some(
                super::CostReduction::new(reduction.filter.clone(), reduction.amount.clone()),
            ),
            ironsmith_core::StaticAbilityPayload::Conditional { ability, condition } => {
                Self::cached_cost_reduction(ability)
                    .map(|reduction| reduction.with_condition(condition.clone()))
            }
            _ => None,
        }
    }

    fn cached_activated_ability_cost_reduction(
        model: &CompiledStaticAbility,
    ) -> Option<super::ActivatedAbilityCostReduction> {
        match &model.payload {
            ironsmith_core::StaticAbilityPayload::ActivatedAbilityCostReduction {
                filter,
                reduction,
                condition,
                per_matching_objects,
                minimum_total_mana,
            } => {
                let mut converted =
                    super::ActivatedAbilityCostReduction::new(filter.clone(), *reduction);
                if let Some(minimum) = minimum_total_mana {
                    converted = converted.with_minimum_total_mana(*minimum);
                }
                if let Some(per_matching_objects) = per_matching_objects {
                    converted = converted.with_per_matching_objects(per_matching_objects.clone());
                }
                if let Some(condition) = condition {
                    converted = converted.with_condition(match condition {
                        ironsmith_core::ActivatedAbilityCostCondition::TargetsExactly {
                            count,
                            filter,
                        } => super::ActivatedAbilityCostCondition::TargetsExactly {
                            count: *count,
                            filter: filter.clone(),
                        },
                    });
                }
                Some(converted)
            }
            ironsmith_core::StaticAbilityPayload::Conditional { ability, .. } => {
                Self::cached_activated_ability_cost_reduction(ability)
            }
            _ => None,
        }
    }

    fn cached_activated_ability_cost_increase(
        model: &CompiledStaticAbility,
    ) -> Option<super::ActivatedAbilityCostIncrease> {
        match &model.payload {
            ironsmith_core::StaticAbilityPayload::ActivatedAbilityCostIncrease {
                filter,
                increase,
            } => Some(super::ActivatedAbilityCostIncrease::new(
                filter.clone(),
                increase.clone(),
            )),
            ironsmith_core::StaticAbilityPayload::Conditional { ability, .. } => {
                Self::cached_activated_ability_cost_increase(ability)
            }
            _ => None,
        }
    }

    fn cached_cost_increase(model: &CompiledStaticAbility) -> Option<super::CostIncrease> {
        match &model.payload {
            ironsmith_core::StaticAbilityPayload::CostIncrease(increase) => Some(
                super::CostIncrease::new(increase.filter.clone(), increase.amount.clone()),
            ),
            ironsmith_core::StaticAbilityPayload::Conditional { ability, condition } => {
                Self::cached_cost_increase(ability)
                    .map(|increase| increase.with_condition(condition.clone()))
            }
            _ => None,
        }
    }

    fn cached_cost_reduction_mana_cost(
        model: &CompiledStaticAbility,
    ) -> Option<super::CostReductionManaCost> {
        match &model.payload {
            ironsmith_core::StaticAbilityPayload::CostReductionManaCost(reduction) => Some(
                super::CostReductionManaCost::new(reduction.filter.clone(), reduction.cost.clone()),
            ),
            ironsmith_core::StaticAbilityPayload::Conditional { ability, condition } => {
                Self::cached_cost_reduction_mana_cost(ability)
                    .map(|reduction| reduction.with_condition(condition.clone()))
            }
            _ => None,
        }
    }

    fn cached_cost_increase_mana_cost(
        model: &CompiledStaticAbility,
    ) -> Option<super::CostIncreaseManaCost> {
        match &model.payload {
            ironsmith_core::StaticAbilityPayload::CostIncreaseManaCost(increase) => Some(
                super::CostIncreaseManaCost::new(increase.filter.clone(), increase.cost.clone()),
            ),
            ironsmith_core::StaticAbilityPayload::Conditional { ability, condition } => {
                Self::cached_cost_increase_mana_cost(ability)
                    .map(|increase| increase.with_condition(condition.clone()))
            }
            _ => None,
        }
    }

    fn cached_this_spell_cost_reduction(
        model: &CompiledStaticAbility,
    ) -> Option<super::ThisSpellCostReduction> {
        match &model.payload {
            ironsmith_core::StaticAbilityPayload::ThisSpellCostReduction(reduction) => {
                Some(super::ThisSpellCostReduction::new(
                    reduction.amount.clone(),
                    reduction.condition.clone(),
                ))
            }
            ironsmith_core::StaticAbilityPayload::Conditional { ability, .. } => {
                Self::cached_this_spell_cost_reduction(ability)
            }
            _ => None,
        }
    }

    fn cached_this_spell_cost_reduction_mana_cost(
        model: &CompiledStaticAbility,
    ) -> Option<super::ThisSpellCostReductionManaCost> {
        match &model.payload {
            ironsmith_core::StaticAbilityPayload::ThisSpellCostReductionManaCost(reduction) => {
                Some(super::ThisSpellCostReductionManaCost::new(
                    reduction.cost.clone(),
                    reduction.condition.clone(),
                ))
            }
            ironsmith_core::StaticAbilityPayload::Conditional { ability, .. } => {
                Self::cached_this_spell_cost_reduction_mana_cost(ability)
            }
            _ => None,
        }
    }

    fn leaf_static_ability(&self) -> Option<StaticAbility> {
        if matches!(self.payload(), ironsmith_core::StaticAbilityPayload::None)
            && let Ok(ability) =
                StaticAbility::from_compiler_model_parts(self.model.id, self.model.label.clone())
        {
            return Some(ability);
        }

        Some(match self.payload() {
            ironsmith_core::StaticAbilityPayload::Anthem(anthem) => {
                let mut converted = match &anthem.filter {
                    Some(filter) => crate::static_abilities::Anthem::new(filter.clone(), 0, 0)
                        .with_values(anthem.power.clone(), anthem.toughness.clone()),
                    None => crate::static_abilities::Anthem::for_source(0, 0)
                        .with_values(anthem.power.clone(), anthem.toughness.clone()),
                };
                if let Some(condition) = &anthem.condition {
                    converted = converted.with_condition(condition.clone());
                }
                StaticAbility::new(converted)
            }
            ironsmith_core::StaticAbilityPayload::AttachedAbilityGrant(grant) => {
                let mut converted = crate::static_abilities::AttachedAbilityGrant::new(
                    Self::ability_from_model(&grant.ability),
                    grant.display.clone(),
                );
                if let Some(condition) = &grant.condition {
                    converted = converted.with_condition(condition.clone());
                }
                StaticAbility::new(converted)
            }
            ironsmith_core::StaticAbilityPayload::Conditional { ability, condition } => {
                let converted = StaticAbility::from_model((**ability).clone());
                converted.with_condition(condition.clone()).unwrap_or_else(|| {
                    StaticAbility::new(
                        crate::static_abilities::GrantAbility::source(converted)
                            .with_condition(condition.clone()),
                    )
                })
            }
            ironsmith_core::StaticAbilityPayload::GrantAbility(grant) => {
                let mut converted = crate::static_abilities::GrantAbility::new(
                    grant.filter.clone(),
                    Self::static_ability_from_ability_model(&grant.ability)?,
                );
                if let Some(condition) = &grant.condition {
                    converted = converted.with_condition(condition.clone());
                }
                StaticAbility::new(converted)
            }
            ironsmith_core::StaticAbilityPayload::GrantObjectAbilityForFilter(grant) => {
                let mut converted = crate::static_abilities::GrantObjectAbilityForFilter::new(
                    grant.filter.clone(),
                    Self::ability_from_model(&grant.ability),
                    grant.display.clone(),
                );
                if let Some(condition) = &grant.condition {
                    converted = converted.with_condition(condition.clone());
                }
                StaticAbility::new(converted)
            }
            ironsmith_core::StaticAbilityPayload::CopyActivatedAbilities(copy) => {
                let converted =
                    crate::static_abilities::CopyActivatedAbilities::new(copy.filter.clone())
                        .with_exclude_source_name(copy.exclude_source_name);
                StaticAbility::copy_activated_abilities(converted)
            }
            ironsmith_core::StaticAbilityPayload::LevelAbility(level) => {
                StaticAbility::with_level_abilities(vec![crate::ability::LevelAbility {
                    min_level: level.min_level,
                    max_level: level.max_level,
                    power_toughness: level.power_toughness,
                    abilities: level
                        .abilities
                        .iter()
                        .cloned()
                        .map(StaticAbility::from_model)
                        .collect(),
                }])
            }
            ironsmith_core::StaticAbilityPayload::Protection(from) => {
                StaticAbility::protection(from.clone())
            }
            ironsmith_core::StaticAbilityPayload::RuleRestriction {
                restriction,
                display,
            } => StaticAbility::restriction(restriction.clone(), display.clone()),
            ironsmith_core::StaticAbilityPayload::PregameAction { kind, text } => {
                StaticAbility::pregame_action(kind.clone(), text.clone())
            }
            ironsmith_core::StaticAbilityPayload::Ward(cost) => StaticAbility::ward(cost.clone()),
            ironsmith_core::StaticAbilityPayload::CanBlockAdditionalCreatureEachCombat(count) => {
                StaticAbility::can_block_additional_creature_each_combat(*count)
            }
            ironsmith_core::StaticAbilityPayload::ExertAttack {
                only_if_not_exerted_this_turn,
                linked_trigger,
                display,
            } => StaticAbility::exert_attack(
                *only_if_not_exerted_this_turn,
                linked_trigger.clone(),
                display.clone(),
            ),
            ironsmith_core::StaticAbilityPayload::EquipmentGrant(abilities) => {
                StaticAbility::equipment_grant(
                    abilities
                        .iter()
                        .cloned()
                        .map(StaticAbility::from_model)
                        .collect(),
                )
            }
            ironsmith_core::StaticAbilityPayload::RemoveAllAbilities(filter) => {
                StaticAbility::remove_all_abilities(filter.clone())
            }
            ironsmith_core::StaticAbilityPayload::RemoveAllAbilitiesExceptMana(filter) => {
                StaticAbility::remove_all_abilities_except_mana(filter.clone())
            }
            ironsmith_core::StaticAbilityPayload::SetBasePowerToughness {
                filter,
                power,
                toughness,
            } => StaticAbility::set_base_power_toughness(filter.clone(), *power, *toughness),
            ironsmith_core::StaticAbilityPayload::AddCardTypes { filter, card_types } => {
                StaticAbility::add_card_types(filter.clone(), card_types.clone())
            }
            ironsmith_core::StaticAbilityPayload::SetCardTypes { filter, card_types } => {
                StaticAbility::set_card_types(filter.clone(), card_types.clone())
            }
            ironsmith_core::StaticAbilityPayload::AddSubtypes { filter, subtypes } => {
                StaticAbility::add_subtypes(filter.clone(), subtypes.clone())
            }
            ironsmith_core::StaticAbilityPayload::AddAllSubtypesOfFamily { filter, family } => {
                StaticAbility::add_all_subtypes_of_family(filter.clone(), *family)
            }
            ironsmith_core::StaticAbilityPayload::SetCreatureSubtypes { filter, subtypes } => {
                StaticAbility::set_creature_subtypes(filter.clone(), subtypes.clone())
            }
            ironsmith_core::StaticAbilityPayload::MakeColorless(filter) => {
                StaticAbility::make_colorless(filter.clone())
            }
            ironsmith_core::StaticAbilityPayload::CostIncreasePerTargetBeyondFirst(amount) => {
                StaticAbility::cost_increase_per_target_beyond_first(*amount)
            }
            ironsmith_core::StaticAbilityPayload::MinimumSpellTotalMana(amount) => {
                StaticAbility::minimum_spell_total_mana(*amount)
            }
            ironsmith_core::StaticAbilityPayload::ChoosePlayerAsEnters(display) => {
                StaticAbility::choose_player_as_enters(display.clone())
            }
            ironsmith_core::StaticAbilityPayload::ChooseCreatureTypeAsEnters(display) => {
                StaticAbility::choose_creature_type_as_enters(display.clone())
            }
            ironsmith_core::StaticAbilityPayload::EnterAsCopyAsEnters { spec, display } => {
                StaticAbility::with_enter_as_copy_as_enters(
                    super::EnterAsCopyAsEntersSpec {
                        filter: spec.filter.clone(),
                        may: spec.may,
                        enters_tapped_if_chosen: spec.enters_tapped_if_chosen,
                        added_card_types: spec.added_card_types.clone(),
                        added_subtypes: spec.added_subtypes.clone(),
                        added_abilities: spec
                            .added_abilities
                            .iter()
                            .map(Self::ability_from_model)
                            .collect(),
                    },
                    display.clone(),
                )
            }
            ironsmith_core::StaticAbilityPayload::DoubleDamageFromSourcesYouControlOfChosenType(
                display,
            ) => StaticAbility::double_damage_from_sources_you_control_of_chosen_type(
                display.clone(),
            ),
            ironsmith_core::StaticAbilityPayload::AdditionalLandPlays(count) => {
                StaticAbility::additional_land_plays(*count)
            }
            ironsmith_core::StaticAbilityPayload::RevealFirstCardYouDrawEachTurn {
                optional,
                your_turns_only,
            } => StaticAbility::reveal_first_card_you_draw_each_turn(*optional, *your_turns_only),
            ironsmith_core::StaticAbilityPayload::ExileToCounteredExileInsteadOfGraveyard {
                player,
                counter_type,
            } => StaticAbility::exile_to_countered_exile_instead_of_graveyard(
                player.clone(),
                *counter_type,
            ),
            ironsmith_core::StaticAbilityPayload::CharacteristicDefiningPt {
                power,
                toughness,
            } => StaticAbility::characteristic_defining_pt(power.clone(), toughness.clone()),
            ironsmith_core::StaticAbilityPayload::DiscardOrRedirectReplacement {
                filter,
                redirect_zone,
            } => StaticAbility::discard_or_redirect_replacement(filter.clone(), *redirect_zone),
            ironsmith_core::StaticAbilityPayload::PayLifeOrEnterTapped(value) => {
                StaticAbility::pay_life_or_enter_tapped(*value)
            }
            ironsmith_core::StaticAbilityPayload::ManaSpendPermission {
                permission,
                display,
            } => StaticAbility::mana_spend_permission(permission.clone(), display.clone()),
            ironsmith_core::StaticAbilityPayload::Landwalk(kind) => match kind {
                ironsmith_core::LandwalkKind::Subtype {
                    subtype,
                    snow: false,
                } => StaticAbility::landwalk(*subtype),
                ironsmith_core::LandwalkKind::Subtype {
                    subtype,
                    snow: true,
                } => StaticAbility::snow_landwalk(*subtype),
                ironsmith_core::LandwalkKind::AnyLand => StaticAbility::any_landwalk(),
                ironsmith_core::LandwalkKind::NonbasicLand => StaticAbility::nonbasic_landwalk(),
                ironsmith_core::LandwalkKind::ArtifactLand => StaticAbility::artifact_landwalk(),
            },
            ironsmith_core::StaticAbilityPayload::Bloodthirst(amount) => {
                StaticAbility::bloodthirst(*amount)
            }
            ironsmith_core::StaticAbilityPayload::CantAttackYouUnlessControllerPaysPerAttacker(
                amount,
            ) => StaticAbility::cant_attack_you_unless_controller_pays_per_attacker(*amount),
            ironsmith_core::StaticAbilityPayload::CantAttackYouUnlessControllerPaysPerAttackerBasicLandTypesAmongLandsYouControl => {
                StaticAbility::cant_attack_you_unless_controller_pays_per_attacker_basic_land_types_among_lands_you_control()
            }
            ironsmith_core::StaticAbilityPayload::Grants(spec) => {
                StaticAbility::grants(Self::grant_spec_from_model(spec))
            }
            ironsmith_core::StaticAbilityPayload::EntersTappedUnlessCondition {
                condition,
                display,
            } => StaticAbility::enters_tapped_unless_condition(condition.clone(), display.clone()),
            ironsmith_core::StaticAbilityPayload::EntersWithCountersIfCondition {
                counter,
                count,
                condition,
                display,
            } => StaticAbility::enters_with_counters_if_condition(
                *counter,
                count.clone(),
                condition.clone(),
                display.clone(),
            ),
            ironsmith_core::StaticAbilityPayload::EntersWithCountersValue { counter, count } => {
                StaticAbility::enters_with_counters_value(*counter, count.clone())
            }
            ironsmith_core::StaticAbilityPayload::EntersTappedForFilter(filter) => {
                StaticAbility::enters_tapped_for_filter(filter.clone())
            }
            ironsmith_core::StaticAbilityPayload::EntersUntappedForFilter(filter) => {
                StaticAbility::enters_untapped_for_filter(filter.clone())
            }
            ironsmith_core::StaticAbilityPayload::EntersWithCountersAndSubtypesForFilter {
                filter,
                counter,
                count,
                subtypes,
            } => StaticAbility::enters_with_counters_and_subtypes_for_filter(
                filter.clone(),
                *counter,
                count.clone(),
                subtypes.clone(),
            ),
            _ => return None,
        })
    }
}

impl StaticAbility {
    pub fn from_model(model: CompiledStaticAbility) -> Self {
        Self::new(StaticAbilityModelInterpreter::new(model))
    }
}

impl StaticAbilityKind for StaticAbilityModelInterpreter {
    fn id(&self) -> StaticAbilityId {
        self.model.id.unwrap_or(StaticAbilityId::RuleFallbackText)
    }

    fn display(&self) -> String {
        if let Some(ability) = self.leaf_static_ability() {
            return ability.display();
        }
        if let Some(reduction) = &self.this_spell_cost_reduction {
            return reduction.display();
        }
        if let Some(reduction) = &self.this_spell_cost_reduction_mana_cost {
            return reduction.display();
        }
        if let Some(reduction) = &self.cost_reduction {
            return reduction.display();
        }
        if let Some(reduction) = &self.cost_reduction_mana_cost {
            return reduction.display();
        }
        if let Some(increase) = &self.cost_increase {
            return increase.display();
        }
        if let Some(increase) = &self.cost_increase_mana_cost {
            return increase.display();
        }
        if let Some(reduction) = &self.activated_ability_cost_reduction {
            return reduction.display();
        }
        if let Some(increase) = &self.activated_ability_cost_increase {
            return increase.display();
        }
        self.model.label.clone()
    }

    fn with_static_condition(&self, condition: crate::ConditionExpr) -> Option<StaticAbility> {
        self.leaf_static_ability()?.with_condition(condition)
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        game: &GameState,
    ) -> Vec<ContinuousEffect> {
        self.leaf_static_ability()
            .map(|ability| ability.generate_effects(source, controller, game))
            .unwrap_or_default()
    }

    fn apply_restrictions(&self, game: &mut GameState, source: ObjectId, controller: PlayerId) {
        if let Some(ability) = self.leaf_static_ability() {
            ability.apply_restrictions(game, source, controller);
        }
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        self.leaf_static_ability()?
            .generate_replacement_effect(source, controller)
    }

    fn is_active(&self, game: &GameState, source: ObjectId) -> bool {
        self.leaf_static_ability()
            .map(|ability| ability.is_active(game, source))
            .unwrap_or(true)
    }

    fn is_keyword(&self) -> bool {
        Self::is_simple_keyword_id(self.id())
    }

    fn grants_evasion(&self) -> bool {
        matches!(
            self.id(),
            StaticAbilityId::Flying
                | StaticAbilityId::Shadow
                | StaticAbilityId::Horsemanship
                | StaticAbilityId::Fear
                | StaticAbilityId::Intimidate
                | StaticAbilityId::Skulk
                | StaticAbilityId::Landwalk
                | StaticAbilityId::Unblockable
        )
    }

    fn is_unblockable(&self) -> bool {
        self.id() == StaticAbilityId::Unblockable
    }

    fn landwalk_kind(&self) -> Option<crate::static_abilities::LandwalkKind> {
        match self.payload() {
            ironsmith_core::StaticAbilityPayload::Landwalk(kind) => {
                Some(Self::core_landwalk_to_runtime(*kind))
            }
            _ => None,
        }
    }

    fn additional_blockable_attackers(&self) -> Option<usize> {
        match self.payload() {
            ironsmith_core::StaticAbilityPayload::CanBlockAdditionalCreatureEachCombat(count) => {
                Some(*count)
            }
            _ => None,
        }
    }

    fn has_first_strike(&self) -> bool {
        self.id() == StaticAbilityId::FirstStrike
    }

    fn has_double_strike(&self) -> bool {
        self.id() == StaticAbilityId::DoubleStrike
    }

    fn has_deathtouch(&self) -> bool {
        self.id() == StaticAbilityId::Deathtouch
    }

    fn has_lifelink(&self) -> bool {
        self.id() == StaticAbilityId::Lifelink
    }

    fn has_trample(&self) -> bool {
        self.id() == StaticAbilityId::Trample
    }

    fn has_vigilance(&self) -> bool {
        self.id() == StaticAbilityId::Vigilance
    }

    fn has_haste(&self) -> bool {
        self.id() == StaticAbilityId::Haste
    }

    fn has_flash(&self) -> bool {
        self.id() == StaticAbilityId::Flash
    }

    fn forbids_paying_life_for_cast_or_activate(&self) -> bool {
        self.leaf_static_ability()
            .is_some_and(|ability| ability.forbids_paying_life_for_cast_or_activate())
    }

    fn forbids_sacrificing_nonland_for_cast_or_activate(&self) -> bool {
        self.leaf_static_ability()
            .is_some_and(|ability| ability.forbids_sacrificing_nonland_for_cast_or_activate())
    }

    fn optional_attack_cost_prompt(
        &self,
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<crate::decisions::context::BooleanContext> {
        self.leaf_static_ability()?
            .optional_attack_cost_prompt(game, source, controller)
    }

    fn pay_optional_attack_cost(
        &self,
        game: &mut GameState,
        source: ObjectId,
        controller: PlayerId,
        trigger_queue: &mut crate::triggers::TriggerQueue,
    ) -> Option<Result<(), String>> {
        self.leaf_static_ability()?.pay_optional_attack_cost(
            game,
            source,
            controller,
            trigger_queue,
        )
    }

    fn has_reach(&self) -> bool {
        self.id() == StaticAbilityId::Reach
    }

    fn has_defender(&self) -> bool {
        self.id() == StaticAbilityId::Defender
    }

    fn has_indestructible(&self) -> bool {
        self.id() == StaticAbilityId::Indestructible
    }

    fn has_hexproof(&self) -> bool {
        self.id() == StaticAbilityId::Hexproof
    }

    fn has_shroud(&self) -> bool {
        self.id() == StaticAbilityId::Shroud
    }

    fn is_changeling(&self) -> bool {
        self.id() == StaticAbilityId::Changeling
    }

    fn has_menace(&self) -> bool {
        self.id() == StaticAbilityId::Menace
    }

    fn has_flying(&self) -> bool {
        self.id() == StaticAbilityId::Flying
    }

    fn has_protection(&self) -> bool {
        matches!(
            self.payload(),
            ironsmith_core::StaticAbilityPayload::Protection(_)
        )
    }

    fn protection_from(&self) -> Option<&crate::ability::ProtectionFrom> {
        match self.payload() {
            ironsmith_core::StaticAbilityPayload::Protection(from) => Some(from),
            _ => None,
        }
    }

    fn ward_cost(&self) -> Option<&crate::cost::TotalCost> {
        match self.payload() {
            ironsmith_core::StaticAbilityPayload::Ward(cost) => Some(cost),
            _ => None,
        }
    }

    fn granted_inline_ability(&self) -> Option<&crate::ability::Ability> {
        self.granted_inline_ability.as_ref()
    }

    fn enter_as_copy_as_enters(&self) -> Option<&super::EnterAsCopyAsEntersSpec> {
        self.enter_as_copy_spec.as_ref()
    }

    fn minimum_total_spell_mana(&self) -> Option<u32> {
        match self.payload() {
            ironsmith_core::StaticAbilityPayload::MinimumSpellTotalMana(amount) => Some(*amount),
            _ => None,
        }
    }

    fn player_choice_as_enters(&self) -> Option<super::ChoosePlayerAsEntersSpec> {
        matches!(
            self.payload(),
            ironsmith_core::StaticAbilityPayload::ChoosePlayerAsEnters(_)
        )
        .then_some(super::ChoosePlayerAsEntersSpec)
    }

    fn creature_type_choice_as_enters(&self) -> Option<super::ChooseCreatureTypeAsEntersSpec> {
        matches!(
            self.payload(),
            ironsmith_core::StaticAbilityPayload::ChooseCreatureTypeAsEnters(_)
        )
        .then_some(super::ChooseCreatureTypeAsEntersSpec)
    }

    fn pregame_action_kind(&self) -> Option<super::PregameActionKind> {
        match self.payload() {
            ironsmith_core::StaticAbilityPayload::PregameAction { kind, .. } => Some(kind.clone()),
            _ => None,
        }
    }

    fn reveal_drawn_card_spec(&self) -> Option<super::RevealDrawnCardSpec> {
        match self.payload() {
            ironsmith_core::StaticAbilityPayload::RevealFirstCardYouDrawEachTurn {
                optional,
                your_turns_only,
            } => Some(super::RevealDrawnCardSpec {
                card_number: 1,
                optional: *optional,
                your_turns_only: *your_turns_only,
            }),
            _ => None,
        }
    }

    fn cost_increase_per_additional_target(&self) -> Option<u32> {
        match self.payload() {
            ironsmith_core::StaticAbilityPayload::CostIncreasePerTargetBeyondFirst(amount) => {
                Some(*amount)
            }
            _ => None,
        }
    }

    fn is_anthem(&self) -> bool {
        matches!(
            self.payload(),
            ironsmith_core::StaticAbilityPayload::Anthem(_)
        )
    }

    fn grants_abilities(&self) -> bool {
        matches!(
            self.payload(),
            ironsmith_core::StaticAbilityPayload::GrantAbility(_)
                | ironsmith_core::StaticAbilityPayload::GrantObjectAbilityForFilter(_)
                | ironsmith_core::StaticAbilityPayload::AttachedAbilityGrant(_)
        )
    }

    fn modifies_costs(&self) -> bool {
        self.cost_reduction.is_some()
            || self.activated_ability_cost_reduction.is_some()
            || self.activated_ability_cost_increase.is_some()
            || self.cost_increase.is_some()
            || self.cost_reduction_mana_cost.is_some()
            || self.cost_increase_mana_cost.is_some()
            || self.this_spell_cost_reduction.is_some()
            || self.this_spell_cost_reduction_mana_cost.is_some()
            || self.cost_increase_per_additional_target().is_some()
            || self.minimum_total_spell_mana().is_some()
    }

    fn this_spell_cost_reduction(&self) -> Option<&super::ThisSpellCostReduction> {
        self.this_spell_cost_reduction.as_ref()
    }

    fn this_spell_cost_reduction_mana_cost(
        &self,
    ) -> Option<&super::ThisSpellCostReductionManaCost> {
        self.this_spell_cost_reduction_mana_cost.as_ref()
    }

    fn cost_reduction(&self) -> Option<&super::CostReduction> {
        self.cost_reduction.as_ref()
    }

    fn activated_ability_cost_reduction(&self) -> Option<&super::ActivatedAbilityCostReduction> {
        self.activated_ability_cost_reduction.as_ref()
    }

    fn activated_ability_cost_increase(&self) -> Option<&super::ActivatedAbilityCostIncrease> {
        self.activated_ability_cost_increase.as_ref()
    }

    fn cost_increase(&self) -> Option<&super::CostIncrease> {
        self.cost_increase.as_ref()
    }

    fn cost_reduction_mana_cost(&self) -> Option<&super::CostReductionManaCost> {
        self.cost_reduction_mana_cost.as_ref()
    }

    fn cost_increase_mana_cost(&self) -> Option<&super::CostIncreaseManaCost> {
        self.cost_increase_mana_cost.as_ref()
    }

    fn level_abilities(&self) -> Option<&[crate::ability::LevelAbility]> {
        self.level_abilities.as_deref()
    }

    fn equipment_grant_abilities(&self) -> Option<&[StaticAbility]> {
        self.equipment_grant_abilities.as_deref()
    }

    fn grant_spec(&self) -> Option<crate::grant::GrantSpec> {
        self.grant_spec.clone()
    }

    fn generic_attack_tax_per_attacker_against_you(
        &self,
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<u32> {
        self.leaf_static_ability()?
            .generic_attack_tax_per_attacker_against_you(game, source, controller)
    }

    fn enters_tapped(&self) -> bool {
        matches!(
            self.payload(),
            ironsmith_core::StaticAbilityPayload::PayLifeOrEnterTapped(_)
                | ironsmith_core::StaticAbilityPayload::EntersTappedForFilter(_)
        )
    }

    fn is_devoid(&self) -> bool {
        match self.payload() {
            ironsmith_core::StaticAbilityPayload::MakeColorless(filter) => {
                filter == &crate::target::ObjectFilter::source()
            }
            _ => false,
        }
    }

    fn has_affinity(&self) -> bool {
        self.id() == StaticAbilityId::AffinityForArtifacts
    }

    fn has_delve(&self) -> bool {
        self.id() == StaticAbilityId::Delve
    }

    fn has_convoke(&self) -> bool {
        self.id() == StaticAbilityId::Convoke
    }

    fn has_improvise(&self) -> bool {
        self.id() == StaticAbilityId::Improvise
    }
}
