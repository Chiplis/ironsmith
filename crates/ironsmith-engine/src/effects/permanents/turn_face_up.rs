//! "Turn [the exiled card / it] face up." effect.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::triggers::TriggerEvent;
use crate::zone::Zone;

pub type TurnFaceUpEffect = ironsmith_core::TurnFaceUpEffect;

impl EffectExecutor for TurnFaceUpEffect {
    fn supports_simultaneous_player_action(&self) -> bool {
        true
    }

    fn prepare_simultaneous_player_action(
        &self,
        _game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        Ok(Box::new(crate::effects::DeferredPlayerActionProposal {
            effect: crate::effect::Effect::new(self.clone()),
            iterated_player: ctx.iteration.iterated_player,
        }))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let targets = crate::effects::helpers::resolve_objects_for_effect(game, ctx, &self.target)
            .map_err(|_| ExecutionError::InvalidTarget)?;
        if targets.is_empty() {
            return Ok(EffectOutcome::target_invalid());
        }

        let mut turned = 0;
        for object_id in targets {
            let Some(object) = game.object(object_id) else {
                continue;
            };
            if !game.is_face_down(object_id) {
                continue;
            }
            let on_battlefield = object.zone == Zone::Battlefield;
            if !game.set_face_up(object_id) {
                continue;
            }
            turned += 1;
            if on_battlefield {
                let event_provenance = game.alloc_child_event_provenance(
                    ctx.provenance,
                    crate::events::EventKind::TurnedFaceUp,
                );
                game.queue_trigger_event(
                    ctx.provenance,
                    TriggerEvent::new_with_provenance(
                        crate::events::TurnedFaceUpEvent::new(object_id, ctx.controller),
                        event_provenance,
                    ),
                );
            }
        }

        Ok(EffectOutcome::count(turned))
    }

    fn get_target_spec(&self) -> Option<&crate::target::ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "card to turn face up"
    }
}
