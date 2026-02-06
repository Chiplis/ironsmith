//! Targeting system types.

use crate::cost::TotalCost;
use crate::ids::{ObjectId, PlayerId};

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

/// The type of cost imposed by ward.
#[derive(Debug, Clone, PartialEq)]
pub enum WardCost {
    /// Pay a mana cost (the most common ward type).
    Mana(TotalCost),
    /// Pay life (e.g., Ward—Pay 3 life).
    Life(u32),
    /// Discard cards (e.g., Ward—Discard a card).
    Discard(u32),
    /// Sacrifice a permanent matching a filter (e.g., Ward—Sacrifice a creature).
    Sacrifice(crate::target::ObjectFilter),
}

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
