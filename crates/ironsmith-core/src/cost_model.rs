use crate::types::CardType;
use crate::{ColorSet, CounterType, ManaCost, ObjectFilter, Value};

#[derive(Debug, Clone, PartialEq)]
pub enum Cost<E> {
    Mana(ManaCost),
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
            } => Cost::RemoveAnyCountersFromSource {
                counter_type,
                display_x,
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
            } => format!("remove any counters {counter_type:?} {display_x}"),
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
        matches!(self, Self::Mana(_))
    }

    fn mana_cost_ref(&self) -> Option<&ManaCost> {
        match self {
            Self::Mana(cost) => Some(cost),
            _ => None,
        }
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

    fn exile_from_hand_details(&self) -> Option<(u32, Option<ColorSet>)> {
        None
    }

    fn mana_cost_ref(&self) -> Option<&ManaCost> {
        None
    }
}

pub trait CoreCostComponent: CostComponent {
    fn tap_cost() -> Self;
}

#[derive(Debug, Clone, PartialEq)]
pub struct TotalCost<C> {
    costs: Vec<C>,
}

impl<C> TotalCost<C> {
    pub fn free() -> Self {
        Self { costs: vec![] }
    }

    pub fn from_cost(cost: C) -> Self {
        Self { costs: vec![cost] }
    }

    pub fn from_costs(costs: Vec<C>) -> Self {
        Self { costs }
    }

    pub fn costs(&self) -> &[C] {
        &self.costs
    }

    pub fn try_map<C2, Error>(
        self,
        mut map_cost: impl FnMut(C) -> Result<C2, Error>,
    ) -> Result<TotalCost<C2>, Error> {
        let mut costs = Vec::with_capacity(self.costs.len());
        for cost in self.costs {
            costs.push(map_cost(cost)?);
        }
        Ok(TotalCost::from_costs(costs))
    }

    pub fn is_free(&self) -> bool {
        self.costs.is_empty()
    }
}

impl<C> Default for TotalCost<C> {
    fn default() -> Self {
        Self { costs: Vec::new() }
    }
}

impl<C: CostComponent> TotalCost<C> {
    pub fn mana(mana_cost: ManaCost) -> Self {
        Self::from_cost(C::mana(mana_cost))
    }

    pub fn non_mana_costs(&self) -> impl Iterator<Item = &C> {
        self.costs.iter().filter(|cost| !cost.is_mana_cost())
    }

    pub fn has_non_mana_costs(&self) -> bool {
        self.non_mana_costs().next().is_some()
    }

    pub fn display(&self) -> String {
        if self.costs.is_empty() {
            return "Free".to_string();
        }
        let parts: Vec<String> = self
            .costs
            .iter()
            .map(|c| c.display())
            .filter(|part| !part.trim().is_empty())
            .collect();
        if parts.is_empty() {
            return "Free".to_string();
        }
        parts.join(", ")
    }

    pub fn mana_cost(&self) -> Option<&ManaCost> {
        self.costs.iter().find_map(|c| c.mana_cost_ref())
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
        stored == query
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
