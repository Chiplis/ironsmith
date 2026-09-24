//! Madness's linked triggered ability (CR 702.35a).
//!
//! "When this card is exiled this way, its owner may cast it by paying [cost]
//! rather than paying its mana cost. If that player doesn't, they put this
//! card into their graveyard."
//!
//! The trigger is created when the madness replacement exiles a discarded
//! card. The spell is cast while it resolves through the normal cast
//! pipeline, so it's a real cast (cast triggers, X, the "Madness" paid label)
//! and it then waits on the stack like any other spell.

use crate::alternative_cast::CastingMethod;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::zone::Zone;

use super::runtime_helpers::with_spell_cast_event;

/// Resolves a madness trigger whose source is the exiled card.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MayCastForMadnessCostEffect;

impl MayCastForMadnessCostEffect {
    pub fn new() -> Self {
        Self
    }
}

fn put_madness_card_into_graveyard(game: &mut GameState, card_id: crate::ids::ObjectId) {
    game.clear_madness_exiled(card_id);
    game.move_object(
        card_id,
        Zone::Graveyard,
        crate::events::cause::EventCause::from_game_rule(),
    );
}

impl EffectExecutor for MayCastForMadnessCostEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let card_id = ctx.source;
        // The card must still be the object the madness replacement exiled.
        let Some(card) = game.object(card_id) else {
            return Ok(EffectOutcome::target_invalid());
        };
        if card.zone != Zone::Exile || !game.is_madness_exiled(card_id) {
            return Ok(EffectOutcome::target_invalid());
        }
        let owner = card.owner;
        let Some((madness_index, madness_cost)) = card
            .alternative_casts
            .iter()
            .enumerate()
            .find_map(|(index, method)| match method {
                crate::alternative_cast::AlternativeCastingMethod::Madness { total_cost } => {
                    Some((index, total_cost.clone()))
                }
                _ => None,
            })
        else {
            put_madness_card_into_graveyard(game, card_id);
            return Ok(EffectOutcome::resolved());
        };

        let wants_to_cast = crate::decisions::make_decision(
            game,
            ctx.decision_maker,
            owner,
            Some(card_id),
            crate::decisions::specs::MadnessSpec::new(card_id, madness_cost),
        );
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        if !wants_to_cast {
            put_madness_card_into_graveyard(game, card_id);
            return Ok(EffectOutcome::resolved());
        }

        let casting_method = CastingMethod::Alternative(madness_index);
        game.authorize_madness_cast(card_id);
        let result = crate::game_loop::cast_spell_from_resolving_effect(
            game,
            card_id,
            Zone::Exile,
            owner,
            &casting_method,
            false,
            None,
            ctx.provenance,
            &mut ctx.decision_maker,
        );
        game.revoke_madness_cast(card_id);
        let result = result.map_err(|error| ExecutionError::Impossible(error.to_string()))?;
        if let Some(new_id) = result {
            return Ok(with_spell_cast_event(
                EffectOutcome::with_objects(vec![new_id]),
                game,
                new_id,
                owner,
                Zone::Exile,
                ctx.provenance,
            ));
        }
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        // The cast didn't happen (for example, no legal targets or the cost
        // couldn't be paid), so the card goes to its owner's graveyard.
        if game
            .object(card_id)
            .is_some_and(|object| object.zone == Zone::Exile)
        {
            put_madness_card_into_graveyard(game, card_id);
        }
        Ok(EffectOutcome::resolved())
    }
}
