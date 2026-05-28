use super::{StaticAbility, StaticAbilityId, StaticAbilityKind, ThisSpellCostCondition};
use crate::continuous::ContinuousEffect;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::replacement::ReplacementEffect;
use std::fmt;

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

#[derive(Clone)]
pub struct StaticAbilityModelInterpreter {
    model: CompiledStaticAbility,
    leaf_static_ability: Option<StaticAbility>,
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
    cost_increase_mana_cost_per_additional_target:
        Option<super::CostIncreaseManaCostPerAdditionalTarget>,
    this_spell_cost_reduction: Option<super::ThisSpellCostReduction>,
    this_spell_cost_reduction_mana_cost: Option<super::ThisSpellCostReductionManaCost>,
}

impl fmt::Debug for StaticAbilityModelInterpreter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let payload = self.payload_debug_summary();
        f.debug_struct("StaticAbilityModelInterpreter")
            .field("id", &self.model.id)
            .field("label", &self.model.label)
            .field("payload", &payload)
            .finish_non_exhaustive()
    }
}

impl StaticAbilityModelInterpreter {
    fn ability_model_debug_summary(ability: &CompiledAbilityModel) -> String {
        match &ability.kind {
            ironsmith_core::AbilityKind::Static(static_ability) => format!(
                "Static({})",
                Self::static_model_debug_summary(static_ability)
            ),
            ironsmith_core::AbilityKind::Triggered(triggered) => format!(
                "TriggeredAbility {{ trigger: {:?}, effects: {:?}, choices: {:?}, intervening_if: {:?} }}",
                triggered.trigger, triggered.effects, triggered.choices, triggered.intervening_if
            ),
            ironsmith_core::AbilityKind::Activated(activated) => format!(
                "ActivatedAbility {{ mana_cost: {:?}, effects: {:?}, choices: {:?}, timing: {:?} }}",
                activated.mana_cost, activated.effects, activated.choices, activated.timing
            ),
        }
    }

    fn static_model_debug_summary(ability: &CompiledStaticAbility) -> String {
        match &ability.payload {
            ironsmith_core::StaticAbilityPayload::SoulbondSharedObjectAbility(granted) => format!(
                "SoulbondSharedObjectAbility({})",
                Self::ability_model_debug_summary(granted)
            ),
            ironsmith_core::StaticAbilityPayload::SoulbondSharedAbility(granted) => format!(
                "SoulbondSharedAbility({})",
                Self::static_model_debug_summary(granted)
            ),
            payload => format!(
                "StaticAbility {{ id: {:?}, label: {:?}, payload: {:?} }}",
                ability.id, ability.label, payload
            ),
        }
    }

    fn payload_debug_summary(&self) -> String {
        match self.payload() {
            ironsmith_core::StaticAbilityPayload::SoulbondSharedObjectAbility(ability) => format!(
                "SoulbondSharedObjectAbility({})",
                Self::ability_model_debug_summary(ability)
            ),
            ironsmith_core::StaticAbilityPayload::SoulbondSharedAbility(ability) => format!(
                "SoulbondSharedAbility({})",
                Self::static_model_debug_summary(ability)
            ),
            ironsmith_core::StaticAbilityPayload::ThisSpellCastRestriction { kind, display } => {
                format!(
                    "ThisSpellCastRestriction {{ kind: {:?}, display: {:?} }}",
                    Self::this_spell_cast_restriction_from_model(kind),
                    display
                )
            }
            payload => format!("{payload:?}"),
        }
    }

    pub fn new(model: CompiledStaticAbility) -> Self {
        let leaf_static_ability = Self::cached_leaf_static_ability(&model);
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
        let cost_increase_mana_cost_per_additional_target =
            Self::cached_cost_increase_mana_cost_per_additional_target(&model);
        let this_spell_cost_reduction = Self::cached_this_spell_cost_reduction(&model);
        let this_spell_cost_reduction_mana_cost =
            Self::cached_this_spell_cost_reduction_mana_cost(&model);
        Self {
            model,
            leaf_static_ability,
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
            cost_increase_mana_cost_per_additional_target,
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

    fn attack_cost_condition_from_model(
        condition: &ironsmith_core::AttackCostCondition,
    ) -> super::AttackCostCondition {
        match condition {
            ironsmith_core::AttackCostCondition::SacrificePermanents { filter, count } => {
                super::AttackCostCondition::SacrificePermanents {
                    filter: filter.clone(),
                    count: *count,
                }
            }
            ironsmith_core::AttackCostCondition::ReturnPermanentsToOwnersHand { filter, count } => {
                super::AttackCostCondition::ReturnPermanentsToOwnersHand {
                    filter: filter.clone(),
                    count: *count,
                }
            }
            ironsmith_core::AttackCostCondition::PayGenericPerSourceCounter {
                counter_type,
                amount_per_counter,
            } => super::AttackCostCondition::PayGenericPerSourceCounter {
                counter_type: *counter_type,
                amount_per_counter: *amount_per_counter,
            },
        }
    }

    fn attacking_group_condition_from_model(
        condition: &ironsmith_core::AttackingGroupAttackCondition,
    ) -> super::AttackingGroupAttackCondition {
        match condition {
            ironsmith_core::AttackingGroupAttackCondition::AtLeastNOtherCreaturesAttack(count) => {
                super::AttackingGroupAttackCondition::AtLeastNOtherCreaturesAttack(*count)
            }
            ironsmith_core::AttackingGroupAttackCondition::BlackOrGreenCreatureAlsoAttacks => {
                super::AttackingGroupAttackCondition::BlackOrGreenCreatureAlsoAttacks
            }
            ironsmith_core::AttackingGroupAttackCondition::CreatureWithGreaterPowerAlsoAttacks => {
                super::AttackingGroupAttackCondition::CreatureWithGreaterPowerAlsoAttacks
            }
        }
    }

    fn defending_player_condition_from_model(
        condition: &ironsmith_core::DefendingPlayerAttackCondition,
    ) -> super::DefendingPlayerAttackCondition {
        match condition {
            ironsmith_core::DefendingPlayerAttackCondition::Controls(filter) => {
                super::DefendingPlayerAttackCondition::Controls(filter.clone())
            }
            ironsmith_core::DefendingPlayerAttackCondition::ControlsEnchantmentOrEnchantedPermanent => {
                super::DefendingPlayerAttackCondition::ControlsEnchantmentOrEnchantedPermanent
            }
            ironsmith_core::DefendingPlayerAttackCondition::HasCardsInGraveyardOrMore(count) => {
                super::DefendingPlayerAttackCondition::HasCardsInGraveyardOrMore(*count)
            }
            ironsmith_core::DefendingPlayerAttackCondition::IsMonarch => {
                super::DefendingPlayerAttackCondition::IsMonarch
            }
            ironsmith_core::DefendingPlayerAttackCondition::IsPoisoned => {
                super::DefendingPlayerAttackCondition::IsPoisoned
            }
        }
    }

    fn cant_attack_unless_condition_from_model(
        condition: &ironsmith_core::CantAttackUnlessConditionSpec,
    ) -> super::CantAttackUnlessConditionSpec {
        match condition {
            ironsmith_core::CantAttackUnlessConditionSpec::AttackCost(cost) => {
                super::CantAttackUnlessConditionSpec::AttackCost(
                    Self::attack_cost_condition_from_model(cost),
                )
            }
            ironsmith_core::CantAttackUnlessConditionSpec::AttackingGroupCondition(condition) => {
                super::CantAttackUnlessConditionSpec::AttackingGroupCondition(
                    Self::attacking_group_condition_from_model(condition),
                )
            }
            ironsmith_core::CantAttackUnlessConditionSpec::BattlefieldCountAtLeast {
                filter,
                count,
            } => super::CantAttackUnlessConditionSpec::BattlefieldCountAtLeast {
                filter: filter.clone(),
                count: *count,
            },
            ironsmith_core::CantAttackUnlessConditionSpec::ControllerControlsMoreThanDefendingPlayer(filter) => {
                super::CantAttackUnlessConditionSpec::ControllerControlsMoreThanDefendingPlayer(
                    filter.clone(),
                )
            }
            ironsmith_core::CantAttackUnlessConditionSpec::ControllerGraveyardHasCardsAtLeast(count) => {
                super::CantAttackUnlessConditionSpec::ControllerGraveyardHasCardsAtLeast(*count)
            }
            ironsmith_core::CantAttackUnlessConditionSpec::DefendingPlayerCondition(condition) => {
                super::CantAttackUnlessConditionSpec::DefendingPlayerCondition(
                    Self::defending_player_condition_from_model(condition),
                )
            }
            ironsmith_core::CantAttackUnlessConditionSpec::OpponentWasDealtDamageThisTurn => {
                super::CantAttackUnlessConditionSpec::OpponentWasDealtDamageThisTurn
            }
            ironsmith_core::CantAttackUnlessConditionSpec::SourceCondition(condition) => {
                super::CantAttackUnlessConditionSpec::SourceCondition(condition.clone())
            }
        }
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
                | StaticAbilityId::PartnerWith
                | StaticAbilityId::StartYourEngines
                | StaticAbilityId::DoctorsCompanion
                | StaticAbilityId::Assist
                | StaticAbilityId::SplitSecond
                | StaticAbilityId::Rebound
                | StaticAbilityId::Cascade
                | StaticAbilityId::ReadAhead
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
        Self::ability_with_inherent_functional_zones(crate::ability::Ability {
            kind,
            functional_zones: ability.functional_zones.clone(),
        })
    }

    fn ability_with_inherent_functional_zones(
        ability: crate::ability::Ability,
    ) -> crate::ability::Ability {
        let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
            return ability;
        };
        match static_ability.id() {
            StaticAbilityId::ExileToExileInsteadOfGraveyard
            | StaticAbilityId::ExileToCounteredExileInsteadOfGraveyard
            | StaticAbilityId::ExileWouldDieInstead => ability.in_zones(vec![
                crate::zone::Zone::Battlefield,
                crate::zone::Zone::Stack,
                crate::zone::Zone::Graveyard,
                crate::zone::Zone::Hand,
                crate::zone::Zone::Library,
                crate::zone::Zone::Exile,
                crate::zone::Zone::Command,
            ]),
            _ => ability,
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
            ironsmith_core::StaticAbilityPayload::SoulbondSharedObjectAbility(ability) => {
                Some(Self::ability_from_model(ability))
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
                    affected_filter: spec.affected_filter.clone(),
                    may: spec.may,
                    enters_tapped_if_chosen: spec.enters_tapped_if_chosen,
                    copy_source_self: spec.copy_source_self,
                    copy_source_enchanted: spec.copy_source_enchanted,
                    name_override: spec.name_override.clone(),
                    added_card_types: spec.added_card_types.clone(),
                    removed_supertypes: spec.removed_supertypes.clone(),
                    added_subtypes: spec.added_subtypes.clone(),
                    added_abilities: spec
                        .added_abilities
                        .iter()
                        .map(Self::ability_from_model)
                        .collect(),
                    set_base_power_toughness: spec.set_base_power_toughness,
                    set_base_power_toughness_from_self: spec.set_base_power_toughness_from_self,
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
                replacement_mana_cost,
                display,
                condition,
                per_matching_objects,
                per_basic_land_types_among,
                minimum_total_mana,
            } => {
                let mut converted = if let Some(replacement_mana_cost) = replacement_mana_cost {
                    super::ActivatedAbilityCostReduction::replacement_mana_cost(
                        filter.clone(),
                        replacement_mana_cost.clone(),
                        display.clone().unwrap_or_else(|| {
                            format!(
                                "You may pay {} rather than pay activated ability costs of {}",
                                replacement_mana_cost.to_oracle(),
                                filter.description()
                            )
                        }),
                    )
                } else {
                    let mut reduction_model =
                        super::ActivatedAbilityCostReduction::new(filter.clone(), *reduction);
                    if let Some(display) = display {
                        reduction_model = reduction_model.with_display(display.clone());
                    }
                    reduction_model
                };
                if let Some(minimum) = minimum_total_mana {
                    converted = converted.with_minimum_total_mana(*minimum);
                }
                if let Some(per_matching_objects) = per_matching_objects {
                    converted = converted.with_per_matching_objects(per_matching_objects.clone());
                }
                if let Some(per_basic_land_types_among) = per_basic_land_types_among {
                    converted = converted
                        .with_per_basic_land_types_among(per_basic_land_types_among.clone());
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
            ironsmith_core::StaticAbilityPayload::Conditional { ability, condition } => {
                Self::cached_activated_ability_cost_reduction(ability)
                    .map(|reduction| reduction.with_static_condition(condition.clone()))
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
                activator,
                non_mana_only,
                condition,
            } => {
                let mut parsed = if let Some(activator) = activator.clone() {
                    super::ActivatedAbilityCostIncrease::for_activator(
                        activator,
                        increase.clone(),
                        *non_mana_only,
                    )
                } else {
                    super::ActivatedAbilityCostIncrease::new(filter.clone(), increase.clone())
                };
                if let Some(condition) = condition.clone() {
                    parsed = parsed.with_condition(condition);
                }
                Some(parsed)
            }
            ironsmith_core::StaticAbilityPayload::Conditional { ability, condition } => {
                Self::cached_activated_ability_cost_increase(ability)
                    .map(|increase| increase.with_condition(condition.clone()))
            }
            _ => None,
        }
    }

    fn cached_cost_increase(model: &CompiledStaticAbility) -> Option<super::CostIncrease> {
        match &model.payload {
            ironsmith_core::StaticAbilityPayload::CostIncrease(increase) => {
                let mut parsed =
                    super::CostIncrease::new(increase.filter.clone(), increase.amount.clone());
                if let Some(condition) = increase.condition.clone() {
                    parsed = parsed.with_condition(condition);
                }
                Some(parsed)
            }
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

    fn cached_cost_increase_mana_cost_per_additional_target(
        model: &CompiledStaticAbility,
    ) -> Option<super::CostIncreaseManaCostPerAdditionalTarget> {
        match &model.payload {
            ironsmith_core::StaticAbilityPayload::CostIncreaseManaCostPerTargetBeyondFirst(
                cost,
            ) => Some(super::CostIncreaseManaCostPerAdditionalTarget::new(
                cost.clone(),
            )),
            ironsmith_core::StaticAbilityPayload::Conditional { ability, .. } => {
                Self::cached_cost_increase_mana_cost_per_additional_target(ability)
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
            ironsmith_core::StaticAbilityPayload::ThisSpellCastRestriction { .. } => None,
            ironsmith_core::StaticAbilityPayload::Conditional { ability, .. } => {
                Self::cached_this_spell_cost_reduction_mana_cost(ability)
            }
            _ => None,
        }
    }

    fn this_spell_cast_restriction_from_model(
        kind: &ironsmith_core::ThisSpellCastRestrictionKind,
    ) -> super::ThisSpellCastRestrictionKind {
        match kind.label.as_str() {
            "during declare attackers step" => {
                super::ThisSpellCastRestrictionKind::during_declare_attackers_step()
            }
            "during declare attackers step if you were attacked" => {
                super::ThisSpellCastRestrictionKind::during_declare_attackers_step_if_you_were_attacked_this_step()
            }
            "during combat" => super::ThisSpellCastRestrictionKind::during_combat(),
            "during combat before blockers" => {
                super::ThisSpellCastRestrictionKind::during_combat_before_blockers_are_declared()
            }
            "during combat after blockers" => {
                super::ThisSpellCastRestrictionKind::during_combat_after_blockers_are_declared()
            }
            "during combat on your turn before blockers" => {
                super::ThisSpellCastRestrictionKind::during_combat_on_your_turn_before_blockers_are_declared()
            }
            "during combat on opponents turn" => {
                super::ThisSpellCastRestrictionKind::during_combat_on_opponents_turn()
            }
            "before attackers are declared" => {
                super::ThisSpellCastRestrictionKind::before_attackers_are_declared()
            }
            "before combat damage step" => {
                super::ThisSpellCastRestrictionKind::before_combat_damage_step()
            }
            "during opponents upkeep" => {
                super::ThisSpellCastRestrictionKind::during_opponents_upkeep()
            }
            "during opponents turn after upkeep" => {
                super::ThisSpellCastRestrictionKind::during_opponents_turn_after_upkeep()
            }
            "during your end step" => super::ThisSpellCastRestrictionKind::during_your_end_step(),
            "if you cast another spell this turn" => {
                super::ThisSpellCastRestrictionKind::if_you_cast_another_spell_this_turn()
            }
            "if you cast another green spell this turn" => {
                super::ThisSpellCastRestrictionKind::if_you_cast_another_green_spell_this_turn()
            }
            "if opponent cast creature spell this turn" => {
                super::ThisSpellCastRestrictionKind::if_opponent_cast_creature_spell_this_turn()
            }
            "if creature is attacking you" => {
                super::ThisSpellCastRestrictionKind::if_creature_is_attacking_you()
            }
            "after combat" => super::ThisSpellCastRestrictionKind::after_combat(),
            "if you control snow land" => {
                super::ThisSpellCastRestrictionKind::if_you_control_snow_land()
            }
            "if you control fewer creatures than each opponent" => {
                super::ThisSpellCastRestrictionKind::if_you_control_fewer_creatures_than_each_opponent()
            }
            label => {
                if let Some(name) = label.strip_prefix("if no permanents named ") {
                    return super::ThisSpellCastRestrictionKind::if_no_permanents_named_on_battlefield(
                        name.to_string(),
                    );
                }
                if let Some(rest) = label.strip_prefix("if you control ")
                    && let Some((count, subtype_name)) = rest.split_once("+ ")
                    && let Ok(count) = count.parse::<u32>()
                    && let Some(subtype) = crate::types::Subtype::all_creature_types()
                        .iter()
                        .copied()
                        .find(|subtype| subtype.display_name() == subtype_name)
                {
                    return super::ThisSpellCastRestrictionKind::if_you_control_subtype_or_more(
                        subtype, count,
                    );
                }
                super::ThisSpellCastRestrictionKind::condition(
                    super::ThisSpellCastCondition::YouControlAtLeast {
                        filter: crate::target::ObjectFilter::default(),
                        count: u32::MAX,
                    },
                )
            }
        }
    }

    fn leaf_static_ability(&self) -> Option<&StaticAbility> {
        self.leaf_static_ability.as_ref()
    }

    fn cached_leaf_static_ability(model: &CompiledStaticAbility) -> Option<StaticAbility> {
        if matches!(&model.payload, ironsmith_core::StaticAbilityPayload::None)
            && let Ok(ability) =
                StaticAbility::from_compiler_model_parts(model.id, model.label.clone())
        {
            return Some(ability);
        }

        Some(match &model.payload {
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
            ironsmith_core::StaticAbilityPayload::AttachedChosenLandwalkGrant(grant) => {
                StaticAbility::attached_chosen_landwalk_grant(grant.display.clone(), grant.snow)
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
                let mut converted =
                    crate::static_abilities::CopyActivatedAbilities::new(copy.filter.clone())
                        .with_exclude_source_name(copy.exclude_source_name)
                        .with_exclude_source_id(copy.exclude_source_id)
                        .with_display(copy.display.clone());
                if let Some(counter) = copy.counter {
                    converted = converted.with_counter(counter);
                }
                if copy.force_once_each_turn {
                    converted = converted.with_once_each_turn();
                }
                StaticAbility::copy_activated_abilities(converted)
            }
            ironsmith_core::StaticAbilityPayload::CopyTriggeredAbilities(copy) => {
                let converted =
                    crate::static_abilities::CopyTriggeredAbilities::new(copy.filter.clone())
                        .with_exclude_source_name(copy.exclude_source_name)
                        .with_display(copy.display.clone());
                StaticAbility::copy_triggered_abilities(converted)
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
            ironsmith_core::StaticAbilityPayload::HexproofFrom(filter) => {
                StaticAbility::hexproof_from(filter.clone())
            }
            ironsmith_core::StaticAbilityPayload::RuleRestriction {
                restriction,
                display,
            } => StaticAbility::restriction(restriction.clone(), display.clone()),
            ironsmith_core::StaticAbilityPayload::PregameAction { kind, text } => {
                StaticAbility::pregame_action(kind.clone(), text.clone())
            }
            ironsmith_core::StaticAbilityPayload::Ward(cost) => StaticAbility::ward(cost.clone()),
            ironsmith_core::StaticAbilityPayload::Morph(cost) => StaticAbility::morph(cost.clone()),
            ironsmith_core::StaticAbilityPayload::Disguise(cost) => {
                StaticAbility::disguise(cost.clone())
            }
            ironsmith_core::StaticAbilityPayload::Megamorph(cost) => {
                StaticAbility::megamorph(cost.clone())
            }
            ironsmith_core::StaticAbilityPayload::CanBlockAdditionalCreatureEachCombat(count) => {
                StaticAbility::can_block_additional_creature_each_combat(*count)
            }
            ironsmith_core::StaticAbilityPayload::CantBeBlockedByMoreThan(count) => {
                StaticAbility::cant_be_blocked_by_more_than(*count)
            }
            ironsmith_core::StaticAbilityPayload::CantBeBlockedExceptByNOrMore(count) => {
                StaticAbility::cant_be_blocked_except_by_n_or_more(*count)
            }
            ironsmith_core::StaticAbilityPayload::CantBeBlockedByPowerOrLess(power) => {
                StaticAbility::cant_be_blocked_by_power_or_less(*power)
            }
            ironsmith_core::StaticAbilityPayload::CantBeBlockedByPowerOrGreater(power) => {
                StaticAbility::cant_be_blocked_by_power_or_greater(*power)
            }
            ironsmith_core::StaticAbilityPayload::CantBeBlockedAsLongAsDefendingPlayerControlsCardTypes(card_types) => {
                if card_types.len() == 1 {
                    StaticAbility::cant_be_blocked_as_long_as_defending_player_controls_card_type(
                        card_types[0],
                    )
                } else {
                    StaticAbility::cant_be_blocked_as_long_as_defending_player_controls_card_types(
                        card_types.clone(),
                    )
                }
            }
            ironsmith_core::StaticAbilityPayload::CantAttackUnlessCondition {
                condition,
                display,
            } => StaticAbility::cant_attack_unless_condition(
                Self::cant_attack_unless_condition_from_model(condition),
                display.clone(),
            ),
            ironsmith_core::StaticAbilityPayload::MayChooseNotToUntapDuringUntapStep(subject) => {
                StaticAbility::may_choose_not_to_untap_during_untap_step(subject.clone())
            }
            ironsmith_core::StaticAbilityPayload::UntapDuringEachOtherPlayersUntapStep {
                filter,
                display,
            } => StaticAbility::untap_during_each_other_players_untap_step(
                filter.clone(),
                display.clone(),
            ),
            ironsmith_core::StaticAbilityPayload::FirstEquipCostAlternative(display) => {
                StaticAbility::first_equip_cost_alternative(display.clone())
            }
            ironsmith_core::StaticAbilityPayload::ControlAttachedPermanent(display) => {
                StaticAbility::control_attached_permanent(display.clone())
            }
            ironsmith_core::StaticAbilityPayload::SetColors { filter, colors } => {
                StaticAbility::set_colors(filter.clone(), *colors)
            }
            ironsmith_core::StaticAbilityPayload::AddColors { filter, colors } => {
                StaticAbility::add_colors(filter.clone(), *colors)
            }
            ironsmith_core::StaticAbilityPayload::SetName { filter, name } => {
                StaticAbility::set_name(filter.clone(), name.clone())
            }
            ironsmith_core::StaticAbilityPayload::AddSupertypes { filter, supertypes } => {
                StaticAbility::add_supertypes(filter.clone(), supertypes.clone())
            }
            ironsmith_core::StaticAbilityPayload::RemoveSupertypes { filter, supertypes } => {
                StaticAbility::remove_supertypes(filter.clone(), supertypes.clone())
            }
            ironsmith_core::StaticAbilityPayload::MaxCreaturesCanAttackEachCombat(maximum) => {
                StaticAbility::max_attackers_each_combat(*maximum)
            }
            ironsmith_core::StaticAbilityPayload::MaxCreaturesCanAttackYouEachCombat(maximum) => {
                StaticAbility::max_attackers_can_attack_you_each_combat(*maximum)
            }
            ironsmith_core::StaticAbilityPayload::MaxCreaturesCanBlockEachCombat(maximum) => {
                StaticAbility::max_blockers_each_combat(*maximum)
            }
            ironsmith_core::StaticAbilityPayload::ChooseBasicLandTypeAsEnters(display) => {
                StaticAbility::choose_basic_land_type_as_enters(display.clone())
            }
            ironsmith_core::StaticAbilityPayload::ChooseLandTypeAsEnters(display) => {
                StaticAbility::choose_land_type_as_enters(display.clone())
            }
            ironsmith_core::StaticAbilityPayload::EnchantedLandIsChosenType(display) => {
                StaticAbility::enchanted_land_is_chosen_type(display.clone())
            }
            ironsmith_core::StaticAbilityPayload::AddChosenCreatureType { filter, display } => {
                StaticAbility::add_chosen_creature_type(filter.clone(), display.clone())
            }
            ironsmith_core::StaticAbilityPayload::SetChosenColor { filter, display } => {
                StaticAbility::set_chosen_color(filter.clone(), display.clone())
            }
            ironsmith_core::StaticAbilityPayload::ReduceMaximumHandSize { player, by } => {
                StaticAbility::reduce_maximum_hand_size(player.clone(), *by)
            }
            ironsmith_core::StaticAbilityPayload::MaximumHandSizeSevenMinusYourGraveyardCardTypes {
                player,
                min_card_types,
            } => StaticAbility::max_hand_size_seven_minus_your_graveyard_card_types(
                player.clone(),
                *min_card_types,
            ),
            ironsmith_core::StaticAbilityPayload::DuplicateMatchingTriggeredAbilities {
                source_filter,
                event_matcher,
                count,
                display,
            } => StaticAbility::duplicate_matching_triggered_abilities(
                source_filter.clone(),
                event_matcher.clone(),
                *count as usize,
                display.clone(),
            ),
            ironsmith_core::StaticAbilityPayload::SuppressMatchingTriggeredAbilities {
                source_filter,
                event_matcher,
                display,
            } => StaticAbility::suppress_matching_triggered_abilities(
                source_filter.clone(),
                event_matcher.clone(),
                display.clone(),
            ),
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
            ironsmith_core::StaticAbilityPayload::SoulbondSharedPowerToughness {
                power,
                toughness,
            } => StaticAbility::soulbond_shared_power_toughness(*power, *toughness),
            ironsmith_core::StaticAbilityPayload::SoulbondSharedAbility(ability) => {
                StaticAbility::soulbond_shared_ability(StaticAbility::from_model(
                    (**ability).clone(),
                ))
            }
            ironsmith_core::StaticAbilityPayload::SoulbondSharedObjectAbility(ability) => {
                StaticAbility::soulbond_shared_object_ability(Self::ability_from_model(ability))
            }
            ironsmith_core::StaticAbilityPayload::RemoveAbilityForFilter { filter, ability } => {
                StaticAbility::remove_ability(
                    filter.clone(),
                    StaticAbility::from_model((**ability).clone()),
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
            ironsmith_core::StaticAbilityPayload::RemoveCardTypes {
                filter,
                card_types,
                condition,
            } => {
                let ability = StaticAbility::remove_card_types(filter.clone(), card_types.clone());
                if let Some(condition) = condition {
                    ability.with_condition(condition.clone()).unwrap_or(ability)
                } else {
                    ability
                }
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
            ironsmith_core::StaticAbilityPayload::SetLandSubtypes { filter, subtypes } => {
                StaticAbility::set_land_subtypes(filter.clone(), subtypes.clone())
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
            ironsmith_core::StaticAbilityPayload::ThisSpellCastRestriction { kind, display } => {
                StaticAbility::this_spell_cast_restriction(
                    Self::this_spell_cast_restriction_from_model(kind),
                    display.clone(),
                )
            }
            ironsmith_core::StaticAbilityPayload::MinimumSpellTotalMana(amount) => {
                StaticAbility::minimum_spell_total_mana(*amount)
            }
            ironsmith_core::StaticAbilityPayload::ChoosePlayerAsEnters(display) => {
                StaticAbility::choose_player_as_enters(display.clone())
            }
            ironsmith_core::StaticAbilityPayload::ChooseCardNameAsEnters(display) => {
                StaticAbility::choose_card_name_as_enters(display.clone())
            }
            ironsmith_core::StaticAbilityPayload::ChooseCreatureTypeAsEnters(display) => {
                StaticAbility::choose_creature_type_as_enters(display.clone())
            }
            ironsmith_core::StaticAbilityPayload::ChooseNamedOptionAsEnters {
                options,
                display,
            } => StaticAbility::choose_named_option_as_enters(options.clone(), display.clone()),
            ironsmith_core::StaticAbilityPayload::EnterAsCopyAsEnters { spec, display } => {
                StaticAbility::with_enter_as_copy_as_enters(
                    super::EnterAsCopyAsEntersSpec {
                        filter: spec.filter.clone(),
                        affected_filter: spec.affected_filter.clone(),
                        may: spec.may,
                        enters_tapped_if_chosen: spec.enters_tapped_if_chosen,
                        copy_source_self: spec.copy_source_self,
                        copy_source_enchanted: spec.copy_source_enchanted,
                        name_override: spec.name_override.clone(),
                        added_card_types: spec.added_card_types.clone(),
                        removed_supertypes: spec.removed_supertypes.clone(),
                        added_subtypes: spec.added_subtypes.clone(),
                        added_abilities: spec
                            .added_abilities
                            .iter()
                            .map(Self::ability_from_model)
                            .collect(),
                        set_base_power_toughness: spec.set_base_power_toughness,
                        set_base_power_toughness_from_self: spec
                            .set_base_power_toughness_from_self,
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
            ironsmith_core::StaticAbilityPayload::ExileToExileInsteadOfGraveyard {
                filter,
                graveyard_owner,
                exclude_cycled,
            } => {
                if *exclude_cycled {
                    StaticAbility::exile_to_exile_instead_of_graveyard_unless_cycled(
                        filter.clone(),
                        graveyard_owner.clone(),
                    )
                } else {
                    StaticAbility::exile_to_exile_instead_of_graveyard(
                        filter.clone(),
                        graveyard_owner.clone(),
                    )
                }
            }
            ironsmith_core::StaticAbilityPayload::ExileWouldDieInstead {
                filter,
                damaged_by,
                exile_with_counters,
                follow_up_effects,
            } => StaticAbility::exile_would_die_instead_with_damage_source_counters_and_follow_up(
                filter.clone(),
                *damaged_by,
                exile_with_counters.clone(),
                follow_up_effects.clone(),
            ),
            ironsmith_core::StaticAbilityPayload::ModifyDamageAmountReplacement {
                source_filter,
                target_player_filter,
                target_object_filter,
                delta,
                display,
            } => StaticAbility::modify_damage_amount_replacement(
                source_filter.clone(),
                target_player_filter.clone(),
                target_object_filter.clone(),
                *delta,
                display.clone(),
            ),
            ironsmith_core::StaticAbilityPayload::MinimumDamageAmountReplacement {
                source_filter,
                target_player_filter,
                target_object_filter,
                floor,
                noncombat_only,
                display,
            } => StaticAbility::minimum_damage_amount_replacement(
                source_filter.clone(),
                target_player_filter.clone(),
                target_object_filter.clone(),
                floor.clone(),
                *noncombat_only,
                display.clone(),
            ),
            ironsmith_core::StaticAbilityPayload::DoubleDamageAmountReplacement {
                source_filter,
                target_player_filter,
                target_object_filter,
                display,
            } => StaticAbility::double_damage_amount_replacement(
                source_filter.clone(),
                target_player_filter.clone(),
                target_object_filter.clone(),
                display.clone(),
            ),
            ironsmith_core::StaticAbilityPayload::DoubleCountersReplacement {
                filter,
                counter_type,
                display,
            } => StaticAbility::double_counters_replacement(
                filter.clone(),
                *counter_type,
                display.clone(),
            ),
            ironsmith_core::StaticAbilityPayload::DoubleTokenCreationReplacement {
                controller,
                display,
            } => StaticAbility::double_token_creation_replacement(
                controller.clone(),
                display.clone(),
            ),
            ironsmith_core::StaticAbilityPayload::KeywordActionReplacement {
                action,
                source_filter,
                replacement_effects,
                display,
            } => StaticAbility::keyword_action_replacement(
                *action,
                source_filter.clone(),
                replacement_effects.clone(),
                display.clone(),
            ),
            ironsmith_core::StaticAbilityPayload::ConditionalDrawReplacement {
                condition,
                replacement_effects,
                display,
            } => StaticAbility::conditional_draw_replacement(
                condition.clone(),
                replacement_effects.clone(),
                display.clone(),
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
            ironsmith_core::StaticAbilityPayload::PreventDamageToSelfRemoveCounter {
                counter_type,
                amount,
            } => StaticAbility::prevent_damage_to_self_remove_counter(*counter_type, *amount),
            ironsmith_core::StaticAbilityPayload::PreventDamageToSelfPutCountersInstead {
                counter_type,
                display,
            } => StaticAbility::prevent_damage_to_self_put_counters_instead(
                *counter_type,
                display.clone(),
            ),
            ironsmith_core::StaticAbilityPayload::PreventConstrainedDamageToSelfPutCountersInstead {
                counter_type,
                display,
                source_filter,
                combat_only,
            } => StaticAbility::prevent_constrained_damage_to_self_put_counters_instead(
                *counter_type,
                display.clone(),
                source_filter.clone(),
                *combat_only,
            ),
            ironsmith_core::StaticAbilityPayload::ReplaceDamageWithCountersInstead {
                counter_type,
                display,
                source_filter,
                target_filter,
                combat_only,
            } => StaticAbility::replace_damage_with_counters_instead(
                *counter_type,
                source_filter.clone(),
                target_filter.clone(),
                *combat_only,
                display.clone(),
            ),
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
                added_abilities,
            } => StaticAbility::enters_with_counters_and_abilities_if_condition(
                *counter,
                count.clone(),
                condition.clone(),
                display.clone(),
                added_abilities
                    .iter()
                    .map(Self::ability_from_model)
                    .collect(),
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
            ironsmith_core::StaticAbilityPayload::EntersWithCharacteristicsForFilter {
                filter,
                card_types,
                subtypes,
                power,
                toughness,
            } => StaticAbility::enters_with_characteristics_for_filter(
                filter.clone(),
                card_types.clone(),
                subtypes.clone(),
                *power,
                *toughness,
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
        if let Some(ability) = self.leaf_static_ability() {
            return ability.id();
        }
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
        if let Some(increase) = &self.cost_increase_mana_cost_per_additional_target {
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
        if let Some(reduction) = &self.activated_ability_cost_reduction {
            return Some(StaticAbility::new(
                reduction.clone().with_static_condition(condition),
            ));
        }
        if let Some(increase) = &self.activated_ability_cost_increase {
            return Some(StaticAbility::new(
                increase.clone().with_condition(condition),
            ));
        }
        if let Some(reduction) = &self.cost_reduction {
            return Some(StaticAbility::new(
                reduction.clone().with_condition(condition),
            ));
        }
        if let Some(reduction) = &self.cost_reduction_mana_cost {
            return Some(StaticAbility::new(
                reduction.clone().with_condition(condition),
            ));
        }
        if let Some(increase) = &self.cost_increase {
            return Some(StaticAbility::new(
                increase.clone().with_condition(condition),
            ));
        }
        if let Some(increase) = &self.cost_increase_mana_cost {
            return Some(StaticAbility::new(
                increase.clone().with_condition(condition),
            ));
        }
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

    fn turn_face_up_cost(&self) -> Option<&crate::cost::TotalCost> {
        self.leaf_static_ability()?.turn_face_up_cost()
    }

    fn is_megamorph(&self) -> bool {
        self.leaf_static_ability()
            .is_some_and(|ability| ability.is_megamorph())
    }

    fn is_disguise(&self) -> bool {
        self.leaf_static_ability()
            .is_some_and(|ability| ability.is_disguise())
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

    fn minimum_blockers(&self) -> Option<usize> {
        match self.payload() {
            ironsmith_core::StaticAbilityPayload::CantBeBlockedExceptByNOrMore(count) => {
                Some(*count)
            }
            _ if self.id() == StaticAbilityId::Menace => Some(2),
            _ => None,
        }
    }

    fn maximum_blockers(&self) -> Option<usize> {
        match self.payload() {
            ironsmith_core::StaticAbilityPayload::CantBeBlockedByMoreThan(count) => Some(*count),
            _ => None,
        }
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

    fn card_name_choice_as_enters(&self) -> Option<super::ChooseCardNameAsEntersSpec> {
        matches!(
            self.payload(),
            ironsmith_core::StaticAbilityPayload::ChooseCardNameAsEnters(_)
        )
        .then_some(super::ChooseCardNameAsEntersSpec)
    }

    fn basic_land_type_choice_as_enters(&self) -> Option<super::ChooseBasicLandTypeAsEntersSpec> {
        matches!(
            self.payload(),
            ironsmith_core::StaticAbilityPayload::ChooseBasicLandTypeAsEnters(_)
        )
        .then_some(super::ChooseBasicLandTypeAsEntersSpec)
    }

    fn land_type_choice_as_enters(&self) -> Option<super::ChooseLandTypeAsEntersSpec> {
        matches!(
            self.payload(),
            ironsmith_core::StaticAbilityPayload::ChooseLandTypeAsEnters(_)
        )
        .then_some(super::ChooseLandTypeAsEntersSpec)
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

    fn cost_increase_mana_cost_per_additional_target(&self) -> Option<&crate::mana::ManaCost> {
        self.cost_increase_mana_cost_per_additional_target
            .as_ref()
            .map(|increase| &increase.cost)
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

    fn color_choice_as_becomes_attached(&self) -> Option<super::ChooseColorAsBecomesAttachedSpec> {
        matches!(
            self.model.id,
            Some(StaticAbilityId::ChooseColorAsBecomesAttached)
        )
        .then_some(super::ChooseColorAsBecomesAttachedSpec)
    }

    fn modifies_costs(&self) -> bool {
        self.cost_reduction.is_some()
            || self.activated_ability_cost_reduction.is_some()
            || self.activated_ability_cost_increase.is_some()
            || self.cost_increase.is_some()
            || self.cost_reduction_mana_cost.is_some()
            || self.cost_increase_mana_cost.is_some()
            || self.cost_increase_mana_cost_per_additional_target.is_some()
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

    fn conditional_spell_keyword_spec(&self) -> Option<super::ConditionalSpellKeywordSpec> {
        self.leaf_static_ability()?.conditional_spell_keyword_spec()
    }

    fn trigger_duplication_spec(&self) -> Option<super::TriggerDuplicationSpec> {
        self.leaf_static_ability()?.trigger_duplication_spec()
    }

    fn trigger_suppression_spec(&self) -> Option<super::TriggerSuppressionSpec> {
        self.leaf_static_ability()?.trigger_suppression_spec()
    }

    fn this_spell_cast_restriction_kind(&self) -> Option<super::ThisSpellCastRestrictionKind> {
        self.leaf_static_ability()?
            .this_spell_cast_restriction_kind()
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
