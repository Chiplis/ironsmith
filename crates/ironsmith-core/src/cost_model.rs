use crate::{ColorSet, ManaCost};

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
