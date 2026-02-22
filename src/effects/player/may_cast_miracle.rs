//! May cast for miracle cost effect implementation.
//!
//! This effect is used by Miracle triggers to present the player with the choice
//! to cast the spell for its miracle cost.
//!
//! This effect uses the triggering event (CardsDrawnEvent) to find the card
//! that was drawn. This is more robust than storing card_id/owner because
//! it automatically handles zone changes.

use crate::effect::{EffectOutcome, EffectResult};
use crate::effects::EffectExecutor;
use crate::events::other::CardsDrawnEvent;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::{GameState, StackEntry};
use crate::zone::Zone;

/// Effect that allows casting a spell for its miracle cost.
///
/// When this effect resolves, it presents the player with a choice to cast
/// the spell for its miracle cost. If they choose yes and can pay the cost,
/// the spell is cast.
///
/// This effect gets the card and owner from the triggering CardsDrawnEvent.
/// The miracle card must be the first card in the event (is_miracle_eligible).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MayCastForMiracleCostEffect;

impl MayCastForMiracleCostEffect {
    /// Create a new may cast for miracle cost effect.
    pub fn new() -> Self {
        Self
    }
}

impl EffectExecutor for MayCastForMiracleCostEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        use crate::alternative_cast::CastingMethod;
        use crate::cost::OptionalCostsPaid;

        // Get card_id and owner from the triggering CardsDrawnEvent
        let Some(ref triggering_event) = ctx.triggering_event else {
            return Err(ExecutionError::Impossible(
                "MayCastForMiracleCostEffect requires a triggering event".to_string(),
            ));
        };

        let Some(drawn) = triggering_event.downcast::<CardsDrawnEvent>() else {
            return Err(ExecutionError::Impossible(
                "MayCastForMiracleCostEffect requires a CardsDrawnEvent".to_string(),
            ));
        };

        // Get the first card drawn (miracle only works on the first card)
        let Some(card_id) = drawn.first_card() else {
            return Ok(EffectOutcome::from_result(EffectResult::Impossible));
        };
        let owner = drawn.player;

        // Verify the card is still in hand
        let obj = game.object(card_id).ok_or(ExecutionError::InvalidTarget)?;

        if obj.zone != Zone::Hand {
            // Card is no longer in hand (may have been discarded or played)
            return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid));
        }

        // Get the miracle cost
        let miracle_cost = obj
            .alternative_casts
            .iter()
            .find_map(|alt| alt.miracle_cost().cloned());

        let Some(miracle_cost) = miracle_cost else {
            // Card doesn't have miracle (shouldn't happen)
            return Ok(EffectOutcome::from_result(EffectResult::Impossible));
        };

        // Find the miracle alternative cast index
        let miracle_index = obj
            .alternative_casts
            .iter()
            .position(|alt| alt.is_miracle());

        let Some(miracle_index) = miracle_index else {
            return Ok(EffectOutcome::from_result(EffectResult::Impossible));
        };

        let card_name = obj.name.clone();

        // Ask the player if they want to cast for miracle cost
        let bool_ctx = crate::decisions::context::BooleanContext::new(
            owner,
            Some(card_id),
            format!(
                "Cast {} for its miracle cost ({:?})?",
                card_name, miracle_cost
            ),
        )
        .with_source_name(&card_name);

        let wants_to_cast = ctx.decision_maker.decide_boolean(game, &bool_ctx);

        if !wants_to_cast {
            // Player chose not to cast - card stays in hand
            return Ok(EffectOutcome::resolved());
        }

        // Player wants to cast for miracle cost
        // Check if they can pay the miracle cost
        let can_pay = game.can_pay_mana_cost(owner, None, &miracle_cost, 0);

        if !can_pay {
            // Can't pay the cost - card stays in hand
            // In a full implementation, we'd let them tap lands, but for simplicity
            // we check current mana pool
            return Ok(EffectOutcome::resolved());
        }

        // Pay the miracle cost
        let _ = game.try_pay_mana_cost(owner, None, &miracle_cost, 0);

        // Get stable_id before moving
        let stable_id = game.object(card_id).map(|o| o.stable_id);

        // Move spell from hand to stack
        if let Some(new_id) = game.move_object(card_id, Zone::Stack) {
            // Create stack entry with miracle casting method
            let stack_entry = StackEntry {
                object_id: new_id,
                controller: owner,
                targets: vec![],
                x_value: None,
                ability_effects: None,
                is_ability: false,
                casting_method: CastingMethod::Alternative(miracle_index),
                optional_costs_paid: OptionalCostsPaid::default(),
                defending_player: None,
                saga_final_chapter_source: None,
                source_stable_id: stable_id,
                source_snapshot: None,
                source_name: Some(card_name),
                triggering_event: None,
                intervening_if: None,
                keyword_payment_contributions: vec![],
                crew_contributors: vec![],
                chosen_modes: None,
            };

            game.push_to_stack(stack_entry);
            Ok(EffectOutcome::from_result(EffectResult::Objects(vec![
                new_id,
            ])))
        } else {
            Ok(EffectOutcome::from_result(EffectResult::Impossible))
        }
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_may_cast_miracle_effect_creation() {
        let effect = MayCastForMiracleCostEffect::new();
        // Effect now has no fields - it gets card_id and owner from triggering event
        assert_eq!(effect, MayCastForMiracleCostEffect);
    }

    #[test]
    fn test_clone_box() {
        let effect = MayCastForMiracleCostEffect::new();
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("MayCastForMiracleCostEffect"));
    }
}
