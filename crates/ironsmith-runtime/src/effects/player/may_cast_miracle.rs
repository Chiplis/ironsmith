//! May cast for miracle cost effect implementation.
//!
//! This effect is used by Miracle triggers to present the player with the choice
//! to cast the spell for its miracle cost.
//!
//! This effect uses the triggering event (CardsDrawnEvent) to find the card
//! that was drawn. This is more robust than storing card_id/owner because
//! it automatically handles zone changes.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::events::other::CardsDrawnEvent;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::{GameState, StackEntry};
use crate::zone::Zone;

use super::runtime_helpers::with_spell_cast_event;

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
            return Ok(EffectOutcome::impossible());
        };
        let owner = drawn.player;

        // Verify the card is still in hand
        let obj = game.object(card_id).ok_or(ExecutionError::InvalidTarget)?;

        if obj.zone != Zone::Hand {
            // Card is no longer in hand (may have been discarded or played)
            return Ok(EffectOutcome::target_invalid());
        }

        // Get the miracle cost
        let miracle_cost = obj
            .alternative_casts
            .iter()
            .find_map(|alt| alt.miracle_cost().cloned());

        let Some(miracle_cost) = miracle_cost else {
            // Card doesn't have miracle (shouldn't happen)
            return Ok(EffectOutcome::impossible());
        };

        // Find the miracle alternative cast index
        let miracle_index = obj
            .alternative_casts
            .iter()
            .position(|alt| alt.is_miracle());

        let Some(miracle_index) = miracle_index else {
            return Ok(EffectOutcome::impossible());
        };

        let card_name = obj.name.clone();

        // Ask the player if they want to cast for miracle cost
        let bool_ctx = crate::decisions::context::BooleanContext::new(
            owner,
            Some(card_id),
            format!(
                "Cast {} for its miracle cost ({})?",
                card_name,
                miracle_cost.to_oracle()
            ),
        )
        .with_source_name(&card_name);

        let wants_to_cast = ctx.decision_maker.decide_boolean(game, &bool_ctx);

        if !wants_to_cast {
            // Player chose not to cast - card stays in hand
            return Ok(EffectOutcome::resolved());
        }

        // Player wants to cast for miracle cost.
        let x_value = if miracle_cost.has_x() {
            Some(0u32)
        } else {
            None
        };

        // Try to pay now; if payment fails, card stays in hand.
        let Some(card_obj) = game.object(card_id) else {
            return Ok(EffectOutcome::resolved());
        };
        let effective_cost =
            crate::decision::calculate_effective_mana_cost(game, owner, card_obj, &miracle_cost);
        if !game.try_pay_mana_cost_with_reason(
            owner,
            Some(card_id),
            &effective_cost,
            0,
            crate::costs::PaymentReason::CastSpell,
        ) {
            return Ok(EffectOutcome::resolved());
        }

        // Get stable_id before moving
        let stable_id = game.object(card_id).map(|o| o.stable_id);

        // Move spell from hand to stack
        if let Some(new_id) = game.move_object_by_effect(card_id, Zone::Stack) {
            if let Some(obj) = game.object_mut(new_id) {
                obj.x_value = x_value;
            }
            // Create stack entry with miracle casting method
            let stack_entry = StackEntry {
                object_id: new_id,
                controller: owner,
                provenance: ctx.provenance,
                targets: vec![],
                target_assignments: vec![],
                x_value,
                ability_effects: None,
                is_ability: false,
                casting_method: CastingMethod::Alternative(miracle_index),
                optional_costs_paid: OptionalCostsPaid::default(),
                defending_player: None,
                chosen_player: None,
                saga_final_chapter_source: None,
                source_stable_id: stable_id,
                source_snapshot: None,
                source_name: Some(card_name),
                triggering_event: None,
                intervening_if: None,
                keyword_payment_contributions: vec![],
                crew_contributors: vec![],
                saddle_contributors: vec![],
                chosen_modes: None,
                tagged_objects: std::collections::HashMap::new(),
            };

            game.push_to_stack(stack_entry);
            Ok(with_spell_cast_event(
                EffectOutcome::with_objects(vec![new_id]),
                game,
                new_id,
                owner,
                Zone::Hand,
                ctx.provenance,
            ))
        } else {
            Ok(EffectOutcome::impossible())
        }
    }
}
