//! Targeting system types.

use crate::cost::TotalCost;
use crate::effect::ChoiceAggregateMetric;
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId};

/// A target-set aggregate restriction with its dynamic maximum and candidate
/// contributions resolved for the current announcement.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedTargetAggregateConstraint {
    pub metric: ChoiceAggregateMetric,
    pub maximum: i32,
    pub target_values: Vec<(Target, i32)>,
    /// "up to one target card of each card type": each selected target must
    /// occupy a different card-type slot. Holds every legal target's slot
    /// bitmask (bit = `CardType as u32`, restricted to the authored slot
    /// types). Validated incrementally by bipartite matching, never by
    /// enumerating the legal sets.
    pub one_per_card_type: Option<Vec<(Target, u32)>>,
}

impl ResolvedTargetAggregateConstraint {
    /// A target set restricted only by one-target-per-card-type slots.
    pub fn one_per_card_type_only(slot_masks: Vec<(Target, u32)>) -> Self {
        Self {
            metric: ChoiceAggregateMetric::ManaValue,
            maximum: i32::MAX,
            target_values: Vec::new(),
            one_per_card_type: Some(slot_masks),
        }
    }

    fn slot_mask_for(&self, target: Target) -> u32 {
        self.one_per_card_type
            .as_ref()
            .and_then(|masks| {
                masks
                    .iter()
                    .find_map(|(candidate, mask)| (*candidate == target).then_some(*mask))
            })
            .unwrap_or(0)
    }

    /// Whether every selected target can occupy a distinct card-type slot.
    pub fn card_type_slots_allow(&self, targets: &[Target]) -> bool {
        if self.one_per_card_type.is_none() {
            return true;
        }
        let masks = targets
            .iter()
            .map(|target| self.slot_mask_for(*target))
            .collect::<Vec<_>>();
        card_type_slot_matching_size(&masks) == masks.len()
    }

    pub fn value_for(&self, target: Target) -> i32 {
        self.target_values
            .iter()
            .find_map(|(candidate, value)| (*candidate == target).then_some(*value))
            .unwrap_or(0)
    }

    pub fn allows(&self, targets: &[Target]) -> bool {
        self.card_type_slots_allow(targets)
            && aggregate_contributions(
            self.metric,
            targets.iter().map(|target| self.value_for(*target)),
        ) <= self.maximum
    }

    pub fn supports_minimum(&self, minimum: usize) -> bool {
        if minimum == 0 {
            return true;
        }
        if let Some(masks) = &self.one_per_card_type {
            // Matchable target sets form a transversal matroid, so the
            // maximum matching bounds every reachable selection size.
            let masks = masks.iter().map(|(_, mask)| *mask).collect::<Vec<_>>();
            if card_type_slot_matching_size(&masks) < minimum {
                return false;
            }
            if self.target_values.is_empty() {
                return true;
            }
        }
        let mut values = self
            .target_values
            .iter()
            .map(|(_, value)| *value)
            .collect::<Vec<_>>();
        if self.metric == ChoiceAggregateMetric::DistinctCardTypes {
            let mut states = std::collections::HashMap::from([(0i32, 0usize)]);
            for value in values {
                for (mask, count) in states.clone() {
                    let mask = mask | value;
                    if mask.count_ones() as i32 <= self.maximum {
                        let entry = states.entry(mask).or_default();
                        *entry = (*entry).max(count + 1);
                    }
                }
            }
            return states.values().any(|count| *count >= minimum);
        }
        values.sort_unstable();
        values.len() >= minimum && values.into_iter().take(minimum).sum::<i32>() <= self.maximum
    }
}

/// Maximum number of entries that can each be assigned a distinct slot bit
/// from their own mask (augmenting-path bipartite matching over at most 32
/// card-type slots).
pub fn card_type_slot_matching_size(masks: &[u32]) -> usize {
    fn augment(masks: &[u32], entry: usize, visited: &mut u32, owner: &mut [Option<usize>; 32]) -> bool {
        for slot in 0..32usize {
            let bit = 1u32 << slot;
            if masks[entry] & bit == 0 || *visited & bit != 0 {
                continue;
            }
            *visited |= bit;
            let free = match owner[slot] {
                None => true,
                Some(previous) => augment(masks, previous, visited, owner),
            };
            if free {
                owner[slot] = Some(entry);
                return true;
            }
        }
        false
    }
    let mut owner = [None; 32];
    let mut matched = 0;
    for entry in 0..masks.len() {
        let mut visited = 0u32;
        if augment(masks, entry, &mut visited, &mut owner) {
            matched += 1;
        }
    }
    matched
}

/// Contributions are numeric for totals and a type bitset for set union.
pub(crate) fn aggregate_contributions(
    metric: ChoiceAggregateMetric,
    values: impl IntoIterator<Item = i32>,
) -> i32 {
    if metric == ChoiceAggregateMetric::DistinctCardTypes {
        values
            .into_iter()
            .fold(0i32, |mask, value| mask | value)
            .count_ones() as i32
    } else {
        values.into_iter().fold(0i32, i32::saturating_add)
    }
}

/// Evaluate a complete object choice using the same characteristic totals as targeting.
pub fn aggregate_object_set_value(
    game: &GameState,
    ids: impl IntoIterator<Item = ObjectId>,
    metric: ChoiceAggregateMetric,
) -> i32 {
    aggregate_contributions(
        metric,
        ids.into_iter()
            .map(|id| aggregate_object_value(game, id, metric)),
    )
}

/// Evaluate one candidate for an aggregate object or target choice.
pub fn aggregate_object_value(
    game: &GameState,
    id: ObjectId,
    metric: ChoiceAggregateMetric,
) -> i32 {
    let Some(object) = game.object(id) else {
        return 0;
    };
    match metric {
        ChoiceAggregateMetric::Power => game
            .calculated_power(id)
            .or_else(|| object.power())
            .unwrap_or(0),
        ChoiceAggregateMetric::Toughness => game
            .calculated_toughness(id)
            .or_else(|| object.toughness())
            .unwrap_or(0),
        ChoiceAggregateMetric::DistinctCardTypes => game
            .current_card_types(id)
            .into_iter()
            .flatten()
            .fold(0i32, |mask, card_type| mask | (1 << card_type as u32)),
        ChoiceAggregateMetric::ManaValue => object
            .mana_cost
            .as_ref()
            .map_or(0, |cost| cost.mana_value() as i32),
        ChoiceAggregateMetric::ManaSymbolsOfColor(color) => object
            .mana_cost
            .as_ref()
            .map_or(0, |cost| mana_symbols_of_color(cost, color)),
    }
}

/// Mana symbols of one color in a mana cost; a hybrid or Phyrexian pip that
/// includes the color is one symbol of it.
pub fn mana_symbols_of_color(cost: &crate::mana::ManaCost, color: crate::color::Color) -> i32 {
    use crate::mana::ManaSymbol;
    cost.pips()
        .iter()
        .filter(|pip| {
            pip.iter().any(|symbol| {
                matches!(
                    (symbol, color),
                    (ManaSymbol::White, crate::color::Color::White)
                        | (ManaSymbol::Blue, crate::color::Color::Blue)
                        | (ManaSymbol::Black, crate::color::Color::Black)
                        | (ManaSymbol::Red, crate::color::Color::Red)
                        | (ManaSymbol::Green, crate::color::Color::Green)
                )
            })
        })
        .count() as i32
}

/// The result of attempting to target something.
#[derive(Debug, Clone, PartialEq)]
pub enum TargetingResult {
    /// Targeting is legal (optionally with ward costs to pay).
    Legal {
        /// Any ward costs that must be paid for this targeting to proceed.
        ward_costs: Vec<PendingWardCost>,
    },
    /// Targeting is invalid for the given reason.
    Invalid(TargetingInvalidReason),
}

impl TargetingResult {
    /// Create a legal targeting result with no ward costs.
    pub fn legal() -> Self {
        TargetingResult::Legal {
            ward_costs: Vec::new(),
        }
    }

    /// Create a legal targeting result with ward costs.
    pub fn legal_with_ward(costs: Vec<PendingWardCost>) -> Self {
        TargetingResult::Legal { ward_costs: costs }
    }

    /// Returns true if targeting is legal (even if ward must be paid).
    pub fn is_legal(&self) -> bool {
        matches!(self, TargetingResult::Legal { .. })
    }

    /// Returns true if targeting is invalid.
    pub fn is_invalid(&self) -> bool {
        matches!(self, TargetingResult::Invalid(_))
    }

    /// Get ward costs if targeting is legal.
    pub fn ward_costs(&self) -> Option<&[PendingWardCost]> {
        match self {
            TargetingResult::Legal { ward_costs } => Some(ward_costs),
            TargetingResult::Invalid(_) => None,
        }
    }
}

/// Reasons why targeting is invalid.
#[derive(Debug, Clone, PartialEq)]
pub enum TargetingInvalidReason {
    /// Target has shroud (can't be targeted by anything).
    HasShroud,
    /// Target has hexproof and the source's controller is an opponent.
    HasHexproof,
    /// Target has hexproof from sources matching a filter, and the source matches.
    HasHexproofFrom,
    /// Target has protection from the source's quality.
    HasProtection,
    /// Target is in a zone where it can't be targeted.
    WrongZone,
    /// Target doesn't match the required filter.
    DoesntMatchFilter,
    /// Target no longer exists.
    DoesntExist,
    /// Target is not on the battlefield (for permanents).
    NotOnBattlefield,
    /// Player is no longer in the game.
    PlayerNotInGame,
    /// Target has "can't be the target of spells or abilities".
    CantBeTargeted,
}

/// A ward cost that needs to be paid when targeting a permanent.
#[derive(Debug, Clone, PartialEq)]
pub struct PendingWardCost {
    pub source_snapshot: Option<crate::snapshot::ObjectSnapshot>,
    /// The permanent with ward being targeted.
    pub target: ObjectId,
    /// The controller of the permanent with ward.
    pub ward_controller: PlayerId,
    /// The cost that must be paid (may be mana, life, or other costs).
    pub cost: WardCost,
}

/// The total cost imposed by ward.
pub type WardCost = TotalCost;

/// The result of attempting to pay ward costs.
#[derive(Debug, Clone, PartialEq)]
pub enum WardPaymentResult {
    /// All ward costs were paid successfully.
    Paid,
    /// Ward costs were not paid; spell/ability is countered.
    NotPaid,
    /// Paying ward costs is not applicable (no ward on target).
    NotApplicable,
}
