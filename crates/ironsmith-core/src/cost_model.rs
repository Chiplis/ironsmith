use crate::types::CardType;
use crate::{ColorSet, CounterType, ManaCost, ObjectFilter, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DynamicManaDisplayHint {
    Default,
    ManaEqualTo,
}

impl Default for DynamicManaDisplayHint {
    fn default() -> Self {
        Self::Default
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DynamicManaCost {
    pub base: ManaCost,
    pub x_value: Option<Value>,
    pub additional_generic: Option<Value>,
    pub multiplier: Option<Value>,
    pub display_hint: DynamicManaDisplayHint,
}

impl DynamicManaCost {
    pub fn new(
        base: ManaCost,
        x_value: Option<Value>,
        additional_generic: Option<Value>,
        multiplier: Option<Value>,
        display_hint: DynamicManaDisplayHint,
    ) -> Self {
        Self {
            base,
            x_value,
            additional_generic,
            multiplier,
            display_hint,
        }
    }

    pub fn from_x(base: ManaCost, x_value: Value) -> Self {
        Self::new(
            base,
            Some(x_value),
            None,
            None,
            DynamicManaDisplayHint::Default,
        )
    }

    pub fn generic_equal_to(value: Value) -> Self {
        Self::new(
            ManaCost::new(),
            None,
            Some(value),
            None,
            DynamicManaDisplayHint::ManaEqualTo,
        )
    }

    pub fn resolved_static_base(&self) -> Option<ManaCost> {
        if self.x_value.is_none() && self.additional_generic.is_none() && self.multiplier.is_none()
        {
            return Some(self.base.clone());
        }
        None
    }

    pub fn display(&self) -> String {
        match self.display_hint {
            DynamicManaDisplayHint::ManaEqualTo => {
                if let Some(value) = self.additional_generic.as_ref() {
                    return format!("mana equal to {value:?}");
                }
            }
            DynamicManaDisplayHint::Default => {}
        }

        let mut text = if self.base.is_empty() {
            "{0}".to_string()
        } else {
            self.base.to_oracle()
        };
        if let Some(value) = self.x_value.as_ref() {
            text.push_str(&format!(", where X is {value:?}"));
        }
        if let Some(value) = self.additional_generic.as_ref() {
            text.push_str(&format!(" plus an additional {{{value:?}}}"));
        }
        if let Some(value) = self.multiplier.as_ref() {
            text.push_str(&format!(" for each {value:?}"));
        }
        text
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Cost<E> {
    Mana(ManaCost),
    DynamicMana(DynamicManaCost),
    Tap,
    Untap,
    DiscardSource,
    SacrificeSelf,
    Sacrifice(ObjectFilter),
    Discard {
        count: u32,
        card_types: Vec<CardType>,
    },
    DiscardHand,
    RemoveCounters {
        counter_type: CounterType,
        count: u32,
    },
    AddCounters {
        counter_type: CounterType,
        count: u32,
    },
    RemoveAnyCountersFromSource {
        counter_type: Option<CounterType>,
        display_x: bool,
        remove_all: bool,
    },
    Energy(Value),
    Mill(Value),
    Life(Value),
    ExileSelf,
    ExileFromHand {
        count: u32,
        color_filter: Option<ColorSet>,
    },
    ReturnSelfToHand,
    Effect(E),
}

impl<E> Cost<E> {
    pub fn effect_ref(&self) -> Option<&E> {
        match self {
            Self::Effect(effect) => Some(effect),
            _ => None,
        }
    }

    pub fn validated_effect(effect: E) -> Self {
        Self::Effect(effect)
    }

    pub fn try_from_runtime_effect(effect: E) -> Result<Self, String> {
        Ok(Self::Effect(effect))
    }

    pub fn mana(mana_cost: ManaCost) -> Self {
        Self::Mana(mana_cost)
    }

    pub fn dynamic_mana(dynamic_mana: DynamicManaCost) -> Self {
        Self::DynamicMana(dynamic_mana)
    }

    pub fn discard_source() -> Self {
        Self::DiscardSource
    }

    pub fn sacrifice_self() -> Self {
        Self::SacrificeSelf
    }

    pub fn sacrifice(filter: ObjectFilter) -> Self {
        Self::Sacrifice(filter)
    }

    pub fn effect(effect: impl Into<E>) -> Self {
        Self::Effect(effect.into())
    }

    pub fn discard(count: u32, card_type: Option<CardType>) -> Self {
        Self::Discard {
            count,
            card_types: card_type.into_iter().collect(),
        }
    }

    pub fn discard_types(count: u32, card_types: Vec<CardType>) -> Self {
        Self::Discard { count, card_types }
    }

    pub fn remove_counters(counter_type: CounterType, count: u32) -> Self {
        Self::RemoveCounters {
            counter_type,
            count,
        }
    }

    pub fn add_counters(counter_type: CounterType, count: u32) -> Self {
        Self::AddCounters {
            counter_type,
            count,
        }
    }

    pub fn remove_any_counters_from_source(
        counter_type: Option<CounterType>,
        display_x: bool,
    ) -> Self {
        Self::RemoveAnyCountersFromSource {
            counter_type,
            display_x,
            remove_all: false,
        }
    }

    pub fn remove_all_counters_from_source(counter_type: Option<CounterType>) -> Self {
        Self::RemoveAnyCountersFromSource {
            counter_type,
            display_x: false,
            remove_all: true,
        }
    }

    pub fn discard_hand() -> Self {
        Self::DiscardHand
    }

    pub fn tap() -> Self {
        Self::Tap
    }

    pub fn untap() -> Self {
        Self::Untap
    }

    pub fn life(amount: impl Into<Value>) -> Self {
        Self::Life(amount.into())
    }

    pub fn energy(amount: impl Into<Value>) -> Self {
        Self::Energy(amount.into())
    }

    pub fn mill(count: impl Into<Value>) -> Self {
        Self::Mill(count.into())
    }

    pub fn exile_self() -> Self {
        Self::ExileSelf
    }

    pub fn exile_from_hand(count: u32, color_filter: Option<ColorSet>) -> Self {
        Self::ExileFromHand {
            count,
            color_filter,
        }
    }

    pub fn return_self_to_hand() -> Self {
        Self::ReturnSelfToHand
    }

    pub fn try_map_effect<E2, Error>(
        self,
        mut map_effect: impl FnMut(E) -> Result<E2, Error>,
    ) -> Result<Cost<E2>, Error> {
        Ok(match self {
            Self::Mana(mana) => Cost::Mana(mana),
            Self::DynamicMana(dynamic_mana) => Cost::DynamicMana(dynamic_mana),
            Self::Tap => Cost::Tap,
            Self::Untap => Cost::Untap,
            Self::DiscardSource => Cost::DiscardSource,
            Self::SacrificeSelf => Cost::SacrificeSelf,
            Self::Sacrifice(filter) => Cost::Sacrifice(filter),
            Self::Discard { count, card_types } => Cost::Discard { count, card_types },
            Self::DiscardHand => Cost::DiscardHand,
            Self::RemoveCounters {
                counter_type,
                count,
            } => Cost::RemoveCounters {
                counter_type,
                count,
            },
            Self::AddCounters {
                counter_type,
                count,
            } => Cost::AddCounters {
                counter_type,
                count,
            },
            Self::RemoveAnyCountersFromSource {
                counter_type,
                display_x,
                remove_all,
            } => Cost::RemoveAnyCountersFromSource {
                counter_type,
                display_x,
                remove_all,
            },
            Self::Energy(amount) => Cost::Energy(amount),
            Self::Mill(count) => Cost::Mill(count),
            Self::Life(amount) => Cost::Life(amount),
            Self::ExileSelf => Cost::ExileSelf,
            Self::ExileFromHand {
                count,
                color_filter,
            } => Cost::ExileFromHand {
                count,
                color_filter,
            },
            Self::ReturnSelfToHand => Cost::ReturnSelfToHand,
            Self::Effect(effect) => Cost::Effect(map_effect(effect)?),
        })
    }
}

impl<E> CostComponent for Cost<E>
where
    E: Clone + std::fmt::Debug + PartialEq,
{
    fn mana(mana_cost: ManaCost) -> Self {
        Self::Mana(mana_cost)
    }

    fn display(&self) -> String {
        match self {
            Self::Mana(cost) => cost.to_oracle(),
            Self::DynamicMana(cost) => cost.display(),
            Self::Tap => "{T}".to_string(),
            Self::Untap => "{Q}".to_string(),
            Self::DiscardSource => "discard this card".to_string(),
            Self::SacrificeSelf => "sacrifice this permanent".to_string(),
            Self::Sacrifice(_) => "sacrifice a permanent".to_string(),
            Self::Discard { count, .. } => format!("discard {count}"),
            Self::DiscardHand => "discard your hand".to_string(),
            Self::RemoveCounters {
                counter_type,
                count,
            } => format!("remove {count} {counter_type:?} counters"),
            Self::AddCounters {
                counter_type,
                count,
            } => format!("put {count} {counter_type:?} counters"),
            Self::RemoveAnyCountersFromSource {
                counter_type,
                display_x,
                remove_all,
            } => format!("remove any counters {counter_type:?} {display_x} {remove_all}"),
            Self::Energy(amount) => format!("pay {amount:?} energy"),
            Self::Mill(count) => format!("mill {count:?}"),
            Self::Life(amount) => format!("pay {amount:?} life"),
            Self::ExileSelf => "exile this card".to_string(),
            Self::ExileFromHand { count, .. } => format!("exile {count} card(s) from hand"),
            Self::ReturnSelfToHand => "return this permanent to its owner's hand".to_string(),
            Self::Effect(_) => "effect".to_string(),
        }
    }

    fn is_mana_cost(&self) -> bool {
        matches!(self, Self::Mana(_) | Self::DynamicMana(_))
    }

    fn mana_cost_ref(&self) -> Option<&ManaCost> {
        match self {
            Self::Mana(cost) => Some(cost),
            _ => None,
        }
    }

    fn dynamic_mana_cost_ref(&self) -> Option<&DynamicManaCost> {
        match self {
            Self::DynamicMana(cost) => Some(cost),
            _ => None,
        }
    }

    fn is_loyalty_activation_cost(&self) -> bool {
        matches!(
            self,
            Self::RemoveCounters {
                counter_type: CounterType::Loyalty,
                ..
            } | Self::AddCounters {
                counter_type: CounterType::Loyalty,
                ..
            } | Self::RemoveAnyCountersFromSource {
                counter_type: Some(CounterType::Loyalty),
                ..
            }
        )
    }
}

impl<E> CoreCostComponent for Cost<E>
where
    E: Clone + std::fmt::Debug + PartialEq,
{
    fn tap_cost() -> Self {
        Self::Tap
    }
}

pub trait CostComponent: Clone + std::fmt::Debug + PartialEq {
    fn mana(mana_cost: ManaCost) -> Self;

    fn display(&self) -> String;

    fn is_mana_cost(&self) -> bool {
        false
    }

    fn requires_tap(&self) -> bool {
        false
    }

    fn life_amount(&self) -> Option<u32> {
        None
    }

    fn is_sacrifice_self(&self) -> bool {
        false
    }

    fn is_loyalty_activation_cost(&self) -> bool {
        false
    }

    fn exile_from_hand_details(&self) -> Option<(u32, Option<ColorSet>)> {
        None
    }

    fn mana_cost_ref(&self) -> Option<&ManaCost> {
        None
    }

    fn dynamic_mana_cost_ref(&self) -> Option<&DynamicManaCost> {
        None
    }
}

pub trait CoreCostComponent: CostComponent {
    fn tap_cost() -> Self;
}

#[derive(Debug, Clone, PartialEq)]
pub enum TotalCostKind<C> {
    All(Vec<C>),
    OneOf(Vec<TotalCost<C>>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TotalCost<C> {
    kind: TotalCostKind<C>,
}

impl<C> TotalCost<C> {
    pub fn free() -> Self {
        Self::from_costs(vec![])
    }

    pub fn from_cost(cost: C) -> Self {
        Self::from_costs(vec![cost])
    }

    pub fn from_costs(costs: Vec<C>) -> Self {
        Self {
            kind: TotalCostKind::All(costs),
        }
    }

    pub fn one_of(branches: Vec<TotalCost<C>>) -> Self {
        Self {
            kind: TotalCostKind::OneOf(branches),
        }
    }

    pub fn kind(&self) -> &TotalCostKind<C> {
        &self.kind
    }

    pub fn as_all(&self) -> Option<&[C]> {
        match &self.kind {
            TotalCostKind::All(costs) => Some(costs),
            TotalCostKind::OneOf(_) => None,
        }
    }

    pub fn as_one_of(&self) -> Option<&[TotalCost<C>]> {
        match &self.kind {
            TotalCostKind::All(_) => None,
            TotalCostKind::OneOf(branches) => Some(branches),
        }
    }

    pub fn costs(&self) -> &[C] {
        self.as_all()
            .expect("TotalCost::costs called for an alternative cost")
    }

    pub fn try_map<C2, Error>(
        self,
        mut map_cost: impl FnMut(C) -> Result<C2, Error>,
    ) -> Result<TotalCost<C2>, Error> {
        self.try_map_with(&mut map_cost)
    }

    fn try_map_with<C2, Error>(
        self,
        map_cost: &mut impl FnMut(C) -> Result<C2, Error>,
    ) -> Result<TotalCost<C2>, Error> {
        match self.kind {
            TotalCostKind::All(all) => {
                let mut costs = Vec::with_capacity(all.len());
                for cost in all {
                    costs.push(map_cost(cost)?);
                }
                Ok(TotalCost::from_costs(costs))
            }
            TotalCostKind::OneOf(branches) => {
                let mut mapped = Vec::with_capacity(branches.len());
                for branch in branches {
                    mapped.push(branch.try_map_with(map_cost)?);
                }
                Ok(TotalCost::one_of(mapped))
            }
        }
    }

    pub fn is_free(&self) -> bool {
        match &self.kind {
            TotalCostKind::All(costs) => costs.is_empty(),
            TotalCostKind::OneOf(branches) => branches.iter().any(Self::is_free),
        }
    }
}

impl<C> Default for TotalCost<C> {
    fn default() -> Self {
        Self::free()
    }
}

impl<C: CostComponent> TotalCost<C> {
    pub fn mana(mana_cost: ManaCost) -> Self {
        Self::from_cost(C::mana(mana_cost))
    }

    pub fn non_mana_costs(&self) -> impl Iterator<Item = &C> {
        self.costs().iter().filter(|cost| !cost.is_mana_cost())
    }

    pub fn has_non_mana_costs(&self) -> bool {
        match &self.kind {
            TotalCostKind::All(costs) => costs.iter().any(|cost| !cost.is_mana_cost()),
            TotalCostKind::OneOf(branches) => branches.iter().any(Self::has_non_mana_costs),
        }
    }

    pub fn has_loyalty_activation_cost(&self) -> bool {
        match &self.kind {
            TotalCostKind::All(costs) => {
                costs.iter().any(CostComponent::is_loyalty_activation_cost)
            }
            TotalCostKind::OneOf(branches) => {
                branches.iter().any(Self::has_loyalty_activation_cost)
            }
        }
    }

    pub fn display(&self) -> String {
        match &self.kind {
            TotalCostKind::All(costs) => {
                if costs.is_empty() {
                    return "Free".to_string();
                }
                let parts: Vec<String> = costs
                    .iter()
                    .map(|c| c.display())
                    .filter(|part| !part.trim().is_empty())
                    .collect();
                if parts.is_empty() {
                    return "Free".to_string();
                }
                parts.join(", ")
            }
            TotalCostKind::OneOf(branches) => {
                let parts: Vec<String> = branches.iter().map(Self::display).collect();
                if parts.is_empty() {
                    "Free".to_string()
                } else {
                    parts.join(" or ")
                }
            }
        }
    }

    pub fn mana_cost(&self) -> Option<&ManaCost> {
        match &self.kind {
            TotalCostKind::All(costs) => costs.iter().find_map(|c| c.mana_cost_ref()),
            TotalCostKind::OneOf(_) => None,
        }
    }

    pub fn dynamic_mana_cost(&self) -> Option<&DynamicManaCost> {
        match &self.kind {
            TotalCostKind::All(costs) => costs.iter().find_map(|c| c.dynamic_mana_cost_ref()),
            TotalCostKind::OneOf(_) => None,
        }
    }
}

impl<C: CostComponent> From<ManaCost> for TotalCost<C> {
    fn from(mana_cost: ManaCost) -> Self {
        Self::mana(mana_cost)
    }
}

impl<C> From<C> for TotalCost<C> {
    fn from(cost: C) -> Self {
        Self::from_cost(cost)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ManaSymbol;

    #[test]
    fn total_cost_all_display_and_free() {
        let cost: TotalCost<Cost<()>> = TotalCost::from_costs(vec![
            Cost::mana(ManaCost::from_symbols(vec![ManaSymbol::Generic(2)])),
            Cost::life(Value::Fixed(3)),
        ]);

        assert!(!cost.is_free());
        assert_eq!(cost.display(), "{2}, pay Fixed(3) life");
        assert!(cost.has_non_mana_costs());
        assert_eq!(
            cost.mana_cost().map(ManaCost::to_oracle),
            Some("{2}".into())
        );
    }

    #[test]
    fn total_cost_one_of_display_and_introspection() {
        let cost: TotalCost<Cost<()>> = TotalCost::one_of(vec![
            TotalCost::mana(ManaCost::from_symbols(vec![ManaSymbol::Black])),
            TotalCost::mana(ManaCost::from_symbols(vec![ManaSymbol::Generic(3)])),
        ]);

        assert!(!cost.is_free());
        assert!(cost.as_all().is_none());
        assert_eq!(cost.as_one_of().map(<[_]>::len), Some(2));
        assert_eq!(cost.display(), "{B} or {3}");
        assert!(cost.mana_cost().is_none());
    }

    #[test]
    fn total_cost_try_map_recurses_through_alternatives() {
        let cost: TotalCost<Cost<&'static str>> = TotalCost::one_of(vec![
            TotalCost::from_cost(Cost::effect("a")),
            TotalCost::from_cost(Cost::effect("b")),
        ]);

        let mapped = cost
            .try_map(|component| component.try_map_effect(|effect| Ok::<_, ()>(effect.len())))
            .unwrap();

        assert_eq!(
            mapped,
            TotalCost::one_of(vec![
                TotalCost::from_cost(Cost::effect(1usize)),
                TotalCost::from_cost(Cost::effect(1usize)),
            ])
        );
    }

    #[test]
    fn dynamic_mana_is_a_cost_component() {
        let dynamic =
            DynamicManaCost::from_x(ManaCost::from_symbols(vec![ManaSymbol::X]), Value::Fixed(4));
        let cost: Cost<()> = Cost::dynamic_mana(dynamic.clone());
        assert!(cost.is_mana_cost());
        assert_eq!(cost.dynamic_mana_cost_ref(), Some(&dynamic));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptionalCost<C> {
    pub label: String,
    pub cost: TotalCost<C>,
    pub repeatable: bool,
    pub returns_to_hand: bool,
}

impl<C> OptionalCost<C> {
    pub fn kicker(cost: TotalCost<C>) -> Self {
        Self {
            label: "Kicker".to_string(),
            cost,
            repeatable: false,
            returns_to_hand: false,
        }
    }

    pub fn multikicker(cost: TotalCost<C>) -> Self {
        Self {
            label: "Multikicker".to_string(),
            cost,
            repeatable: true,
            returns_to_hand: false,
        }
    }

    pub fn replicate(cost: TotalCost<C>) -> Self {
        Self {
            label: "Replicate".to_string(),
            cost,
            repeatable: true,
            returns_to_hand: false,
        }
    }

    pub fn buyback(cost: TotalCost<C>) -> Self {
        Self {
            label: "Buyback".to_string(),
            cost,
            repeatable: false,
            returns_to_hand: true,
        }
    }

    pub fn entwine(cost: TotalCost<C>) -> Self {
        Self {
            label: "Entwine".to_string(),
            cost,
            repeatable: false,
            returns_to_hand: false,
        }
    }

    pub fn squad(cost: TotalCost<C>) -> Self {
        Self {
            label: "Squad".to_string(),
            cost,
            repeatable: true,
            returns_to_hand: false,
        }
    }

    pub fn offspring(cost: TotalCost<C>) -> Self {
        Self {
            label: "Offspring".to_string(),
            cost,
            repeatable: false,
            returns_to_hand: false,
        }
    }

    pub fn custom(label: impl Into<String>, cost: TotalCost<C>) -> Self {
        Self {
            label: label.into(),
            cost,
            repeatable: false,
            returns_to_hand: false,
        }
    }

    pub fn repeatable(mut self) -> Self {
        self.repeatable = true;
        self
    }

    pub fn returns_to_hand(mut self) -> Self {
        self.returns_to_hand = true;
        self
    }

    pub fn try_map<C2, Error>(
        self,
        map_cost: impl FnMut(C) -> Result<C2, Error>,
    ) -> Result<OptionalCost<C2>, Error> {
        Ok(OptionalCost {
            label: self.label,
            cost: self.cost.try_map(map_cost)?,
            repeatable: self.repeatable,
            returns_to_hand: self.returns_to_hand,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OptionalCostsPaid {
    pub costs: Vec<(String, u32)>,
}

impl OptionalCostsPaid {
    fn label_matches_query(stored: &str, query: &str) -> bool {
        stored.eq_ignore_ascii_case(query)
            || (query.eq_ignore_ascii_case("Kicker")
                && stored.to_ascii_lowercase().starts_with("kicker "))
            || (query.eq_ignore_ascii_case("Gift")
                && stored.to_ascii_lowercase().starts_with("gift "))
            || (query.eq_ignore_ascii_case("Conspire")
                && stored.to_ascii_lowercase().starts_with("conspire"))
            || (query.eq_ignore_ascii_case("Behold")
                && stored
                    .to_ascii_lowercase()
                    .starts_with("as an additional cost to cast this spell, you may behold "))
    }

    pub fn new(num_optional_costs: usize) -> Self {
        Self {
            costs: vec![("".to_string(), 0); num_optional_costs],
        }
    }

    pub fn from_costs<C>(costs: &[OptionalCost<C>]) -> Self {
        Self {
            costs: costs.iter().map(|c| (c.label.clone(), 0)).collect(),
        }
    }

    pub fn any_paid(&self) -> bool {
        self.costs.iter().any(|(_, n)| *n > 0)
    }

    pub fn was_paid(&self, index: usize) -> bool {
        self.costs.get(index).map(|(_, n)| *n > 0).unwrap_or(false)
    }

    pub fn was_paid_label(&self, label: &str) -> bool {
        self.costs
            .iter()
            .any(|(l, n)| Self::label_matches_query(l, label) && *n > 0)
    }

    pub fn times_paid(&self, index: usize) -> u32 {
        self.costs.get(index).map(|(_, n)| *n).unwrap_or(0)
    }

    pub fn times_paid_label(&self, label: &str) -> u32 {
        self.costs
            .iter()
            .filter(|(l, _)| Self::label_matches_query(l, label))
            .map(|(_, n)| *n)
            .sum()
    }

    pub fn pay(&mut self, index: usize) {
        if let Some((_, times)) = self.costs.get_mut(index) {
            *times += 1;
        }
    }

    pub fn pay_times(&mut self, index: usize, times: u32) {
        if let Some((_, t)) = self.costs.get_mut(index) {
            *t += times;
        }
    }

    pub fn pay_label(&mut self, label: &str) {
        if let Some((_, times)) = self.costs.iter_mut().find(|(l, _)| *l == label) {
            *times += 1;
        }
    }

    pub fn mark_label_paid(&mut self, label: &str) {
        if let Some((_, times)) = self.costs.iter_mut().find(|(l, _)| *l == label) {
            *times += 1;
        } else {
            self.costs.push((label.to_string(), 1));
        }
    }

    pub fn was_kicked(&self) -> bool {
        self.was_paid_label("Kicker") || self.was_paid_label("Multikicker")
    }

    pub fn kick_count(&self) -> u32 {
        self.times_paid_label("Kicker") + self.times_paid_label("Multikicker")
    }

    pub fn was_bought_back(&self) -> bool {
        self.was_paid_label("Buyback")
    }

    pub fn was_entwined(&self) -> bool {
        self.was_paid_label("Entwine")
    }
}
