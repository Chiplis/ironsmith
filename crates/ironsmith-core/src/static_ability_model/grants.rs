use super::*;
use crate::ConditionConjunction;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LandwalkKind {
    Subtype { subtype: Subtype, snow: bool },
    AnyLand,
    NonbasicLand,
    ArtifactLand,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
/// The condition guarding this static ability, over whatever vocabulary the
/// phase using it speaks. Defaults to the resolved [`Condition`], so the
/// runtime spells it the way it always has.
pub struct Anthem<Cond = Condition> {
    pub filter: Option<ObjectFilter>,
    pub power: AnthemValue,
    pub toughness: AnthemValue,
    pub condition: Option<Cond>,
    /// Original leading set quantifier, retained only for compiled-text surface.
    pub set_quantifier_surface: Option<SetQuantifierSurface>,
    /// True when the original oracle text expressed the scaling count with a
    /// "where X is …" clause rather than "for each …". Purely a surface hint
    /// for rendering; the count itself is identical either way.
    pub count_uses_where_x: bool,
    /// True when the authored fixed modifier was introduced as "an
    /// additional P/T". This does not change layer semantics; it preserves
    /// the additive presentation when independently executable static
    /// siblings are rejoined.
    pub additional_surface: bool,
    /// Absolute power/toughness from an Oracle "gets P/T instead" continuation.
    ///
    /// The executable anthem stores only the conditional delta so layer 7c
    /// semantics remain additive. This typed surface fact lets rendering recover
    /// the replacement wording without rediscovering it from Oracle text.
    pub replacement_surface: Option<AnthemReplacementSurface>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnthemReplacementSurface {
    pub power: i32,
    pub toughness: i32,
}

impl<Cond> Anthem<Cond> {
    /// Translate this ability's condition into another phase's vocabulary.
    pub fn try_map_condition<Cond2, Err>(
        self,
        map_condition: impl FnOnce(Cond) -> Result<Cond2, Err>,
    ) -> Result<Anthem<Cond2>, Err> {
        let Anthem {
            condition,
            filter,
            power,
            toughness,
            set_quantifier_surface,
            count_uses_where_x,
            additional_surface,
            replacement_surface,
        } = self;
        Ok(Anthem {
            condition: condition.map(map_condition).transpose()?,
            filter,
            power,
            toughness,
            set_quantifier_surface,
            count_uses_where_x,
            additional_surface,
            replacement_surface,
        })
    }
}

impl<Cond: ConditionConjunction> Anthem<Cond> {
    pub fn new(filter: ObjectFilter, power: i32, toughness: i32) -> Self {
        Self {
            filter: Some(filter),
            power: AnthemValue::Fixed(power),
            toughness: AnthemValue::Fixed(toughness),
            condition: None,
            set_quantifier_surface: None,
            count_uses_where_x: false,
            additional_surface: false,
            replacement_surface: None,
        }
    }
    pub fn for_source(power: i32, toughness: i32) -> Self {
        Self {
            filter: None,
            power: AnthemValue::Fixed(power),
            toughness: AnthemValue::Fixed(toughness),
            condition: None,
            set_quantifier_surface: None,
            count_uses_where_x: false,
            additional_surface: false,
            replacement_surface: None,
        }
    }
    pub fn with_values(mut self, power: AnthemValue, toughness: AnthemValue) -> Self {
        self.power = power;
        self.toughness = toughness;
        self
    }
    pub fn with_condition(mut self, condition: Cond) -> Self {
        self.condition = Some(condition);
        self
    }
    pub fn with_count_uses_where_x(mut self, uses_where_x: bool) -> Self {
        self.count_uses_where_x = uses_where_x;
        self
    }
    pub fn with_additional_surface(mut self, additional_surface: bool) -> Self {
        self.additional_surface = additional_surface;
        self
    }
    pub fn with_set_quantifier_surface(mut self, surface: Option<SetQuantifierSurface>) -> Self {
        self.set_quantifier_surface = surface;
        self
    }
    pub fn with_replacement_surface(mut self, power: i32, toughness: i32) -> Self {
        self.replacement_surface = Some(AnthemReplacementSurface { power, toughness });
        self
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct AttachedAbilityGrant<T, E, C, Cond, ICond = Condition> {
    pub ability: AbilityModel<T, E, C, Cond, ICond>,
    pub additional_abilities: Vec<AbilityModel<T, E, C, Cond, ICond>>,
    pub display: String,
    pub condition: Option<ICond>,
    pub protection_does_not_remove_controlled_attachments: bool,
}

impl<T, E, C, Cond, ICond> AttachedAbilityGrant<T, E, C, Cond, ICond> {
    pub fn new(ability: AbilityModel<T, E, C, Cond, ICond>, display: impl Into<String>) -> Self {
        Self {
            ability,
            additional_abilities: Vec::new(),
            display: display.into(),
            condition: None,
            protection_does_not_remove_controlled_attachments: false,
        }
    }
    pub fn with_additional_abilities(
        mut self,
        abilities: Vec<AbilityModel<T, E, C, Cond, ICond>>,
    ) -> Self {
        self.additional_abilities = abilities;
        self
    }
    pub fn with_condition(mut self, condition: ICond) -> Self {
        self.condition = Some(condition);
        self
    }
    pub fn with_protection_attachment_exception(mut self, enabled: bool) -> Self {
        self.protection_does_not_remove_controlled_attachments = enabled;
        self
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    pub fn with_condition<C>(self, _condition: C) -> Self {
        self
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct GrantAbility<T, E, C, Cond, ICond = Condition> {
    pub filter: ObjectFilter,
    pub ability: AbilityModel<T, E, C, Cond, ICond>,
    pub condition: Option<ICond>,
    /// Original leading set quantifier, retained only for compiled-text surface.
    pub set_quantifier_surface: Option<SetQuantifierSurface>,
}

impl<T, E, C, Cond, ICond> GrantAbility<T, E, C, Cond, ICond> {
    pub fn new(filter: ObjectFilter, ability: AbilityModel<T, E, C, Cond, ICond>) -> Self {
        Self {
            filter,
            ability,
            condition: None,
            set_quantifier_surface: None,
        }
    }
    pub fn source(ability: impl Into<AbilityModel<T, E, C, Cond, ICond>>) -> Self {
        Self {
            filter: ObjectFilter::source(),
            ability: ability.into(),
            condition: None,
            set_quantifier_surface: None,
        }
    }
    pub fn with_condition(mut self, condition: ICond) -> Self {
        self.condition = Some(condition);
        self
    }
    pub fn with_set_quantifier_surface(mut self, surface: Option<SetQuantifierSurface>) -> Self {
        self.set_quantifier_surface = surface;
        self
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, PartialEq)]
pub struct GrantObjectAbilityForFilter<T, E, C, Cond, ICond = Condition> {
    pub filter: ObjectFilter,
    pub ability: AbilityModel<T, E, C, Cond, ICond>,
    pub additional_abilities: Vec<AbilityModel<T, E, C, Cond, ICond>>,
    pub display: String,
    pub condition: Option<ICond>,
    /// Original leading set quantifier, retained only for compiled-text surface.
    pub set_quantifier_surface: Option<SetQuantifierSurface>,
}

impl<T, E, C, Cond> std::fmt::Debug for GrantObjectAbilityForFilter<T, E, C, Cond>
where
    T: std::fmt::Debug,
    E: std::fmt::Debug,
    C: std::fmt::Debug,
    Cond: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GrantObjectAbilityForFilter")
            .field("filter", &self.filter)
            .field("ability", &self.ability)
            .field("additional_abilities", &self.additional_abilities)
            .field(
                "generated_modification",
                &format!("AddAbilityGeneric({:?})", self.ability),
            )
            .field("display", &self.display)
            .field("condition", &self.condition)
            .field("set_quantifier_surface", &self.set_quantifier_surface)
            .finish()
    }
}

impl<T, E, C, Cond, ICond> GrantObjectAbilityForFilter<T, E, C, Cond, ICond> {
    pub fn new(
        filter: ObjectFilter,
        ability: AbilityModel<T, E, C, Cond, ICond>,
        display: impl Into<String>,
    ) -> Self {
        Self {
            filter,
            ability,
            additional_abilities: Vec::new(),
            display: display.into(),
            condition: None,
            set_quantifier_surface: None,
        }
    }
    pub fn with_additional_abilities(
        mut self,
        abilities: Vec<AbilityModel<T, E, C, Cond, ICond>>,
    ) -> Self {
        self.additional_abilities = abilities;
        self
    }
    pub fn with_condition(mut self, condition: ICond) -> Self {
        self.condition = Some(condition);
        self
    }
    pub fn with_set_quantifier_surface(mut self, surface: Option<SetQuantifierSurface>) -> Self {
        self.set_quantifier_surface = surface;
        self
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct CopyActivatedAbilities {
    pub filter: ObjectFilter,
    pub counter: Option<CounterType>,
    pub only_loyalty: bool,
    pub exclude_source_name: bool,
    pub exclude_source_id: bool,
    pub force_once_each_turn: bool,
    pub display: String,
}

impl CopyActivatedAbilities {
    pub fn new(filter: ObjectFilter) -> Self {
        Self {
            filter,
            counter: None,
            only_loyalty: false,
            exclude_source_name: false,
            exclude_source_id: true,
            force_once_each_turn: false,
            display: "Has all activated abilities of matching objects".to_string(),
        }
    }
    pub fn with_exclude_source_name(mut self, exclude: bool) -> Self {
        self.exclude_source_name = exclude;
        self
    }
    pub fn with_exclude_source_id(mut self, exclude: bool) -> Self {
        self.exclude_source_id = exclude;
        self
    }
    pub fn with_display(mut self, display: impl Into<String>) -> Self {
        self.display = display.into();
        self
    }
    pub fn with_counter(mut self, counter: CounterType) -> Self {
        self.counter = Some(counter);
        self
    }
    pub fn with_only_loyalty(mut self) -> Self {
        self.only_loyalty = true;
        self
    }
    pub fn with_once_each_turn(mut self) -> Self {
        self.force_once_each_turn = true;
        self
    }
    pub fn with_condition<C>(self, _condition: C) -> Self {
        self
    }
}

/// Selects which complete static-ability instances may be inherited.
///
/// `Any` is the common case and deliberately preserves the selected ability's
/// complete payload. The color-protection selector represents Oracle's
/// narrower "protection from any color" category without treating unrelated
/// protection qualities as colors.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaticAbilityVariantSelector {
    Any(StaticAbilityId),
    ProtectionFromColor,
}

impl StaticAbilityVariantSelector {
    pub const fn any(id: StaticAbilityId) -> Self {
        Self::Any(id)
    }

    pub const fn ability_id(self) -> StaticAbilityId {
        match self {
            Self::Any(id) => id,
            Self::ProtectionFromColor => StaticAbilityId::Protection,
        }
    }
}

/// Inherit complete static-ability variants from objects matching a filter.
///
/// Unlike a fixed ability grant, this preserves payloads such as a protection
/// quality, a landwalk kind, or a qualified-hexproof filter.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct CopyStaticAbilityVariants {
    pub filter: ObjectFilter,
    pub selectors: Vec<StaticAbilityVariantSelector>,
    pub exclude_source_id: bool,
    pub display: String,
}

impl CopyStaticAbilityVariants {
    pub fn new(
        filter: ObjectFilter,
        selectors: Vec<StaticAbilityVariantSelector>,
        display: impl Into<String>,
    ) -> Self {
        Self {
            filter,
            selectors,
            exclude_source_id: true,
            display: display.into(),
        }
    }

    pub fn with_exclude_source_id(mut self, exclude: bool) -> Self {
        self.exclude_source_id = exclude;
        self
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct CopyTriggeredAbilities {
    pub filter: ObjectFilter,
    pub exclude_source_name: bool,
    pub display: String,
}

impl CopyTriggeredAbilities {
    pub fn new(filter: ObjectFilter) -> Self {
        Self {
            filter,
            exclude_source_name: false,
            display: "Has all triggered abilities of matching objects".to_string(),
        }
    }
    pub fn with_exclude_source_name(mut self, exclude: bool) -> Self {
        self.exclude_source_name = exclude;
        self
    }
    pub fn with_display(mut self, display: impl Into<String>) -> Self {
        self.display = display.into();
        self
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct CostReductionCharacteristicIntersection {
    pub characteristic: crate::ObjectCharacteristic,
    pub comparison: ObjectFilter,
    /// Authored comparison-set surface, such as
    /// "cards exiled with this creature".
    pub comparison_surface: Option<String>,
}

impl CostReductionCharacteristicIntersection {
    pub fn new(characteristic: crate::ObjectCharacteristic, comparison: ObjectFilter) -> Self {
        Self {
            characteristic,
            comparison,
            comparison_surface: None,
        }
    }

    pub fn with_comparison_surface(mut self, surface: impl Into<String>) -> Self {
        self.comparison_surface = Some(surface.into());
        self
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
/// The condition guarding this static ability, over whatever vocabulary the
/// phase using it speaks. Defaults to the resolved [`Condition`], so the
/// runtime spells it the way it always has.
pub struct CostReduction<Cond = Condition> {
    pub filter: ObjectFilter,
    pub amount: Value,
    pub condition: Option<Cond>,
    pub per_target: bool,
    /// Count distinct values of one characteristic shared by the candidate
    /// spell and the comparison set, then reduce by `amount` for each.
    pub characteristic_intersection: Option<CostReductionCharacteristicIntersection>,
}

impl<Cond> CostReduction<Cond> {
    /// Translate this ability's condition into another phase's vocabulary.
    pub fn try_map_condition<Cond2, Err>(
        self,
        map_condition: impl FnOnce(Cond) -> Result<Cond2, Err>,
    ) -> Result<CostReduction<Cond2>, Err> {
        let CostReduction {
            condition,
            filter,
            amount,
            per_target,
            characteristic_intersection,
        } = self;
        Ok(CostReduction {
            condition: condition.map(map_condition).transpose()?,
            filter,
            amount,
            per_target,
            characteristic_intersection,
        })
    }
}

impl<Cond: ConditionConjunction> CostReduction<Cond> {
    pub fn new(filter: ObjectFilter, amount: Value) -> Self {
        Self {
            filter,
            amount,
            condition: None,
            per_target: false,
            characteristic_intersection: None,
        }
    }

    pub fn with_condition(mut self, condition: Cond) -> Self {
        self.condition = Some(match self.condition.take() {
            Some(existing) => existing.and(condition),
            None => condition,
        });
        self
    }

    pub fn with_per_target(mut self) -> Self {
        self.per_target = true;
        self
    }

    pub fn with_characteristic_intersection(
        mut self,
        intersection: CostReductionCharacteristicIntersection,
    ) -> Self {
        self.characteristic_intersection = Some(intersection);
        self
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct OptionalLifeAdditionalCost {
    pub label: String,
    pub life_cost: u32,
}

impl OptionalLifeAdditionalCost {
    pub fn new(label: impl Into<String>, life_cost: u32) -> Self {
        Self {
            label: label.into(),
            life_cost,
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
/// The condition guarding this static ability, over whatever vocabulary the
/// phase using it speaks. Defaults to the resolved [`Condition`], so the
/// runtime spells it the way it always has.
pub struct CostReductionManaCost<Cond = Condition> {
    pub filter: ObjectFilter,
    pub cost: ManaCost,
    pub condition: Option<Cond>,
    pub per_target: bool,
    pub optional_life_additional_cost: Option<OptionalLifeAdditionalCost>,
}

impl<Cond> CostReductionManaCost<Cond> {
    /// Translate this ability's condition into another phase's vocabulary.
    pub fn try_map_condition<Cond2, Err>(
        self,
        map_condition: impl FnOnce(Cond) -> Result<Cond2, Err>,
    ) -> Result<CostReductionManaCost<Cond2>, Err> {
        let CostReductionManaCost {
            condition,
            filter,
            cost,
            per_target,
            optional_life_additional_cost,
        } = self;
        Ok(CostReductionManaCost {
            condition: condition.map(map_condition).transpose()?,
            filter,
            cost,
            per_target,
            optional_life_additional_cost,
        })
    }
}

impl<Cond: ConditionConjunction> CostReductionManaCost<Cond> {
    pub fn new(filter: ObjectFilter, cost: ManaCost) -> Self {
        Self {
            filter,
            cost,
            condition: None,
            per_target: false,
            optional_life_additional_cost: None,
        }
    }

    pub fn with_optional_life_additional_cost(
        mut self,
        label: impl Into<String>,
        life_cost: u32,
    ) -> Self {
        self.optional_life_additional_cost =
            Some(OptionalLifeAdditionalCost::new(label, life_cost));
        self
    }
    pub fn with_condition(mut self, condition: Cond) -> Self {
        self.condition = Some(match self.condition.take() {
            Some(existing) => existing.and(condition),
            None => condition,
        });
        self
    }

    pub fn with_per_target(mut self) -> Self {
        self.per_target = true;
        self
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
/// The condition guarding this static ability, over whatever vocabulary the
/// phase using it speaks. Defaults to the resolved [`Condition`], so the
/// runtime spells it the way it always has.
pub struct CostIncrease<Cond = Condition> {
    pub filter: ObjectFilter,
    pub amount: Value,
    pub condition: Option<Cond>,
    pub per_target: bool,
}

impl<Cond> CostIncrease<Cond> {
    /// Translate this ability's condition into another phase's vocabulary.
    pub fn try_map_condition<Cond2, Err>(
        self,
        map_condition: impl FnOnce(Cond) -> Result<Cond2, Err>,
    ) -> Result<CostIncrease<Cond2>, Err> {
        let CostIncrease {
            condition,
            filter,
            amount,
            per_target,
        } = self;
        Ok(CostIncrease {
            condition: condition.map(map_condition).transpose()?,
            filter,
            amount,
            per_target,
        })
    }
}

impl<Cond: ConditionConjunction> CostIncrease<Cond> {
    pub fn new(filter: ObjectFilter, amount: Value) -> Self {
        Self {
            filter,
            amount,
            condition: None,
            per_target: false,
        }
    }
    pub fn with_condition(mut self, condition: Cond) -> Self {
        self.condition = Some(match self.condition.take() {
            Some(existing) => existing.and(condition),
            None => condition,
        });
        self
    }

    pub fn with_per_target(mut self) -> Self {
        self.per_target = true;
        self
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
/// The condition guarding this static ability, over whatever vocabulary the
/// phase using it speaks. Defaults to the resolved [`Condition`], so the
/// runtime spells it the way it always has.
pub struct CostIncreaseManaCost<Cond = Condition> {
    pub filter: ObjectFilter,
    pub cost: ManaCost,
    pub condition: Option<Cond>,
    pub per_target: bool,
}

impl<Cond> CostIncreaseManaCost<Cond> {
    /// Translate this ability's condition into another phase's vocabulary.
    pub fn try_map_condition<Cond2, Err>(
        self,
        map_condition: impl FnOnce(Cond) -> Result<Cond2, Err>,
    ) -> Result<CostIncreaseManaCost<Cond2>, Err> {
        let CostIncreaseManaCost {
            condition,
            filter,
            cost,
            per_target,
        } = self;
        Ok(CostIncreaseManaCost {
            condition: condition.map(map_condition).transpose()?,
            filter,
            cost,
            per_target,
        })
    }
}

impl<Cond: ConditionConjunction> CostIncreaseManaCost<Cond> {
    pub fn new(filter: ObjectFilter, cost: ManaCost) -> Self {
        Self {
            filter,
            cost,
            condition: None,
            per_target: false,
        }
    }
    pub fn with_condition(mut self, condition: Cond) -> Self {
        self.condition = Some(match self.condition.take() {
            Some(existing) => existing.and(condition),
            None => condition,
        });
        self
    }

    pub fn with_per_target(mut self) -> Self {
        self.per_target = true;
        self
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct ThisSpellCostReduction<Cond> {
    pub amount: Value,
    pub condition: Cond,
    pub affinity_filter: Option<ObjectFilter>,
    /// Limits this self-reduction to one alternative casting method.
    ///
    /// This is distinct from an object filter because the ability lives on the
    /// spell being cast and must inspect the casting choice currently being
    /// evaluated (for example, "less to cast this way" after flashback).
    pub alternative_cast: Option<crate::AlternativeCastKind>,
}

impl<Cond> ThisSpellCostReduction<Cond> {
    pub fn new(amount: Value, condition: Cond) -> Self {
        Self {
            amount,
            condition,
            affinity_filter: None,
            alternative_cast: None,
        }
    }

    pub fn with_affinity_filter(mut self, filter: ObjectFilter) -> Self {
        self.affinity_filter = Some(filter);
        self
    }

    pub fn with_alternative_cast(mut self, kind: crate::AlternativeCastKind) -> Self {
        self.alternative_cast = Some(kind);
        self
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct ThisSpellCostReductionManaCost<Cond> {
    pub cost: ManaCost,
    pub repetitions: Option<Value>,
    pub condition: Cond,
}

impl<Cond> ThisSpellCostReductionManaCost<Cond> {
    pub fn new(cost: ManaCost, condition: Cond) -> Self {
        Self {
            cost,
            repetitions: None,
            condition,
        }
    }

    pub fn with_repetitions(mut self, repetitions: Value) -> Self {
        self.repetitions = Some(repetitions);
        self
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct SetColorsForFilter {
    pub filter: ObjectFilter,
    pub color: ColorSet,
}

impl SetColorsForFilter {
    pub fn new(filter: ObjectFilter, color: ColorSet) -> Self {
        Self { filter, color }
    }
    pub fn with_condition<C>(self, _condition: C) -> Self {
        self
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
/// The condition guarding this static ability, over whatever vocabulary the
/// phase using it speaks. Defaults to the resolved [`Condition`], so the
/// runtime spells it the way it always has.
pub struct RemoveCardTypesForFilter<Cond = Condition> {
    pub filter: ObjectFilter,
    pub types: Vec<CardType>,
    pub condition: Option<Cond>,
}

impl<Cond> RemoveCardTypesForFilter<Cond> {
    /// Translate this ability's condition into another phase's vocabulary.
    pub fn try_map_condition<Cond2, Err>(
        self,
        map_condition: impl FnOnce(Cond) -> Result<Cond2, Err>,
    ) -> Result<RemoveCardTypesForFilter<Cond2>, Err> {
        let RemoveCardTypesForFilter {
            condition,
            filter,
            types,
        } = self;
        Ok(RemoveCardTypesForFilter {
            condition: condition.map(map_condition).transpose()?,
            filter,
            types,
        })
    }
}

impl<Cond: ConditionConjunction> RemoveCardTypesForFilter<Cond> {
    pub fn new(filter: ObjectFilter, types: Vec<CardType>) -> Self {
        Self {
            filter,
            types,
            condition: None,
        }
    }
    pub fn with_condition(mut self, condition: Cond) -> Self {
        self.condition = Some(condition);
        self
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub enum ActivatedAbilityCostCondition {
    TargetsExactly { count: usize, filter: ObjectFilter },
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub enum AttackCostCondition {
    PayGenericPerSourceCounter {
        counter_type: CounterType,
        amount_per_counter: u32,
    },
    ReturnPermanentsToOwnersHand {
        filter: ObjectFilter,
        count: u32,
    },
    SacrificePermanents {
        filter: ObjectFilter,
        count: u32,
    },
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub enum AttackingGroupAttackCondition {
    AtLeastNOtherCreaturesAttack(u32),
    BlackOrGreenCreatureAlsoAttacks,
    CreatureWithGreaterPowerAlsoAttacks,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[expect(
    clippy::large_enum_variant,
    reason = "defending-player conditions preserve typed object filters inline"
)]
pub enum DefendingPlayerAttackCondition {
    Controls(ObjectFilter),
    ControlsEnchantmentOrEnchantedPermanent,
    HasCardsInGraveyardOrMore(u32),
    IsMonarch,
    IsPoisoned,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[expect(
    clippy::large_enum_variant,
    reason = "attack restrictions preserve typed conditions and filters inline"
)]
pub enum CantAttackUnlessConditionSpec<Cond = Condition> {
    AttackCost(AttackCostCondition),
    AttackingGroupCondition(AttackingGroupAttackCondition),
    BattlefieldCountAtLeast { filter: ObjectFilter, count: u32 },
    ControllerControlsMoreThanDefendingPlayer(ObjectFilter),
    ControllerGraveyardHasCardsAtLeast(u32),
    DefendingPlayerCondition(DefendingPlayerAttackCondition),
    OpponentWasDealtDamageThisTurn,
    SourceCondition(Cond),
}
impl<Cond> CantAttackUnlessConditionSpec<Cond> {
    /// Translate the source condition into another phase's vocabulary.
    pub fn try_map_condition<Cond2, Err>(
        self,
        map_condition: impl FnOnce(Cond) -> Result<Cond2, Err>,
    ) -> Result<CantAttackUnlessConditionSpec<Cond2>, Err> {
        use CantAttackUnlessConditionSpec as Spec;
        Ok(match self {
            Spec::SourceCondition(condition) => Spec::SourceCondition(map_condition(condition)?),
            Spec::AttackCost(cost) => Spec::AttackCost(cost),
            Spec::AttackingGroupCondition(group) => Spec::AttackingGroupCondition(group),
            Spec::BattlefieldCountAtLeast { filter, count } => {
                Spec::BattlefieldCountAtLeast { filter, count }
            }
            Spec::ControllerControlsMoreThanDefendingPlayer(filter) => {
                Spec::ControllerControlsMoreThanDefendingPlayer(filter)
            }
            Spec::ControllerGraveyardHasCardsAtLeast(count) => {
                Spec::ControllerGraveyardHasCardsAtLeast(count)
            }
            Spec::DefendingPlayerCondition(condition) => Spec::DefendingPlayerCondition(condition),
            Spec::OpponentWasDealtDamageThisTurn => Spec::OpponentWasDealtDamageThisTurn,
        })
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct EnterAsCopyAsEntersSpec<T, E, C, Cond, ICond = Condition> {
    pub filter: ObjectFilter,
    pub affected_filter: Option<ObjectFilter>,
    pub may: bool,
    pub enters_tapped_if_chosen: bool,
    /// If present, the copied characteristics expire at this duration instead
    /// of replacing the entering object's copiable values permanently.
    pub copy_duration: Option<Until>,
    pub linked_exile_pair: Option<EnterAsCopyLinkedExilePairSpec>,
    pub copy_source_self: bool,
    pub copy_source_enchanted: bool,
    pub name_override: Option<String>,
    pub added_colors: ColorSet,
    pub added_card_types: Vec<CardType>,
    pub removed_supertypes: Vec<Supertype>,
    pub added_subtypes: Vec<Subtype>,
    pub added_abilities: Vec<AbilityModel<T, E, C, Cond, ICond>>,
    pub set_base_power_toughness: Option<(i32, i32)>,
    pub set_base_power_toughness_from_self: bool,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnterAsCopyLinkedExilePairSpec {
    pub counter_type: CounterType,
}

impl<T, E, C, Cond, ICond> crate::GrantStaticAbility for StaticAbility<T, E, C, Cond, ICond>
where
    T: Clone + PartialEq + std::fmt::Debug + 'static,
    E: Clone + PartialEq + std::fmt::Debug + 'static,
    C: Clone + PartialEq + std::fmt::Debug + 'static,
    Cond: Clone + PartialEq + std::fmt::Debug + 'static,
    ICond: Clone + PartialEq + std::fmt::Debug + ConditionConjunction + 'static,
{
    fn grant_flash() -> Self {
        Self::flash()
    }

    fn grant_display(&self) -> String {
        self.display()
    }

    fn grant_has_flash(&self) -> bool {
        self.id() == StaticAbilityId::Flash
    }
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
