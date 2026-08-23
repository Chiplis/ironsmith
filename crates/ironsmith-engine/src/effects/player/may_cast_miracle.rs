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
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::other::CardsDrawnEvent;
use crate::game_state::GameState;
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

        let card_name = obj.name.to_string();

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
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }

        if !wants_to_cast {
            // Player chose not to cast - card stays in hand
            return Ok(EffectOutcome::resolved());
        }

        let casting_method = CastingMethod::Alternative(miracle_index);
        let result = crate::game_loop::cast_spell_from_resolving_effect(
            game,
            card_id,
            Zone::Hand,
            owner,
            &casting_method,
            false,
            None,
            ctx.provenance,
            &mut ctx.decision_maker,
        )
        .map_err(|error| ExecutionError::Impossible(error.to_string()))?;
        if let Some(new_id) = result {
            Ok(with_spell_cast_event(
                EffectOutcome::with_objects(vec![new_id]),
                game,
                new_id,
                owner,
                Zone::Hand,
                ctx.provenance,
            ))
        } else if ctx.decision_maker.awaiting_choice() {
            Ok(EffectOutcome::count(0))
        } else {
            Ok(EffectOutcome::impossible())
        }
    }
}
