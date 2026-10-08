//! "Target creature attacks <player> this turn if able" (CR 508.1d).
use crate::effect::EffectOutcome;
use crate::effects::helpers::{resolve_objects_for_effect, resolve_player_from_spec};
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;
use crate::zone::Zone;

pub use ironsmith_core::MustAttackPlayerThisTurnEffect;

impl EffectExecutor for MustAttackPlayerThisTurnEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let objects = match resolve_objects_for_effect(game, ctx, &self.target) {
            Ok(objects) => objects,
            Err(ExecutionError::InvalidTarget) => return Ok(EffectOutcome::target_invalid()),
            Err(error) => return Err(error),
        };
        if self.controllers_next_combat {
            let mut count = 0;
            for id in objects {
                if !game
                    .object(id)
                    .is_some_and(|object| object.zone == Zone::Battlefield)
                    || !game.current_is_creature(id)
                {
                    continue;
                }
                let Some(controller) = game.controller_of_id(id) else {
                    continue;
                };
                // CR 508.1d: the requirement applies during the controller's
                // next combat phase and is spent when that combat ends.
                game.effect_store
                    .next_combat_attack_requirements
                    .push((id, controller));
                count += 1;
            }
            return Ok(EffectOutcome::count(count));
        }
        let player = match resolve_player_from_spec(game, &self.player, ctx) {
            Ok(player) => player,
            Err(ExecutionError::InvalidTarget) => return Ok(EffectOutcome::target_invalid()),
            Err(error) => return Err(error),
        };
        let turn = game.turn.turn_number;
        let mut count = 0;
        for id in objects {
            if !game
                .object(id)
                .is_some_and(|object| object.zone == Zone::Battlefield)
                || !game.current_is_creature(id)
            {
                continue;
            }
            // Read by attack-requirement scoring (CR 508.1d) for this turn's
            // declarations and cleared during cleanup.
            game.effect_store
                .attack_player_requirements
                .push((id, player, turn));
            count += 1;
        }
        Ok(EffectOutcome::count(count))
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "creature that must attack"
    }
}
