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
}

impl ResolvedTargetAggregateConstraint {
    pub fn value_for(&self, target: Target) -> i32 {
        self.target_values
            .iter()
            .find_map(|(candidate, value)| (*candidate == target).then_some(*value))
            .unwrap_or(0)
    }

    pub fn allows(&self, targets: &[Target]) -> bool {
        targets
            .iter()
            .map(|target| self.value_for(*target))
            .sum::<i32>()
            <= self.maximum
    }

    pub fn supports_minimum(&self, minimum: usize) -> bool {
        if minimum == 0 {
            return true;
        }
        let mut values = self
            .target_values
            .iter()
            .map(|(_, value)| *value)
            .collect::<Vec<_>>();
        values.sort_unstable();
        values.len() >= minimum && values.into_iter().take(minimum).sum::<i32>() <= self.maximum
    }
}

pub(crate) fn aggregate_object_value(
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
        ChoiceAggregateMetric::ManaValue => object
            .mana_cost
            .as_ref()
            .map_or(0, |cost| cost.mana_value() as i32),
    }
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
