//! Cost system for abilities and spells.
//!
//! Costs represent what must be paid to cast a spell or activate an ability.
//! A total cost is a conjunction of individual costs that must all be paid.
//!
//! The main types are:
//! - `TotalCost`: A complete cost (conjunction of Cost components)
//! - `Cost` (in the `costs` module): Individual cost components (trait objects)

use crate::costs::Cost;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::mana::ManaCost;
use crate::object::CounterType;
use crate::target::ChooseSpec;
pub type TotalCost = ironsmith_core::TotalCost<Cost>;
pub type OptionalCost = ironsmith_core::OptionalCost<Cost>;
pub type OptionalCostsPaid = ironsmith_core::OptionalCostsPaid;

impl ironsmith_core::CostComponent for Cost {
    fn mana(mana_cost: ManaCost) -> Self {
        Self::mana(mana_cost)
    }

    fn display(&self) -> String {
        self.display()
    }

    fn is_mana_cost(&self) -> bool {
        self.is_mana_cost()
    }

    fn requires_tap(&self) -> bool {
        self.requires_tap()
    }

    fn life_amount(&self) -> Option<u32> {
        self.life_amount()
    }

    fn is_sacrifice_self(&self) -> bool {
        self.is_sacrifice_self()
    }

    fn exile_from_hand_details(&self) -> Option<(u32, Option<crate::color::ColorSet>)> {
        self.exile_from_hand_details()
    }

    fn mana_cost_ref(&self) -> Option<&ManaCost> {
        self.mana_cost_ref()
    }

    fn dynamic_mana_cost_ref(&self) -> Option<&ironsmith_core::DynamicManaCost> {
        self.dynamic_mana_cost_ref()
    }

    fn is_loyalty_activation_cost(&self) -> bool {
        fn is_source(spec: &ChooseSpec) -> bool {
            matches!(spec.base(), ChooseSpec::Source)
        }

        self.effect_ref().is_some_and(|effect| {
            effect
                .downcast_ref::<crate::effects::PutCountersEffect>()
                .is_some_and(|put| {
                    put.counter_type == CounterType::Loyalty && is_source(&put.target)
                })
                || effect
                    .downcast_ref::<crate::effects::RemoveCountersEffect>()
                    .is_some_and(|remove| {
                        remove.counter_type == CounterType::Loyalty && is_source(&remove.target)
                    })
                || effect
                    .downcast_ref::<crate::effects::RemoveAnyCountersFromSourceEffect>()
                    .is_some_and(|remove| remove.counter_type == Some(CounterType::Loyalty))
        })
    }
}

impl ironsmith_core::CoreCostComponent for Cost {
    fn tap_cost() -> Self {
        Self::tap()
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

impl std::fmt::Display for CostPaymentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CostPaymentError::SourceNotFound => f.write_str("Source object not found"),
            CostPaymentError::PlayerNotFound => f.write_str("Player not found"),
            CostPaymentError::InsufficientMana => f.write_str("Not enough mana"),
            CostPaymentError::AlreadyTapped => f.write_str("That permanent is already tapped"),
            CostPaymentError::SummoningSickness => {
                f.write_str("That creature has summoning sickness")
            }
            CostPaymentError::AlreadyUntapped => f.write_str("That permanent is already untapped"),
            CostPaymentError::InsufficientLife => f.write_str("Not enough life"),
            CostPaymentError::SourceNotOnBattlefield => {
                f.write_str("The source is not on the battlefield")
            }
            CostPaymentError::NoValidSacrificeTarget => {
                f.write_str("No valid permanent can be sacrificed")
            }
            CostPaymentError::InsufficientCardsInHand => f.write_str("Not enough cards in hand"),
            CostPaymentError::InsufficientCounters => {
                f.write_str("Not enough counters on the source")
            }
            CostPaymentError::InsufficientEnergy => f.write_str("Not enough energy counters"),
            CostPaymentError::InsufficientCardsToExile => {
                f.write_str("Not enough cards in hand to exile")
            }
            CostPaymentError::InsufficientCardsInGraveyard => {
                f.write_str("Not enough cards in the graveyard")
            }
            CostPaymentError::NoValidReturnTarget => {
                f.write_str("No valid permanent can be returned to hand")
            }
            CostPaymentError::InsufficientCardsToReveal => {
                f.write_str("Not enough cards in hand to reveal")
            }
            CostPaymentError::Other(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for CostPaymentError {}

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
    can_pay_cost_with_reason(
        game,
        source_id,
        player,
        cost,
        crate::costs::PaymentReason::Other,
    )
}

pub fn can_pay_cost_with_reason(
    game: &GameState,
    source_id: ObjectId,
    player: PlayerId,
    cost: &TotalCost,
    reason: crate::costs::PaymentReason,
) -> Result<(), CostPaymentError> {
    use crate::costs::{CostCheckContext, can_pay_with_check_context};

    let ctx = CostCheckContext::new(source_id, player).with_reason(reason);

    match cost.kind() {
        ironsmith_core::TotalCostKind::All(costs) => {
            for cost_component in costs {
                let adjusted_component =
                    adjusted_component_for_check(game, player, source_id, cost_component, reason)?;
                game.validate_cost_for_payment_reason(
                    player,
                    source_id,
                    &adjusted_component,
                    reason,
                )?;
                can_pay_with_check_context(&*adjusted_component.0, game, &ctx)?;
            }
            Ok(())
        }
        ironsmith_core::TotalCostKind::OneOf(branches) => {
            if branches.iter().any(|branch| {
                can_pay_cost_with_reason(game, source_id, player, branch, reason).is_ok()
            }) {
                Ok(())
            } else {
                Err(CostPaymentError::Other(
                    "no payable alternative cost branch".to_string(),
                ))
            }
        }
    }
}

pub(crate) fn adjusted_component_for_check(
    game: &GameState,
    player: PlayerId,
    source_id: ObjectId,
    cost_component: &crate::costs::Cost,
    reason: crate::costs::PaymentReason,
) -> Result<crate::costs::Cost, CostPaymentError> {
    if let Some(mana_cost) = cost_component.mana_cost_ref() {
        return Ok(crate::costs::Cost::mana(
            game.adjust_mana_cost_for_payment_reason(player, Some(source_id), mana_cost, reason),
        ));
    }
    if let Some(dynamic_mana) = cost_component.dynamic_mana_cost_ref() {
        if let Some(static_base) = dynamic_mana.resolved_static_base() {
            return Ok(crate::costs::Cost::mana(
                game.adjust_mana_cost_for_payment_reason(
                    player,
                    Some(source_id),
                    &static_base,
                    reason,
                ),
            ));
        }
        return Err(CostPaymentError::Other(
            "dynamic mana cost requires an execution context".to_string(),
        ));
    }
    Ok(cost_component.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mana::ManaSymbol;

    #[test]
    fn test_free_cost() {
        let cost = TotalCost::free();
        assert!(cost.is_free());
        assert!(cost.mana_cost().is_none());
        assert!(!cost.has_non_mana_costs());
    }

    #[test]
    fn test_mana_cost() {
        let mana = ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)], vec![ManaSymbol::White]]);
        let cost = TotalCost::mana(mana.clone());

        assert!(!cost.is_free());
        assert_eq!(cost.mana_cost(), Some(&mana));
        assert!(!cost.has_non_mana_costs());
    }

    #[test]
    fn gift_prefix_lookup_matches_descriptive_gift_label() {
        let paid = OptionalCostsPaid {
            costs: vec![("Gift a tapped Fish".to_string(), 1)],
        };

        assert!(paid.was_paid_label("Gift"));
        assert_eq!(paid.times_paid_label("Gift"), 1);
    }
}
