//! Cost system for abilities and spells.
//!
//! Costs represent what must be paid to cast a spell or activate an ability.
//! A total cost is a conjunction of individual costs that must all be paid.
//!
//! The main types are:
//! - `TotalCost`: A complete cost (conjunction of Cost components)
//! - `Cost` (in the `costs` module): Individual cost components (trait objects)
//! - `PermanentFilter`: Filter for selecting permanents in sacrifice costs

use crate::costs::Cost;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::mana::ManaCost;
use crate::types::{CardType, Subtype};

/// Filter for selecting permanents (used in sacrifice costs, etc.)
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PermanentFilter {
    /// Required card types (permanent must have at least one)
    pub card_types: Vec<CardType>,

    /// Required subtypes (permanent must have at least one, if non-empty)
    pub subtypes: Vec<Subtype>,

    /// If true, must be a permanent you control other than the source
    pub other: bool,

    /// If true, must be a token
    pub token: bool,

    /// If true, must be a nontoken
    pub nontoken: bool,
}

impl PermanentFilter {
    /// Create a filter for any permanent.
    pub fn any() -> Self {
        Self::default()
    }

    /// Create a filter for creatures.
    pub fn creature() -> Self {
        Self {
            card_types: vec![CardType::Creature],
            ..Default::default()
        }
    }

    /// Create a filter for artifacts.
    pub fn artifact() -> Self {
        Self {
            card_types: vec![CardType::Artifact],
            ..Default::default()
        }
    }

    /// Create a filter for enchantments.
    pub fn enchantment() -> Self {
        Self {
            card_types: vec![CardType::Enchantment],
            ..Default::default()
        }
    }

    /// Create a filter for lands.
    pub fn land() -> Self {
        Self {
            card_types: vec![CardType::Land],
            ..Default::default()
        }
    }

    /// Require the permanent to be "another" (not the source).
    pub fn other(mut self) -> Self {
        self.other = true;
        self
    }

    /// Require the permanent to be a token.
    pub fn token(mut self) -> Self {
        self.token = true;
        self
    }

    /// Require the permanent to be a nontoken.
    pub fn nontoken(mut self) -> Self {
        self.nontoken = true;
        self
    }

    /// Add a required card type.
    pub fn with_type(mut self, card_type: CardType) -> Self {
        self.card_types.push(card_type);
        self
    }

    /// Add a required subtype.
    pub fn with_subtype(mut self, subtype: Subtype) -> Self {
        self.subtypes.push(subtype);
        self
    }
}

/// A complete cost that must be paid (conjunction of individual costs).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TotalCost {
    costs: Vec<Cost>,
}

impl TotalCost {
    /// Create an empty cost (free).
    pub fn free() -> Self {
        Self { costs: vec![] }
    }

    /// Create a cost from a single Cost component.
    pub fn from_cost(cost: Cost) -> Self {
        Self { costs: vec![cost] }
    }

    /// Create a mana-only cost.
    pub fn mana(mana_cost: ManaCost) -> Self {
        Self::from_cost(Cost::mana(mana_cost))
    }

    /// Create a cost from multiple Cost components.
    pub fn from_costs(costs: Vec<Cost>) -> Self {
        Self { costs }
    }

    /// Get the individual cost components.
    pub fn costs(&self) -> &[Cost] {
        &self.costs
    }

    /// Get a human-readable display of this cost.
    pub fn display(&self) -> String {
        if self.costs.is_empty() {
            return "Free".to_string();
        }
        self.costs
            .iter()
            .map(|c| c.display())
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Check if this is a free cost (no components).
    pub fn is_free(&self) -> bool {
        self.costs.is_empty()
    }

    /// Get the mana cost component, if any.
    pub fn mana_cost(&self) -> Option<&ManaCost> {
        self.costs.iter().find_map(|c| c.mana_cost_ref())
    }
}

impl From<ManaCost> for TotalCost {
    fn from(mana_cost: ManaCost) -> Self {
        Self::mana(mana_cost)
    }
}

impl From<Cost> for TotalCost {
    fn from(cost: Cost) -> Self {
        Self::from_cost(cost)
    }
}

// ============================================================================
// Optional Costs (Kicker, Buyback, Entwine, etc.)
// ============================================================================

/// An optional cost that can be paid when casting a spell.
///
/// Examples:
/// - Kicker {2}{R} (pay once for additional effect)
/// - Multikicker {1}{G} (pay any number of times)
/// - Buyback {3} (pay to return spell to hand)
/// - Entwine {2} (pay to get both modes of a modal spell)
#[derive(Debug, Clone, PartialEq)]
pub struct OptionalCost {
    /// Label shown to player (e.g., "Kicker", "Buyback", "Multikicker")
    pub label: &'static str,

    /// The cost to pay for this optional cost
    pub cost: TotalCost,

    /// Can this be paid multiple times? (Multikicker, Replicate)
    pub repeatable: bool,

    /// If true, spell returns to hand instead of graveyard after resolution (Buyback)
    pub returns_to_hand: bool,
}

impl OptionalCost {
    /// Create a simple kicker cost.
    pub fn kicker(cost: TotalCost) -> Self {
        Self {
            label: "Kicker",
            cost,
            repeatable: false,
            returns_to_hand: false,
        }
    }

    /// Create a multikicker cost (can be paid any number of times).
    pub fn multikicker(cost: TotalCost) -> Self {
        Self {
            label: "Multikicker",
            cost,
            repeatable: true,
            returns_to_hand: false,
        }
    }

    /// Create a buyback cost (spell returns to hand).
    pub fn buyback(cost: TotalCost) -> Self {
        Self {
            label: "Buyback",
            cost,
            repeatable: false,
            returns_to_hand: true,
        }
    }

    /// Create an entwine cost (for modal spells, choose all modes).
    pub fn entwine(cost: TotalCost) -> Self {
        Self {
            label: "Entwine",
            cost,
            repeatable: false,
            returns_to_hand: false,
        }
    }

    /// Create a custom optional cost with a specific label.
    pub fn custom(label: &'static str, cost: TotalCost) -> Self {
        Self {
            label,
            cost,
            repeatable: false,
            returns_to_hand: false,
        }
    }

    /// Make this cost repeatable.
    pub fn repeatable(mut self) -> Self {
        self.repeatable = true;
        self
    }

    /// Make this cost return the spell to hand.
    pub fn returns_to_hand(mut self) -> Self {
        self.returns_to_hand = true;
        self
    }
}

/// Tracks which optional costs were paid during casting.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OptionalCostsPaid {
    /// For each optional cost: (label, times_paid)
    pub costs: Vec<(&'static str, u32)>,
}

impl OptionalCostsPaid {
    /// Create a new tracker with no costs paid.
    pub fn new(num_optional_costs: usize) -> Self {
        Self {
            costs: vec![("", 0); num_optional_costs],
        }
    }

    /// Create a tracker from a list of optional costs.
    pub fn from_costs(costs: &[OptionalCost]) -> Self {
        Self {
            costs: costs.iter().map(|c| (c.label, 0)).collect(),
        }
    }

    /// Check if any optional cost was paid.
    pub fn any_paid(&self) -> bool {
        self.costs.iter().any(|(_, n)| *n > 0)
    }

    /// Check if the optional cost at the given index was paid at least once.
    pub fn was_paid(&self, index: usize) -> bool {
        self.costs.get(index).map(|(_, n)| *n > 0).unwrap_or(false)
    }

    /// Check if the optional cost with the given label was paid.
    pub fn was_paid_label(&self, label: &str) -> bool {
        self.costs.iter().any(|(l, n)| *l == label && *n > 0)
    }

    /// Get the number of times the optional cost at the given index was paid.
    pub fn times_paid(&self, index: usize) -> u32 {
        self.costs.get(index).map(|(_, n)| *n).unwrap_or(0)
    }

    /// Get the number of times the optional cost with the given label was paid.
    pub fn times_paid_label(&self, label: &str) -> u32 {
        self.costs
            .iter()
            .find(|(l, _)| *l == label)
            .map(|(_, n)| *n)
            .unwrap_or(0)
    }

    /// Record that an optional cost was paid once.
    pub fn pay(&mut self, index: usize) {
        if let Some((_, times)) = self.costs.get_mut(index) {
            *times += 1;
        }
    }

    /// Record that an optional cost was paid N times.
    pub fn pay_times(&mut self, index: usize, times: u32) {
        if let Some((_, t)) = self.costs.get_mut(index) {
            *t += times;
        }
    }

    /// Record that an optional cost with the given label was paid once.
    pub fn pay_label(&mut self, label: &str) {
        if let Some((_, times)) = self.costs.iter_mut().find(|(l, _)| *l == label) {
            *times += 1;
        }
    }

    /// Check if the cost labeled "Kicker" was paid.
    pub fn was_kicked(&self) -> bool {
        self.was_paid_label("Kicker") || self.was_paid_label("Multikicker")
    }

    /// Get the total number of times the kicker was paid (for multikicker).
    pub fn kick_count(&self) -> u32 {
        self.times_paid_label("Kicker") + self.times_paid_label("Multikicker")
    }

    /// Check if buyback was paid.
    pub fn was_bought_back(&self) -> bool {
        self.was_paid_label("Buyback")
    }

    /// Check if entwine was paid.
    pub fn was_entwined(&self) -> bool {
        self.was_paid_label("Entwine")
    }
}

// ============================================================================
// Cost Payment Validation
// ============================================================================

/// Error type for when a cost cannot be paid.
#[derive(Debug, Clone, PartialEq)]
pub enum CostPaymentError {
    /// The source object doesn't exist.
    SourceNotFound,

    /// The player doesn't exist.
    PlayerNotFound,

    /// Not enough mana to pay the mana cost.
    InsufficientMana,

    /// Can't tap - permanent is already tapped.
    AlreadyTapped,

    /// Can't tap - creature has summoning sickness (rule 302.6).
    SummoningSickness,

    /// Can't untap - permanent is already untapped.
    AlreadyUntapped,

    /// Not enough life to pay the life cost.
    InsufficientLife,

    /// Source not on battlefield (for sacrifice/exile self).
    SourceNotOnBattlefield,

    /// No valid permanent to sacrifice.
    NoValidSacrificeTarget,

    /// Not enough cards in hand to discard.
    InsufficientCardsInHand,

    /// Not enough counters on the source.
    InsufficientCounters,

    /// Not enough energy counters.
    InsufficientEnergy,

    /// Not enough cards in hand matching the filter for exile.
    InsufficientCardsToExile,

    /// Not enough cards in graveyard matching the filter.
    InsufficientCardsInGraveyard,

    /// No valid permanent to return to hand.
    NoValidReturnTarget,

    /// Not enough cards in hand to reveal.
    InsufficientCardsToReveal,

    /// Generic/other failure while validating or paying a cost.
    Other(String),
}

/// Check if a player can pay an activated ability's or spell's cost.
///
/// This checks all cost components against the current game state.
/// The `source_id` is the permanent or spell whose cost is being paid.
pub fn can_pay_cost(
    game: &GameState,
    source_id: ObjectId,
    player: PlayerId,
    cost: &TotalCost,
) -> Result<(), CostPaymentError> {
    use crate::costs::{CostCheckContext, can_pay_with_check_context};

    let ctx = CostCheckContext::new(source_id, player);

    for cost_component in cost.costs() {
        can_pay_with_check_context(&*cost_component.0, game, &ctx)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mana::ManaSymbol;

    #[test]
    fn test_free_cost() {
        let cost = TotalCost::free();
        assert!(cost.is_free());
        // Note: tap is now in cost_effects, not TotalCost
        assert!(cost.mana_cost().is_none());
    }

    #[test]
    fn test_mana_cost() {
        let mana = ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)], vec![ManaSymbol::White]]);
        let cost = TotalCost::mana(mana.clone());

        assert!(!cost.is_free());
        // Note: tap is now in cost_effects, not TotalCost
        assert_eq!(cost.mana_cost(), Some(&mana));
    }
}
